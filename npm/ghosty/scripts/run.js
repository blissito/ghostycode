const { spawnSync } = require("child_process");
const { getBinaryPath } = require("./install");

const pkg = require("../package.json");

function isVersionFlag(args = process.argv.slice(2)) {
  return args.includes("--version") || args.includes("-V");
}

function printVersionFallback(binaryName) {
  const binVersion =
    process.env.GHOSTY_VERSION ||
    process.env.DEEPSEEK_TUI_VERSION ||
    process.env.DEEPSEEK_VERSION ||
    pkg.ghostyBinaryVersion || pkg.deepseekBinaryVersion || pkg.version;
  console.log(`${binaryName} (npm wrapper) v${pkg.version}`);
  console.log(`binary version: v${binVersion}`);
  console.log(`repo: ${pkg.repository?.url || "N/A"}`);
}

async function run(binaryName, options = {}) {
  const args = options.args || process.argv.slice(2);
  const resolveBinaryPath = options.getBinaryPath || getBinaryPath;
  const spawn = options.spawnSync || spawnSync;
  const exit = options.exit || process.exit;
  const versionFlag = isVersionFlag(args);

  let binaryPath;
  try {
    binaryPath = await resolveBinaryPath(binaryName);
  } catch (error) {
    if (versionFlag) {
      printVersionFallback(binaryName);
      return exit(0);
    }
    throw error;
  }

  const result = spawn(binaryPath, args, {
    stdio: "inherit",
  });
  if (result.error) {
    if (versionFlag) {
      printVersionFallback(binaryName);
      return exit(0);
    }
    throw result.error;
  }
  return exit(result.status ?? 1);
}

async function runGhostyCode() {
  await run("ghosty");
}

async function runGhostyCodeTui() {
  // v0.9.5 single-binary: tui is now an alias to ghosty (kept for backwards compat, will warn)
  if (!process.env.GHOSTY_SUPPRESS_TUI_DEPRECATION) {
    process.stderr.write("ghosty-tui: deprecated alias to `ghosty` (single binary since v0.9.5). Use `ghosty` instead.\n");
  }
  await run("ghosty");
}

module.exports = {
  run,
  runGhostyCode,
  runGhostyCodeTui,
  _internal: { isVersionFlag, printVersionFallback },
};

if (require.main === module) {
  const command = process.argv[1] || "";
  if (command.includes("tui")) {
    runGhostyCodeTui();
  } else {
    runGhostyCode();
  }
}
