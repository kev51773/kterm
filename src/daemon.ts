import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  PaneInstance,
  TabData,
  tabsMap,
  currentWindowId,
  activeTabId,
  currentLayouts,
  hasHadTabs,
  setHasHadTabs,
  setCurrentLayouts,
  activeAppConfig,
  unsplitShrinkPending,
} from './state';
import { DAEMON_URL, WS_URL, loadInitialConfig } from './config';

export interface DaemonCallbacks {
  createTabLocal: (data: TabData) => void;
  updateTabLocal: (data: TabData) => void;
  removeTabLocal: (id: string) => void;
  switchTab: (id: string) => void;
  renderActiveLayout: () => void;
  renderTabBarHeaders: () => void;
  containsTab: (node: any, targetId: string) => boolean;
  getTabIdsInNode: (node: any) => string[];
  splitGrowPending: () => boolean;
}

let callbacks: DaemonCallbacks | null = null;

export function registerDaemonCallbacks(cb: DaemonCallbacks) {
  callbacks = cb;
}

export function connectWebSocket(instance: PaneInstance) {
  const wsUrl = `${WS_URL}/tabs/${instance.id}/ws?window=${encodeURIComponent(currentWindowId)}`;
  const ws = new WebSocket(wsUrl);
  instance.ws = ws;

  ws.onopen = () => {
    const cols = activeAppConfig.default_cols || 120;
    const rows = activeAppConfig.default_rows || 30;
    const doResize = () => {
      if (instance.element.isConnected) {
        instance.term.resize(cols, rows);
        ws.send(JSON.stringify({ type: 'resize', cols, rows }));
      } else {
        requestAnimationFrame(doResize);
      }
    };
    doResize();
  };

  instance.term.onResize(({ cols, rows }) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'resize', cols, rows }));
    }
  });

  ws.onmessage = (event) => {
    if (!instance.historyApplied) {
      instance.historyApplied = true;
      instance.term.write(event.data, () => {
        const isGrowPending = callbacks ? callbacks.splitGrowPending() : false;
        if (instance.element.isConnected && !isGrowPending && !unsplitShrinkPending) {
          instance.fitAddon.fit();
        }
      });
    } else {
      instance.term.write(event.data);
    }
  };

  ws.onerror = (err) => {
    console.error(`WebSocket error on tab ${instance.id}:`, err);
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

export async function syncTabs() {
  if (!callbacks) return;
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
        ? (currentLayouts.find((n) => callbacks!.containsTab(n, previousActiveTabId))
            ? callbacks.getTabIdsInNode(currentLayouts.find((n) => callbacks!.containsTab(n, previousActiveTabId))!)
            : [])
        : [];

      let tabListChanged = false;
      for (const id of tabsMap.keys()) {
        if (!remoteIds.has(id)) {
          callbacks.removeTabLocal(id);
          tabListChanged = true;
        }
      }

      for (const tabData of remoteTabs) {
        if (!tabsMap.has(tabData.id)) {
          callbacks.createTabLocal(tabData);
          tabListChanged = true;
        } else {
          callbacks.updateTabLocal(tabData);
        }
      }

      const newLayouts = await layoutRes.json();
      const layoutChanged = tabListChanged || JSON.stringify(newLayouts) !== JSON.stringify(currentLayouts);
      setCurrentLayouts(newLayouts);

      if (remoteTabs.length > 0) {
        setHasHadTabs(true);
      } else if (hasHadTabs && remoteTabs.length === 0) {
        getCurrentWindow().close();
        return;
      }

      if ((!activeTabId || !tabsMap.has(activeTabId)) && remoteTabs.length > 0) {
        const sibling = previousActiveGroup.find(
          (id) => id !== previousActiveTabId && tabsMap.has(id)
        );
        callbacks.switchTab(sibling ?? remoteTabs[0].id);
      } else {
        if (layoutChanged) {
          callbacks.renderActiveLayout();
        }
        callbacks.renderTabBarHeaders();
      }
    }
  } catch (e) {
    console.warn('Failed to sync tabs with daemon', e);
  }
}

export async function initDaemonConnection() {
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
