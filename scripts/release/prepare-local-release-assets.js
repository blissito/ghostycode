#!/usr/bin/env node

const crypto = require("crypto");
const fs = require("fs/promises");
const path = require("path");

const {
  allAssetNames,
  CHECKSUM_MANIFEST,
  detectBinaryNames,
} = require("../../npm/ghostycode/scripts/artifacts");

const WINDOWS_LAUNCHER = "ghosty.bat";
const WINDOWS_CLI_ASSET = "ghosty-windows-x64.exe";

async function sha256(filePath) {
  const content = await fs.readFile(filePath);
  return crypto.createHash("sha256").update(content).digest("hex");
}

async function main() {
  const prepareAllAssets =
    process.env.DEEPSEEK_TUI_PREPARE_ALL_ASSETS === "1" ||
    process.env.DEEPSEEK_PREPARE_ALL_ASSETS === "1";
  const outputDir = path.resolve(
    process.argv[2] || path.join("target", "npm-release-assets"),
  );
  const buildDir = path.resolve(
    process.argv[3] || path.join("target", "release"),
  );
  const { ghosty, tui } = detectBinaryNames();
  const isWindows = process.platform === "win32";

  const assets = [
    {
      source: path.join(buildDir, isWindows ? "ghosty.exe" : "ghosty"),
      target: ghosty,
    },
    {
      source: path.join(buildDir, isWindows ? "ghosty-tui.exe" : "ghosty-tui"),
      target: tui,
    },
  ];

  if (prepareAllAssets) {
    for (const assetName of allAssetNames()) {
      if (assetName === WINDOWS_LAUNCHER) {
        continue;
      }
      if (assets.some((asset) => asset.target === assetName)) {
        continue;
      }
      assets.push({
        source: assetName.startsWith("ghosty-tui")
          ? path.join(buildDir, isWindows ? "ghosty-tui.exe" : "ghosty-tui")
          : path.join(buildDir, isWindows ? "ghosty.exe" : "ghosty"),
        target: assetName,
      });
    }
  }

  await fs.mkdir(outputDir, { recursive: true });

  const manifestLines = [];
  for (const asset of assets) {
    const outputPath = path.join(outputDir, asset.target);
    await fs.copyFile(asset.source, outputPath);
    manifestLines.push(`${await sha256(outputPath)}  ${asset.target}`);
  }

  if (assets.some((asset) => asset.target === WINDOWS_CLI_ASSET)) {
    const batContent = [
      "@echo off",
      "where wt >nul 2>nul",
      "set NO_ANIMATIONS=1",
      'if "%ERRORLEVEL%"=="0" (',
      '    wt --title Ghosty Code cmd /k "%~dp0ghosty-windows-x64.exe"',
      ") else (",
      '    "%~dp0ghosty-windows-x64.exe"',
      ")",
      "",
    ].join("\r\n");
    const batPath = path.join(outputDir, WINDOWS_LAUNCHER);
    await fs.writeFile(batPath, batContent, "utf8");
    const batHash = await sha256(batPath);
    manifestLines.push(`${batHash}  ${WINDOWS_LAUNCHER}`);
    console.log(`Generated ${batPath}`);
  }

  manifestLines.sort();
  const manifestPath = path.join(outputDir, CHECKSUM_MANIFEST);
  await fs.writeFile(manifestPath, `${manifestLines.join("\n")}\n`, "utf8");

  console.log(`Prepared ${assets.length} assets in ${outputDir}`);
  console.log(`Wrote checksum manifest ${manifestPath}`);
}

main().catch((error) => {
  console.error("Failed to prepare local release assets:", error.message);
  process.exit(1);
});
