import { invoke } from '@tauri-apps/api/core';
import { AppConfig } from './components/SettingsModal';
import { activeAppConfig, setActiveAppConfig, tabsMap } from './state';

export const isMac =
  typeof navigator !== 'undefined' &&
  (/Macintosh|Mac OS X/i.test(navigator.userAgent) ||
    (navigator as any).userAgentData?.platform === 'macOS' ||
    navigator.platform?.toUpperCase().indexOf('MAC') >= 0);

export const PROFILES = isMac
  ? [
      { id: 'zsh', label: 'Zsh' },
      { id: 'bash', label: 'Bash' },
      { id: 'fish', label: 'Fish' },
    ]
  : [
      { id: 'powershell', label: 'PowerShell' },
      { id: 'cmd', label: 'Command Prompt' },
      { id: 'wsl', label: 'WSL' },
      { id: 'git-bash', label: 'Git Bash' },
    ];

export const CAMPBELL_THEME = {
  black: '#0C0C0C',
  red: '#C50F1F',
  green: '#13A10E',
  yellow: '#C19C00',
  blue: '#0037DA',
  magenta: '#881798',
  cyan: '#3A96DD',
  white: '#CCCCCC',
  brightBlack: '#767676',
  brightRed: '#E74856',
  brightGreen: '#16C60C',
  brightYellow: '#F9F1A5',
  brightBlue: '#3B78FF',
  brightMagenta: '#B4009E',
  brightCyan: '#61D6D6',
  brightWhite: '#F2F2F2',
};

type AdjustWindowFn = (targetCols?: number, targetRows?: number, force?: boolean) => Promise<void>;
let adjustWindowFn: AdjustWindowFn | null = null;

export function registerAdjustWindowForGrid(fn: AdjustWindowFn) {
  adjustWindowFn = fn;
}

export function applyAppConfig(config: AppConfig) {
  setActiveAppConfig(config);
  if (config.theme?.highlight) {
    document.documentElement.style.setProperty('--highlight-color', config.theme.highlight);
  }
  if (config.theme?.title_bar) {
    document.documentElement.style.setProperty('--title-bar-bg', config.theme.title_bar);
  }
  if (config.theme?.active_tab) {
    document.documentElement.style.setProperty('--active-tab-bg', config.theme.active_tab);
  }
  if (config.theme?.inactive_tab) {
    document.documentElement.style.setProperty('--inactive-tab-bg', config.theme.inactive_tab);
  }
  if (config.theme?.background) {
    document.documentElement.style.setProperty('--terminal-bg', config.theme.background);
  }
  if (config.theme?.tab_hover) {
    document.documentElement.style.setProperty('--tab-hover-bg', config.theme.tab_hover);
  }
  if (config.theme?.active_tab_fg) {
    document.documentElement.style.setProperty('--active-tab-fg', config.theme.active_tab_fg);
  }
  if (config.theme?.inactive_tab_fg) {
    document.documentElement.style.setProperty('--inactive-tab-fg', config.theme.inactive_tab_fg);
  }
  for (const instance of tabsMap.values()) {
    instance.term.options.fontFamily = config.font.family;
    instance.term.options.fontSize = config.font.size;
    instance.term.options.cursorStyle = 'bar';
    instance.term.options.drawBoldTextInBrightColors = true;
    instance.term.options.minimumContrastRatio = 1.2;
    instance.term.options.theme = {
      ...CAMPBELL_THEME,
      background: config.theme.background,
      foreground: config.theme.foreground,
      cursor: config.theme.foreground,
      cursorAccent: config.theme.background,
      selectionBackground: config.theme.highlight ? `${config.theme.highlight}44` : 'rgba(255, 255, 255, 0.25)',
    };
    if ((instance.term as any)._core?._charSizeService) {
      (instance.term as any)._core._charSizeService.clear();
    }
    if (instance.historyApplied) {
      instance.fitAddon.fit();
      instance.term.refresh(0, instance.term.rows - 1);
    }
  }
  document.querySelectorAll<HTMLElement>('.split-pane-wrapper').forEach((el) => {
    el.style.padding = `${config.terminal_padding}px`;
  });
  if (config.default_cols && config.default_rows && adjustWindowFn) {
    adjustWindowFn(config.default_cols, config.default_rows, true);
  }
}

export async function loadInitialConfig() {
  try {
    const cfg = await invoke<AppConfig>('get_config');
    if (cfg) {
      applyAppConfig(cfg);
    }
  } catch (e) {
    console.warn('Failed to load initial config via IPC', e);
  }
}

export function getContainerGridDimensions(): { cols: number; rows: number } {
  if (activeAppConfig.default_cols && activeAppConfig.default_rows) {
    return {
      cols: activeAppConfig.default_cols,
      rows: activeAppConfig.default_rows,
    };
  }
  const terminalContainerEl = document.getElementById('terminal-container');
  const width = terminalContainerEl?.clientWidth || 800;
  const height = terminalContainerEl?.clientHeight || 500;
  const fontWidth = 8.42;
  const fontHeight = 17.0;
  const cols = Math.max(20, Math.floor((width - 16) / fontWidth));
  const rows = Math.max(5, Math.floor((height - 16) / fontHeight));
  return { cols, rows };
}
