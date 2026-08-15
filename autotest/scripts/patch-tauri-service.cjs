// Re-applies the maxAttempts=1 patch to @wdio/tauri-service after npm install.
// Why: the service's plugin-availability check retries 100x (~7.5s, 200-300
// executeAsyncScript POSTs) before every focus command when the app has no
// tauri-plugin-wdio registered. With no plugin the check can never pass, so the
// retry storm is pure waste (was ~13min per 13-test suite). 1 attempt per call
// makes the check a fast no-op warn. Restores when the app registers the real
// plugin.
const fs = require('fs');
const path = require('path');

const targets = [
  'node_modules/@wdio/tauri-service/dist/cjs/index.js',
  'node_modules/@wdio/tauri-service/dist/esm/index.js',
];

let changed = 0;
for (const rel of targets) {
  const file = path.join(__dirname, '..', rel);
  if (!fs.existsSync(file)) {
    console.log(`skip (missing): ${rel}`);
    continue;
  }
  let src = fs.readFileSync(file, 'utf8');
  const before = src;
  src = src.replace(
    'const maxAttempts = 100;',
    'const maxAttempts = 1;'
  );
  // Modern msedgedriver prints "Microsoft Edge WebDriver X.Y.Z", which the
  // service's /MSEdgeDriver/ regex never matches — so it never reuses a PATH
  // driver and re-downloads into %TEMP%\msedgedriver on every launch (~42MB
  // each, never cleaned). See scripts/ensure-msedgedriver.cjs.
  src = src.replace(
    'versionOutput.match(/MSEdgeDriver ([\\d.]+)/)',
    'versionOutput.match(/(?:MSEdgeDriver|Edge WebDriver) ([\\d.]+)/)'
  );
  if (src !== before) {
    fs.writeFileSync(file, src);
    changed++;
    console.log(`patched: ${rel}`);
  } else {
    console.log(`already-patched: ${rel}`);
  }
}
if (changed === 0) {
  console.warn('patch-tauri-service: no files changed — service build layout may have changed');
  process.exitCode = 1;
}
