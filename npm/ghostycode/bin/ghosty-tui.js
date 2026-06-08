#!/usr/bin/env node

const { runGhostyTui } = require("../scripts/run");

runGhostyTui().catch((error) => {
  console.error("Failed to start ghosty-tui:", error.message);
  process.exit(1);
});
