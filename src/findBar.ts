import { Terminal } from '@xterm/xterm';
import { tabsMap, activeAppConfig } from './state';

export interface ActiveFindBar {
  el: HTMLElement;
  paneId: string;
  term: Terminal;
  matches: Array<{ line: number; col: number; len: number }>;
  currentIndex: number;
  onResize?: () => void;
  disposeResultListener?: () => void;
}

export let activeFindBar: ActiveFindBar | null = null;

export function positionFindBar() {
  if (!activeFindBar) return;
  const paneInst = tabsMap.get(activeFindBar.paneId);
  if (!paneInst) return;
  const rect = paneInst.element.getBoundingClientRect();
  activeFindBar.el.style.position = 'fixed';
  activeFindBar.el.style.top = `${Math.max(42, rect.top + 6)}px`;
  activeFindBar.el.style.right = `${Math.max(12, window.innerWidth - rect.right + 6)}px`;
  activeFindBar.el.style.zIndex = '9999';
}

export function showFindBar(term: Terminal, paneId: string) {
  if (activeFindBar) {
    closeFindBar();
  }

  const inst = tabsMap.get(paneId);
  if (!inst) return;

  term.options.theme = {
    ...term.options.theme,
    selectionBackground: '#00e5ff',
    selectionInactiveBackground: '#00e5ff',
  };

  const searchAddon = inst.findSearchAddon;

  const findEl = document.createElement('div');
  findEl.className = 'terminal-find-bar';
  findEl.innerHTML = `
    <input type="text" class="find-input" placeholder="Find in terminal..." />
    <span class="find-count">0 of 0</span>
    <button class="find-btn find-prev" title="Previous (Shift+Enter)">▲</button>
    <button class="find-btn find-next" title="Next (Enter)">▼</button>
    <button class="find-btn find-close" title="Close (Esc)">✕</button>
  `;

  document.body.appendChild(findEl);

  const inputEl = findEl.querySelector('.find-input') as HTMLInputElement;
  const countEl = findEl.querySelector('.find-count') as HTMLElement;
  const prevBtn = findEl.querySelector('.find-prev') as HTMLButtonElement;
  const nextBtn = findEl.querySelector('.find-next') as HTMLButtonElement;
  const closeBtn = findEl.querySelector('.find-close') as HTMLButtonElement;

  const onResize = () => positionFindBar();
  window.addEventListener('resize', onResize);

  const resultListener = searchAddon.onDidChangeResults(({ resultIndex, resultCount }) => {
    if (resultCount === 0) {
      countEl.textContent = 'No results';
    } else {
      countEl.textContent = `${resultIndex + 1} of ${resultCount}`;
    }
  });

  activeFindBar = {
    el: findEl,
    paneId,
    term,
    matches: [],
    currentIndex: -1,
    onResize,
    disposeResultListener: () => resultListener.dispose(),
  };

  positionFindBar();
  inputEl.focus();

  findEl.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && (e.code === 'KeyF' || e.key === 'f' || e.key === 'F')) {
      e.preventDefault();
      e.stopPropagation();
      inputEl.focus();
      inputEl.select();
    }
  });

  function performSearch() {
    if (!activeFindBar) return;
    const query = inputEl.value;
    if (!query) {
      countEl.textContent = '0 of 0';
      searchAddon.clearDecorations();
      term.clearSelection();
      return;
    }

    searchAddon.findNext(query, {
      incremental: true,
      decorations: {
        matchBackground: '#ff007f',
        matchOverviewRuler: '#ff007f',
        activeMatchBackground: '#00e5ff',
        activeMatchColorOverviewRuler: '#00e5ff',
      },
    });
  }

  function nextMatch() {
    if (!activeFindBar || !inputEl.value) return;
    searchAddon.findNext(inputEl.value, {
      decorations: {
        matchBackground: '#ff007f',
        matchOverviewRuler: '#ff007f',
        activeMatchBackground: '#00e5ff',
        activeMatchColorOverviewRuler: '#00e5ff',
      },
    });
  }

  function prevMatch() {
    if (!activeFindBar || !inputEl.value) return;
    searchAddon.findPrevious(inputEl.value, {
      decorations: {
        matchBackground: '#ff007f',
        matchOverviewRuler: '#ff007f',
        activeMatchBackground: '#00e5ff',
        activeMatchColorOverviewRuler: '#00e5ff',
      },
    });
  }

  inputEl.addEventListener('input', performSearch);

  inputEl.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && (e.code === 'KeyF' || e.key === 'f' || e.key === 'F')) {
      e.preventDefault();
      e.stopPropagation();
      inputEl.focus();
      inputEl.select();
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        prevMatch();
      } else {
        nextMatch();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeFindBar();
    }
  });

  prevBtn.addEventListener('click', prevMatch);
  nextBtn.addEventListener('click', nextMatch);
  closeBtn.addEventListener('click', closeFindBar);
}

export function closeFindBar() {
  if (activeFindBar) {
    if (activeFindBar.onResize) {
      window.removeEventListener('resize', activeFindBar.onResize);
    }
    if (activeFindBar.disposeResultListener) {
      activeFindBar.disposeResultListener();
    }
    const inst = tabsMap.get(activeFindBar.paneId);
    if (inst) {
      inst.findSearchAddon.clearDecorations();
      inst.term.options.theme = {
        ...inst.term.options.theme,
        selectionBackground: activeAppConfig.theme.highlight ? `${activeAppConfig.theme.highlight}44` : 'rgba(255, 255, 255, 0.25)',
        selectionInactiveBackground: 'transparent',
      };
    }
    activeFindBar.term.clearSelection();
    activeFindBar.el.remove();
    activeFindBar = null;
  }
}

export function getTerminalBufferText(term: Terminal): string {
  const buffer = term.buffer.active;
  const lines: string[] = [];
  for (let i = 0; i < buffer.length; i++) {
    const line = buffer.getLine(i);
    if (line) {
      lines.push(line.translateToString(true));
    }
  }
  let result = lines.join('\n');
  result = result.replace(/[\u001b\u009b][\[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g, '');
  return result;
}
