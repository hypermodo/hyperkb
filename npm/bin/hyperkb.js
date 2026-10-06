#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawn, execSync } = require('child_process');

const PLATFORM_MAP = {
  darwin: 'darwin',
  linux: 'linux',
  win32: 'windows',
};

const ARCH_MAP = {
  arm64: 'arm64',
  x64: 'x64',
};

function getBinaryName() {
  const platform = PLATFORM_MAP[process.platform];
  const arch = ARCH_MAP[process.arch];
  if (!platform || !arch) {
    throw new Error(`Unsupported platform/architecture: ${process.platform} ${process.arch}`);
  }
  const ext = platform === 'windows' ? '.exe' : '';
  return `hyperkb-${platform}-${arch}${ext}`;
}

function resolveBinary() {
  // 1. Explicit override via environment variable
  if (process.env.HYPERKB_BIN && fs.existsSync(process.env.HYPERKB_BIN)) {
    return process.env.HYPERKB_BIN;
  }

  const binaryName = getBinaryName();
  const binDir = __dirname;

  // 2. Bundled platform-specific binary in npm package
  const bundledPlatformBin = path.join(binDir, binaryName);
  if (fs.existsSync(bundledPlatformBin)) {
    return bundledPlatformBin;
  }

  // 3. Generic bundled binary in npm package
  const bundledGenericBin = path.join(binDir, process.platform === 'win32' ? 'hyperkb.exe' : 'hyperkb');
  if (fs.existsSync(bundledGenericBin)) {
    return bundledGenericBin;
  }

  // 4. Local workspace development target (if running inside or adjacent to hyperkb repo)
  const devReleaseBin = path.resolve(__dirname, '../../target/release', process.platform === 'win32' ? 'hyperkb.exe' : 'hyperkb');
  if (fs.existsSync(devReleaseBin)) {
    return devReleaseBin;
  }

  // 5. Standard user installation locations
  const userPaths = [
    path.join(os.homedir(), '.cargo', 'bin', 'hyperkb'),
    path.join(os.homedir(), '.local', 'bin', 'hyperkb'),
    '/opt/homebrew/bin/hyperkb',
    '/usr/local/bin/hyperkb',
    path.join(os.homedir(), '.hyperkb', 'bin', binaryName),
  ];

  for (const candidate of userPaths) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  // 6. Check system PATH
  try {
    const whichCmd = process.platform === 'win32' ? 'where hyperkb' : 'which hyperkb';
    const systemPath = execSync(whichCmd, { stdio: ['ignore', 'pipe', 'ignore'], encoding: 'utf-8' }).trim().split('\n')[0];
    if (systemPath && fs.existsSync(systemPath)) {
      return systemPath;
    }
  } catch (_) {
    // Not found in system PATH
  }

  // If not found anywhere, report clear guidance
  throw new Error(
    `HyperKB pre-compiled native binary not found for ${process.platform}-${process.arch}.\n` +
    `Expected binary: ${binaryName}\n` +
    `Please set HYPERKB_BIN=/path/to/hyperkb or build the release binary.`
  );
}

function main() {
  let binaryPath;
  try {
    binaryPath = resolveBinary();
  } catch (err) {
    console.error(`[hyperkb] Error: ${err.message}`);
    process.exit(1);
  }

  // Ensure executable permissions on Unix
  if (process.platform !== 'win32') {
    try {
      fs.chmodSync(binaryPath, 0o755);
    } catch (_) {}
  }

  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env,
  });

  child.on('error', (err) => {
    console.error(`[hyperkb] Failed to start native binary: ${err.message}`);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code ?? 0);
    }
  });

  const forwardSignal = (sig) => {
    if (child.pid) {
      try {
        child.kill(sig);
      } catch (_) {}
    }
  };

  process.on('SIGINT', () => forwardSignal('SIGINT'));
  process.on('SIGTERM', () => forwardSignal('SIGTERM'));
  process.on('SIGHUP', () => forwardSignal('SIGHUP'));
}

main();
