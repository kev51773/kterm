import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { renderLayoutTree, LayoutNode } from './components/SplitGrid';

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

const tabsListEl = document.getElementById('tabs-list') as HTMLElement;
const terminalContainerEl = document.getElementById('terminal-container') as HTMLElement;
const addTabBtn = document.getElementById('add-tab-btn') as HTMLButtonElement;

// Context Menu Element
const contextMenuEl = document.createElement('div');
contextMenuEl.className = 'context-menu';
contextMenuEl.style.display = 'none';
document.body.appendChild(contextMenuEl);

window.addEventListener('click', () => {
  contextMenuEl.style.display = 'none';
});

addTabBtn.addEventListener('click', async () => {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ profile: 'powershell', window: currentWindowId }),
    });
    if (res.ok) {
      const tabData: TabData = await res.json();
      await syncTabs();
      switchTab(tabData.id);
    }
  } catch (e) {
    console.error('Failed to create new tab', e);
  }
});

async function syncTabs() {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs?window=${encodeURIComponent(currentWindowId)}`);
    if (!res.ok) return;
    const remoteTabs: TabData[] = await res.json();
    const remoteIds = new Set(remoteTabs.map((t) => t.id));

    // Snapshot siblings of the active pane BEFORE removals, so we can stay
    // within the same layout group if the active pane exits.
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

    const layoutRes = await fetch(`${DAEMON_URL}/layout?window=${encodeURIComponent(currentWindowId)}`);
    if (layoutRes.ok) {
      const newLayouts: LayoutNode[] = await layoutRes.json();
      const layoutChanged = tabListChanged || JSON.stringify(newLayouts) !== JSON.stringify(currentLayouts);
      currentLayouts = newLayouts;

      if (tabsMap.size === 0) {
        activeTabId = null;
        activePaneId = null;
        terminalContainerEl.innerHTML = '';
        renderTabBarHeaders();

        // Auto-create new tab so window is never empty or unresponsive
        fetch(`${DAEMON_URL}/tabs`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ profile: 'powershell', window: currentWindowId }),
        }).then(async (createRes) => {
          if (createRes.ok) {
            const newTab: TabData = await createRes.json();
            createTabLocal(newTab);
            switchTab(newTab.id);
          }
        });
      } else if ((!activeTabId || !tabsMap.has(activeTabId)) && remoteTabs.length > 0) {
        // Prefer a sibling pane from the same layout group before falling back to tab[0].
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


async function initDaemonConnection() {
  let attempts = 0;
  while (attempts < 30) {
    try {
      const health = await fetch(`${DAEMON_URL}/health`);
      if (health.ok) {
        await syncTabs();
        return;
      }
    } catch {
      // Daemon starting up
    }
    attempts++;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  console.error('Daemon unreachable after retries');
}

function createTabLocal(tabData: TabData) {
  const id = tabData.id;

  const pane = document.createElement('div');
  pane.className = 'terminal-pane';
  pane.id = `pane-${id}`;

  const term = new Terminal({
    cursorBlink: true,
    fontFamily: 'Consolas, "Courier New", monospace',
    fontSize: 14,
    theme: {
      background: '#0d0e11',
      foreground: '#cccccc',
    },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(pane);

  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (e.type === 'keydown') {
      const isCtrlShift = (e.ctrlKey || e.metaKey) && e.shiftKey;
      const isAltShift = e.altKey && e.shiftKey;

      if ((isCtrlShift || isAltShift) && e.code === 'ArrowRight') {
        e.preventDefault();
        e.stopPropagation();
        splitPane(id, 'right');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowLeft') {
        e.preventDefault();
        e.stopPropagation();
        splitPane(id, 'left');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowDown') {
        e.preventDefault();
        e.stopPropagation();
        splitPane(id, 'down');
        return false;
      }
      if ((isCtrlShift || isAltShift) && e.code === 'ArrowUp') {
        e.preventDefault();
        e.stopPropagation();
        splitPane(id, 'up');
        return false;
      }
      if ((isCtrlShift || isAltShift) && (e.code === 'Delete' || e.code === 'KeyW')) {
        e.preventDefault();
        e.stopPropagation();
        unsplitPane(id);
        return false;
      }
    }
    return true;
  });



  pane.addEventListener('contextmenu', (e: MouseEvent) => {
    e.preventDefault();
    showContextMenu(e.clientX, e.clientY, id);
  });

  const instance: TabInstance = {
    id,
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
  const wsUrl = `${WS_URL}/tabs/${instance.id}/ws`;
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

    // Encase title in [] for split tab groups
    const displayTitle = paneIds.length > 1 ? `[${rawTitle}]` : rawTitle;

    const tabEl = document.createElement('div');
    tabEl.className = 'tab-item';
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
          body: JSON.stringify({ targets: paneIds }),
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

    setTimeout(() => {
      for (const [id, instance] of tabsMap.entries()) {
        if (containsTab(activeLayoutNode, id)) {
          instance.fitAddon.fit();
        }
      }
      if (activePaneId) {
        tabsMap.get(activePaneId)?.term.focus();
      }
    }, 50);
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
      body: JSON.stringify({ targets: [id] }),
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


// Context Menu Logic
function showContextMenu(x: number, y: number, targetTabId: string) {
  contextMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-split-right">Split Right <span class="context-menu-shortcut">Ctrl+Shift+Right</span></div>
    <div class="context-menu-item" id="ctx-split-left">Split Left <span class="context-menu-shortcut">Ctrl+Shift+Left</span></div>
    <div class="context-menu-item" id="ctx-split-down">Split Down <span class="context-menu-shortcut">Ctrl+Shift+Down</span></div>
    <div class="context-menu-item" id="ctx-split-up">Split Up <span class="context-menu-shortcut">Ctrl+Shift+Up</span></div>
    <div class="context-menu-item" id="ctx-unsplit">Un-split Pane <span class="context-menu-shortcut">Ctrl+Shift+Del</span></div>
    <div class="context-menu-item" id="ctx-close">Close Pane</div>
  `;

  contextMenuEl.style.left = `${x}px`;
  contextMenuEl.style.top = `${y}px`;
  contextMenuEl.style.display = 'block';

  document.getElementById('ctx-split-right')?.addEventListener('click', () => {
    splitPane(targetTabId, 'right');
  });

  document.getElementById('ctx-split-left')?.addEventListener('click', () => {
    splitPane(targetTabId, 'left');
  });

  document.getElementById('ctx-split-down')?.addEventListener('click', () => {
    splitPane(targetTabId, 'down');
  });

  document.getElementById('ctx-split-up')?.addEventListener('click', () => {
    splitPane(targetTabId, 'up');
  });

  document.getElementById('ctx-unsplit')?.addEventListener('click', () => {
    unsplitPane(targetTabId);
  });

  document.getElementById('ctx-close')?.addEventListener('click', () => {
    closeTab(targetTabId);
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

// Global Keyboard Shortcuts
window.addEventListener('keydown', (e: KeyboardEvent) => {
  if (!activePaneId) return;

  const isCtrlShift = (e.ctrlKey || e.metaKey) && e.shiftKey;
  const isAltShift = e.altKey && e.shiftKey;

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



// Automatic resize observer for active layout
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
