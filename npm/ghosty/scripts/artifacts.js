const path = require("path");
const os = require("os");

const CHECKSUM_MANIFEST = "ghosty-artifacts-sha256.txt";
const BUNDLE_CHECKSUM_MANIFEST = "ghosty-bundles-sha256.txt";
const WINDOWS_INSTALLER_ASSET = "GhostyCodeSetup.exe";

// Compatibility bridge introduced in v0.9.5: already-shipped v0.9.4 clients
// require these names before
// they will advertise or install a newer release. They contain byte-identical
// copies of the consolidated `ghosty` runtime; current installers do not
// expose them as a third command.
// GhostyCode publica UN solo comando: `ghosty`. El pipeline de release sigue
// emitiendo `ghosty-tui-*` como copias byte-idénticas (compatibilidad con
// instalaciones previas y con el updater viejo); forman parte del inventario
// del GitHub Release pero el instalador de npm no las descarga (ASSET_MATRIX).
const LEGACY_TUI_BRIDGE_ASSET_NAMES = [
  "ghosty-tui-linux-x64",
  "ghosty-tui-linux-arm64",
  "ghosty-tui-android-arm64",
  "ghosty-tui-macos-x64",
  "ghosty-tui-macos-arm64",
  "ghosty-tui-windows-x64.exe",
  "ghosty-tui-windows-arm64.exe",
];

const CNB_BINARY_ASSET_NAMES = ["ghosty-linux-x64"];
const CNB_RELEASE_ASSET_NAMES = [
  ...CNB_BINARY_ASSET_NAMES,
  CHECKSUM_MANIFEST,
];

const BUNDLE_ASSET_NAMES = [
  "ghosty-linux-x64.tar.gz",
  "ghosty-linux-arm64.tar.gz",
  "ghosty-android-arm64.tar.gz",
  "ghosty-macos-x64.tar.gz",
  "ghosty-macos-arm64.tar.gz",
  "ghosty-windows-x64.zip",
  "ghosty-windows-x64-portable.zip",
  "ghosty-windows-arm64.zip",
  "ghosty-windows-arm64-portable.zip",
];

const ASSET_MATRIX = {
  linux: {
    x64: ["ghosty-linux-x64"],
    arm64: ["ghosty-linux-arm64"],
  },
  android: {
    arm64: ["ghosty-android-arm64"],
  },
  darwin: {
    x64: ["ghosty-macos-x64"],
    arm64: ["ghosty-macos-arm64"],
  },
  win32: {
    x64: ["ghosty-windows-x64.exe", "ghosty.bat"],
    arm64: ["ghosty-windows-arm64.exe"],
  },
};

// HarmonyPC (openharmony) is an x86_64 Linux-compatible environment; map it to
// the linux binary family so npm install succeeds without a separate build target.
const PLATFORM_ALIASES = {
  openharmony: "linux",
};

function detectBinaryNames() {
  const rawPlatform = os.platform();
  const platform = PLATFORM_ALIASES[rawPlatform] || rawPlatform;
  const arch = os.arch();
  const defaults = ASSET_MATRIX[platform];
  if (!defaults) {
    const supported = Object.keys(ASSET_MATRIX).map(p => `'${p}'`).join(', ');
    throw new Error(
      `Unsupported platform: ${rawPlatform}. Supported platforms: ${supported}.\n\n` +
      unsupportedBuildHint(),
    );
  }
  const pair = defaults[arch];
  if (!pair) {
    const supported = Object.keys(defaults).map(a => `'${a}'`).join(', ');
    const hint = platform === "linux" && arch === "riscv64" ? unsupportedRiscvHint() : unsupportedBuildHint();
    throw new Error(
      `Unsupported architecture: ${arch} on platform ${platform}. ` +
      `Supported architectures: ${supported}.\n\n` +
      hint,
    );
  }
  // Un solo binario por plataforma: el runtime está consolidado en `ghosty`.
  // Windows añade `ghosty.bat` como lanzador, no como segundo comando.
  return {
    platform,
    arch,
    ghosty: pair[0],
  };
}

function unsupportedBuildHint() {
  return [
    "No prebuilt binary is available for this platform/architecture combo.",
    "You can still run ghosty by building from source with Cargo (single binary):",
    "",
    "  # Requires Rust 1.88+ (https://rustup.rs)",
    "  cargo install ghosty-cli --locked   # provides `ghosty`",
    "",
    "Or build from a checkout:",
    "",
    "  git clone https://github.com/blissito/ghostycode.git",
    "  cd GhostyCode",
    "  cargo install --path crates/cli --locked   # single binary",
    "",
    "See https://github.com/blissito/ghostycode/blob/main/docs/INSTALL.md",
    "for cross-compilation, mirror, Linux ARM64, FreeBSD, and winget specifics.",
  ].join("\n");
}

function unsupportedRiscvHint() {
  return [
    "Linux riscv64 prebuilt binaries are temporarily unavailable.",
    "GhostyCode currently depends on rquickjs-sys, which does not ship",
    "riscv64gc-unknown-linux-gnu bindings in the locked dependency set.",
    "",
    "Track the release notes and docs/INSTALL.md for the next RISC-V support update.",
  ].join("\n");
}

function executableName(base, platform) {
  return platform === "win32" ? `${base}.exe` : base;
}

function ensureTrailingSlash(baseUrl) {
  const trimmed = String(baseUrl).trim();
  return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
}

function githubReleaseBaseUrl(version, repo = "blissito/ghostycode") {
  return `https://github.com/${repo}/releases/download/v${version}/`;
}

function cnbReleaseBaseUrl(version) {
  return `https://cnb.cool/ghosty.net/ghosty/-/releases/download/v${version}/`;
}

function releaseAssetUrlFromBase(baseName, baseUrl) {
  return new URL(baseName, ensureTrailingSlash(baseUrl)).toString();
}

function explicitReleaseBase(env = process.env) {
  const candidates = [
    env.GHOSTY_RELEASE_BASE_URL,
    env.DEEPSEEK_TUI_RELEASE_BASE_URL,
    env.DEEPSEEK_RELEASE_BASE_URL,
  ];
  for (const candidate of candidates) {
    const override = String(candidate || "").trim();
    if (override) {
      return ensureTrailingSlash(override);
    }
  }
  return "";
}

function hasExplicitReleaseBase(env = process.env) {
  return Boolean(explicitReleaseBase(env));
}

function isCnbSupportedTarget(
  rawPlatform = os.platform(),
  arch = os.arch(),
) {
  const platform = PLATFORM_ALIASES[rawPlatform] || rawPlatform;
  return platform === "linux" && arch === "x64";
}

function releaseBaseUrl(version, repo = "blissito/ghostycode") {
  // GHOSTY_RELEASE_BASE_URL is the canonical override.
  // DEEPSEEK_TUI_RELEASE_BASE_URL / DEEPSEEK_RELEASE_BASE_URL are legacy aliases.
  const override = explicitReleaseBase();
  if (override) {
    return override;
  }
  // When GHOSTY_USE_CNB_MIRROR is set, use the CNB (China-friendly)
  // mirror that already builds and publishes binary release assets.
  if (usesCnbMirror()) {
    assertCnbMirrorSupportedPlatform();
    return cnbReleaseBaseUrl(version);
  }
  return githubReleaseBaseUrl(version, repo);
}

function usesCnbMirror(env = process.env) {
  return !hasExplicitReleaseBase(env) && env.GHOSTY_USE_CNB_MIRROR === "1";
}

function shouldRaceFirstPartyMirrors(
  env = process.env,
  rawPlatform = os.platform(),
  arch = os.arch(),
) {
  return (
    isCnbSupportedTarget(rawPlatform, arch) &&
    !hasExplicitReleaseBase(env) &&
    env.GHOSTY_USE_CNB_MIRROR !== "1"
  );
}

function firstPartyReleaseSources(version, repo = "blissito/ghostycode") {
  return [
    {
      id: "github",
      label: "GitHub Releases",
      baseUrl: githubReleaseBaseUrl(version, repo),
    },
    {
      id: "cnb",
      label: "CNB first-party mirror",
      baseUrl: cnbReleaseBaseUrl(version),
    },
  ];
}

function assertCnbMirrorSupportedPlatform(
  rawPlatform = os.platform(),
  arch = os.arch(),
) {
  if (isCnbSupportedTarget(rawPlatform, arch)) {
    return;
  }
  throw new Error(
    "GHOSTY_USE_CNB_MIRROR=1 currently supports only Linux x64 " +
      `(including OpenHarmony x64); detected ${rawPlatform} ${arch}. ` +
      "Use the GitHub Release or set GHOSTY_RELEASE_BASE_URL to a " +
      "complete mirror for this platform.",
  );
}

function releaseAssetUrl(baseName, version, repo = "blissito/ghostycode") {
  return releaseAssetUrlFromBase(baseName, releaseBaseUrl(version, repo));
}

function checksumManifestUrl(version, repo = "blissito/ghostycode") {
  return releaseAssetUrl(CHECKSUM_MANIFEST, version, repo);
}

function releaseBinaryDirectory() {
  return path.join(__dirname, "..", "bin", "downloads");
}

function allAssetNames() {
  const names = [];
  for (const platformAssets of Object.values(ASSET_MATRIX)) {
    for (const assets of Object.values(platformAssets)) {
      names.push(...assets);
    }
  }
  return Array.from(new Set(names));
}

function allReleaseAssetNames() {
  return [
    ...allAssetNames(),
    ...LEGACY_TUI_BRIDGE_ASSET_NAMES,
    ...BUNDLE_ASSET_NAMES,
    WINDOWS_INSTALLER_ASSET,
    BUNDLE_CHECKSUM_MANIFEST,
    CHECKSUM_MANIFEST,
  ];
}

function checksummedReleaseAssetNames() {
  return allReleaseAssetNames().filter((name) => name !== CHECKSUM_MANIFEST);
}

module.exports = {
  allAssetNames,
  allReleaseAssetNames,
  assertCnbMirrorSupportedPlatform,
  BUNDLE_ASSET_NAMES,
  BUNDLE_CHECKSUM_MANIFEST,
  CHECKSUM_MANIFEST,
  checksummedReleaseAssetNames,
  CNB_BINARY_ASSET_NAMES,
  CNB_RELEASE_ASSET_NAMES,
  checksumManifestUrl,
  cnbReleaseBaseUrl,
  detectBinaryNames,
  executableName,
  explicitReleaseBase,
  firstPartyReleaseSources,
  githubReleaseBaseUrl,
  hasExplicitReleaseBase,
  isCnbSupportedTarget,
  LEGACY_TUI_BRIDGE_ASSET_NAMES,
  releaseAssetUrl,
  releaseAssetUrlFromBase,
  releaseBaseUrl,
  releaseBinaryDirectory,
  shouldRaceFirstPartyMirrors,
  usesCnbMirror,
  WINDOWS_INSTALLER_ASSET,
};
