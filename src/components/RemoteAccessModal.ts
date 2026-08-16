import { invoke } from '@tauri-apps/api/core';

export interface RemoteStatus {
  enabled: boolean;
  port: number;
  address: string | null;
}

export class RemoteAccessModal {
  private overlay: HTMLElement | null = null;

  public async open() {
    if (this.overlay) return;
    this.render();
    await this.updateStatus();
  }

  public close() {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private render() {
    this.overlay = document.createElement('div');
    this.overlay.className = 'settings-modal-overlay';

    const modal = document.createElement('div');
    modal.className = 'settings-modal';
    modal.style.maxWidth = '460px';

    // Header
    const header = document.createElement('div');
    header.className = 'settings-modal-header';
    header.innerHTML = `
      <span>Remote Access</span>
      <button class="settings-close-btn">&times;</button>
    `;

    const closeBtn = header.querySelector('.settings-close-btn');
    const handleClose = (e: Event) => {
      e.stopPropagation();
      e.preventDefault();
      this.close();
    };
    if (closeBtn) {
      closeBtn.addEventListener('click', handleClose);
    }

    // Body
    const body = document.createElement('div');
    body.className = 'settings-modal-body';
    body.style.flexDirection = 'column';
    body.style.padding = '20px';

    body.innerHTML = `
      <div style="margin-bottom: 16px; font-size: 13px; color: #8b949e; line-height: 1.4;">
        Access your desktop terminal sessions securely from a mobile device or browser on your local network.
      </div>
      
      <div class="settings-group">
        <label>Server Status</label>
        <div id="remote-status-badge" style="display:inline-block; padding:4px 10px; border-radius:6px; background:#21262d; color:#8b949e; font-weight:600; font-size:12px;">Loading...</div>
      </div>
      
      <div style="background:#161b22; border:1px solid #30363d; border-radius:8px; padding:16px; margin-top:12px;">
        <div class="settings-section-header" style="margin-top:0;">Server & Password Setup</div>
        <div class="settings-group">
          <label>Port</label>
          <input type="number" id="remote-port-input" value="8080" min="1024" max="65535" style="width:100%; padding:8px; background:#0d0e11; border:1px solid #30363d; border-radius:6px; color:#cccccc;" />
        </div>
        <div class="settings-group">
          <label>Password</label>
          <input type="password" id="remote-pw-input" placeholder="Enter password to secure server" style="width:100%; padding:8px; background:#0d0e11; border:1px solid #30363d; border-radius:6px; color:#cccccc;" />
        </div>
        <div id="remote-err-msg" style="color:#f85149; font-size:12px; margin-bottom:12px; display:none;"></div>
        <div style="display:flex; gap:10px; margin-top:16px;">
          <button id="btn-start-remote" class="settings-save-btn" style="flex:1;">Start Remote Access</button>
          <button id="btn-stop-remote" class="settings-cancel-btn" style="display:none;">Stop Server</button>
        </div>
      </div>
    `;

    // Footer
    const footer = document.createElement('div');
    footer.className = 'settings-modal-footer';
    footer.innerHTML = `<button class="settings-cancel-btn">Close</button>`;
    const cancelBtn = footer.querySelector('.settings-cancel-btn');
    if (cancelBtn) cancelBtn.addEventListener('click', handleClose);

    modal.appendChild(header);
    modal.appendChild(body);
    modal.appendChild(footer);
    this.overlay.appendChild(modal);

    const startBtn = body.querySelector('#btn-start-remote');
    if (startBtn) {
      startBtn.addEventListener('click', async () => {
        const portIn = body.querySelector('#remote-port-input') as HTMLInputElement | null;
        const pwIn = body.querySelector('#remote-pw-input') as HTMLInputElement | null;
        const errEl = body.querySelector('#remote-err-msg') as HTMLElement | null;
        if (errEl) errEl.style.display = 'none';

        const port = parseInt(portIn?.value || '8080', 10);
        const password = pwIn?.value.trim() || '';

        if (!password) {
          if (errEl) {
            errEl.textContent = 'Password is required to enable Remote Access';
            errEl.style.display = 'block';
          }
          return;
        }

        try {
          await invoke('enable_remote_server', { port, password });
          await this.updateStatus();
        } catch (e: any) {
          if (errEl) {
            errEl.textContent = String(e || 'Failed to start server');
            errEl.style.display = 'block';
          }
        }
      });
    }

    const stopBtn = body.querySelector('#btn-stop-remote');
    if (stopBtn) {
      stopBtn.addEventListener('click', async () => {
        try {
          await invoke('disable_remote_server');
          await this.updateStatus();
        } catch (e) {
          console.error('Failed to stop remote server:', e);
        }
      });
    }

    const keyListener = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        window.removeEventListener('keydown', keyListener);
        this.close();
      }
    };
    window.addEventListener('keydown', keyListener);

    document.body.appendChild(this.overlay);
  }

  private async updateStatus() {
    if (!this.overlay) return;
    try {
      const st = await invoke<RemoteStatus>('get_remote_status');
      const badge = this.overlay.querySelector('#remote-status-badge') as HTMLElement | null;
      const stopBtn = this.overlay.querySelector('#btn-stop-remote') as HTMLElement | null;
      const startBtn = this.overlay.querySelector('#btn-start-remote') as HTMLElement | null;
      const portIn = this.overlay.querySelector('#remote-port-input') as HTMLInputElement | null;
      if (portIn && st.port) portIn.value = String(st.port);

      if (badge) {
        if (st.enabled && st.address) {
          badge.style.background = '#238636';
          badge.style.color = '#ffffff';
          badge.textContent = `LISTENING (${st.address})`;
          if (stopBtn) stopBtn.style.display = 'inline-block';
          if (startBtn) startBtn.textContent = 'Update & Restart Server';
        } else {
          badge.style.background = '#21262d';
          badge.style.color = '#8b949e';
          badge.textContent = 'STOPPED';
          if (stopBtn) stopBtn.style.display = 'none';
          if (startBtn) startBtn.textContent = 'Start Remote Access';
        }
      }
    } catch (e) {
      console.error('Failed to get remote status:', e);
    }
  }
}
