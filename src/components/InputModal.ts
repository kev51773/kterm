export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export function showInputModal(options: {
  title: string;
  placeholder?: string;
  initialValue?: string;
  confirmLabel?: string;
  onConfirm: (val: string) => void;
}) {
  const overlay = document.createElement('div');
  overlay.className = 'custom-modal-overlay';

  const modal = document.createElement('div');
  modal.className = 'custom-input-modal';

  modal.innerHTML = `
    <div class="custom-modal-header">
      <span>${escapeHtml(options.title)}</span>
      <button class="custom-modal-close-btn">✕</button>
    </div>
    <div class="custom-modal-body">
      <input type="text" class="custom-modal-input" placeholder="${escapeHtml(options.placeholder || '')}" value="${escapeHtml(options.initialValue || '')}" />
    </div>
    <div class="custom-modal-footer">
      <button class="custom-modal-btn custom-modal-btn-secondary cancel-btn">Cancel</button>
      <button class="custom-modal-btn custom-modal-btn-primary confirm-btn">${escapeHtml(options.confirmLabel || 'OK')}</button>
    </div>
  `;

  overlay.appendChild(modal);
  document.body.appendChild(overlay);

  const inputEl = modal.querySelector('.custom-modal-input') as HTMLInputElement;
  const confirmBtn = modal.querySelector('.confirm-btn') as HTMLButtonElement;
  const cancelBtn = modal.querySelector('.cancel-btn') as HTMLButtonElement;
  const closeBtn = modal.querySelector('.custom-modal-close-btn') as HTMLButtonElement;

  inputEl.focus();
  inputEl.select();

  function close() {
    overlay.remove();
  }

  function handleConfirm() {
    const val = inputEl.value;
    close();
    options.onConfirm(val);
  }

  confirmBtn.addEventListener('click', handleConfirm);
  cancelBtn.addEventListener('click', close);
  closeBtn.addEventListener('click', close);

  inputEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    } else if (e.key === 'Escape') {
      close();
    }
  });

  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });
}
