import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import '@xterm/xterm/css/xterm.css';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import {
  TabData,
  PaneInstance,
  tabsMap,
  activeAppConfig,
  currentWindowId,
  activeTabId,
  activePaneId,
  setActiveTabId,
  setActivePaneId,
  hasAdjustedWindowSize,
  setHasAdjustedWindowSize,
} from './state';
import { DAEMON_URL, CAMPBELL_THEME } from './config';
import { updatePaneHighlights } from './highlights';
import { showFindBar, getTerminalBufferText } from './findBar';
import { connectWebSocket } from './daemon';
import { showTerminalContextMenu } from './components/ContextMenu';

export interface TerminalCallbacks {
  spawnDefaultTab: () => void;
  closeTab: (id: string) => Promise<void>;
  cycleTabs: (reverse?: boolean) => void;
  switchTab: (id: string) => void;
  splitPane: (targetId: string, direction: 'right' | 'left' | 'down' | 'up') => void;
  unsplitPane: (targetId: string) => void;
  getTabIdsInNode: (node: any) => string[];
  getCurrentLayouts: () => any[];
  openSettings: () => void;
  setFocusedPane: (id: string) => void;
  activeLayoutIsSplit: () => boolean;
}

let callbacks: TerminalCallbacks | null = null;

export function registerTerminalCallbacks(cb: TerminalCallbacks) {
  callbacks = cb;
}

export function createTabLocal(tabData: TabData) {
  const id = tabData.id;

  const pane = document.createElement('div');
  pane.className = 'terminal-pane';
  pane.id = `pane-${id}`;

  const term = new Terminal({
    allowProposedApi: true,
    cursorBlink: true,
    cursorStyle: 'bar',
    drawBoldTextInBrightColors: true,
    minimumContrastRatio: 1.2,
    fontFamily: activeAppConfig.font.family,
    fontSize: activeAppConfig.font.size,
    cols: activeAppConfig.default_cols || 120,
    rows: activeAppConfig.default_rows || 30,
    theme: {
      ...CAMPBELL_THEME,
      background: activeAppConfig.theme.background,
      foreground: activeAppConfig.theme.foreground,
      cursor: activeAppConfig.theme.foreground,
      cursorAccent: activeAppConfig.theme.background,
      selectionBackground: activeAppConfig.theme.highlight ? `${activeAppConfig.theme.highlight}44` : 'rgba(255, 255, 255, 0.25)',
      selectionInactiveBackground: 'transparent',
    },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  const findSearchAddon = new SearchAddon();
  term.loadAddon(findSearchAddon);

  term.open(pane);

  term.onWriteParsed(() => {
    updatePaneHighlights(id);
  });

  // Smart Ctrl+C & Custom Key Handlers
  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (e.type === 'keydown') {
      const isCtrl = e.ctrlKey || e.metaKey;
      const isShift = e.shiftKey;

      // Smart Ctrl+C
      if (isCtrl && !isShift && e.code === 'KeyC') {
        if (term.hasSelection()) {
          navigator.clipboard.writeText(term.getSelection());
          term.clearSelection();
          return false;
        }
        return true;
      }

      // Ctrl+V or Ctrl+Shift+V (Universal Paste)
      if (isCtrl && (e.code === 'KeyV' || e.key === 'v' || e.key === 'V')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat) {
          pasteToPane(id);
        }
        return false;
      }

      // New Tab with Default Shell (Ctrl+Shift++)
      if (isCtrl && isShift && (e.code === 'Equal' || e.key === '+' || e.code === 'NumpadAdd')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.spawnDefaultTab();
        }
        return false;
      }

      // Close Current Tab (Ctrl+Shift+-)
      if (isCtrl && isShift && (e.code === 'Minus' || e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && activeTabId && callbacks) {
          callbacks.closeTab(activeTabId);
        }
        return false;
      }

      // Cycle Tabs (Ctrl+Tab / Ctrl+Shift+Tab)
      if (isCtrl && e.code === 'Tab') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.cycleTabs(isShift);
        }
        return false;
      }

      // Jump to Tab Index 1..9 (Ctrl+Shift+1..9)
      if (isCtrl && isShift && e.code.startsWith('Digit')) {
        const digit = parseInt(e.code.replace('Digit', ''), 10);
        if (digit >= 1 && digit <= 9 && callbacks) {
          e.preventDefault();
          e.stopImmediatePropagation();
          const currentLayouts = callbacks.getCurrentLayouts();
          if (!e.repeat && digit - 1 < currentLayouts.length) {
            const targetIds = callbacks.getTabIdsInNode(currentLayouts[digit - 1]);
            if (targetIds.length > 0) callbacks.switchTab(targetIds[0]);
          }
          return false;
        }
      }

      // Open Settings Shortcut
      if (isCtrl && e.code === 'Comma') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.openSettings();
        }
        return false;
      }

      // Find in Pane Shortcut (Ctrl+Shift+F or Ctrl+F)
      if (isCtrl && e.code === 'KeyF') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat) {
          showFindBar(term, id);
        }
        return false;
      }

      // Split & Unsplit Shortcuts
      const isCtrlShift = isCtrl && isShift;
      const isAltShift = e.altKey && isShift;

      if ((isCtrlShift || isAltShift) && e.code === 'ArrowRight') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.splitPane(id, 'right');
        }
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowLeft') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.splitPane(id, 'left');
        }
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowDown') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.splitPane(id, 'down');
        }
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowUp') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.splitPane(id, 'up');
        }
        return false;
      }
      if ((isCtrlShift || isAltShift) && (e.code === 'Delete' || e.code === 'KeyW')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat && callbacks) {
          callbacks.unsplitPane(id);
        }
        return false;
      }
    }
    return true;
  });

  // Middle-Click Paste
  pane.addEventListener('auxclick', (e: MouseEvent) => {
    if (e.button === 1) {
      e.preventDefault();
      e.stopPropagation();
      pasteToPane(id);
    }
  });

  // Context Menu
  pane.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (callbacks) callbacks.setFocusedPane(id);
    showTerminalContextMenu(e.clientX, e.clientY, term, id, e);
  });

  const instance: PaneInstance = {
    id,
    profile: tabData.profile,
    title: tabData.title || `Tab ${id}`,
    badge: tabData.badge,
    color: tabData.color,
    elevated: tabData.elevated,
    term,
    fitAddon,
    findSearchAddon,
    ws: null,
    wsClosingIntentionally: false,
    element: pane,
    historyApplied: false,
  };

  tabsMap.set(id, instance);
  connectWebSocket(instance);
}

export function updateTabLocal(tabData: TabData) {
  const id = tabData.id;
  const instance = tabsMap.get(id);
  if (!instance) return;

  instance.title = tabData.title || `Tab ${id}`;
  instance.badge = tabData.badge;
  instance.color = tabData.color;
  instance.elevated = tabData.elevated;
}

export function removeTabLocal(id: string) {
  const instance = tabsMap.get(id);
  if (!instance) return;

  if (instance.ws) {
    instance.wsClosingIntentionally = true;
    instance.ws.close();
  }
  instance.term.dispose();
  instance.element.remove();
  tabsMap.delete(id);

  if (activeTabId === id || activePaneId === id) {
    setActiveTabId(null);
    setActivePaneId(null);
  }
}

export function getXtermCellDimensions(): { width: number; height: number } {
  const firstInstance = Array.from(tabsMap.values())[0];
  if (firstInstance && firstInstance.term) {
    const core = (firstInstance.term as any)._core;
    if (core && core._renderService && core._renderService.dimensions) {
      const cssCell = core._renderService.dimensions.css.cell;
      if (cssCell && cssCell.width > 0 && cssCell.height > 0) {
        return { width: cssCell.width, height: cssCell.height };
      }
    }
  }

  const dummyContainer = document.createElement('div');
  dummyContainer.style.position = 'absolute';
  dummyContainer.style.visibility = 'hidden';
  dummyContainer.style.width = '200px';
  dummyContainer.style.height = '200px';
  document.body.appendChild(dummyContainer);

  const dummyTerm = new Terminal({
    fontFamily: activeAppConfig.font?.family || 'Consolas, monospace',
    fontSize: activeAppConfig.font?.size || 14,
  });
  dummyTerm.open(dummyContainer);

  let width = 8.42;
  let height = 17.0;
  const core = (dummyTerm as any)._core;
  if (core && core._renderService && core._renderService.dimensions) {
    const cssCell = core._renderService.dimensions.css.cell;
    if (cssCell && cssCell.width > 0 && cssCell.height > 0) {
      width = cssCell.width;
      height = cssCell.height;
    }
  }

  dummyTerm.dispose();
  document.body.removeChild(dummyContainer);
  return { width, height };
}

export async function adjustWindowForGrid(
  targetCols: number = 120,
  targetRows: number = 30,
  force: boolean = false
): Promise<void> {
  if (hasAdjustedWindowSize && !force) return;

  const { width: cellWidth, height: cellHeight } = getXtermCellDimensions();
  const padding = activeAppConfig.terminal_padding || 8;

  const scrollbarWidth = 16;
  const isSplit = callbacks ? callbacks.activeLayoutIsSplit() : false;
  const splitReserve = isSplit ? 8 : 0;
  const availWidth = targetCols * cellWidth + scrollbarWidth + 0.5;
  const availHeight = targetRows * cellHeight + splitReserve + 0.5;

  const neededWidth = Math.ceil(availWidth + padding * 2);
  const neededHeight = Math.ceil(availHeight + 41 + padding * 2);

  const priorWidth = window.innerWidth;
  const priorHeight = window.innerHeight;
  const needsResize = priorWidth !== neededWidth || priorHeight !== neededHeight;

  try {
    const appWindow = getCurrentWindow();
    await appWindow.show();
    await appWindow.setSize(new LogicalSize(neededWidth, neededHeight));
  } catch (e) {
    console.warn('Could not set window size via Tauri API:', e);
  }

  try {
    await fetch(`${DAEMON_URL}/windows/size`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ window: currentWindowId, width: neededWidth, height: neededHeight }),
    });
  } catch (e) {}

  setHasAdjustedWindowSize(true);

  if (!needsResize) {
    for (const instance of tabsMap.values()) {
      if (instance.historyApplied) {
        instance.fitAddon.fit();
      }
    }
  }
}

export async function pasteToPane(tabId: string) {
  const inst = tabsMap.get(tabId);
  if (inst) {
    if (callbacks) callbacks.setFocusedPane(tabId);
    inst.term.focus();
  }
  try {
    let text = '';
    try {
      const res = await fetch(`${DAEMON_URL}/clipboard`);
      if (res.ok) {
        const data = await res.json();
        text = data.text || '';
      }
    } catch {}

    if (!text) {
      text = await navigator.clipboard.readText();
    }
    if (!text) return;

    if (inst) {
      if (inst.ws && inst.ws.readyState === WebSocket.OPEN) {
        inst.ws.send(text);
      } else {
        inst.term.paste(text);
      }
      inst.term.focus();
    }
  } catch (e) {
    console.error('Failed to paste clipboard text:', e);
  }
}

export async function exportPaneBuffer(term: Terminal, paneId: string) {
  try {
    const filePath = await save({
      title: 'Export Terminal Buffer',
      defaultPath: `terminal-buffer-${paneId}.txt`,
      filters: [{ name: 'Text File', extensions: ['txt'] }],
    });

    if (!filePath) return;

    const bufferText = getTerminalBufferText(term);
    await writeTextFile(filePath, bufferText);
    console.log(`[kterm] Buffer exported to: ${filePath}`);
  } catch (err) {
    console.error('Failed to export terminal buffer:', err);
  }
}
