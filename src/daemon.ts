import { invoke, Channel } from '@tauri-apps/api/core';
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
import { loadInitialConfig } from './config';

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
  const onDataChannel = new Channel<string>();
  onDataChannel.onmessage = (text: string) => {
    if (!instance.historyApplied) {
      instance.historyApplied = true;
      instance.term.write(text, () => {
        const isGrowPending = callbacks ? callbacks.splitGrowPending() : false;
        if (instance.element.isConnected && !isGrowPending && !unsplitShrinkPending) {
          instance.fitAddon.fit();
        }
      });
    } else {
      instance.term.write(text);
    }
  };

  invoke('attach_pty', { tabId: instance.id, onDataChannel }).catch((err) => {
    console.error(`attach_pty IPC error on tab ${instance.id}:`, err);
  });

  const cols = activeAppConfig.default_cols || 120;
  const rows = activeAppConfig.default_rows || 30;
  const doResize = () => {
    if (instance.element.isConnected) {
      instance.term.resize(cols, rows);
      invoke('resize_pty', { tabId: instance.id, cols, rows });
    } else {
      requestAnimationFrame(doResize);
    }
  };
  doResize();

  instance.term.onResize(({ cols, rows }) => {
    invoke('resize_pty', { tabId: instance.id, cols, rows });
  });

  instance.term.onData((data) => {
    invoke('send_pty_input', { tabId: instance.id, data });
  });
}

export async function syncTabs() {
  if (!callbacks) return;
  try {
    const [remoteTabs, newLayouts] = await Promise.all([
      invoke<TabData[]>('list_tabs', { window: currentWindowId }),
      invoke<any[]>('get_window_layout', { window: currentWindowId }),
    ]);

    const remoteIds = new Set(remoteTabs.map((t) => t.id));

    const previousActiveTabId = activeTabId;
    const previousActiveGroup = previousActiveTabId
      ? currentLayouts.find((n) => callbacks!.containsTab(n, previousActiveTabId))
        ? callbacks.getTabIdsInNode(
            currentLayouts.find((n) => callbacks!.containsTab(n, previousActiveTabId))!
          )
        : []
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

    const layoutChanged =
      tabListChanged || JSON.stringify(newLayouts) !== JSON.stringify(currentLayouts);
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
  } catch (e) {
    console.warn('Failed to sync tabs via IPC', e);
  }
}

export async function initDaemonConnection() {
  await loadInitialConfig();
  await syncTabs();
}
