export interface AppConfig {
  default_profile: string;
  ring_buffer_kb: number;
  terminal_padding: number;
  font: {
    family: string;
    size: number;
  };
  theme: {
    background: string;
    foreground: string;
    accent: string;
  };
}

export class SettingsModal {
  private overlay: HTMLElement | null = null;
  private currentConfig: AppConfig | null = null;
  private daemonUrl: string;
  private onSaveCallback: (config: AppConfig) => void;

  constructor(daemonUrl: string, onSaveCallback: (config: AppConfig) => void) {
    this.daemonUrl = daemonUrl;
    this.onSaveCallback = onSaveCallback;
  }

  public async open() {
    if (this.overlay) return;
    await this.fetchConfig();
    this.render();
  }

  public close() {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private async fetchConfig() {
    try {
      const res = await fetch(`${this.daemonUrl}/config`);
      if (res.ok) {
        this.currentConfig = await res.json();
      }
    } catch (e) {
      console.error('Failed to fetch config:', e);
      this.currentConfig = {
        default_profile: 'powershell',
        ring_buffer_kb: 256,
        terminal_padding: 8,
        font: { family: 'Consolas, "Courier New", monospace', size: 14 },
        theme: { background: '#0d0e11', foreground: '#cccccc', accent: '#61afef' },
      };
    }
  }

  private render() {
    if (!this.currentConfig) return;

    this.overlay = document.createElement('div');
    this.overlay.className = 'settings-modal-overlay';

    const modal = document.createElement('div');
    modal.className = 'settings-modal';

    // Header
    const header = document.createElement('div');
    header.className = 'settings-modal-header';
    header.innerHTML = `
      <span>Settings</span>
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
      closeBtn.addEventListener('mousedown', handleClose);
    }

    // Body
    const body = document.createElement('div');
    body.className = 'settings-modal-body';

    // Sidebar
    const sidebar = document.createElement('div');
    sidebar.className = 'settings-sidebar';
    sidebar.innerHTML = `
      <div class="settings-nav-item active" data-tab="appearance">Appearance</div>
      <div class="settings-nav-item" data-tab="general">General</div>
      <div class="settings-nav-item" data-tab="keybindings">Keybindings</div>
    `;

    // Content container
    const content = document.createElement('div');
    content.className = 'settings-content';

    const renderTabContent = (tabName: string) => {
      content.innerHTML = '';
      if (tabName === 'appearance') {
        content.innerHTML = `
          <div class="settings-group">
            <label>Font Family</label>
            <input type="text" id="cfg-font-family" value="${this.currentConfig!.font.family}" />
          </div>
          <div class="settings-group">
            <label>Font Size (px)</label>
            <input type="number" id="cfg-font-size" value="${this.currentConfig!.font.size}" min="8" max="36" />
          </div>
          <div class="settings-group">
            <label>Terminal Padding (px)</label>
            <input type="number" id="cfg-padding" value="${this.currentConfig!.terminal_padding}" min="0" max="32" />
          </div>
          <div class="settings-group">
            <label>Background Color</label>
            <input type="text" id="cfg-theme-bg" value="${this.currentConfig!.theme.background}" />
          </div>
          <div class="settings-group">
            <label>Foreground Color</label>
            <input type="text" id="cfg-theme-fg" value="${this.currentConfig!.theme.foreground}" />
          </div>
          <div class="settings-group">
            <label>Accent Color</label>
            <input type="text" id="cfg-theme-accent" value="${this.currentConfig!.theme.accent}" />
          </div>
        `;
      } else if (tabName === 'general') {
        content.innerHTML = `
          <div class="settings-group">
            <label>Default Profile</label>
            <select id="cfg-default-profile">
              <option value="powershell" ${this.currentConfig!.default_profile === 'powershell' ? 'selected' : ''}>PowerShell</option>
              <option value="cmd" ${this.currentConfig!.default_profile === 'cmd' ? 'selected' : ''}>Command Prompt</option>
              <option value="wsl" ${this.currentConfig!.default_profile === 'wsl' ? 'selected' : ''}>WSL</option>
              <option value="git-bash" ${this.currentConfig!.default_profile === 'git-bash' ? 'selected' : ''}>Git Bash</option>
            </select>
          </div>
          <div class="settings-group">
            <label>Ring Buffer Capacity (KB per tab)</label>
            <input type="number" id="cfg-ring-buffer" value="${this.currentConfig!.ring_buffer_kb}" min="64" max="4096" step="64" />
          </div>
        `;
      } else if (tabName === 'keybindings') {
        content.innerHTML = `
          <div class="keybindings-list">
            <div class="keybinding-row"><span>New Tab (Active Shell)</span><span class="keybinding-key">Ctrl + Shift + T</span></div>
            <div class="keybinding-row"><span>Cycle Tabs Forward</span><span class="keybinding-key">Ctrl + Tab</span></div>
            <div class="keybinding-row"><span>Cycle Tabs Backward</span><span class="keybinding-key">Ctrl + Shift + Tab</span></div>
            <div class="keybinding-row"><span>Jump to Tab 1..9</span><span class="keybinding-key">Ctrl + Shift + 1..9</span></div>
            <div class="keybinding-row"><span>Open Settings</span><span class="keybinding-key">Ctrl + ,</span></div>
            <div class="keybinding-row"><span>Smart Copy / Cancel</span><span class="keybinding-key">Ctrl + C</span></div>
            <div class="keybinding-row"><span>Split Right</span><span class="keybinding-key">Ctrl + Shift + Right</span></div>
            <div class="keybinding-row"><span>Split Down</span><span class="keybinding-key">Ctrl + Shift + Down</span></div>
          </div>
        `;
      }
    };

    renderTabContent('appearance');

    sidebar.querySelectorAll('.settings-nav-item').forEach((item) => {
      item.addEventListener('click', (e) => {
        const target = e.currentTarget as HTMLElement;
        sidebar.querySelectorAll('.settings-nav-item').forEach((i) => i.classList.remove('active'));
        target.classList.add('active');
        const tab = target.getAttribute('data-tab') || 'appearance';
        renderTabContent(tab);
      });
    });

    body.appendChild(sidebar);
    body.appendChild(content);

    // Footer
    const footer = document.createElement('div');
    footer.className = 'settings-modal-footer';
    footer.innerHTML = `
      <button class="settings-cancel-btn">Cancel</button>
      <button class="settings-save-btn">Save Settings</button>
    `;

    const cancelBtn = footer.querySelector('.settings-cancel-btn');
    if (cancelBtn) {
      cancelBtn.addEventListener('click', handleClose);
      cancelBtn.addEventListener('mousedown', handleClose);
    }

    footer.querySelector('.settings-save-btn')?.addEventListener('click', () => this.save());

    modal.appendChild(header);
    modal.appendChild(body);
    modal.appendChild(footer);
    this.overlay.appendChild(modal);

    this.overlay.addEventListener('click', (e) => {
      if (e.target === this.overlay) this.close();
    });

    const keyListener = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        window.removeEventListener('keydown', keyListener);
        this.close();
      }
    };
    window.addEventListener('keydown', keyListener);

    document.body.appendChild(this.overlay);
  }

  private async save() {
    if (!this.currentConfig) return;

    const fontFamilyEl = document.getElementById('cfg-font-family') as HTMLInputElement | null;
    if (fontFamilyEl) this.currentConfig.font.family = fontFamilyEl.value;

    const fontSizeEl = document.getElementById('cfg-font-size') as HTMLInputElement | null;
    if (fontSizeEl) this.currentConfig.font.size = parseInt(fontSizeEl.value, 10) || 14;

    const paddingEl = document.getElementById('cfg-padding') as HTMLInputElement | null;
    if (paddingEl) this.currentConfig.terminal_padding = parseInt(paddingEl.value, 10) || 8;

    const themeBgEl = document.getElementById('cfg-theme-bg') as HTMLInputElement | null;
    if (themeBgEl) this.currentConfig.theme.background = themeBgEl.value;

    const themeFgEl = document.getElementById('cfg-theme-fg') as HTMLInputElement | null;
    if (themeFgEl) this.currentConfig.theme.foreground = themeFgEl.value;

    const themeAccentEl = document.getElementById('cfg-theme-accent') as HTMLInputElement | null;
    if (themeAccentEl) this.currentConfig.theme.accent = themeAccentEl.value;

    const profileEl = document.getElementById('cfg-default-profile') as HTMLSelectElement | null;
    if (profileEl) this.currentConfig.default_profile = profileEl.value;

    const bufferEl = document.getElementById('cfg-ring-buffer') as HTMLInputElement | null;
    if (bufferEl) this.currentConfig.ring_buffer_kb = parseInt(bufferEl.value, 10) || 256;

    try {
      const res = await fetch(`${this.daemonUrl}/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(this.currentConfig),
      });
      if (res.ok) {
        this.onSaveCallback(this.currentConfig);
        this.close();
      }
    } catch (e) {
      console.error('Failed to save config:', e);
    }
  }
}
