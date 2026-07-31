import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

const termContainer = document.getElementById('terminal') as HTMLElement;

const term = new Terminal({
  cursorBlink: true,
  fontFamily: 'Consolas, "Courier New", monospace',
  fontSize: 14,
  theme: {
    background: '#0c0c0c',
    foreground: '#cccccc',
  },
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(termContainer);
fitAddon.fit();

window.addEventListener('resize', () => {
  fitAddon.fit();
});

async function initTerminalSession() {
  const DAEMON_URL = 'http://127.0.0.1:9999';
  const WS_URL = 'ws://127.0.0.1:9999';

  let tabId = 'tab-101';

  try {
    const res = await fetch(`${DAEMON_URL}/tabs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ profile: 'powershell' }),
    });

    if (res.ok) {
      const data = await res.json();
      if (data.id) {
        tabId = data.id;
      }
    }
  } catch (e) {
    console.warn('Failed to auto-spawn tab via REST, attempting direct WS connect', e);
  }

  connectWebSocket(`${WS_URL}/tabs/${tabId}/ws`);
}

function connectWebSocket(wsUrl: string) {
  const ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    term.write('\r\n\x1b[32m[kterm connected to daemon]\x1b[0m\r\n\r\n');
  };

  ws.onmessage = (event) => {
    term.write(event.data);
  };

  ws.onclose = () => {
    term.write('\r\n\x1b[31m[kterm disconnected from daemon]\x1b[0m\r\n');
    setTimeout(() => connectWebSocket(wsUrl), 2000);
  };

  ws.onerror = (err) => {
    console.error('WebSocket error:', err);
  };

  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  });
}

initTerminalSession();
