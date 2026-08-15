import { Terminal } from '@xterm/xterm';
import { tabsMap, currentWindowId } from '../state';
import { DAEMON_URL } from '../config';
import { showFindBar } from '../findBar';
import { showInputModal, escapeHtml } from './InputModal';
import { showHighlightsModal } from './HighlightsModal';

export const contextMenuEl = document.createElement('div');
contextMenuEl.className = 'context-menu';
contextMenuEl.style.display = 'none';
document.body.appendChild(contextMenuEl);

export function positionContextMenu(x: number, y: number) {
  contextMenuEl.style.display = 'block';

  const menuWidth = contextMenuEl.offsetWidth || 180;
  const menuHeight = contextMenuEl.offsetHeight || 200;

  const winWidth = window.innerWidth;
  const winHeight = window.innerHeight;

  let finalX = x;
  let finalY = y;

  if (finalX + menuWidth > winWidth - 8) {
    finalX = Math.max(8, winWidth - menuWidth - 8);
  }

  if (finalY + menuHeight > winHeight - 8) {
    finalY = Math.max(8, winHeight - menuHeight - 8);
  }

  if (finalX + menuWidth + 180 > winWidth - 8) {
    contextMenuEl.classList.add('open-left');
  } else {
    contextMenuEl.classList.remove('open-left');
  }

  contextMenuEl.style.left = `${finalX}px`;
  contextMenuEl.style.top = `${finalY}px`;
}

export function adjustHexBrightness(hex: string, factor: number): string {
  let cleanHex = hex.trim().replace('#', '');
  if (cleanHex.length === 3) {
    cleanHex = cleanHex.split('').map((c) => c + c).join('');
  }
  let num = parseInt(cleanHex, 16);
  if (isNaN(num)) return hex;
  let r = Math.min(255, Math.max(0, Math.round(((num >> 16) & 0xff) * factor)));
  let g = Math.min(255, Math.max(0, Math.round(((num >> 8) & 0xff) * factor)));
  let b = Math.min(255, Math.max(0, Math.round((num & 0xff) * factor)));
  return '#' + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
}

export function showTabColorModal(options: {
  initialValue?: string;
  onConfirm: (val: string) => void;
}) {
  const overlay = document.createElement('div');
  overlay.className = 'custom-modal-overlay';

  const modal = document.createElement('div');
  modal.className = 'custom-input-modal';

  const initialVal = options.initialValue || '';
  let hexVal = initialVal.trim();
  if (!/^#[0-9A-Fa-f]{6}$/.test(hexVal)) {
    hexVal = '#1e88e5';
  }

  const presetSwatches = [
    '#e53935',
    '#fb8c00',
    '#fdd835',
    '#43a047',
    '#1e88e5',
    '#8e24aa',
    '#00acc1',
    '#d81b60',
  ];

  const swatchesHtml = presetSwatches
    .map(
      (c) =>
        `<button type="button" class="color-swatch-btn${initialVal.toLowerCase() === c.toLowerCase() ? ' active' : ''}" data-color="${c}" style="background-color: ${c};" title="${c}"></button>`
    )
    .join('');

  modal.innerHTML = `
    <div class="custom-modal-header">
      <span>Set Tab Color</span>
      <button class="custom-modal-close-btn">✕</button>
    </div>
    <div class="custom-modal-body">
      <div class="color-swatches-row">
        ${swatchesHtml}
        <button type="button" class="color-swatch-btn clear-swatch${!initialVal ? ' active' : ''}" data-color="" title="Clear Color">✕</button>
      </div>
      <div class="color-picker-row">
        <input type="color" class="color-picker-swatch" value="${hexVal}" title="Color Swatch" />
        <input type="range" class="color-brightness-slider" min="0" max="200" value="100" title="Adjust Brightness" />
        <input type="text" class="custom-modal-input color-picker-text" placeholder="Hex color or name (empty to clear)" value="${escapeHtml(initialVal)}" />
      </div>
    </div>
    <div class="custom-modal-footer">
      <button class="custom-modal-btn custom-modal-btn-secondary cancel-btn">Cancel</button>
      <button class="custom-modal-btn custom-modal-btn-primary confirm-btn">Set Color</button>
    </div>
  `;

  overlay.appendChild(modal);
  document.body.appendChild(overlay);

  const textEl = modal.querySelector('.custom-modal-input') as HTMLInputElement;
  const pickerEl = modal.querySelector('.color-picker-swatch') as HTMLInputElement;
  const sliderEl = modal.querySelector('.color-brightness-slider') as HTMLInputElement;
  const confirmBtn = modal.querySelector('.confirm-btn') as HTMLButtonElement;
  const cancelBtn = modal.querySelector('.cancel-btn') as HTMLButtonElement;
  const closeBtn = modal.querySelector('.custom-modal-close-btn') as HTMLButtonElement;
  const swatchBtns = modal.querySelectorAll<HTMLButtonElement>('.color-swatch-btn');

  let baseColor = initialVal;

  function updateActiveSwatch(color: string) {
    swatchBtns.forEach((btn) => {
      const btnColor = btn.getAttribute('data-color') || '';
      if (btnColor.toLowerCase() === color.toLowerCase()) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    });
  }

  swatchBtns.forEach((btn) => {
    btn.addEventListener('click', () => {
      const color = btn.getAttribute('data-color') || '';
      textEl.value = color;
      baseColor = color;
      sliderEl.value = '100';
      if (/^#[0-9A-Fa-f]{6}$/.test(color)) {
        pickerEl.value = color;
      }
      updateActiveSwatch(color);
    });
  });

  pickerEl.addEventListener('input', () => {
    baseColor = pickerEl.value;
    sliderEl.value = '100';
    textEl.value = pickerEl.value;
    updateActiveSwatch(pickerEl.value);
  });

  textEl.addEventListener('input', () => {
    const val = textEl.value.trim();
    if (/^#[0-9A-Fa-f]{6}$/.test(val)) {
      baseColor = val;
      sliderEl.value = '100';
      pickerEl.value = val;
    }
    updateActiveSwatch(val);
  });

  sliderEl.addEventListener('input', () => {
    const base = baseColor || textEl.value;
    const factor = parseInt(sliderEl.value, 10) / 100;
    const adjusted = adjustHexBrightness(base, factor);
    textEl.value = adjusted;
    if (/^#[0-9A-Fa-f]{6}$/.test(adjusted)) {
      pickerEl.value = adjusted;
    }
    updateActiveSwatch(adjusted);
  });

  function close() {
    overlay.remove();
  }

  function handleConfirm() {
    const val = textEl.value;
    close();
    options.onConfirm(val);
  }

  confirmBtn.addEventListener('click', handleConfirm);
  cancelBtn.addEventListener('click', close);
  closeBtn.addEventListener('click', close);

  textEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    } else if (e.key === 'Escape') {
      close();
    }
  });

  textEl.focus();
  textEl.select();
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });
}

// Action callbacks registered from external handlers
export interface ContextMenuActions {
  pasteToPane: (paneId: string) => void;
  splitPane: (targetId: string, direction: 'right' | 'left' | 'down' | 'up') => void;
  unsplitPane: (targetId: string) => void;
  exportPaneBuffer: (term: Terminal, paneId: string) => void;
  renderTabBarHeaders: () => void;
  syncTabs: () => Promise<void>;
  closeTab: (tabId: string) => Promise<void>;
}

let menuActions: ContextMenuActions | null = null;

export function registerContextMenuActions(actions: ContextMenuActions) {
  menuActions = actions;
}

export function showTerminalContextMenu(x: number, y: number, term: Terminal, targetPaneId: string) {
  contextMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-copy">Copy <span class="context-menu-shortcut">Ctrl+Shift+C</span></div>
    <div class="context-menu-item" id="ctx-paste">Paste <span class="context-menu-shortcut">Ctrl+Shift+V</span></div>
    <div class="context-menu-item" id="ctx-find">Find <span class="context-menu-shortcut">Ctrl+Shift+F</span></div>
    <div class="context-menu-divider"></div>
    <div class="context-menu-item has-submenu" id="ctx-split-menu">
      <span>Split</span>
      <span class="context-menu-shortcut">▶</span>
      <div class="context-submenu">
        <div class="context-menu-item" id="ctx-split-right">Split Right <span class="context-menu-shortcut">Ctrl+Shift+Right</span></div>
        <div class="context-menu-item" id="ctx-split-left">Split Left <span class="context-menu-shortcut">Ctrl+Shift+Left</span></div>
        <div class="context-menu-item" id="ctx-split-down">Split Down <span class="context-menu-shortcut">Ctrl+Shift+Down</span></div>
        <div class="context-menu-item" id="ctx-split-up">Split Up <span class="context-menu-shortcut">Ctrl+Shift+Up</span></div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-item" id="ctx-unsplit">Un-split Pane <span class="context-menu-shortcut">Ctrl+Shift+W</span></div>
      </div>
    </div>
    <div class="context-menu-item" id="ctx-highlights">Highlights...</div>
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" id="ctx-export-buffer">Export buffer</div>
  `;

  positionContextMenu(x, y);

  document.getElementById('ctx-copy')?.addEventListener('click', () => {
    if (term.hasSelection()) {
      navigator.clipboard.writeText(term.getSelection());
    }
    contextMenuEl.style.display = 'none';
    term.focus();
  });

  document.getElementById('ctx-paste')?.addEventListener('click', () => {
    if (menuActions) menuActions.pasteToPane(targetPaneId);
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-find')?.addEventListener('click', () => {
    showFindBar(term, targetPaneId);
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-right')?.addEventListener('click', () => {
    if (menuActions) menuActions.splitPane(targetPaneId, 'right');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-left')?.addEventListener('click', () => {
    if (menuActions) menuActions.splitPane(targetPaneId, 'left');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-down')?.addEventListener('click', () => {
    if (menuActions) menuActions.splitPane(targetPaneId, 'down');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-split-up')?.addEventListener('click', () => {
    if (menuActions) menuActions.splitPane(targetPaneId, 'up');
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-unsplit')?.addEventListener('click', () => {
    if (menuActions) menuActions.unsplitPane(targetPaneId);
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-highlights')?.addEventListener('click', () => {
    showHighlightsModal(term, targetPaneId);
    contextMenuEl.style.display = 'none';
  });

  document.getElementById('ctx-export-buffer')?.addEventListener('click', () => {
    if (menuActions) menuActions.exportPaneBuffer(term, targetPaneId);
    contextMenuEl.style.display = 'none';
  });
}

export function showTabHeaderContextMenu(x: number, y: number, targetTabId: string) {
  contextMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-tab-rename">Rename</div>
    <div class="context-menu-item" id="ctx-tab-badge">Set Badge...</div>
    <div class="context-menu-item" id="ctx-tab-color">Set Color...</div>
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" id="ctx-tab-close">Close Tab <span class="context-menu-shortcut">Ctrl+Shift+-</span></div>
  `;

  positionContextMenu(x, y);

  document.getElementById('ctx-tab-rename')?.addEventListener('click', () => {
    contextMenuEl.style.display = 'none';
    const currentInst = tabsMap.get(targetTabId);
    const currentTitle = currentInst?.title || '';
    showInputModal({
      title: 'Rename Tab',
      placeholder: 'Enter tab name...',
      initialValue: currentTitle,
      confirmLabel: 'Rename',
      onConfirm: async (newTitle) => {
        if (newTitle !== null && newTitle.trim() !== '') {
          try {
            if (currentInst) {
              currentInst.title = newTitle.trim();
            }
            if (menuActions) menuActions.renderTabBarHeaders();
            const res = await fetch(`${DAEMON_URL}/tabs/title`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                targets: [targetTabId],
                title: newTitle.trim(),
                window: currentWindowId,
              }),
            });
            if (res.ok && menuActions) {
              await menuActions.syncTabs();
            }
          } catch (e) {
            console.error('Failed to rename tab', e);
          }
        }
      },
    });
  });

  document.getElementById('ctx-tab-badge')?.addEventListener('click', () => {
    contextMenuEl.style.display = 'none';
    const currentInst = tabsMap.get(targetTabId);
    const currentBadge = currentInst?.badge || '';
    showInputModal({
      title: 'Set Tab Badge',
      placeholder: 'Enter badge text (leave empty to clear)...',
      initialValue: currentBadge,
      confirmLabel: 'Set Badge',
      onConfirm: async (newBadge) => {
        if (newBadge !== null) {
          const badgeVal = newBadge.trim();
          try {
            if (currentInst) {
              currentInst.badge = badgeVal;
            }
            if (menuActions) menuActions.renderTabBarHeaders();
            const res = await fetch(`${DAEMON_URL}/tabs/badge`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                targets: [targetTabId],
                badge: badgeVal,
                window: currentWindowId,
              }),
            });
            if (res.ok && menuActions) {
              await menuActions.syncTabs();
            }
          } catch (e) {
            console.error('Failed to set tab badge', e);
          }
        }
      },
    });
  });

  document.getElementById('ctx-tab-color')?.addEventListener('click', () => {
    contextMenuEl.style.display = 'none';
    const currentInst = tabsMap.get(targetTabId);
    const currentColor = currentInst?.color || '';
    showTabColorModal({
      initialValue: currentColor,
      onConfirm: async (newColor) => {
        if (newColor !== null) {
          const colorVal = newColor.trim();
          try {
            if (currentInst) {
              currentInst.color = colorVal;
            }
            if (menuActions) menuActions.renderTabBarHeaders();
            const res = await fetch(`${DAEMON_URL}/tabs/color`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                targets: [targetTabId],
                color: colorVal,
                window: currentWindowId,
              }),
            });
            if (res.ok && menuActions) {
              await menuActions.syncTabs();
            }
          } catch (e) {
            console.error('Failed to set tab color', e);
          }
        }
      },
    });
  });

  document.getElementById('ctx-tab-close')?.addEventListener('click', () => {
    if (menuActions) menuActions.closeTab(targetTabId);
    contextMenuEl.style.display = 'none';
  });
}
