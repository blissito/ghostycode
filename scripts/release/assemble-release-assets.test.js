#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync, spawnSync } = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const {
  allReleaseAssetNames,
  BUNDLE_ASSET_NAMES,
  BUNDLE_CHECKSUM_MANIFEST,
  CHECKSUM_MANIFEST,
  checksummedReleaseAssetNames,
} = require("../../npm/ghosty/scripts/artifacts");
const {
  assemble,
  parseChecksumManifest,
  verifyAssetDirectory,
  windowsLauncherContents,
} = require("./assemble-release-assets");

const repoRoot = path.resolve(__dirname, "..", "..");

function toCrlf(text) {
  return text.replace(/\r\n/g, "\n").replace(/\n/g, "\r\n");
}

function sha256(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function createBundleInputs(input) {
  fs.mkdirSync(input, { recursive: true });
  for (const name of allReleaseAssetNames().filter((asset) =>
    /^(ghosty|ghosty-tui)-(linux|android|macos|windows)-/.test(asset) &&
    !asset.endsWith(".tar.gz") &&
    !asset.endsWith(".zip"),
  )) {
    const artifactDirectory = path.join(input, name);
    fs.mkdirSync(artifactDirectory, { recursive: true });
    // GitHub's artifact transport normalizes regular files to 0644. The
    // bundler must restore executable modes for non-Windows archives.
    fs.writeFileSync(path.join(artifactDirectory, name), `fixture:${name}\n`, { mode: 0o644 });
  }
}

function runBundle(input, output, sourceDateEpoch) {
  const env = { ...process.env };
  if (sourceDateEpoch === undefined) {
    delete env.SOURCE_DATE_EPOCH;
  } else {
    env.SOURCE_DATE_EPOCH = sourceDateEpoch;
  }
  return spawnSync(
    "bash",
    [path.join(repoRoot, "scripts/release/create-release-bundles.sh"), input, output],
    { cwd: repoRoot, encoding: "utf8", env },
  );
}

function makeIntermediateArtifacts(root) {
  const generated = new Set(["ghosty.bat", CHECKSUM_MANIFEST]);
  const copied = allReleaseAssetNames().filter((name) => !generated.has(name));
  for (const name of copied) {
    if (name === BUNDLE_CHECKSUM_MANIFEST) {
      continue;
    }
    const artifactDirectory = BUNDLE_ASSET_NAMES.includes(name)
      ? path.join(root, "ghosty-bundles")
      : path.join(root, name);
    fs.mkdirSync(artifactDirectory, { recursive: true });
    fs.writeFileSync(path.join(artifactDirectory, name), `fixture:${name}\n`);
  }

  const bundleManifestDirectory = path.join(root, "ghosty-bundles");
  fs.mkdirSync(bundleManifestDirectory, { recursive: true });
  const rows = BUNDLE_ASSET_NAMES.map((name) => {
    const matches = fs
      .readdirSync(root, { recursive: true })
      .map((entry) => path.join(root, entry))
      .filter((entry) => path.basename(entry) === name && fs.statSync(entry).isFile());
    assert.equal(matches.length, 1, `fixture should contain one ${name}`);
    return `${sha256(matches[0])}  ${name}`;
  }).sort();
  fs.writeFileSync(
    path.join(bundleManifestDirectory, BUNDLE_CHECKSUM_MANIFEST),
    `${rows.join("\n")}\n`,
  );
}

test("authoritative release inventory contains seven targets and 34 bridge assets", () => {
  const assets = allReleaseAssetNames();
  assert.equal(assets.length, 34);
  assert.equal(checksummedReleaseAssetNames().length, 33);
  for (const required of [
    "ghosty-android-arm64",
    "ghosty-tui-android-arm64",
    "ghosty-windows-arm64.exe",
    "ghosty-tui-windows-arm64.exe",
    "ghosty-windows-arm64.zip",
    "ghosty-tui-android-arm64",
    "ghosty-tui-windows-arm64.exe",
    "GhostyCodeSetup.exe",
  ]) {
    assert.ok(assets.includes(required), `missing ${required}`);
  }
});

test("NSIS installer ships the Windows Terminal launcher and Start Menu shortcut", () => {
  const nsi = fs.readFileSync(path.join(repoRoot, "scripts/installer/ghosty.nsi"), "utf8");
  const bat = fs.readFileSync(path.join(repoRoot, "scripts/installer/ghosty.bat"), "utf8");

  assert.match(nsi, /^\s*File "ghosty\.bat"\s*$/m);
  assert.match(
    nsi,
    /CreateShortCut "\$SMPROGRAMS\\\$\{PRODUCT_NAME\}\\\$\{PRODUCT_NAME\}\.lnk" "\$INSTDIR\\bin\\ghosty\.bat"/,
  );
  assert.match(nsi, /^\s*Delete "\$INSTDIR\\bin\\ghosty\.bat"\s*$/m);
  assert.match(nsi, /^\s*Delete "\$SMPROGRAMS\\\$\{PRODUCT_NAME\}\\\$\{PRODUCT_NAME\}\.lnk"\s*$/m);
  assert.match(nsi, /^\s*RMDir "\$SMPROGRAMS\\\$\{PRODUCT_NAME\}"\s*$/m);

  const batNormalized = bat.replace(/\r\n/g, "\n");
  assert.match(batNormalized, /where wt >nul 2>nul/);
  assert.match(batNormalized, /wt --title Ghosty cmd \/k "%~dp0ghosty\.exe"/);
  assert.match(batNormalized, /"%~dp0ghosty\.exe"/);
  assert.doesNotMatch(batNormalized, /ghosty-windows-x64\.exe/);
  assert.ok(
    batNormalized.startsWith("@echo off\n"),
    "installer launcher must start with @echo off",
  );
  assert.match(
    windowsLauncherContents(),
    /\r\n/,
    "the GitHub/npm ghosty.bat asset must be generated with CRLF",
  );
});

test("assembly creates and verifies the exact release asset directory", async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ghosty-asset-assembly-"));
  const input = path.join(tempRoot, "input");
  const output = path.join(tempRoot, "output");
  try {
    fs.mkdirSync(input, { recursive: true });
    makeIntermediateArtifacts(input);
    await assemble(input, output);
    await assert.doesNotReject(() => verifyAssetDirectory(output));

    assert.deepEqual(
      fs.readdirSync(output).sort(),
      [...allReleaseAssetNames()].sort(),
    );
    assert.equal(
      fs.readFileSync(path.join(output, "ghosty.bat"), "utf8"),
      windowsLauncherContents(),
    );

    const checksums = parseChecksumManifest(
      fs.readFileSync(path.join(output, CHECKSUM_MANIFEST), "utf8"),
      CHECKSUM_MANIFEST,
    );
    assert.deepEqual([...checksums.keys()].sort(), [...checksummedReleaseAssetNames()].sort());
  } finally {
    fs.rmSync(tempRoot, { force: true, recursive: true });
  }
});

test("bundle helper emits reproducible timestamped tar and zip archives from paths with spaces", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ghosty-bundle-assembly-"));
  const input = path.join(tempRoot, "input artifacts with spaces");
  const output = path.join(tempRoot, "output bundles with spaces");
  const repeatedOutput = path.join(tempRoot, "output bundles repeated with spaces");
  const sourceDateEpoch = "1700000000";
  try {
    createBundleInputs(input);
    assert.equal(runBundle(input, output, sourceDateEpoch).status, 0);
    assert.equal(runBundle(input, repeatedOutput, sourceDateEpoch).status, 0);
    assert.deepEqual(
      fs.readdirSync(output).sort(),
      [...BUNDLE_ASSET_NAMES, BUNDLE_CHECKSUM_MANIFEST].sort(),
    );
    const checksums = parseChecksumManifest(
      fs.readFileSync(path.join(output, BUNDLE_CHECKSUM_MANIFEST), "utf8"),
      BUNDLE_CHECKSUM_MANIFEST,
    );
    for (const name of BUNDLE_ASSET_NAMES) {
      assert.equal(checksums.get(name), sha256(path.join(output, name)));
      assert.deepEqual(
        fs.readFileSync(path.join(output, name)),
        fs.readFileSync(path.join(repeatedOutput, name)),
        `${name} should be byte-reproducible for identical inputs`,
      );
    }
    assert.deepEqual(
      fs.readFileSync(path.join(output, BUNDLE_CHECKSUM_MANIFEST)),
      fs.readFileSync(path.join(repeatedOutput, BUNDLE_CHECKSUM_MANIFEST)),
      "bundle checksum manifest should be reproducible",
    );

    const linuxEntries = execFileSync(
      "tar",
      ["-tzf", path.join(output, "ghosty-linux-x64.tar.gz")],
      { encoding: "utf8" },
    ).trim().split("\n").sort();
    assert.deepEqual(linuxEntries, [
      "ghosty-linux-x64/",
      "ghosty-linux-x64/ghosty-tui",
      "ghosty-linux-x64/ghosty",
      "ghosty-linux-x64/install.sh",
    ]);
    const extracted = path.join(tempRoot, "tar extracted");
    fs.mkdirSync(extracted);
    execFileSync(
      "tar",
      ["-xzf", path.join(output, "ghosty-linux-x64.tar.gz"), "-C", extracted],
      { stdio: "pipe" },
    );
    for (const entry of ["ghosty", "ghosty-tui", "install.sh"]) {
      const mode = fs.statSync(path.join(extracted, "ghosty-linux-x64", entry)).mode & 0o777;
      assert.equal(mode, 0o755, `${entry} should remain executable after artifact transport`);
      assert.equal(
        Math.trunc(fs.statSync(path.join(extracted, "ghosty-linux-x64", entry)).mtimeMs / 1000),
        Number(sourceDateEpoch),
        `${entry} should retain the source commit timestamp`,
      );
    }
    const expectedLauncher = toCrlf(
      fs.readFileSync(path.join(repoRoot, "scripts/installer/ghosty.bat"), "utf8"),
    );
    const expectedInstallBat = toCrlf(
      fs.readFileSync(path.join(repoRoot, "scripts/release/install.bat"), "utf8"),
    );
    const zipListing = (name) =>
      execFileSync("unzip", ["-Z1", path.join(output, name)], { encoding: "utf8" })
        .trim()
        .split("\n")
        .sort();
    const zipFile = (archive, inner) =>
      execFileSync("unzip", ["-p", path.join(output, archive), inner]);

    assert.deepEqual(zipListing("ghosty-windows-x64.zip"), [
      "ghosty-windows-x64/",
      "ghosty-windows-x64/ghosty-tui.exe",
      "ghosty-windows-x64/ghosty.bat",
      "ghosty-windows-x64/ghosty.exe",
      "ghosty-windows-x64/install.bat",
    ]);
    assert.deepEqual(zipListing("ghosty-windows-arm64.zip"), [
      "ghosty-windows-arm64/",
      "ghosty-windows-arm64/ghosty-tui.exe",
      "ghosty-windows-arm64/ghosty.bat",
      "ghosty-windows-arm64/ghosty.exe",
      "ghosty-windows-arm64/install.bat",
    ]);
    const portableEntries = zipListing("ghosty-windows-arm64-portable.zip");
    assert.deepEqual(portableEntries, [
      "ghosty-windows-arm64-portable/",
      "ghosty-windows-arm64-portable/ghosty-tui.exe",
      "ghosty-windows-arm64-portable/ghosty.bat",
      "ghosty-windows-arm64-portable/ghosty.exe",
    ]);
    assert.deepEqual(zipListing("ghosty-windows-x64-portable.zip"), [
      "ghosty-windows-x64-portable/",
      "ghosty-windows-x64-portable/ghosty-tui.exe",
      "ghosty-windows-x64-portable/ghosty.bat",
      "ghosty-windows-x64-portable/ghosty.exe",
    ]);

    for (const [archive, prefix, includeInstall] of [
      ["ghosty-windows-x64.zip", "ghosty-windows-x64", true],
      ["ghosty-windows-arm64.zip", "ghosty-windows-arm64", true],
      ["ghosty-windows-x64-portable.zip", "ghosty-windows-x64-portable", false],
      ["ghosty-windows-arm64-portable.zip", "ghosty-windows-arm64-portable", false],
    ]) {
      const launcher = zipFile(archive, `${prefix}/ghosty.bat`);
      assert.deepEqual(
        launcher,
        Buffer.from(expectedLauncher, "utf8"),
        `${archive} must ship the NSIS launcher with CRLF`,
      );
      assert.match(launcher.toString("utf8"), /where wt >nul 2>nul/);
      assert.match(launcher.toString("utf8"), /ghosty\.exe/);
      assert.doesNotMatch(launcher.toString("utf8"), /ghosty-windows-x64\.exe/);
      if (includeInstall) {
        const installBat = zipFile(archive, `${prefix}/install.bat`);
        assert.deepEqual(
          installBat,
          Buffer.from(expectedInstallBat, "utf8"),
          `${archive} install.bat must be staged with CRLF`,
        );
        assert.match(installBat.toString("utf8"), /ghosty\.bat/);
      }
    }

    const zipExtracted = path.join(tempRoot, "zip extracted");
    fs.mkdirSync(zipExtracted);
    execFileSync(
      "unzip",
      ["-qq", path.join(output, "ghosty-windows-arm64-portable.zip"), "-d", zipExtracted],
      { env: { ...process.env, TZ: "UTC" }, stdio: "pipe" },
    );
    for (const entry of ["ghosty.exe", "ghosty-tui.exe", "ghosty.bat"]) {
      assert.equal(
        Math.trunc(
          fs.statSync(path.join(zipExtracted, "ghosty-windows-arm64-portable", entry)).mtimeMs / 1000,
        ),
        Number(sourceDateEpoch),
        `${entry} should retain the source commit timestamp`,
      );
    }
  } finally {
    fs.rmSync(tempRoot, { force: true, recursive: true });
  }
});

test("bundle helper rejects missing, malformed, and ZIP-unrepresentable release epochs", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ghosty-bundle-input-validation-"));
  const input = path.join(tempRoot, "input");
  try {
    createBundleInputs(input);

    const missingEpoch = runBundle(input, path.join(tempRoot, "missing-epoch"));
    assert.notEqual(missingEpoch.status, 0);
    assert.match(missingEpoch.stderr, /SOURCE_DATE_EPOCH is required/);

    const malformedEpoch = runBundle(input, path.join(tempRoot, "malformed-epoch"), "not-an-epoch");
    assert.notEqual(malformedEpoch.status, 0);
    assert.match(malformedEpoch.stderr, /SOURCE_DATE_EPOCH must be an integer Unix timestamp/);

    const preZipEpoch = runBundle(input, path.join(tempRoot, "pre-zip-epoch"), "0");
    assert.notEqual(preZipEpoch.status, 0);
    assert.match(preZipEpoch.stderr, /SOURCE_DATE_EPOCH must be between 315532800/);
  } finally {
    fs.rmSync(tempRoot, { force: true, recursive: true });
  }
});

test("bundle helper names a missing required release artifact", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ghosty-bundle-missing-artifact-"));
  const input = path.join(tempRoot, "input");
  try {
    createBundleInputs(input);
    fs.rmSync(path.join(input, "ghosty-tui-linux-x64", "ghosty-tui-linux-x64"));
    const result = runBundle(input, path.join(tempRoot, "output"), "1700000000");
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /missing required release artifact for linux-x64: .*ghosty-tui-linux-x64/);
  } finally {
    fs.rmSync(tempRoot, { force: true, recursive: true });
  }
});

test("verification rejects modified and unexpected assets", async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "ghosty-asset-tamper-"));
  const input = path.join(tempRoot, "input");
  const output = path.join(tempRoot, "output");
  try {
    fs.mkdirSync(input, { recursive: true });
    makeIntermediateArtifacts(input);
    await assemble(input, output);
    fs.appendFileSync(path.join(output, "ghosty-linux-x64"), "tampered\n");
    await assert.rejects(
      () => verifyAssetDirectory(output),
      /checksum mismatch for ghosty-linux-x64/,
    );

    fs.writeFileSync(path.join(output, "unexpected.txt"), "unexpected\n");
    await assert.rejects(
      () => verifyAssetDirectory(output),
      /unexpected: unexpected\.txt/,
    );
  } finally {
    fs.rmSync(tempRoot, { force: true, recursive: true });
  }
});
