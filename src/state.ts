import { Terminal, IMarker, IDecoration } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { LayoutNode } from './components/SplitGrid';
import { AppConfig } from './components/SettingsModal';

export interface TabData {
  id: string;
  pid: number;
  profile: string;
  window_id: string;
  title: string;
  badge?: string;
  color?: string;
  elevated?: boolean;
}

export interface PaneInstance {
  id: string;
  profile: string;
  title: string;
  badge?: string;
  color?: string;
  elevated?: boolean;
  term: Terminal;
  fitAddon: FitAddon;
  findSearchAddon: SearchAddon;
  ws: WebSocket | null;
  wsClosingIntentionally: boolean;
  element: HTMLElement;
  historyApplied: boolean;
}

export type TabInstance = PaneInstance;

const urlParams = new URLSearchParams(window.location.search);
export const currentWindowId = urlParams.get('window') || 'win-1';

export const tabsMap = new Map<string, PaneInstance>();
export const paneHighlightsMap = new Map<string, string[]>();
export const activePaneDecorationsMap = new Map<string, Array<{ marker: IMarker; decoration: IDecoration }>>();

export let activeTabId: string | null = null;
export function setActiveTabId(id: string | null) {
  activeTabId = id;
}

export let activePaneId: string | null = null;
export function setActivePaneId(id: string | null) {
  activePaneId = id;
}

export let currentLayouts: LayoutNode[] = [];
export function setCurrentLayouts(layouts: LayoutNode[]) {
  currentLayouts = layouts;
}

export let activeAppConfig: AppConfig = {
  default_profile: 'powershell',
  default_cols: 120,
  default_rows: 30,
  ring_buffer_kb: 256,
  terminal_padding: 8,
  font: { family: 'Consolas, "Courier New", monospace', size: 14 },
  theme: { background: '#0d0e11', foreground: '#cccccc', highlight: '#61afef' },
};
export function setActiveAppConfig(config: AppConfig) {
  activeAppConfig = config;
}

export let hasHadTabs = false;
export function setHasHadTabs(val: boolean) {
  hasHadTabs = val;
}

export let hasAdjustedWindowSize = false;
export function setHasAdjustedWindowSize(val: boolean) {
  hasAdjustedWindowSize = val;
}

export let lastAdjustedWasSplit = false;
export function setLastAdjustedWasSplit(val: boolean) {
  lastAdjustedWasSplit = val;
}

export let unsplitShrinkPending = false;
export function setUnsplitShrinkPending(val: boolean) {
  unsplitShrinkPending = val;
}
