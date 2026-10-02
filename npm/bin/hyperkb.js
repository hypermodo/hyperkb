#!/usr/bin/env node
const path = require('path');
const { spawnSync } = require('child_process');

const binPath = path.join(__dirname, 'hyperkb');

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: 'inherit'
});

if (result.error) {
  console.error(`Failed to launch hyperkb: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status !== null ? result.status : 0);
