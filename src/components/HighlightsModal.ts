import { Terminal } from '@xterm/xterm';
import { paneHighlightsMap } from '../state';
import { updatePaneHighlights } from '../highlights';
import { escapeHtml } from './InputModal';

export function showHighlightsModal(_term: Terminal, paneId: string) {
  const overlay = document.createElement('div');
  overlay.className = 'custom-modal-overlay';

  const modal = document.createElement('div');
  modal.className = 'highlights-modal';

  modal.innerHTML = `
    <div class="custom-modal-header">
      <span>Pane Highlights (${escapeHtml(paneId)})</span>
      <button class="custom-modal-close-btn">✕</button>
    </div>
    <div class="custom-modal-body">
      <div class="highlights-add-row">
        <input type="text" class="highlights-input" placeholder="Add word or phrase..." />
        <button class="highlights-add-btn">Add</button>
      </div>
      <div class="highlights-list"></div>
    </div>
    <div class="custom-modal-footer">
      <button class="custom-modal-btn custom-modal-btn-primary highlights-done-btn">Done</button>
    </div>
  `;

  overlay.appendChild(modal);
  document.body.appendChild(overlay);

  const inputEl = modal.querySelector('.highlights-input') as HTMLInputElement;
  const addBtn = modal.querySelector('.highlights-add-btn') as HTMLButtonElement;
  const closeBtn = modal.querySelector('.custom-modal-close-btn') as HTMLButtonElement;
  const doneBtn = modal.querySelector('.highlights-done-btn') as HTMLButtonElement;

  function renderList() {
    const listEl = modal.querySelector('.highlights-list') as HTMLElement;
    if (!listEl) return;
    const words = paneHighlightsMap.get(paneId) || [];
    if (words.length === 0) {
      listEl.innerHTML = `<div class="highlights-empty">No active highlights for this shell.</div>`;
      return;
    }
    listEl.innerHTML = words
      .map(
        (w, idx) => `
      <div class="highlight-item">
        <span class="highlight-word-text">${escapeHtml(w)}</span>
        <button class="highlight-remove-btn" data-index="${idx}" title="Remove highlight">✕</button>
      </div>
    `
      )
      .join('');

    listEl.querySelectorAll('.highlight-remove-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        const target = e.currentTarget as HTMLElement;
        const index = parseInt(target.getAttribute('data-index') || '-1', 10);
        if (index >= 0) {
          const list = paneHighlightsMap.get(paneId) || [];
          list.splice(index, 1);
          paneHighlightsMap.set(paneId, list);
          renderList();
          updatePaneHighlights(paneId);
        }
      });
    });
  }

  renderList();
  inputEl.focus();

  function addWord() {
    const val = inputEl.value.trim();
    if (!val) return;
    const list = paneHighlightsMap.get(paneId) || [];
    if (!list.includes(val)) {
      list.push(val);
      paneHighlightsMap.set(paneId, list);
      renderList();
      updatePaneHighlights(paneId);
    }
    inputEl.value = '';
    inputEl.focus();
  }

  addBtn.addEventListener('click', addWord);
  inputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      addWord();
    } else if (e.key === 'Escape') {
      close();
    }
  });

  function close() {
    overlay.remove();
  }

  closeBtn.addEventListener('click', close);
  doneBtn.addEventListener('click', close);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });
}
