import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export const AUTOTEST_DIR = path.resolve(__dirname, '..', '..')
export const REPO_DIR = path.resolve(AUTOTEST_DIR, '..')

export const APP_BINARY = path.resolve(REPO_DIR, 'src-tauri', 'target', 'release', 'kterm.exe')

export const TMP_DIR = path.join(AUTOTEST_DIR, 'tmp')
export const APPDATA_DIR = path.join(TMP_DIR, 'appdata')
export const KTERM_CONFIG = path.join(APPDATA_DIR, 'kterm', 'config.json')
export const USER_APPDATA_DIR = process.env.APPDATA ?? path.join(process.env.USERPROFILE ?? '', 'AppData', 'Roaming')

export const SCREENSHOT_BASELINE = path.join(AUTOTEST_DIR, 'screenshots', 'baseline')
export const SCREENSHOT_ACTUAL = path.join(AUTOTEST_DIR, 'screenshots', 'actual')
export const ARTIFACT_BASELINE = path.join(AUTOTEST_DIR, 'artifacts', 'baseline')
export const ARTIFACT_ACTUAL = path.join(AUTOTEST_DIR, 'artifacts', 'actual')
export const DIFF_DIR = path.join(AUTOTEST_DIR, 'screenshots', 'diff')

export const DAEMON_BASE_URL = 'http://127.0.0.1:9999'

export const PINNED_CONFIG = {
  default_profile: 'powershell',
  default_cols: 120,
  default_rows: 30,
  ring_buffer_kb: 256,
  terminal_padding: 8,
  font: { family: "Consolas, 'Courier New', monospace", size: 14 },
  theme: {
    background: '#0d0e11',
    foreground: '#cccccc',
    highlight: '#61afef',
    title_bar: '#21252b',
    active_tab: '#0d0e11',
    inactive_tab: '#181a1f',
    tab_hover: '#282c34',
    active_tab_fg: '#ffffff',
    inactive_tab_fg: '#abb2bf',
  },
}

export const UPDATE_BASELINE = process.env.UPDATE_BASELINE === '1' || process.env.UPDATE_BASELINE === 'true'
