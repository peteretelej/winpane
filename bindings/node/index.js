// Loader for winpane native addon.
// Tries a local .node file first (dev builds), then the scoped platform package.

const { existsSync } = require('fs');
const { join } = require('path');

const { platform, arch } = process;

let nativeBinding = null;
let loadError = null;

const platforms = {
  'win32-x64': {
    localFile: 'winpane.win32-x64-msvc.node',
    package: '@winpane/win32-x64-msvc',
  },
  'win32-arm64': {
    localFile: 'winpane.win32-arm64-msvc.node',
    package: '@winpane/win32-arm64-msvc',
  },
};

const target = platforms[`${platform}-${arch}`];

if (!target) {
  throw new Error(
    `winpane: unsupported platform ${platform}-${arch}. ` +
      `Supported: ${Object.keys(platforms).join(', ')}`,
  );
}

// 1. Try local .node file (dev / build-from-source)
const localPath = join(__dirname, target.localFile);
if (existsSync(localPath)) {
  try {
    nativeBinding = require(localPath);
  } catch (e) {
    loadError = e;
  }
}

// 2. Try scoped platform package (npm install)
if (!nativeBinding) {
  try {
    nativeBinding = require(target.package);
    loadError = null;
  } catch (e) {
    loadError = e;
  }
}

if (!nativeBinding) {
  throw new Error(
    `winpane: failed to load native addon for ${platform}-${arch}.\n` +
      `Tried: ${localPath}, ${target.package}\n` +
      (loadError ? `Last error: ${loadError.message}` : ''),
  );
}

module.exports = nativeBinding;
