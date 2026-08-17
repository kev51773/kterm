import { invoke } from '@tauri-apps/api/core';
import {
  TabData,
  tabsMap,
  currentWindowId,
  activeTabId,
  activePaneId,
  currentLayouts,
  activeAppConfig,
  setActiveTabId,
  setActivePaneId,
} from './state';
import { getContainerGridDimensions } from './config';
import { createTabLocal, removeTabLocal } from './terminal';
import { renderActiveLayout, containsTab, getTabIdsInNode } from './splits';
import { syncTabs } from './daemon';
import { showTabHeaderContextMenu } from './components/ContextMenu';
import { clearTabPulseByPane, isPanePulsing } from './attentionBell';

let checkTabOverflowFn: (() => void) | null = null;

export function registerCheckTabOverflow(fn: () => void) {
  checkTabOverflowFn = fn;
}

export function setFocusedPane(tabId: string) {
  clearTabPulseByPane(tabId);
  setActivePaneId(tabId);
  setActiveTabId(tabId);

  for (const [id, instance] of tabsMap.entries()) {
    if (id === tabId) {
      instance.element.classList.add('active-focus');
      instance.term.focus();
    } else {
      instance.element.classList.remove('active-focus');
    }
  }

  const terminalContainerEl = document.getElementById('terminal-container');
  if (terminalContainerEl) {
    const wrappers = terminalContainerEl.querySelectorAll('.split-pane-wrapper');
    wrappers.forEach((w) => {
      const el = w as HTMLElement;
      if (el.dataset.tabId === tabId) {
        el.classList.add('active-focus');
      } else {
        el.classList.remove('active-focus');
      }
    });
  }

  renderTabBarHeaders();
}

export function switchTab(id: string) {
  if (!tabsMap.has(id)) return;
  setActiveTabId(id);
  setActivePaneId(id);

  renderActiveLayout();
  setFocusedPane(id);
}

export async function closeTab(id: string) {
  try {
    await invoke('close_tabs', { targets: [id], window: currentWindowId });
    removeTabLocal(id);
    await syncTabs();
  } catch (e) {
    console.error('Failed to close tab', e);
  }
}

export function cycleTabs(reverse: boolean = false) {
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

export async function spawnTabWithProfile(
  profile: string = activeAppConfig.default_profile,
  elevated: boolean = false
) {
  try {
    const { cols, rows } = getContainerGridDimensions();
    const tabData = await invoke<TabData>('create_tab', {
      payload: { profile, window: currentWindowId, cols, rows, elevated },
    });

    if (tabData) {
      createTabLocal(tabData);
      await syncTabs();
      switchTab(tabData.id);
    }
  } catch (e: any) {
    console.error(`Failed to create new tab with profile ${profile}`, e);
    alert(String(e) || 'Launching Administrator tabs requires running kterm as Administrator.');
  }
}

export function spawnDefaultTab() {
  spawnTabWithProfile(activeAppConfig.default_profile, false);
}

export function renderTabBarHeaders() {
  const tabsListEl = document.getElementById('tabs-list');
  if (!tabsListEl) return;

  tabsListEl.innerHTML = '';

  for (const layoutNode of currentLayouts) {
    const paneIds = getTabIdsInNode(layoutNode);
    if (paneIds.length === 0) continue;

    const focusedPaneId = paneIds.includes(activePaneId!) ? activePaneId! : paneIds[0];
    const tabInst = tabsMap.get(focusedPaneId);
    const rawTitle = tabInst?.title || focusedPaneId;

    const tabEl = document.createElement('div');
    tabEl.className = 'tab-item';
    tabEl.setAttribute('data-tauri-drag-region', 'false');
    tabEl.setAttribute('data-pane-ids', paneIds.join(','));
    if (activeTabId && paneIds.includes(activeTabId)) {
      tabEl.classList.add('active');
    }
    if (paneIds.some((pId) => isPanePulsing(pId))) {
      tabEl.classList.add('tab-pulsing');
    }

    const titleEl = document.createElement('span');
    titleEl.className = 'tab-title';

    if (paneIds.length > 1) {
      const openBracket = document.createElement('span');
      openBracket.className = 'tab-title-bracket';
      openBracket.textContent = '[';

      const titleText = document.createTextNode(rawTitle);

      const closeBracket = document.createElement('span');
      closeBracket.className = 'tab-title-bracket';
      closeBracket.textContent = ']';

      titleEl.appendChild(openBracket);
      titleEl.appendChild(titleText);
      titleEl.appendChild(closeBracket);
    } else {
      titleEl.textContent = rawTitle;
    }

    tabEl.appendChild(titleEl);

    const isElevated = paneIds.some((pId) => tabsMap.get(pId)?.elevated);
    if (isElevated) {
      tabEl.classList.add('tab-admin');
      const adminBadgeEl = document.createElement('span');
      adminBadgeEl.className = 'tab-admin-badge';
      adminBadgeEl.title = 'Running as Administrator';
      adminBadgeEl.innerHTML = `<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0c-.26 0-.51.1-.7.28L2.28 5.29A1 1 0 0 0 2 6v4c0 3.5 3.5 5.8 5.7 6a.98.98 0 0 0 .6 0C10.5 15.8 14 13.5 14 10V6a1 1 0 0 0-.28-.71L8.7 1.28A.99.99 0 0 0 8 0z"/></svg> <span>ADMIN</span>`;
      tabEl.appendChild(adminBadgeEl);
    }

    if (tabInst?.badge) {
      const badgeEl = document.createElement('span');
      badgeEl.className = 'tab-badge';
      badgeEl.textContent = tabInst.badge;
      tabEl.appendChild(badgeEl);
    }

    if (tabInst?.color) {
      tabEl.style.borderColor = tabInst.color;
    }

    const closeEl = document.createElement('button');
    closeEl.className = 'tab-close-btn';
    closeEl.setAttribute('title', 'Close Tab');
    closeEl.innerHTML = `<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z"/></svg>`;
    closeEl.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      try {
        await invoke('close_tabs', { targets: paneIds, window: currentWindowId });
        await syncTabs();
      } catch (e) {
        console.error('Failed to close tab group', e);
      }
    });

    tabEl.appendChild(closeEl);

    tabEl.addEventListener('click', () => {
      switchTab(focusedPaneId);
    });

    tabEl.addEventListener('contextmenu', (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      showTabHeaderContextMenu(e.clientX, e.clientY, focusedPaneId);
    });

    tabsListEl.appendChild(tabEl);
  }

  if (checkTabOverflowFn) checkTabOverflowFn();
  const activeTabEl = tabsListEl.querySelector('.tab-item.active');
  if (activeTabEl) {
    activeTabEl.scrollIntoView({ behavior: 'smooth', inline: 'nearest', block: 'nearest' });
  }
}
