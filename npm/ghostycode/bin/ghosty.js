#!/usr/bin/env node

const { runGhosty } = require("../scripts/run");

runGhosty().catch((error) => {
  console.error("Failed to start ghosty:", error.message);
  process.exit(1);
});
