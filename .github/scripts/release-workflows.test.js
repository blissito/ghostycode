#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const {
  allAssetNames,
  allReleaseAssetNames,
  BUNDLE_ASSET_NAMES,
  LEGACY_TUI_BRIDGE_ASSET_NAMES,
} = require(path.join(repoRoot, "npm", "ghosty", "scripts", "artifacts"));

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function valuesForKey(source, key) {
  const expression = new RegExp(`^\\s+${key}:\\s+([^#\\s]+)\\s*$`, "gm");
  return [...source.matchAll(expression)].map((match) => match[1]);
}

function namedStep(source, name) {
  const marker = `      - name: ${name}\n`;
  const start = source.indexOf(marker);
  assert.notEqual(start, -1, `missing workflow step: ${name}`);
  const next = source.indexOf("\n      - ", start + marker.length);
  return source.slice(start, next === -1 ? source.length : next);
}

const ci = read(".github/workflows/ci.yml");
const nightly = read(".github/workflows/nightly.yml");
const candidate = read(".github/workflows/release-candidate.yml");
const artifacts = read(".github/workflows/release-artifacts.yml");
const release = read(".github/workflows/release.yml");
const republish = read(".github/workflows/release-republish.yml");
const releaseDockerfile = read("packaging/docker/Dockerfile.release");
const cnb = read(".cnb.yml");
const bundles = read("scripts/release/create-release-bundles.sh");
const cliDispatcher = read("crates/cli/src/lib.rs");
const runbook = read("docs/RELEASE_RUNBOOK.md");

const ciTestJob = ci.slice(ci.indexOf("\n  test:\n"));
assert.match(
  ciTestJob,
  /matrix\.os == 'ubuntu-latest'.*github\.event_name == 'pull_request'/,
  "heavy pull requests must select the real Ubuntu test lane",
);
assert.match(
  ciTestJob,
  /cargo nextest run --workspace --all-features --locked --profile ci/,
  "the Ubuntu pull-request lane must run workspace nextest",
);
assert.match(
  ciTestJob,
  /name: Linux test location \(CNB\)/,
  "the non-PR CNB fallback must be named explicitly",
);

assert.match(ci, /^  workflow_dispatch:\n    inputs:\n      expected_sha:/m);
const manualForceBlock = ci.match(
  /if \[\[ "\$\{EVENT_NAME\}" == "workflow_dispatch" \]\]; then([\s\S]*?)\n\s+if \[\[ "\$\{EVENT_NAME\}" == "schedule" \]\]; then/,
);
assert.ok(manualForceBlock, "CI must have a dedicated manual-dispatch force-full branch");
for (const output of ["heavy", "workflow", "mobile", "actions"]) {
  assert.match(manualForceBlock[1], new RegExp(`echo "${output}=true"`));
}
assert.match(manualForceBlock[1], /#EXPECTED_SHA.*-ne 40/s);
assert.match(manualForceBlock[1], /actual.*EXPECTED_SHA/s);
const expectedNightlyTargets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-musl",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "aarch64-pc-windows-msvc",
].sort();
assert.deepEqual([...new Set(valuesForKey(nightly, "target"))].sort(), expectedNightlyTargets);
assert.deepEqual(
  [
    ...valuesForKey(nightly, "primary_artifact"),
    ...valuesForKey(nightly, "alias_artifact"),
  ].sort(),
  [
    "ghosty-linux-x64",
    "ghosty-tui-linux-x64",
    "ghosty-linux-arm64",
    "ghosty-tui-linux-arm64",
    "ghosty-macos-x64",
    "ghosty-tui-macos-x64",
    "ghosty-macos-arm64",
    "ghosty-tui-macos-arm64",
    "ghosty-windows-x64.exe",
    "ghosty-tui-windows-x64.exe",
    "ghosty-windows-arm64.exe",
    "ghosty-tui-windows-arm64.exe",
  ].sort(),
);
assert.match(
  nightly,
  /cargo build --release --locked --target \$\{\{ matrix\.target \}\} -p ghosty-cli/,
);
assert.match(nightly, /startsWith\(matrix\.target, 'x86_64-'\).*runner\.arch == 'X64'/s);
assert.match(nightly, /startsWith\(matrix\.target, 'aarch64-'\).*runner\.arch == 'ARM64'/s);
const nightlyArmMuslSetup = namedStep(nightly, "Install Linux ARM64 musl toolchain");
assert.match(nightlyArmMuslSetup, /matrix\.target == 'aarch64-unknown-linux-musl'/);
assert.match(nightlyArmMuslSetup, /apt-get install -y binutils musl-tools/);
assert.match(nightlyArmMuslSetup, /rustup target add --toolchain stable aarch64-unknown-linux-musl/);
const nightlyArmStaticSmoke = namedStep(
  nightly,
  "Verify static Linux ARM64 binary and launch",
);
assert.match(
  nightlyArmStaticSmoke,
  /matrix\.target == 'aarch64-unknown-linux-musl' && runner\.arch == 'ARM64'/,
);
assert.match(nightlyArmStaticSmoke, /readelf -l "\$\{bin_path\}"/);
assert.match(nightlyArmStaticSmoke, /grep -Fq 'INTERP'/);
assert.match(nightlyArmStaticSmoke, /"\$\{bin_path\}" --version/);
// The nightly alias artifact is named ghosty-tui-* on purpose: the rebase
// onto 0.9.11 kept the legacy binary name as a byte-identical alias for
// old consumers (it was codew-* upstream). What must not happen is the
// nightly compiling a separate ghosty-tui binary, which the next line guards.
assert.doesNotMatch(nightly, /target\/[^\n]*\/ghosty-tui(?:\.exe)?/);
assert.match(nightly, /cp "\$\{bin_path\}" "\$\{dir\}\/\$\{artifact\}"/);
assert.match(nightly, /cmp -s[\s\S]*nightly-primary[\s\S]*nightly-alias/);
assert.equal((nightly.match(/retention-days: 14/g) || []).length, 2);

assert.match(candidate, /^  workflow_dispatch:\n    inputs:\n      expected_sha:/m);
assert.doesNotMatch(candidate, /^  (push|pull_request|schedule):/m);
assert.match(candidate, /uses: \.\/\.github\/workflows\/release-artifacts\.yml/);
assert.match(candidate, /source_sha: \$\{\{ needs\.resolve\.outputs\.sha \}\}/);
assert.match(candidate, /^  web:\n/m);
assert.doesNotMatch(
  candidate,
  /ref: \$\{\{ needs\.resolve\.outputs\.sha \}\}/,
  "candidate jobs must checkout GITHUB_SHA, not interpolate the dispatch SHA into ref",
);
assert.match(candidate, /cache-dependency-path: web\/package-lock\.json/);
assert.match(candidate, /package-manager-cache: false/);
assert.match(candidate, /working-directory: web/);
for (const command of [
  "npm ci",
  "npm run check:facts",
  "npm run prebuild",
  "npm run check:docs",
  "npm test",
  "npm run lint",
  "npx tsc --noEmit",
  "npm run build",
]) {
  assert.match(candidate, new RegExp(`run: ${command.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`));
}
assert.match(candidate, /^    needs: \[resolve, web\]$/m);
assert.match(candidate, /needs\.web\.result == 'success'/);

for (const [label, workflow] of [
  ["release candidate", candidate],
  ["shared artifact", artifacts],
]) {
  for (const forbidden of [
    /contents:\s*write/,
    /packages:\s*write/,
    /softprops\/action-gh-release/,
    /docker\/login-action/,
    /docker\/build-push-action/,
    /\bgh release\b/,
    /\bnpm publish\b/,
    /\bcargo publish\b/,
    /\bgit push\b/,
  ]) {
    assert.doesNotMatch(workflow, forbidden, `${label} workflow contains publication capability`);
  }
}

for (const [label, workflow] of [
  ["release candidate", candidate],
  ["shared artifact", artifacts],
  ["public release", release],
  ["release republish", republish],
]) {
  const remoteActions = [...workflow.matchAll(/^\s+(?:-\s+)?uses:\s+([^@\s]+)@([^#\s]+)/gm)]
    .map((match) => ({ action: match[1], ref: match[2] }))
    .filter(({ action }) => !action.startsWith("./"));
  assert.ok(remoteActions.length > 0, `${label} workflow must exercise pinned actions`);
  for (const { action, ref } of remoteActions) {
    assert.match(
      ref,
      /^[0-9a-f]{40}$/,
      `${label} action ${action} must use an audited full commit SHA`,
    );
  }
}

const republishHomebrewJob = republish.match(/\n  homebrew:\n([\s\S]*)$/);
assert.ok(republishHomebrewJob, "republish must retain a Homebrew recovery job");
const republishHomebrewCheckout = namedStep(
  republishHomebrewJob[0],
  "Checkout release infrastructure",
);
assert.match(
  republishHomebrewCheckout,
  /ref: \$\{\{ github\.event\.repository\.default_branch \}\}/,
  "Homebrew recovery must use the repaired default-branch infrastructure",
);
assert.doesNotMatch(
  republishHomebrewCheckout,
  /needs\.resolve\.outputs\.sha/,
  "Homebrew recovery must not resurrect release-tag infrastructure",
);
assert.match(republishHomebrewJob[0], /gh release download "\$\{\{ needs\.resolve\.outputs\.tag \}\}"/);
assert.match(republishHomebrewJob[0], /MANIFEST: \/tmp\/ghosty-artifacts-sha256\.txt/);

assert.match(artifacts, /^  workflow_call:/m);
assert.match(artifacts, /^permissions:\n  contents: read$/m);
const expectedTargets = [
  "x86_64-unknown-linux-musl",
  "aarch64-unknown-linux-musl",
  "aarch64-linux-android",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "aarch64-pc-windows-msvc",
].sort();
assert.deepEqual([...new Set(valuesForKey(artifacts, "target"))].sort(), expectedTargets);

const releaseMuslBuild = namedStep(artifacts, "Build static Linux binaries (musl)");
assert.match(releaseMuslBuild, /endsWith\(matrix\.target, '-unknown-linux-musl'\)/);
assert.match(releaseMuslBuild, /apt-get install -y binutils musl-tools/);
assert.match(releaseMuslBuild, /rustup target add --toolchain stable \$\{\{ matrix\.target \}\}/);
assert.match(
  releaseMuslBuild,
  /cargo build --profile dist --locked --target \$\{\{ matrix\.target \}\} -p ghosty-cli/,
);
const releaseStaticSmoke = namedStep(
  artifacts,
  "Verify static Linux binaries and launch on matching native runners",
);
assert.match(releaseStaticSmoke, /endsWith\(matrix\.target, '-unknown-linux-musl'\)/);
assert.match(
  releaseStaticSmoke,
  /startsWith\(matrix\.target, 'aarch64-'\) && runner\.arch == 'ARM64'/,
);
assert.match(releaseStaticSmoke, /readelf -l "\$\{bin_path\}"/);
assert.match(releaseStaticSmoke, /grep -Fq 'INTERP'/);
assert.match(releaseStaticSmoke, /"\$\{bin_path\}" --version/);

const builtAssetNames = [
  ...valuesForKey(artifacts, "cli_artifact"),
  ...valuesForKey(artifacts, "shim_artifact"),
  ...valuesForKey(artifacts, "compat_tui_artifact"),
];
assert.equal(builtAssetNames.length, 21);
assert.deepEqual(
  [...new Set(builtAssetNames)].sort(),
  [
    ...allAssetNames().filter((name) => name !== "ghosty.bat"),
    ...LEGACY_TUI_BRIDGE_ASSET_NAMES,
  ].sort(),
);
assert.match(
  artifacts,
  /stage_binary "\$\{\{ matrix\.cli_binary \}\}" "\$\{\{ matrix\.compat_tui_artifact \}\}"/,
  "legacy TUI bridge assets must be staged from the one compiled ghosty binary",
);
const bundleInvocations = [...bundles.matchAll(
  /^bundle (\S+) \\\n\s+\S+ \S+ (tar\.gz|zip) (""|portable)$/gm,
)].map((match) => {
  const variant = match[3] === "portable" ? "-portable" : "";
  return `ghosty-${match[1]}${variant}.${match[2]}`;
});
assert.deepEqual(bundleInvocations.sort(), [...BUNDLE_ASSET_NAMES].sort());
assert.match(artifacts, /aarch64-pc-windows-msvc/);
assert.match(artifacts, /aarch64-linux-android/);
assert.match(artifacts, /ghosty-tui-windows-arm64\.exe/);
assert.match(artifacts, /GhostyCodeSetup\.exe/);
assert.match(artifacts, /assemble-release-assets\.js --verify release-assets/);
assert.match(artifacts, /GHOSTY_SMOKE_ASSETS_DIR/);
assert.match(artifacts, /^  pin:\n/m);
assert.match(artifacts, /Require source_sha equals github\.sha/);
assert.doesNotMatch(
  artifacts,
  /ref: \$\{\{ inputs\.source_sha \}\}/,
  "artifact jobs must checkout GITHUB_SHA, not interpolate the caller SHA into ref",
);
assert.match(artifacts, /prefix-key: v1-\$\{\{ runner\.os \}\}-\$\{\{ runner\.arch \}\}-stable/);
assert.equal(
  (artifacts.match(/package-manager-cache: false/g) || []).length,
  2,
  "assemble and smoke must disable setup-node's implicit npm cache",
);
const bundleStep = namedStep(artifacts, "Create and checksum platform archives");
assert.match(bundleStep, /SOURCE_SHA: \$\{\{ github\.sha \}\}/);
assert.match(bundleStep, /git show -s --format=%ct "\$\{SOURCE_SHA\}"/);
assert.match(
  bundleStep,
  /SOURCE_DATE_EPOCH="\$\{source_date_epoch\}"[\s\\]+bash scripts\/release\/create-release-bundles\.sh artifacts bundles/,
);
assert.doesNotMatch(bundleStep, /inputs\.source_sha/);
assert.doesNotMatch(bundleStep, /\bdate\b/, "bundle timestamps must come from the pinned source commit, not wall-clock time");

const rustCacheBlocks = [...artifacts.matchAll(/uses: Swatinem\/rust-cache@[\s\S]*?(?=\n      - )/g)].map(
  (match) => match[0],
);
assert.ok(rustCacheBlocks.length >= 1, "shared artifact workflow must pin rust-cache");
for (const block of rustCacheBlocks) {
  assert.doesNotMatch(block, /github\.(event|ref|sha)|inputs\./);
}

const parity = release.match(/\n  parity:\n([\s\S]*?)\n  artifacts:\n/);
assert.ok(parity, "public release must retain a parity job");
assert.doesNotMatch(
  parity[1],
  /ref: \$\{\{ needs\.resolve\.outputs\.sha \}\}/,
  "parity must checkout GITHUB_SHA after resolve, not interpolate the tag SHA into ref",
);
assert.match(parity[1], /prefix-key: v1-\$\{\{ runner\.os \}\}-\$\{\{ runner\.arch \}\}-stable/);
const parityRustCache = [...parity[1].matchAll(/uses: Swatinem\/rust-cache@[\s\S]*?(?=\n      - )/g)].map(
  (match) => match[0],
);
assert.equal(parityRustCache.length, 1, "parity must pin exactly one rust-cache");
assert.doesNotMatch(parityRustCache[0], /github\.(event|ref|sha)|inputs\./);

// 27, not upstream's 34: GhostyCode ships two asset families (ghosty-* and the
// byte-identical ghosty-tui-* compat copies), not three. See 75fe13c2a.
assert.equal(allReleaseAssetNames().length, 27);
assert.match(release, /^  artifacts:\n/m);
assert.match(release, /uses: \.\/\.github\/workflows\/release-artifacts\.yml/);
assert.doesNotMatch(release, /^  (build|bundle|windows-installer):/m);
assert.match(release, /name: ghosty-release-assets\n\s+path: artifacts/);
assert.match(release, /files: artifacts\/\*/);
assert.equal(
  (release.match(/ensure-release-assets-absent\.js/g) || []).length,
  2,
  "public release must refuse existing assets before work and immediately before upload",
);
assert.match(release, /overwrite_files:\s*false/);
assert.match(release, /fail_on_unmatched_files:\s*true/);

assert.match(release, /^  docker-build:\n/m);
assert.match(release, /^  docker:\n/m);
assert.match(release, /runner: ubuntu-latest\n\s+platform: linux\/amd64/);
assert.match(release, /runner: ubuntu-24\.04-arm\n\s+platform: linux\/arm64/);
assert.match(release, /cli_artifact: ghosty-linux-x64/);
assert.match(release, /cli_artifact: ghosty-linux-arm64/);
assert.match(release, /shim_artifact: ghosty-tui-linux-x64/);
assert.match(release, /shim_artifact: ghosty-tui-linux-arm64/);
assert.doesNotMatch(
  release,
  /docker\/setup-qemu-action/,
  "public container publication must not funnel both architectures through QEMU",
);
const releaseDockerBytes = namedStep(release, "Verify native release bytes");
assert.match(releaseDockerBytes, /CLI_ARTIFACT: \$\{\{ matrix\.cli_artifact \}\}/);
assert.match(releaseDockerBytes, /SHIM_ARTIFACT: \$\{\{ matrix\.shim_artifact \}\}/);
assert.match(
  releaseDockerBytes,
  /mv -- "docker-context\/bin\/\$\{CLI_ARTIFACT\}" docker-context\/bin\/ghosty/,
);
assert.match(
  releaseDockerBytes,
  /mv -- "docker-context\/bin\/\$\{SHIM_ARTIFACT\}" docker-context\/bin\/ghosty-tui/,
);
assert.match(releaseDockerBytes, /cmp docker-context\/bin\/ghosty docker-context\/bin\/ghosty-tui/);
const releaseDockerBuild = namedStep(release, "Assemble and push native image by digest");
assert.match(releaseDockerBuild, /context: docker-context/);
assert.match(releaseDockerBuild, /file: infra\/packaging\/docker\/Dockerfile\.release/);
assert.match(releaseDockerBuild, /platforms: \$\{\{ matrix\.platform \}\}/);
assert.match(releaseDockerBuild, /provenance: mode=max/);
assert.match(releaseDockerBuild, /sbom: true/);
assert.match(releaseDockerBuild, /push-by-digest=true/);
const releaseDockerManifest = namedStep(release, "Publish multi-architecture manifest");
assert.match(releaseDockerManifest, /Expected exactly two native image digests/);
assert.match(releaseDockerManifest, /docker buildx imagetools create/);
const releaseDockerSmoke = namedStep(release, "Verify and smoke published container");
assert.match(releaseDockerSmoke, /linux\/amd64/);
assert.match(releaseDockerSmoke, /linux\/arm64/);
assert.match(releaseDockerSmoke, /--entrypoint ghosty/);
assert.match(releaseDockerSmoke, /--entrypoint ghosty-tui/);

// No npm job assertions: the rebase onto 0.9.11 (ca88985fb) dropped the
// trusted-publishing job on purpose. Ghosty publishes the npm wrapper by
// hand after the GitHub release; see docs/RELEASE_RUNBOOK.md.

assert.match(releaseDockerfile, /^FROM debian:bookworm-slim$/m);
assert.match(releaseDockerfile, /ca-certificates/);
assert.match(releaseDockerfile, /libdbus-1-3/);
assert.match(releaseDockerfile, /COPY .*bin\/ghosty \/usr\/local\/bin\/ghosty/);
assert.match(releaseDockerfile, /COPY .*bin\/ghosty-tui \/usr\/local\/bin\/ghosty-tui/);
assert.match(releaseDockerfile, /^USER ghosty$/m);
assert.doesNotMatch(
  releaseDockerfile,
  /\bcargo\s+build\b|^FROM\s+rust:/m,
  "release container assembly must reuse the already-verified release binaries",
);

assert.match(runbook, /release[- ]candidate/i);
assert.match(runbook, /expected_sha/);
assert.match(runbook, /27/);
assert.match(runbook, /does not create a tag/i);
assert.match(runbook, /explicit.*approval/i);
assert.match(runbook, /last[- ]useful[- ]log/i, "runbook must document the last-useful-log rule (#5496)");
assert.match(runbook, /404 logs/i, "runbook must document the 404-log cancellation rule (#5496)");

const cnbRustGates = cnb.match(
  /\.rust_workspace_gates_stage: &rust_workspace_gates_stage([\s\S]*?)\n\.linux_rust_gates:/,
);
assert.ok(cnbRustGates, "CNB must retain the shared Rust workspace gate");
assert.match(
  cnbRustGates[1],
  /timeout: 45m[\s\S]*export CARGO_BUILD_JOBS=1[\s\S]*export CARGO_PROFILE_TEST_DEBUG=0[\s\S]*cargo check --workspace --all-targets --locked[\s\S]*cargo clippy --workspace --all-targets --all-features --locked -- -D warnings[\s\S]*RUST_MIN_STACK=16777216 cargo test --workspace --all-features --locked/,
  "CNB must serialize the memory-heavy Rust gate and preserve the workspace test stack contract",
);
assert.match(
  cnbRustGates[1],
  /export HOME="\$\{hermetic_home\}"[\s\S]*export GHOSTY_HOME="\$\{hermetic_home\}\/\.ghosty"[\s\S]*unset GHOSTY_CONFIG_PATH DEEPSEEK_CONFIG_PATH DEEPSEEK_HOME/,
  "CNB workspace tests must not read a populated runner ~/.ghosty (#5355)",
);

const nextest = read(".config/nextest.toml");
const integrationGroup = nextest.search(/^filter = 'binary\(integration\)'$/m);
const telemetryGroup = nextest.indexOf(
  "filter = 'binary(integration) & test(/^telemetry_contract::/)'",
);
const execGroup = nextest.indexOf(
  "filter = 'binary(integration) & test(/^exec_persistent_service::/)'",
);
assert.ok(integrationGroup >= 0, "nextest must bound the integration binary");
assert.ok(
  telemetryGroup >= 0 && telemetryGroup < integrationGroup,
  "telemetry-contract override must precede binary(integration); first matching group wins",
);
assert.ok(
  execGroup >= 0 && execGroup < integrationGroup,
  "exec_persistent_service override must precede binary(integration); first matching group wins",
);
assert.match(nextest, /exec-persistent-service = \{ max-threads = 1 \}/);
assert.equal(
  (cnb.match(/^\s+- \*rust_workspace_gates_stage$/gm) || []).length,
  2,
  "both CNB Rust pipelines must reuse the constrained workspace gate",
);

const cnbPreflight = cnb.match(
  /\.linux_release_preflight: &linux_release_preflight([\s\S]*?)\nmain:/,
);
assert.ok(cnbPreflight, "CNB must retain a dedicated release preflight");
const cnbBuild = cnbPreflight[1].indexOf(
  "cargo build --jobs 2 --release --locked -p ghosty-cli",
);
const cnbAlias = cnbPreflight[1].indexOf(
  "cp target/release/ghosty target/release/ghosty-tui",
);
const cnbSmoke = cnbPreflight[1].indexOf("node scripts/release/npm-wrapper-smoke.js");
assert.ok(cnbBuild >= 0, "CNB release preflight must build the consolidated runtime");
assert.ok(cnbAlias > cnbBuild, "CNB release preflight must materialize ghosty-tui after the build");
assert.ok(cnbSmoke > cnbAlias, "CNB release preflight must materialize ghosty-tui before smoke");

const cnbTagRelease = cnb.match(/\$:\n  tag_push:\n([\s\S]*)$/);
assert.ok(cnbTagRelease, "CNB must retain a tag release pipeline");
const cnbTagStamp = cnbTagRelease[1].indexOf(
  'export GHOSTY_BUILD_SHA="$commit_sha"',
);
const cnbTagBuild = cnbTagRelease[1].indexOf(
  "cargo build --jobs 2 --release --locked \\",
);
assert.match(cnbTagRelease[1], /checkout_sha="\$\(git rev-parse 'HEAD\^\{commit\}'\)"/);
assert.match(cnbTagRelease[1], /commit_sha="\$\{CNB_COMMIT:-\$\{checkout_sha\}\}"/);
assert.match(cnbTagRelease[1], /CNB_COMMIT[\s\S]*does not match checkout[\s\S]*exit 1/);
assert.ok(cnbTagStamp >= 0, "CNB tag releases must stamp the consolidated runtime");
assert.ok(cnbTagBuild > cnbTagStamp, "CNB tag releases must stamp before compiling");

// The three assertions that used to live here read scripts/release/install.sh,
// the per-bundle archive installer. 5551d9df3 ("install.sh: una sola fuente")
// deleted that copy on purpose — scripts/install.sh is the single live
// installer — but left this file reading the deleted path, so every run since
// died on ENOENT before reaching the checks below.
assert.doesNotMatch(
  cliDispatcher,
  /ghosty_config::auto_model::classify/,
  "the CLI dispatcher must leave auto routing to the provider-aware runtime",
);

// #5496: every release-lane job carries an explicit `timeout-minutes`.
//
// GitHub's default is 360 minutes, so an assigned-but-dead runner sits for six
// hours before anything reclaims it — observed on the v0.9.9 train as a job
// stuck `in_progress` with 404 logs. Timeouts are containment, not recovery:
// the runbook keeps the 404-log cancel/rerun rule for infrastructure failures.
//
// A job that calls a reusable workflow (`uses:`) cannot carry the key at all —
// GitHub rejects it — so the callee owns its own caps. That is why the artifact
// bounds live in release-artifacts.yml rather than in its callers.
function jobsWithoutTimeout(source) {
  const lines = source.split("\n");
  const jobsAt = lines.findIndex((line) => /^jobs:\s*$/.test(line));
  assert.notEqual(jobsAt, -1, "workflow must declare jobs");
  const offenders = [];
  for (let i = jobsAt + 1; i < lines.length; i += 1) {
    const header = lines[i].match(/^  ([A-Za-z0-9_-]+):\s*$/);
    if (!header) continue;
    let reusable = false;
    let capped = false;
    for (let j = i + 1; j < lines.length; j += 1) {
      if (/^  [A-Za-z0-9_-]+:\s*$/.test(lines[j])) break;
      if (/^    uses:/.test(lines[j])) reusable = true;
      if (/^    timeout-minutes:\s*\d+\s*$/.test(lines[j])) capped = true;
    }
    if (!reusable && !capped) offenders.push(header[1]);
  }
  return offenders;
}

assert.deepEqual(
  jobsWithoutTimeout("jobs:\n  uncapped:\n    runs-on: ubuntu-latest\n"),
  ["uncapped"],
  "jobsWithoutTimeout must detect an uncapped job",
);
assert.deepEqual(
  jobsWithoutTimeout("jobs:\n  reusable:\n    uses: ./.github/workflows/reusable.yml\n"),
  [],
  "jobsWithoutTimeout must skip reusable workflow callers",
);
assert.deepEqual(
  jobsWithoutTimeout("jobs:\n  capped:\n    runs-on: ubuntu-latest\n    timeout-minutes: 15\n"),
  [],
  "jobsWithoutTimeout must accept a capped job",
);

for (const [name, source] of [
  ["release-candidate.yml", candidate],
  ["release-artifacts.yml", artifacts],
  ["release.yml", release],
  ["release-republish.yml", republish],
  ["ci.yml", ci],
  ["nightly.yml", nightly],
]) {
  assert.deepEqual(
    jobsWithoutTimeout(source),
    [],
    `${name}: every job must set timeout-minutes (#5496)`,
  );
}

// The Windows artifact build historically runs 40-45 minutes, so its cap has to
// keep real margin — a tight bound here fails healthy releases.
const buildTimeout = artifacts.match(/^  build:\n(?:.*\n)*?    timeout-minutes: (\d+)$/m);
assert.ok(buildTimeout, "release-artifacts build job must be capped");
assert.ok(
  Number(buildTimeout[1]) >= 60,
  `artifact build cap ${buildTimeout[1]}m leaves no margin over a healthy 40-45m Windows build`,
);

function jobTimeout(source, job) {
  const match = source.match(
    new RegExp(`^  ${job}:\\n(?:.*\\n)*?    timeout-minutes: (\\d+)$`, "m"),
  );
  assert.ok(match, `${job} must declare timeout-minutes`);
  return Number(match[1]);
}

// Pin the measured release-lane budget: fast setup and packaging fail quickly,
// while cross-platform compilation keeps real margin over the 40-45m Windows
// build observed on the release train.
assert.equal(jobTimeout(candidate, "resolve"), 10);
assert.equal(jobTimeout(candidate, "web"), 15);
assert.equal(jobTimeout(artifacts, "pin"), 10);
assert.equal(jobTimeout(artifacts, "build"), 90);
for (const job of ["bundle", "windows-installer", "assemble", "smoke"]) {
  assert.equal(jobTimeout(artifacts, job), 15, `${job} must keep the 15m packaging cap`);
}
assert.equal(jobTimeout(nightly, "build"), 90);
assert.equal(jobTimeout(release, "resolve"), 10);
assert.equal(jobTimeout(release, "parity"), 20);

console.log(
  "Workflow contracts OK: 6-target/12-asset single-runtime nightly and exact-head 7-target/27-asset release candidate.",
);
