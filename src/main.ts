import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

const DAEMON_URL = 'http://127.0.0.1:9999';
const WS_URL = 'ws://127.0.0.1:9999';

// Extract window label from URL parameter (e.g., ?window=win-2) or default to 'win-1'
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
  term: Terminal;
  fitAddon: FitAddon;
  ws: WebSocket | null;
  element: HTMLElement;
  tabElement: HTMLElement;
}

const tabsMap = new Map<string, TabInstance>();
let activeTabId: string | null = null;

const tabsListEl = document.getElementById('tabs-list') as HTMLElement;
const terminalContainerEl = document.getElementById('terminal-container') as HTMLElement;
const addTabBtn = document.getElementById('add-tab-btn') as HTMLButtonElement;

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

    // Remove tabs that no longer belong to this window or daemon
    for (const id of tabsMap.keys()) {
      if (!remoteIds.has(id)) {
        removeTabLocal(id);
      }
    }

    // Add or update remote tabs belonging to this window
    for (const tabData of remoteTabs) {
      if (!tabsMap.has(tabData.id)) {
        createTabLocal(tabData);
      } else {
        updateTabLocal(tabData);
      }
    }

    if (!activeTabId && remoteTabs.length > 0) {
      switchTab(remoteTabs[0].id);
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
      // Daemon is starting up, retry shortly
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
  terminalContainerEl.appendChild(pane);

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

  const tabEl = document.createElement('div');
  tabEl.className = 'tab-item';
  tabEl.id = `tab-header-${id}`;

  const titleEl = document.createElement('span');
  titleEl.className = 'tab-title';
  titleEl.textContent = tabData.title || `Tab ${id}`;

  const closeEl = document.createElement('span');
  closeEl.className = 'tab-close';
  closeEl.innerHTML = '&times;';
  closeEl.addEventListener('click', (ev) => {
    ev.stopPropagation();
    closeTab(id);
  });

  tabEl.appendChild(titleEl);
  if (tabData.badge) {
    const badgeEl = document.createElement('span');
    badgeEl.className = 'tab-badge';
    badgeEl.textContent = tabData.badge;
    tabEl.appendChild(badgeEl);
  }
  tabEl.appendChild(closeEl);

  tabEl.addEventListener('click', () => {
    switchTab(id);
  });

  tabsListEl.appendChild(tabEl);

  const instance: TabInstance = {
    id,
    term,
    fitAddon,
    ws: null,
    element: pane,
    tabElement: tabEl,
  };

  tabsMap.set(id, instance);
  connectWebSocket(instance);
}

function updateTabLocal(tabData: TabData) {
  const id = tabData.id;
  const instance = tabsMap.get(id);
  if (!instance) return;

  const titleEl = instance.tabElement.querySelector('.tab-title');
  if (titleEl) {
    titleEl.textContent = tabData.title || `Tab ${id}`;
  }

  let badgeEl = instance.tabElement.querySelector('.tab-badge');
  if (tabData.badge) {
    if (!badgeEl) {
      badgeEl = document.createElement('span');
      badgeEl.className = 'tab-badge';
      const closeBtn = instance.tabElement.querySelector('.tab-close');
      instance.tabElement.insertBefore(badgeEl, closeBtn);
    }
    badgeEl.textContent = tabData.badge;
  } else if (badgeEl) {
    badgeEl.remove();
  }

  if (tabData.color) {
    instance.tabElement.style.borderColor = tabData.color;
  }
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
    instance.term.write('\r\n\x1b[31m[Disconnected]\x1b[0m\r\n');
  };

  instance.term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  });
}

function switchTab(id: string) {
  if (!tabsMap.has(id)) return;
  activeTabId = id;

  for (const [tabId, instance] of tabsMap.entries()) {
    if (tabId === id) {
      instance.element.classList.add('active');
      instance.tabElement.classList.add('active');
      setTimeout(() => {
        instance.fitAddon.fit();
        instance.term.focus();
      }, 30);
    } else {
      instance.element.classList.remove('active');
      instance.tabElement.classList.remove('active');
    }
  }
}

async function closeTab(id: string) {
  try {
    await fetch(`${DAEMON_URL}/tabs/close`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ targets: [id] }),
    });
    removeTabLocal(id);
  } catch (e) {
    console.error('Failed to close tab', e);
  }
}

function removeTabLocal(id: string) {
  const instance = tabsMap.get(id);
  if (!instance) return;

  if (instance.ws) {
    instance.ws.close();
  }
  instance.term.dispose();
  instance.element.remove();
  instance.tabElement.remove();
  tabsMap.delete(id);

  if (activeTabId === id) {
    activeTabId = null;
    const remainingIds = Array.from(tabsMap.keys());
    if (remainingIds.length > 0) {
      switchTab(remainingIds[0]);
    }
  }
}

// Automatic resize observer for perfect terminal alignment
const resizeObserver = new ResizeObserver(() => {
  if (activeTabId) {
    const instance = tabsMap.get(activeTabId);
    if (instance) {
      instance.fitAddon.fit();
    }
  }
});

resizeObserver.observe(terminalContainerEl);

// Initialize with daemon retry loop & periodic sync
initDaemonConnection();
setInterval(syncTabs, 2000);
