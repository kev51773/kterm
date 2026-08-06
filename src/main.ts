import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { renderLayoutTree, LayoutNode } from './components/SplitGrid';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SettingsModal, AppConfig } from './components/SettingsModal';

const DAEMON_URL = 'http://127.0.0.1:9999';
const WS_URL = 'ws://127.0.0.1:9999';

const urlParams = new URLSearchParams(window.location.search);
const currentWindowId = urlParams.get('window') || 'win-1';

document.title = `kterm.exe - A scriptable terminal - ${currentWindowId}`;

interface TabData {
  id: string;
  pid: number;
  profile: string;
  window_id: string;
  title: string;
  badge?: string;
  color?: string;
}

interface TabInstance {
  id: string;
  profile: string;
  title: string;
  badge?: string;
  color?: string;
  term: Terminal;
  fitAddon: FitAddon;
  ws: WebSocket | null;
  wsClosingIntentionally: boolean;
  element: HTMLElement;
}

const tabsMap = new Map<string, TabInstance>();
let activeTabId: string | null = null;
let activePaneId: string | null = null;
let currentLayouts: LayoutNode[] = [];
let activeAppConfig: AppConfig = {
  default_profile: 'powershell',
  ring_buffer_kb: 256,
  terminal_padding: 8,
  font: { family: 'Consolas, "Courier New", monospace', size: 14 },
  theme: { background: '#0d0e11', foreground: '#cccccc', highlight: '#61afef' },
};

const tabsListEl = document.getElementById('tabs-list') as HTMLElement;
const terminalContainerEl = document.getElementById('terminal-container') as HTMLElement;
const addTabBtn = document.getElementById('add-tab-btn') as HTMLButtonElement;
const tabDropdownBtn = document.getElementById('tab-dropdown-btn') as HTMLButtonElement;
const settingsBtn = document.getElementById('settings-btn') as HTMLButtonElement;
const winMinBtn = document.getElementById('win-min-btn') as HTMLButtonElement;
const winMaxBtn = document.getElementById('win-max-btn') as HTMLButtonElement;
const winCloseBtn = document.getElementById('win-close-btn') as HTMLButtonElement;

const PROFILES = [
  { id: 'powershell', label: 'PowerShell' },
  { id: 'cmd', label: 'Command Prompt' },
  { id: 'wsl', label: 'WSL' },
  { id: 'git-bash', label: 'Git Bash' },
];

// Window Dragging & Controls Setup
const tabBarEl = document.getElementById('tab-bar') as HTMLElement;
if (tabBarEl) {
  tabBarEl.addEventListener('dblclick', async (e: MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target === tabBarEl || target.id === 'tabs-list') {
      try {
        await getCurrentWindow().toggleMaximize();
      } catch (err) {
        console.error('Toggle maximize failed:', err);
      }
    }
  });
}

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
const settingsModal = new SettingsModal(DAEMON_URL, (newConfig) => {
  applyAppConfig(newConfig);
});
if (settingsBtn) settingsBtn.addEventListener('click', () => settingsModal.open());

function applyAppConfig(config: AppConfig) {
  activeAppConfig = config;
  if (config.theme?.highlight) {
    document.documentElement.style.setProperty('--highlight-color', config.theme.highlight);
  }
  for (const instance of tabsMap.values()) {
    instance.term.options.fontFamily = config.font.family;
    instance.term.options.fontSize = config.font.size;
    instance.term.options.theme = {
      background: config.theme.background,
      foreground: config.theme.foreground,
    };
    instance.fitAddon.fit();
  }
  document.querySelectorAll<HTMLElement>('.split-pane-wrapper').forEach((el) => {
    el.style.padding = `${config.terminal_padding}px`;
  });
}

async function loadInitialConfig() {
  try {
    const res = await fetch(`${DAEMON_URL}/config`);
    if (res.ok) {
      const cfg = await res.json();
      applyAppConfig(cfg);
    }
  } catch (e) {
    console.warn('Failed to load initial config from daemon', e);
  }
}

function getContainerGridDimensions() {
  const width = terminalContainerEl.clientWidth || 800;
  const height = terminalContainerEl.clientHeight || 500;
  const fontWidth = 9;
  const fontHeight = 17;
  const cols = Math.max(20, Math.floor((width - 16) / fontWidth));
  const rows = Math.max(5, Math.floor((height - 16) / fontHeight));
  return { cols, rows };
}

async function spawnTabWithProfile(profile: string = activeAppConfig.default_profile) {
  try {
    const { cols, rows } = getContainerGridDimensions();
    const res = await fetch(`${DAEMON_URL}/tabs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ profile, window: currentWindowId, cols, rows }),
    });
    if (res.ok) {
      const tabData: TabData = await res.json();
      createTabLocal(tabData);
      await syncTabs();
      switchTab(tabData.id);
    }
  } catch (e) {
    console.error(`Failed to create new tab with profile ${profile}`, e);
  }
}

// Profile Dropdown Menu Element
const profileDropdownEl = document.createElement('div');
profileDropdownEl.className = 'profile-dropdown-menu';
profileDropdownEl.style.display = 'none';

PROFILES.forEach(({ id, label }) => {
  const item = document.createElement('div');
  item.className = 'profile-dropdown-item';
  item.textContent = label;
  item.addEventListener('click', (e) => {
    e.stopPropagation();
    profileDropdownEl.style.display = 'none';
    spawnTabWithProfile(id);
  });
  profileDropdownEl.appendChild(item);
});
document.body.appendChild(profileDropdownEl);

// Context Menu Element
const contextMenuEl = document.createElement('div');
contextMenuEl.className = 'context-menu';
contextMenuEl.style.display = 'none';
document.body.appendChild(contextMenuEl);

window.addEventListener('click', () => {
  contextMenuEl.style.display = 'none';
  profileDropdownEl.style.display = 'none';
});

addTabBtn.addEventListener('click', () => {
  spawnTabWithProfile(activeAppConfig.default_profile);
});

if (tabDropdownBtn) {
  tabDropdownBtn.addEventListener('click', (e) => {
    e.stopPropagation();
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

let hasHadTabs = false;

async function syncTabs() {
  try {
    const [res, layoutRes] = await Promise.all([
      fetch(`${DAEMON_URL}/tabs?window=${encodeURIComponent(currentWindowId)}`),
      fetch(`${DAEMON_URL}/layout?window=${encodeURIComponent(currentWindowId)}`),
    ]);

    if (res.ok && layoutRes.ok) {
      const remoteTabs: TabData[] = await res.json();
      const remoteIds = new Set(remoteTabs.map((t) => t.id));

      const previousActiveTabId = activeTabId;
      const previousActiveGroup = previousActiveTabId
        ? (currentLayouts.find((n) => containsTab(n, previousActiveTabId))
            ? getTabIdsInNode(currentLayouts.find((n) => containsTab(n, previousActiveTabId))!)
            : [])
        : [];

      let tabListChanged = false;
      for (const id of tabsMap.keys()) {
        if (!remoteIds.has(id)) {
          removeTabLocal(id);
          tabListChanged = true;
        }
      }

      for (const tabData of remoteTabs) {
        if (!tabsMap.has(tabData.id)) {
          createTabLocal(tabData);
          tabListChanged = true;
        } else {
          updateTabLocal(tabData);
        }
      }

      const newLayouts: LayoutNode[] = await layoutRes.json();
      const layoutChanged = tabListChanged || JSON.stringify(newLayouts) !== JSON.stringify(currentLayouts);
      currentLayouts = newLayouts;

      if (remoteTabs.length > 0) {
        hasHadTabs = true;
      } else if (hasHadTabs && remoteTabs.length === 0) {
        getCurrentWindow().close();
        return;
      }

      if ((!activeTabId || !tabsMap.has(activeTabId)) && remoteTabs.length > 0) {
        const sibling = previousActiveGroup.find(
          (id) => id !== previousActiveTabId && tabsMap.has(id)
        );
        switchTab(sibling ?? remoteTabs[0].id);
      } else if (layoutChanged) {
        renderActiveLayout();
        renderTabBarHeaders();
      }
    }
  } catch (e) {
    console.warn('Failed to sync tabs with daemon', e);
  }
}

(window as any).__triggerSyncTabs = syncTabs;

async function initDaemonConnection() {
  let attempts = 0;
  while (attempts < 30) {
    try {
      const health = await fetch(`${DAEMON_URL}/health`);
      if (health.ok) {
        await loadInitialConfig();
        await syncTabs();
        if (tabsMap.size > 0) {
          return;
        }
      }
    } catch {
      // Daemon starting up
    }
    attempts++;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

function createTabLocal(tabData: TabData) {
  const id = tabData.id;

  const pane = document.createElement('div');
  pane.className = 'terminal-pane';
  pane.id = `pane-${id}`;

  const term = new Terminal({
    cursorBlink: true,
    fontFamily: activeAppConfig.font.family,
    fontSize: activeAppConfig.font.size,
    theme: {
      background: activeAppConfig.theme.background,
      foreground: activeAppConfig.theme.foreground,
    },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(pane);

  // Smart Ctrl+C & Custom Key Handlers
  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (e.type === 'keydown') {
      const isCtrl = e.ctrlKey || e.metaKey;
      const isShift = e.shiftKey;

      // Smart Ctrl+C
      if (isCtrl && !isShift && e.code === 'KeyC') {
        if (term.hasSelection()) {
          navigator.clipboard.writeText(term.getSelection());
          return false; // Prevent sending SIGINT when copying text
        }
        return true; // Send SIGINT (\x03) when no selection
      }

      // New Tab with Active Profile (Ctrl+Shift+T)
      if (isCtrl && isShift && e.code === 'KeyT') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat) {
          spawnDefaultTab();
        }
        return false;
      }

      // Cycle Tabs (Ctrl+Tab / Ctrl+Shift+Tab)
      if (isCtrl && e.code === 'Tab') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat) {
          cycleTabs(isShift);
        }
        return false;
      }

      // Open Settings Shortcut
      if (isCtrl && e.code === 'Comma') {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!e.repeat) {
          settingsModal.open();
        }
        return false;
      }

      // Split & Unsplit Shortcuts
      const isCtrlShift = isCtrl && isShift;
      const isAltShift = e.altKey && isShift;

      if ((isCtrlShift || isAltShift) && e.code === 'ArrowRight') {
        e.preventDefault();
        splitPane(id, 'right');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowLeft') {
        e.preventDefault();
        splitPane(id, 'left');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowDown') {
        e.preventDefault();
        splitPane(id, 'down');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowUp') {
        e.preventDefault();
        splitPane(id, 'up');
        return false;
      }
      if ((isCtrlShift || isAltShift) && (e.code === 'Delete' || e.code === 'KeyW')) {
        e.preventDefault();
        unsplitPane(id);
        return false;
      }
    }
    return true;
  });

  // Terminal Pane Context Menu (Copy/Paste)
  pane.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    showTerminalContextMenu(e.clientX, e.clientY, term);
  });

  const instance: TabInstance = {
    id,
    profile: tabData.profile,
    title: tabData.title || `Tab ${id}`,
    badge: tabData.badge,
    color: tabData.color,
    term,
    fitAddon,
    ws: null,
    wsClosingIntentionally: false,
    element: pane,
  };

  tabsMap.set(id, instance);
  connectWebSocket(instance);
}

function updateTabLocal(tabData: TabData) {
  const id = tabData.id;
  const instance = tabsMap.get(id);
  if (!instance) return;

  instance.title = tabData.title || `Tab ${id}`;
  instance.badge = tabData.badge;
  instance.color = tabData.color;
}

function connectWebSocket(instance: TabInstance) {
  const wsUrl = `${WS_URL}/tabs/${instance.id}/ws?window=${encodeURIComponent(currentWindowId)}`;
  const ws = new WebSocket(wsUrl);
  instance.ws = ws;

  ws.onopen = () => {
    instance.fitAddon.fit();
    const cols = instance.term.cols || 100;
    const rows = instance.term.rows || 30;
    ws.send(JSON.stringify({ type: 'resize', cols, rows }));
  };

  instance.term.onResize(({ cols, rows }) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'resize', cols, rows }));
    }
  });

  ws.onmessage = (event) => {
    instance.term.write(event.data);
  };

  ws.onclose = () => {
    if (instance.wsClosingIntentionally) return;
    setTimeout(() => {
      syncTabs();
    }, 100);
  };

  instance.term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  });
}

function getTabIdsInNode(node: LayoutNode): string[] {
  if (node.type === 'pane') {
    return [node.tab_id];
  }
  return [...getTabIdsInNode(node.first), ...getTabIdsInNode(node.second)];
}

function renderTabBarHeaders() {
  tabsListEl.innerHTML = '';

  for (const layoutNode of currentLayouts) {
    const paneIds = getTabIdsInNode(layoutNode);
    if (paneIds.length === 0) continue;

    const focusedPaneId = paneIds.includes(activePaneId!) ? activePaneId! : paneIds[0];
    const tabInst = tabsMap.get(focusedPaneId);
    const rawTitle = tabInst?.title || focusedPaneId;

    const displayTitle = paneIds.length > 1 ? `[${rawTitle}]` : rawTitle;

    const tabEl = document.createElement('div');
    tabEl.className = 'tab-item';
    tabEl.setAttribute('data-tauri-drag-region', 'false');
    if (activeTabId && paneIds.includes(activeTabId)) {
      tabEl.classList.add('active');
    }

    const titleEl = document.createElement('span');
    titleEl.className = 'tab-title';
    titleEl.textContent = displayTitle;
    tabEl.appendChild(titleEl);

    if (tabInst?.badge) {
      const badgeEl = document.createElement('span');
      badgeEl.className = 'tab-badge';
      badgeEl.textContent = tabInst.badge;
      tabEl.appendChild(badgeEl);
    }

    if (tabInst?.color) {
      tabEl.style.borderColor = tabInst.color;
    }

    const closeEl = document.createElement('span');
    closeEl.className = 'tab-close';
    closeEl.innerHTML = '&times;';
    closeEl.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      try {
        const res = await fetch(`${DAEMON_URL}/tabs/close`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ targets: paneIds, window: currentWindowId }),
        });
        if (res.ok) {
          await syncTabs();
        }
      } catch (e) {
        console.error('Failed to close tab group', e);
      }
    });

    tabEl.appendChild(closeEl);

    tabEl.addEventListener('click', () => {
      switchTab(focusedPaneId);
    });

    // Tab Header Right-Click Context Menu
    tabEl.addEventListener('contextmenu', (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      showTabHeaderContextMenu(e.clientX, e.clientY, paneIds[0]);
    });

    tabsListEl.appendChild(tabEl);
  }
}

function containsTab(node: LayoutNode, targetId: string): boolean {
  if (node.type === 'pane') {
    return node.tab_id === targetId;
  }
  return containsTab(node.first, targetId) || containsTab(node.second, targetId);
}

function renderActiveLayout() {
  if (!activeTabId) return;

  const activeLayoutNode = currentLayouts.find((node) => containsTab(node, activeTabId!));

  terminalContainerEl.innerHTML = '';

  // Toggle single-pane class to control focus border highlight
  if (activeLayoutNode && activeLayoutNode.type === 'pane') {
    terminalContainerEl.classList.add('single-pane');
  } else {
    terminalContainerEl.classList.remove('single-pane');
  }

  if (activeLayoutNode) {
    const layoutEl = renderLayoutTree(
      activeLayoutNode,
      (tabId: string) => {
        const inst = tabsMap.get(tabId);
        if (!inst) return null;
        inst.element.classList.add('active');
        if (tabId === activePaneId) {
          inst.element.classList.add('active-focus');
        } else {
          inst.element.classList.remove('active-focus');
        }
        return inst.element;
      },
      activePaneId,
      {
        onRatioChange: async (splitId, ratio) => {
          try {
            await fetch(`${DAEMON_URL}/layout/ratio`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ split_id: splitId, ratio }),
            });
          } catch (e) {
            console.error('Failed to update ratio', e);
          }
        },
        onPaneFocus: (tabId) => {
          setFocusedPane(tabId);
        },
      }
    );
    terminalContainerEl.appendChild(layoutEl);

    // Apply terminal padding from config
    document.querySelectorAll<HTMLElement>('.split-pane-wrapper').forEach((el) => {
      el.style.padding = `${activeAppConfig.terminal_padding}px`;
    });

    // Fix Bug 2: Recalculate fit after DOM insertion
    requestAnimationFrame(() => {
      for (const [id, instance] of tabsMap.entries()) {
        if (containsTab(activeLayoutNode, id)) {
          instance.fitAddon.fit();
        }
      }
      if (activePaneId) {
        tabsMap.get(activePaneId)?.term.focus();
      }
    });
  }
}

function setFocusedPane(tabId: string) {
  activePaneId = tabId;
  activeTabId = tabId;
  for (const [id, instance] of tabsMap.entries()) {
    if (id === tabId) {
      instance.element.classList.add('active-focus');
      instance.term.focus();
    } else {
      instance.element.classList.remove('active-focus');
    }
  }

  const wrappers = terminalContainerEl.querySelectorAll('.split-pane-wrapper');
  wrappers.forEach((w) => {
    const el = w as HTMLElement;
    if (el.dataset.tabId === tabId) {
      el.classList.add('active-focus');
    } else {
      el.classList.remove('active-focus');
    }
  });

  renderTabBarHeaders();
}

function switchTab(id: string) {
  if (!tabsMap.has(id)) return;
  activeTabId = id;
  activePaneId = id;

  renderActiveLayout();
  setFocusedPane(id);
}

async function closeTab(id: string) {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs/close`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ targets: [id], window: currentWindowId }),
    });
    if (res.ok) {
      removeTabLocal(id);
      await syncTabs();
    }
  } catch (e) {
    console.error('Failed to close tab', e);
  }
}

function removeTabLocal(id: string) {
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
    activeTabId = null;
    activePaneId = null;
  }
}

// Terminal Pane Context Menu (Copy/Paste)
function showTerminalContextMenu(x: number, y: number, term: Terminal) {
  contextMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-copy">Copy <span class="context-menu-shortcut">Ctrl+Shift+C</span></div>
    <div class="context-menu-item" id="ctx-paste">Paste <span class="context-menu-shortcut">Ctrl+Shift+V</span></div>
  `;

  contextMenuEl.style.left = `${x}px`;
  contextMenuEl.style.top = `${y}px`;
  contextMenuEl.style.display = 'block';

  document.getElementById('ctx-copy')?.addEventListener('click', () => {
    if (term.hasSelection()) {
      navigator.clipboard.writeText(term.getSelection());
    }
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-paste')?.addEventListener('click', async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        term.paste(text);
      }
    } catch (e) {
      console.error('Clipboard paste failed:', e);
    }
    contextMenuEl.style.display = 'none';
  });
}

// Tab Header Context Menu (New Tab, Split, Close)
function showTabHeaderContextMenu(x: number, y: number, targetTabId: string) {
  contextMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-new-tab">New Tab <span class="context-menu-shortcut">Ctrl+Shift+T</span></div>
    <div class="context-menu-item" id="ctx-split-right">Split Right <span class="context-menu-shortcut">Ctrl+Shift+Right</span></div>
    <div class="context-menu-item" id="ctx-split-down">Split Down <span class="context-menu-shortcut">Ctrl+Shift+Down</span></div>
    <div class="context-menu-item" id="ctx-close-tab">Close Tab</div>
  `;

  contextMenuEl.style.left = `${x}px`;
  contextMenuEl.style.top = `${y}px`;
  contextMenuEl.style.display = 'block';

  document.getElementById('ctx-new-tab')?.addEventListener('click', () => {
    spawnTabWithProfile(activeAppConfig.default_profile);
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-right')?.addEventListener('click', () => {
    splitPane(targetTabId, 'right');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-down')?.addEventListener('click', () => {
    splitPane(targetTabId, 'down');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-close-tab')?.addEventListener('click', () => {
    closeTab(targetTabId);
    contextMenuEl.style.display = 'none';
  });
}

async function splitPane(targetId: string, direction: 'right' | 'left' | 'down' | 'up') {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs/${targetId}/split`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ direction }),
    });
    if (res.ok) {
      const data = await res.json();
      await syncTabs();
      setFocusedPane(data.new_tab_id);
    }
  } catch (e) {
    console.error('Failed to split pane', e);
  }
}

async function unsplitPane(targetId: string) {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs/${targetId}/unsplit`, {
      method: 'POST',
    });
    if (res.ok) {
      await syncTabs();
      switchTab(targetId);
    }
  } catch (e) {
    console.error('Failed to unsplit pane', e);
  }
}

function cycleTabs(reverse: boolean = false) {
  const tabGroups = currentLayouts;
  if (tabGroups.length <= 1) return;

  const currentIdx = tabGroups.findIndex((node) => containsTab(node, activeTabId!));
  let nextIdx = currentIdx;

  if (reverse) {
    nextIdx = (currentIdx - 1 + tabGroups.length) % tabGroups.length;
  } else {
    nextIdx = (currentIdx + 1) % tabGroups.length;
  }

  const nextIds = getTabIdsInNode(tabGroups[nextIdx]);
  if (nextIds.length > 0) {
    switchTab(nextIds[0]);
  }
}

function spawnDefaultTab() {
  spawnTabWithProfile(activeAppConfig.default_profile);
}

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

  // New Tab with Active Profile (Ctrl+Shift+T)
  if (isCtrl && isShift && e.code === 'KeyT') {
    e.preventDefault();
    spawnDefaultTab();
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

const resizeObserver = new ResizeObserver(() => {
  if (activeTabId) {
    const activeLayoutNode = currentLayouts.find((node) => containsTab(node, activeTabId!));
    if (activeLayoutNode) {
      for (const [id, instance] of tabsMap.entries()) {
        if (containsTab(activeLayoutNode, id)) {
          instance.fitAddon.fit();
        }
      }
    }
  }
});

resizeObserver.observe(terminalContainerEl);

initDaemonConnection();
setInterval(syncTabs, 2000);

// Export Layout Script Handler
const exportBtn = document.getElementById('export-script-btn') as HTMLButtonElement;
if (exportBtn) {
  exportBtn.addEventListener('click', () => {
    triggerExportSave();
  });
}

async function triggerExportSave(): Promise<void> {
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
