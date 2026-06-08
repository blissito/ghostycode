const path = require("path");
const os = require("os");

const CHECKSUM_MANIFEST = "ghosty-artifacts-sha256.txt";

const ASSET_MATRIX = {
  linux: {
    x64: ["ghosty-linux-x64", "ghosty-tui-linux-x64"],
    arm64: ["ghosty-linux-arm64", "ghosty-tui-linux-arm64"],
    riscv64: ["ghosty-linux-riscv64", "ghosty-tui-linux-riscv64"],
  },
  darwin: {
    x64: ["ghosty-macos-x64", "ghosty-tui-macos-x64"],
    arm64: ["ghosty-macos-arm64", "ghosty-tui-macos-arm64"],
  },
  win32: {
    x64: ["ghosty-windows-x64.exe", "ghosty-tui-windows-x64.exe", "ghosty.bat"],
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
    throw new Error(
      `Unsupported architecture: ${arch} on platform ${platform}. ` +
      `Supported architectures: ${supported}.\n\n` +
      unsupportedBuildHint(),
    );
  }
  return {
    platform,
    arch,
    ghosty: pair[0],
    tui: pair[1],
  };
}

function unsupportedBuildHint() {
  return [
    "No prebuilt binary is available for this platform/architecture combo.",
    "You can still run ghosty by building from source with Cargo:",
    "",
    "  # Requires Rust 1.88+ (https://rustup.rs)",
    "  cargo install ghosty-cli --locked   # provides `ghosty`",
    "  cargo install ghosty-tui --locked   # provides `ghosty-tui`",
    "",
    "Or build from a checkout:",
    "",
    "  git clone https://github.com/blissito/ghostycode.git",
    "  cd Ghosty",
    "  cargo install --path crates/cli --locked",
    "  cargo install --path crates/tui --locked",
    "",
    "See https://github.com/blissito/ghostycode/blob/main/docs/INSTALL.md",
    "for cross-compilation, mirror, and Linux ARM64 specifics.",
  ].join("\n");
}

function executableName(base, platform) {
  return platform === "win32" ? `${base}.exe` : base;
}

function releaseBaseUrl(version, repo = "blissito/ghostycode") {
  // GHOSTY_RELEASE_BASE_URL is the canonical override.
  // DEEPSEEK_TUI_RELEASE_BASE_URL / DEEPSEEK_RELEASE_BASE_URL are legacy aliases.
  const override =
    process.env.GHOSTY_RELEASE_BASE_URL ||
    process.env.DEEPSEEK_TUI_RELEASE_BASE_URL ||
    process.env.DEEPSEEK_RELEASE_BASE_URL;
  if (override) {
    const trimmed = String(override).trim();
    return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
  }
  // When GHOSTY_USE_CNB_MIRROR is set, use the CNB (China-friendly)
  // mirror that already builds and publishes binary release assets.
  if (process.env.GHOSTY_USE_CNB_MIRROR) {
    return `https://cnb.cool/blissito/ghostycode/-/releases/v${version}/`;
  }
  return `https://github.com/${repo}/releases/download/v${version}/`;
}

function releaseAssetUrl(baseName, version, repo = "blissito/ghostycode") {
  return new URL(baseName, releaseBaseUrl(version, repo)).toString();
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
  return [...allAssetNames(), CHECKSUM_MANIFEST];
}

module.exports = {
  allAssetNames,
  allReleaseAssetNames,
  CHECKSUM_MANIFEST,
  checksumManifestUrl,
  detectBinaryNames,
  executableName,
  releaseAssetUrl,
  releaseBaseUrl,
  releaseBinaryDirectory,
};
