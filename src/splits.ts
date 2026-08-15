import { renderLayoutTree, LayoutNode } from './components/SplitGrid';
import {
  tabsMap,
  currentLayouts,
  activeTabId,
  activePaneId,
  activeAppConfig,
  unsplitShrinkPending,
  lastAdjustedWasSplit,
  setLastAdjustedWasSplit,
  setUnsplitShrinkPending,
} from './state';
import { DAEMON_URL } from './config';
import { getXtermCellDimensions, adjustWindowForGrid } from './terminal';
import { syncTabs } from './daemon';

export function containsTab(node: LayoutNode, targetId: string): boolean {
  if (node.type === 'pane') {
    return node.tab_id === targetId;
  }
  return containsTab(node.first, targetId) || containsTab(node.second, targetId);
}

export function getTabIdsInNode(node: LayoutNode): string[] {
  if (node.type === 'pane') {
    return [node.tab_id];
  }
  return [...getTabIdsInNode(node.first), ...getTabIdsInNode(node.second)];
}

export function activeLayoutIsSplit(): boolean {
  if (!activeTabId) return false;
  const node = currentLayouts.find((n) => containsTab(n, activeTabId!));
  return !!node && node.type !== 'pane';
}

export function splitGrowPending(): boolean {
  if (!activeTabId) return false;
  const node = currentLayouts.find((n) => containsTab(n, activeTabId!));
  if (!node || node.type === 'pane') return false;
  const { height: cellHeight } = getXtermCellDimensions();
  const padding = activeAppConfig.terminal_padding || 8;
  const rows = activeAppConfig.default_rows || 30;
  const terminalContainerEl = document.getElementById('terminal-container');
  if (!terminalContainerEl) return false;
  return terminalContainerEl.offsetHeight < Math.ceil(rows * cellHeight + 8 + 0.5) + padding * 2;
}

type SetFocusedPaneFn = (id: string) => void;
let setFocusedPaneFn: SetFocusedPaneFn | null = null;

export function registerSetFocusedPane(fn: SetFocusedPaneFn) {
  setFocusedPaneFn = fn;
}

export function renderActiveLayout() {
  if (!activeTabId) return;

  const terminalContainerEl = document.getElementById('terminal-container');
  if (!terminalContainerEl) return;

  const activeLayoutNode = currentLayouts.find((node) => containsTab(node, activeTabId!));

  terminalContainerEl.innerHTML = '';

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
          if (setFocusedPaneFn) setFocusedPaneFn(tabId);
        },
      }
    );
    terminalContainerEl.appendChild(layoutEl);

    document.querySelectorAll<HTMLElement>('.split-pane-wrapper').forEach((el) => {
      el.style.padding = `${activeAppConfig.terminal_padding}px`;
    });

    requestAnimationFrame(() => {
      const isSplit = activeLayoutNode.type !== 'pane';
      const targetRows = activeAppConfig.default_rows || 30;
      const fitActivePanes = () => {
        for (const [id, instance] of tabsMap.entries()) {
          if (containsTab(activeLayoutNode, id) && instance.historyApplied && !unsplitShrinkPending) {
            instance.fitAddon.fit();
          }
        }
      };
      if (isSplit && !lastAdjustedWasSplit) {
        setLastAdjustedWasSplit(true);
        if (splitGrowPending()) {
          adjustWindowForGrid(activeAppConfig.default_cols || 120, targetRows, true);
        } else {
          fitActivePanes();
        }
      } else {
        if (!isSplit && lastAdjustedWasSplit) {
          setLastAdjustedWasSplit(false);
          setUnsplitShrinkPending(true);
          adjustWindowForGrid(activeAppConfig.default_cols || 120, activeAppConfig.default_rows || 30, true);
        } else {
          fitActivePanes();
        }
      }
      if (activePaneId) {
        tabsMap.get(activePaneId)?.term.focus();
      }
    });
  }
}

export async function splitPane(targetId: string, direction: 'right' | 'left' | 'down' | 'up') {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs/${targetId}/split`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ direction }),
    });
    if (res.ok) {
      const data = await res.json();
      await syncTabs();
      if (setFocusedPaneFn) setFocusedPaneFn(data.new_tab_id);
    }
  } catch (e) {
    console.error('Failed to split pane', e);
  }
}

export async function unsplitPane(targetId: string) {
  try {
    const res = await fetch(`${DAEMON_URL}/tabs/${targetId}/unsplit`, {
      method: 'POST',
    });
    if (res.ok) {
      await syncTabs();
      const instance = tabsMap.get(targetId);
      if (instance && setFocusedPaneFn) {
        setFocusedPaneFn(targetId);
      }
    }
  } catch (e) {
    console.error('Failed to unsplit pane', e);
  }
}
