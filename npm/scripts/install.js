#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const pkg = require('../package.json');
const VERSION = pkg.version;
const REPO = 'hypermodo/hyperkb';

const PLATFORM_MAP = {
  darwin: 'apple-darwin',
  linux: 'unknown-linux-musl'
};

const ARCH_MAP = {
  x64: 'x86_64',
  arm64: 'aarch64'
};

const platform = PLATFORM_MAP[process.platform];
const arch = ARCH_MAP[process.arch];

if (!platform || !arch) {
  console.error(`Unsupported platform or architecture: ${process.platform} (${process.arch})`);
  process.exit(1);
}

const target = `${arch}-${platform}`;
const archiveName = `hyperkb-v${VERSION}-${target}.tar.gz`;
const downloadUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;

const binDir = path.join(__dirname, '..', 'bin');
const destArchive = path.join(binDir, archiveName);
const destBinary = path.join(binDir, 'hyperkb');

if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

function download(url, dest, cb) {
  https.get(url, (res) => {
    if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
      return download(res.headers.location, dest, cb);
    }
    if (res.statusCode !== 200) {
      return cb(new Error(`Download failed with HTTP ${res.statusCode}: ${url}`));
    }
    const file = fs.createWriteStream(dest);
    res.pipe(file);
    file.on('finish', () => file.close(cb));
  }).on('error', cb);
}

console.log(`==> Fetching HyperKB v${VERSION} for ${target}...`);
download(downloadUrl, destArchive, (err) => {
  if (err) {
    console.error(`Error downloading pre-compiled binary: ${err.message}`);
    process.exit(1);
  }

  try {
    execSync(`tar -xzf "${destArchive}" -C "${binDir}"`);
    fs.unlinkSync(destArchive);
    fs.chmodSync(destBinary, 0o755);
    console.log(`==> Successfully installed native HyperKB binary.`);
  } catch (extractErr) {
    console.error(`Extraction failed: ${extractErr.message}`);
    process.exit(1);
  }
});
