import { activeAppConfig, activeTabId, PaneInstance } from './state';

const taskStartTimes = new Map<string, number>();
const pulsingTabs = new Set<string>();
const pulseTimers = new Map<string, number>();

let audioCtx: AudioContext | null = null;

function playBellChime() {
  try {
    if (!audioCtx) {
      const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext;
      if (!AudioContextClass) return;
      audioCtx = new AudioContextClass();
    }
    if (audioCtx.state === 'suspended') {
      audioCtx.resume();
    }

    const now = audioCtx.currentTime;

    // Dual-tone chime (E6 ~ 1318.5Hz + B6 ~ 1975.5Hz)
    const osc1 = audioCtx.createOscillator();
    const osc2 = audioCtx.createOscillator();
    const gain = audioCtx.createGain();

    osc1.type = 'sine';
    osc1.frequency.setValueAtTime(1318.5, now);

    osc2.type = 'sine';
    osc2.frequency.setValueAtTime(1975.5, now);

    gain.gain.setValueAtTime(0.001, now);
    gain.gain.exponentialRampToValueAtTime(0.25, now + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.8);

    osc1.connect(gain);
    osc2.connect(gain);
    gain.connect(audioCtx.destination);

    osc1.start(now);
    osc2.start(now);
    osc1.stop(now + 0.85);
    osc2.stop(now + 0.85);
  } catch (e) {
    console.warn('Audio chime playback error:', e);
  }
}

function findTabElementForPane(paneId: string): HTMLElement | null {
  const tabElements = document.querySelectorAll<HTMLElement>('.tab-item');
  for (const el of Array.from(tabElements)) {
    const paneIdsAttr = el.getAttribute('data-pane-ids');
    if (paneIdsAttr && paneIdsAttr.split(',').includes(paneId)) {
      return el;
    }
  }
  return null;
}

export function onTerminalInput(paneId: string, data: string): void {
  // Clear pulse on user interaction
  clearTabPulseByPane(paneId);

  // Record start timestamp when user presses Enter
  if (data.includes('\r') || data.includes('\n')) {
    taskStartTimes.set(paneId, Date.now());
  }
}

export function checkTaskCompletion(instance: PaneInstance): void {
  const paneId = instance.id;
  if (!taskStartTimes.has(paneId)) return;

  const startTime = taskStartTimes.get(paneId)!;
  const elapsedMs = Date.now() - startTime;

  // Skip initial Enter keypress echo (first 300ms)
  if (elapsedMs < 300) return;

  const buffer = instance.term.buffer.active;
  const cursorLine = buffer.getLine(buffer.baseY + buffer.cursorY);
  const lineText = cursorLine ? cursorLine.translateToString(true).trim() : '';

  const isPrompt =
    lineText.endsWith('>') ||
    lineText.endsWith('$') ||
    lineText.endsWith('#') ||
    lineText.includes('PS ');

  if (!isPrompt) return;

  taskStartTimes.delete(paneId);
  const elapsedSec = elapsedMs / 1000;

  const threshold = activeAppConfig.reminder_seconds ?? 10;
  const trigger = activeAppConfig.reminder_trigger ?? 'unfocused';
  const audioEnabled = activeAppConfig.reminder_audio ?? true;
  const pulseEnabled = activeAppConfig.reminder_pulse ?? true;

  console.log(`[AttentionBell] Task finished in ${elapsedSec.toFixed(1)}s. Trigger: ${trigger}, Threshold: ${threshold}s`);

  if (trigger === 'never' || elapsedSec < threshold) {
    return;
  }

  const isWindowUnfocused = !document.hasFocus();
  const isTabUnfocused = activeTabId !== paneId;
  const isAlreadySelected = !isTabUnfocused && !isWindowUnfocused;

  if (trigger === 'unfocused' && isAlreadySelected) {
    console.log('[AttentionBell] Skipped alert: Tab & window currently focused.');
    return;
  }

  console.log('[AttentionBell] Alerting user!');
  if (audioEnabled) {
    playBellChime();
  }

  if (pulseEnabled) {
    markTabPulsingByPane(paneId, isAlreadySelected);
  }
}

export function markTabPulsingByPane(paneId: string, isAlreadySelected: boolean = false): void {
  const el = findTabElementForPane(paneId);
  if (el) {
    el.classList.add('tab-pulsing');
    pulsingTabs.add(paneId);

    if (pulseTimers.has(paneId)) {
      clearTimeout(pulseTimers.get(paneId)!);
      pulseTimers.delete(paneId);
    }

    if (isAlreadySelected) {
      const timer = window.setTimeout(() => {
        clearTabPulseByPane(paneId);
      }, 10000);
      pulseTimers.set(paneId, timer as any);
    }
  }
}

export function isPanePulsing(paneId: string): boolean {
  return pulsingTabs.has(paneId);
}

export function clearTabPulseByPane(paneId: string): void {
  if (pulseTimers.has(paneId)) {
    clearTimeout(pulseTimers.get(paneId)!);
    pulseTimers.delete(paneId);
  }
  const el = findTabElementForPane(paneId);
  if (el) {
    el.classList.remove('tab-pulsing');
  }
  pulsingTabs.delete(paneId);
}

export function clearAllTabPulses(): void {
  pulseTimers.forEach((timer) => clearTimeout(timer));
  pulseTimers.clear();
  const elements = document.querySelectorAll('.tab-item.tab-pulsing');
  elements.forEach((el) => el.classList.remove('tab-pulsing'));
  pulsingTabs.clear();
}
