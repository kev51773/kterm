import { IMarker, IDecoration } from '@xterm/xterm';
import { tabsMap, paneHighlightsMap, activePaneDecorationsMap } from './state';

export function clearPaneHighlightDecorations(paneId: string) {
  const decs = activePaneDecorationsMap.get(paneId);
  if (decs) {
    for (const item of decs) {
      item.decoration.dispose();
      item.marker.dispose();
    }
    activePaneDecorationsMap.delete(paneId);
  }
}

export let highlightFlashPhase = 0;

export function updatePaneHighlights(paneId: string) {
  const inst = tabsMap.get(paneId);
  if (!inst) return;

  clearPaneHighlightDecorations(paneId);

  const words = paneHighlightsMap.get(paneId);
  if (!words || words.length === 0) return;

  const validWords = words.filter((w) => w.trim().length > 0);
  if (validWords.length === 0) return;

  const buffer = inst.term.buffer.active;
  const createdDecs: Array<{ marker: IMarker; decoration: IDecoration }> = [];
  const isPhaseA = highlightFlashPhase === 0;
  const matchBg = isPhaseA ? '#f1c40f' : '#0044ff';
  const matchFg = isPhaseA ? '#0033cc' : '#ffff00';

  for (let i = 0; i < buffer.length; i++) {
    const lineObj = buffer.getLine(i);
    if (!lineObj) continue;
    const str = lineObj.translateToString(true);
    if (!str) continue;

    for (const rawWord of validWords) {
      const escaped = rawWord.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(escaped, 'gi');
      let match: RegExpExecArray | null;

      while ((match = regex.exec(str)) !== null) {
        const col = match.index;
        const width = match[0].length;
        const matchedText = match[0];
        const markerOffset = i - (buffer.baseY + buffer.cursorY);
        const marker = inst.term.registerMarker(markerOffset);
        if (marker) {
          const decoration = inst.term.registerDecoration({
            marker,
            x: col,
            width,
            backgroundColor: matchBg,
            overviewRulerOptions: { color: matchBg, position: 'center' },
          });
          if (decoration) {
            decoration.onRender((el: HTMLElement) => {
              el.textContent = matchedText;
              el.style.color = matchFg;
              el.style.backgroundColor = matchBg;
              el.style.fontWeight = 'bold';
              el.style.display = 'flex';
              el.style.alignItems = 'center';
              el.style.pointerEvents = 'none';
              el.style.userSelect = 'none';
              el.style.overflow = 'hidden';
              el.style.whiteSpace = 'pre';
            });
            createdDecs.push({ marker, decoration });
          }
        }
      }
    }
  }

  activePaneDecorationsMap.set(paneId, createdDecs);
}

export function initHighlightsInterval() {
  setInterval(() => {
    let hasAnyHighlights = false;
    for (const words of paneHighlightsMap.values()) {
      if (words && words.length > 0) {
        hasAnyHighlights = true;
        break;
      }
    }
    if (!hasAnyHighlights) return;

    highlightFlashPhase = (highlightFlashPhase + 1) % 2;

    for (const [paneId, words] of paneHighlightsMap.entries()) {
      if (words && words.length > 0) {
        updatePaneHighlights(paneId);
      }
    }
  }, 600);
}
