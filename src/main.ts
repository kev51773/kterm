import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SettingsModal } from './components/SettingsModal';
import {
  tabsMap,
  currentWindowId,
  activeTabId,
  activePaneId,
  currentLayouts,
  activeAppConfig,
  unsplitShrinkPending,
  setUnsplitShrinkPending,
} from './state';
import { DAEMON_URL, registerAdjustWindowForGrid, applyAppConfig } from './config';
import {
  initDaemonConnection,
  syncTabs,
  registerDaemonCallbacks,
} from './daemon';
import { initHighlightsInterval } from './highlights';
import { showFindBar } from './findBar';
import {
  createTabLocal,
  updateTabLocal,
  removeTabLocal,
  adjustWindowForGrid,
  pasteToPane,
  exportPaneBuffer,
  registerTerminalCallbacks,
  getXtermCellDimensions,
} from './terminal';
import {
  containsTab,
  getTabIdsInNode,
  activeLayoutIsSplit,
  splitGrowPending,
  renderActiveLayout,
  splitPane,
  unsplitPane,
  registerSetFocusedPane,
} from './splits';
import {
  switchTab,
  cycleTabs,
  closeTab,
  setFocusedPane,
  renderTabBarHeaders,
  spawnTabWithProfile,
  spawnDefaultTab,
  registerCheckTabOverflow,
} from './tabs';
import {
  contextMenuEl,
  hideContextMenu,
  registerContextMenuActions,
} from './components/ContextMenu';
import {
  profileDropdownEl,
  profileSubMenuEl,
  closeProfileSubMenu,
  registerProfileDropdownActions,
  renderProfileDropdownMenu,
} from './components/ProfileDropdown';
import { setupTabBarControls } from './components/TabBar';

document.title = `kterm.exe - A scriptable terminal - ${currentWindowId}`;

// Register cross-module callbacks
registerAdjustWindowForGrid(adjustWindowForGrid);
registerDaemonCallbacks({
  createTabLocal,
  updateTabLocal,
  removeTabLocal,
  switchTab,
  renderActiveLayout,
  renderTabBarHeaders,
  containsTab,
  getTabIdsInNode,
  splitGrowPending,
});
registerTerminalCallbacks({
  spawnDefaultTab,
  closeTab,
  cycleTabs,
  switchTab,
  splitPane,
  unsplitPane,
  getTabIdsInNode,
  getCurrentLayouts: () => currentLayouts,
  openSettings: () => settingsModal.open(),
  setFocusedPane,
  activeLayoutIsSplit,
});
registerSetFocusedPane(setFocusedPane);
registerContextMenuActions({
  pasteToPane,
  splitPane,
  unsplitPane,
  exportPaneBuffer,
  renderTabBarHeaders,
  syncTabs,
  closeTab,
});
registerProfileDropdownActions({
  spawnTabWithProfile,
  triggerExportSave,
  openSettingsModal: () => settingsModal.open(),
});

// Setup TabBar controls
const tabBarEl = document.getElementById('tab-bar');
const tabsScrollContainer = document.getElementById('tabs-scroll-container');
const tabsListEl = document.getElementById('tabs-list');
const tabScrollLeftBtn = document.getElementById('tab-scroll-left') as HTMLButtonElement;
const tabScrollRightBtn = document.getElementById('tab-scroll-right') as HTMLButtonElement;

const tabBarControls = setupTabBarControls(
  tabBarEl,
  tabsScrollContainer,
  tabsListEl,
  tabScrollLeftBtn,
  tabScrollRightBtn
);
if (tabBarControls) {
  registerCheckTabOverflow(tabBarControls.checkTabOverflow);
}

// Global capture-phase shortcut trap (prevents browser find dialog under all conditions)
window.addEventListener('keydown', (e: KeyboardEvent) => {
  const isCtrl = e.ctrlKey || e.metaKey;
  if (isCtrl && (e.code === 'KeyF' || e.key === 'f' || e.key === 'F')) {
    e.preventDefault();
    e.stopPropagation();
    if (activePaneId) {
      const inst = tabsMap.get(activePaneId);
      if (inst) {
        showFindBar(inst.term, activePaneId);
      }
    }
  }
}, true);

// Disable browser default right-click context menu globally
window.addEventListener('contextmenu', (e: MouseEvent) => {
  e.preventDefault();
});

// Window controls setup
const winMinBtn = document.getElementById('win-min-btn') as HTMLButtonElement;
const winMaxBtn = document.getElementById('win-max-btn') as HTMLButtonElement;
const winCloseBtn = document.getElementById('win-close-btn') as HTMLButtonElement;

if (winMinBtn) {
  winMinBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    e.preventDefault();
    try {
      await getCurrentWindow().minimize();
    } catch (err) {
      console.error('Minimize failed:', err);
    }
  });
}
if (winMaxBtn) {
  winMaxBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    e.preventDefault();
    try {
      await getCurrentWindow().toggleMaximize();
    } catch (err) {
      console.error('Maximize failed:', err);
    }
  });
}
if (winCloseBtn) {
  winCloseBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    e.preventDefault();
    try {
      await fetch(`${DAEMON_URL}/windows/close`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ window: currentWindowId }),
      });
    } catch (err) {
      console.error('Close via daemon failed:', err);
      try {
        await getCurrentWindow().close();
      } catch (e2) {}
    }
  });
}

// Settings Modal Setup
const settingsBtn = document.getElementById('settings-btn') as HTMLButtonElement;
const settingsModal = new SettingsModal(DAEMON_URL, (newConfig) => {
  applyAppConfig(newConfig);
});
if (settingsBtn) settingsBtn.addEventListener('click', () => settingsModal.open());

// Top bar button listeners
const addTabBtn = document.getElementById('add-tab-btn') as HTMLButtonElement;
if (addTabBtn) {
  addTabBtn.addEventListener('click', () => {
    spawnTabWithProfile(activeAppConfig.default_profile, false);
  });
}

const tabDropdownBtn = document.getElementById('tab-dropdown-btn') as HTMLButtonElement;
if (tabDropdownBtn) {
  tabDropdownBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileSubMenu();
    contextMenuEl.style.display = 'none';
    const isVisible = profileDropdownEl.style.display === 'block';
    if (isVisible) {
      profileDropdownEl.style.display = 'none';
    } else {
      const rect = tabDropdownBtn.getBoundingClientRect();
      profileDropdownEl.style.top = `${rect.bottom + 4}px`;
      profileDropdownEl.style.left = `${rect.left}px`;
      profileDropdownEl.style.display = 'block';
    }
  });
}

renderProfileDropdownMenu();
document.body.appendChild(profileDropdownEl);

window.addEventListener('click', (e) => {
  const target = e.target as Node;
  if (contextMenuEl.contains(target) || profileDropdownEl.contains(target) || profileSubMenuEl.contains(target)) return;
  hideContextMenu();
  profileDropdownEl.style.display = 'none';
  closeProfileSubMenu();
});

(window as any).__triggerSyncTabs = syncTabs;

// Global Keyboard Shortcuts
window.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.repeat) return;

  const isCtrl = e.ctrlKey || e.metaKey;
  const isShift = e.shiftKey;

  // Open Settings Modal (Ctrl+,)
  if (isCtrl && e.code === 'Comma') {
    e.preventDefault();
    settingsModal.open();
    return;
  }

  // New Tab with Default Shell (Ctrl+Shift++)
  if (isCtrl && isShift && (e.code === 'Equal' || e.key === '+' || e.code === 'NumpadAdd')) {
    e.preventDefault();
    spawnDefaultTab();
    return;
  }

  // Close Current Tab (Ctrl+Shift+-)
  if (isCtrl && isShift && (e.code === 'Minus' || e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract')) {
    e.preventDefault();
    if (activeTabId) {
      closeTab(activeTabId);
    }
    return;
  }

  // Cycle Tabs (Ctrl+Tab / Ctrl+Shift+Tab)
  if (isCtrl && e.code === 'Tab') {
    e.preventDefault();
    cycleTabs(isShift);
    return;
  }

  // Jump to Tab Index 1..9 (Ctrl+Shift+1..9)
  if (isCtrl && isShift && e.code.startsWith('Digit')) {
    const digit = parseInt(e.code.replace('Digit', ''), 10);
    if (digit >= 1 && digit <= 9) {
      const idx = digit - 1;
      if (idx < currentLayouts.length) {
        e.preventDefault();
        const targetIds = getTabIdsInNode(currentLayouts[idx]);
        if (targetIds.length > 0) {
          switchTab(targetIds[0]);
        }
      }
    }
    return;
  }

  if (!activePaneId) return;

  const isCtrlShift = isCtrl && isShift;
  const isAltShift = e.altKey && isShift;

  if ((isCtrlShift || isAltShift) && e.code === 'ArrowRight') {
    e.preventDefault();
    splitPane(activePaneId, 'right');
  } else if ((isCtrlShift || isAltShift) && e.code === 'ArrowLeft') {
    e.preventDefault();
    splitPane(activePaneId, 'left');
  } else if ((isCtrlShift || isAltShift) && e.code === 'ArrowDown') {
    e.preventDefault();
    splitPane(activePaneId, 'down');
  } else if ((isCtrlShift || isAltShift) && e.code === 'ArrowUp') {
    e.preventDefault();
    splitPane(activePaneId, 'up');
  } else if ((isCtrlShift || isAltShift) && (e.code === 'Delete' || e.code === 'KeyW')) {
    e.preventDefault();
    unsplitPane(activePaneId);
  }
});

// ResizeObserver for terminal container
const terminalContainerEl = document.getElementById('terminal-container');
if (terminalContainerEl) {
  const resizeObserver = new ResizeObserver(() => {
    if (unsplitShrinkPending) {
      const { height: cellHeight } = getXtermCellDimensions();
      const padding = activeAppConfig.terminal_padding || 8;
      const rows = activeAppConfig.default_rows || 30;
      const settledHeight = Math.ceil(rows * cellHeight + 0.5 + 41 + padding * 2) - 41;
      if (terminalContainerEl.offsetHeight <= settledHeight) {
        setUnsplitShrinkPending(false);
      }
    }
    if (activeTabId) {
      const activeLayoutNode = currentLayouts.find((node) => containsTab(node, activeTabId!));
      if (activeLayoutNode) {
        for (const [id, instance] of tabsMap.entries()) {
          if (containsTab(activeLayoutNode, id) && instance.historyApplied && !unsplitShrinkPending) {
            instance.fitAddon.fit();
          }
        }
      }
    }
  });
  resizeObserver.observe(terminalContainerEl);
}

initHighlightsInterval();

// Export Layout Script Handler
const exportBtn = document.getElementById('export-script-btn') as HTMLButtonElement;
if (exportBtn) {
  exportBtn.addEventListener('click', () => {
    triggerExportSave();
  });
}

export async function triggerExportSave(): Promise<void> {
  try {
    const filePath = await save({
      title: 'Save YAML Layout',
      defaultPath: 'kterm-layout.yaml',
      filters: [{ name: 'YAML Session File', extensions: ['yaml', 'yml'] }],
    });

    if (!filePath) return;

    const res = await fetch(
      `${DAEMON_URL}/export-layout?window=${encodeURIComponent(currentWindowId)}`
    );
    if (!res.ok) {
      console.error('Export fetch failed:', await res.text());
      return;
    }
    const yamlText = await res.text();

    await writeTextFile(filePath, yamlText);
    console.log(`[kterm] YAML Layout saved to: ${filePath}`);

    try {
      const scRes = await fetch(`${DAEMON_URL}/export-shortcut`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: filePath }),
      });
      if (scRes.ok) {
        const scData = await scRes.json();
        console.log(`[kterm] Shortcut saved to: ${scData.shortcut}`);
      } else {
        console.error('[kterm] Shortcut creation failed:', await scRes.text());
      }
    } catch (scErr) {
      console.error('[kterm] Shortcut creation request error:', scErr);
    }
  } catch (err) {
    console.error('[kterm] Export failed:', err);
  }
}

initDaemonConnection();
setInterval(syncTabs, 2000);
