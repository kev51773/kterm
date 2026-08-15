// Pins msedgedriver.exe matching the installed WebView2 runtime so
// @wdio/tauri-service reuses it from PATH instead of downloading a fresh copy
// into %TEMP%\msedgedriver on every launch (~42MB each, never cleaned — 754
// dirs / 44GB accumulated). The service reuses any PATH driver whose major
// version matches Edge's, so a pinned driver eliminates the downloads until
// Edge's major bumps, at which point this re-downloads once.
const { execFileSync, execSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const CACHE_DIR = path.join(__dirname, '..', '.cache', 'msedgedriver');
const DRIVER = path.join(CACHE_DIR, 'msedgedriver.exe');

const REG_PATHS = [
  'HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
];

function webView2Version() {
  for (const reg of REG_PATHS) {
    try {
      const out = execSync(`reg query "${reg}" /v pv`, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
      const m = out.match(/pv\s+REG_SZ\s+([\d.]+)/);
      if (m) return m[1];
    } catch {}
  }
  return undefined;
}

function driverMajor() {
  if (!fs.existsSync(DRIVER)) return undefined;
  try {
    const out = execFileSync(DRIVER, ['--version'], { encoding: 'utf8', timeout: 5000, stdio: ['ignore', 'pipe', 'ignore'] });
    return out.match(/(?:MSEdgeDriver|Edge WebDriver) ([\d.]+)/)?.[1]?.split('.')[0];
  } catch {
    return undefined;
  }
}

function ps(script, args = []) {
  const file = path.join(CACHE_DIR, 'ensure-driver.ps1');
  fs.mkdirSync(CACHE_DIR, { recursive: true });
  fs.writeFileSync(file, script, 'utf8');
  try {
    return execSync(`powershell -NoProfile -ExecutionPolicy Bypass -File "${file}" ${args.join(' ')}`, {
      encoding: 'utf8',
      timeout: 120000,
    });
  } finally {
    fs.rmSync(file, { force: true });
  }
}

function download(edgeVersion) {
  const major = edgeVersion.split('.')[0];
  const arch = os.arch() === 'x64' ? 'win64' : 'win32';
  const latest = ps(`
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$r = Invoke-WebRequest -Uri 'https://msedgedriver.microsoft.com/LATEST_RELEASE_${major}' -UseBasicParsing -TimeoutSec 10
$text = if ($r.Content -is [byte[]]) { [System.Text.Encoding]::Unicode.GetString($r.Content) } else { [string]$r.Content }
$text.Trim()
`).trim().replace(/[^0-9.]/g, '');
  if (!latest.startsWith(major)) throw new Error(`no driver version for Edge ${edgeVersion} (LATEST_RELEASE_${major} -> '${latest}')`);
  const cache = CACHE_DIR.replace(/\\/g, '\\\\');
  const zip = path.join(CACHE_DIR, 'edgedriver.zip').replace(/\\/g, '\\\\');
  ps(`
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$zip = '${zip}'
$cache = '${cache}'
Invoke-WebRequest -Uri 'https://msedgedriver.microsoft.com/${latest}/edgedriver_${arch}.zip' -OutFile $zip -UseBasicParsing -TimeoutSec 60
Expand-Archive -Path $zip -DestinationPath $cache -Force
Remove-Item $zip
`);
  return latest;
}

function ensure() {
  const edgeVersion = webView2Version();
  const wanted = edgeVersion?.split('.')[0];
  const have = driverMajor();
  if (wanted && have === wanted) return { status: 'reuse', driverPath: DRIVER };
  if (!wanted) return { status: 'skipped', driverPath: fs.existsSync(DRIVER) ? DRIVER : undefined };
  if (have) console.warn(`[ensure-msedgedriver] driver major ${have} != Edge major ${wanted} — re-downloading`);
  const driverVersion = download(edgeVersion);
  if (!fs.existsSync(DRIVER)) throw new Error('download extracted but msedgedriver.exe missing');
  return { status: 'downloaded', driverPath: DRIVER, driverVersion };
}

module.exports = { ensure, DRIVER, CACHE_DIR };

if (require.main === module) {
  const r = ensure();
  if (r.driverPath) process.env.PATH = `${path.dirname(r.driverPath)};${process.env.PATH}`;
  console.log(`[ensure-msedgedriver] ${r.status}${r.driverVersion ? ` ${r.driverVersion}` : ''} -> ${r.driverPath}`);
}
