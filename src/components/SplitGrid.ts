export type SplitDirection = 'horizontal' | 'vertical';

export type LayoutNode =
  | { type: 'pane'; tab_id: string }
  | {
      type: 'split';
      id: string;
      direction: SplitDirection;
      ratio: number;
      first: LayoutNode;
      second: LayoutNode;
    };

export interface SplitGridCallbacks {
  onRatioChange: (splitId: string, ratio: number) => void;
  onPaneFocus: (tabId: string) => void;
}

export function renderLayoutTree(
  node: LayoutNode,
  getPaneElement: (tabId: string) => HTMLElement | null,
  activePaneId: string | null,
  callbacks: SplitGridCallbacks
): HTMLElement {
  if (node.type === 'pane') {
    const wrapper = document.createElement('div');
    wrapper.className = 'split-pane-wrapper';
    if (activePaneId && node.tab_id === activePaneId) {
      wrapper.classList.add('active-focus');
    }
    wrapper.dataset.tabId = node.tab_id;

    const paneEl = getPaneElement(node.tab_id);
    if (paneEl) {
      wrapper.appendChild(paneEl);
    }

    wrapper.addEventListener('mousedown', () => {
      callbacks.onPaneFocus(node.tab_id);
    });

    return wrapper;
  }

  const container = document.createElement('div');
  container.className = `split-container ${node.direction}`;
  container.dataset.splitId = node.id;

  const firstWrapper = document.createElement('div');
  firstWrapper.className = 'split-sub-container';
  firstWrapper.style.flex = `${node.ratio} ${node.ratio} 0%`;

  const firstChild = renderLayoutTree(node.first, getPaneElement, activePaneId, callbacks);
  firstWrapper.appendChild(firstChild);

  const divider = document.createElement('div');
  divider.className = `split-divider ${node.direction}`;

  const secondWrapper = document.createElement('div');
  secondWrapper.className = 'split-sub-container';
  const remainingRatio = 1 - node.ratio;
  secondWrapper.style.flex = `${remainingRatio} ${remainingRatio} 0%`;

  const secondChild = renderLayoutTree(node.second, getPaneElement, activePaneId, callbacks);
  secondWrapper.appendChild(secondChild);

  // Drag handle logic
  let isDragging = false;

  divider.addEventListener('mousedown', (e: MouseEvent) => {
    e.preventDefault();
    isDragging = true;
    document.body.style.cursor = node.direction === 'horizontal' ? 'col-resize' : 'row-resize';
  });

  const onMouseMove = (e: MouseEvent) => {
    if (!isDragging) return;
    const rect = container.getBoundingClientRect();
    let newRatio = node.ratio;

    if (node.direction === 'horizontal') {
      const offsetX = e.clientX - rect.left;
      newRatio = offsetX / rect.width;
    } else {
      const offsetY = e.clientY - rect.top;
      newRatio = offsetY / rect.height;
    }

    newRatio = Math.max(0.1, Math.min(0.9, newRatio));
    firstWrapper.style.flex = `${newRatio} ${newRatio} 0%`;
    secondWrapper.style.flex = `${1 - newRatio} ${1 - newRatio} 0%`;
  };

  const onMouseUp = () => {
    if (isDragging) {
      isDragging = false;
      document.body.style.cursor = '';
      const flexVal = parseFloat(firstWrapper.style.flexGrow || `${node.ratio}`);
      callbacks.onRatioChange(node.id, flexVal);
    }
  };

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);

  container.appendChild(firstWrapper);
  container.appendChild(divider);
  container.appendChild(secondWrapper);

  return container;
}
