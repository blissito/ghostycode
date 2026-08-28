#!/usr/bin/env node

const { runGhostyCode } = require("../scripts/run");

runGhostyCode().catch((error) => {
  console.error("Failed to start ghosty:", error.message);
  process.exit(1);
});
