import { invoke } from '@tauri-apps/api/core';

export interface AppConfig {
  default_profile: string;
  default_cols?: number;
  default_rows?: number;
  ring_buffer_kb: number;
  terminal_padding: number;
  font: {
    family: string;
    size: number;
  };
  theme: {
    background: string;
    foreground: string;
    highlight: string;
    title_bar?: string;
    active_tab?: string;
    inactive_tab?: string;
    tab_hover?: string;
    active_tab_fg?: string;
    inactive_tab_fg?: string;
  };
}

function adjustHexBrightness(hex: string, factor: number): string {
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

function renderColorInput(id: string, label: string, val: string): string {
  let hexVal = val ? val.trim() : '#000000';
  if (!/^#[0-9A-Fa-f]{6}$/.test(hexVal)) {
    hexVal = '#000000';
  }
  return `
    <div class="settings-group">
      <label>${label}</label>
      <div class="color-picker-row">
        <input type="color" id="${id}-picker" value="${hexVal}" class="color-picker-swatch" title="Color Swatch" />
        <input type="range" id="${id}-brightness" min="0" max="200" value="100" class="color-brightness-slider" title="Adjust Brightness" />
        <input type="text" id="${id}" value="${val}" class="color-picker-text" />
      </div>
    </div>
  `;
}

export class SettingsModal {
  private overlay: HTMLElement | null = null;
  private currentConfig: AppConfig | null = null;
  private initialConfig: AppConfig | null = null;
  private isSaved: boolean = false;
  private onSaveCallback: (config: AppConfig) => void;

  constructor(arg1: any, arg2?: (config: AppConfig) => void) {
    if (typeof arg1 === 'function') {
      this.onSaveCallback = arg1;
    } else if (arg2) {
      this.onSaveCallback = arg2;
    } else {
      this.onSaveCallback = () => {};
    }
  }

  public async open() {
    if (this.overlay) return;
    this.isSaved = false;
    await this.fetchConfig();
    if (this.currentConfig) {
      this.initialConfig = JSON.parse(JSON.stringify(this.currentConfig));
    }
    this.render();
  }

  public close() {
    if (!this.isSaved && this.initialConfig) {
      const restoreConfig = this.initialConfig;
      this.initialConfig = null;
      this.onSaveCallback(restoreConfig);
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private async fetchConfig() {
    try {
      this.currentConfig = await invoke<AppConfig>('get_config');
    } catch (e) {
      console.error('Failed to fetch config:', e);
      this.currentConfig = {
        default_profile: 'powershell',
        default_cols: 120,
        default_rows: 30,
        ring_buffer_kb: 256,
        terminal_padding: 8,
        font: { family: 'Consolas, "Courier New", monospace', size: 14 },
        theme: { background: '#0d0e11', foreground: '#cccccc', highlight: '#61afef' },
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
        const curFont = this.currentConfig!.font.family || 'Consolas, monospace';
        const FONT_OPTIONS = [
          { label: 'Consolas', value: 'Consolas, monospace' },
          { label: 'Cascadia Code', value: 'Cascadia Code, Consolas, monospace' },
          { label: 'Cascadia Mono', value: 'Cascadia Mono, Consolas, monospace' },
          { label: 'Courier New', value: 'Courier New, monospace' },
          { label: 'Lucida Console', value: 'Lucida Console, monospace' },
          { label: 'Fira Code', value: 'Fira Code, Consolas, monospace' },
          { label: 'JetBrains Mono', value: 'JetBrains Mono, Consolas, monospace' },
          { label: 'Source Code Pro', value: 'Source Code Pro, Consolas, monospace' },
          { label: 'Custom Font...', value: 'custom' },
        ];

        const normCur = curFont.replace(/['"]/g, '').toLowerCase().trim();
        let matchedPreset = FONT_OPTIONS.find((o) => {
          if (o.value === 'custom') return false;
          const normVal = o.value.replace(/['"]/g, '').toLowerCase().trim();
          const normLabel = o.label.replace(/['"]/g, '').toLowerCase().trim();
          return normCur === normVal || normCur.startsWith(normLabel);
        });
        let isCustom = !matchedPreset;
        let selectedPresetVal = matchedPreset ? matchedPreset.value : 'custom';

        content.innerHTML = `
          <div class="settings-section-header">Font & Layout</div>
          <div class="settings-group">
            <label>Font Family</label>
            <select id="cfg-font-family-select">
              ${FONT_OPTIONS.map((o) => `<option value="${o.value}" ${o.value === selectedPresetVal ? 'selected' : ''}>${o.label}</option>`).join('')}
            </select>
            <input type="text" id="cfg-font-family-custom" value="${curFont}" style="margin-top: 4px; display: ${isCustom ? 'block' : 'none'};" placeholder="Enter custom font family..." />
          </div>
          <div class="settings-group">
            <label>Font Size (px)</label>
            <input type="number" id="cfg-font-size" value="${this.currentConfig!.font.size}" min="8" max="36" />
          </div>
          <div class="settings-group">
            <label>Terminal Padding (px)</label>
            <input type="number" id="cfg-padding" value="${this.currentConfig!.terminal_padding}" min="0" max="32" />
          </div>

          <div class="settings-section-header">Title & Tab Colors</div>
          ${renderColorInput('cfg-theme-title-bar', 'Title Bar Background', this.currentConfig!.theme.title_bar || '#21252b')}
          ${renderColorInput('cfg-theme-active-tab', 'Active Tab Background', this.currentConfig!.theme.active_tab || '#0d0e11')}
          ${renderColorInput('cfg-theme-active-tab-fg', 'Active Tab Font Color', this.currentConfig!.theme.active_tab_fg || '#ffffff')}
          ${renderColorInput('cfg-theme-inactive-tab', 'Inactive Tab Background', this.currentConfig!.theme.inactive_tab || '#181a1f')}
          ${renderColorInput('cfg-theme-inactive-tab-fg', 'Inactive Tab Font Color', this.currentConfig!.theme.inactive_tab_fg || '#abb2bf')}
          ${renderColorInput('cfg-theme-tab-hover', 'Tab Hover Background', this.currentConfig!.theme.tab_hover || '#282c34')}

          <div class="settings-section-header">Terminal Palette</div>
          ${renderColorInput('cfg-theme-bg', 'Background Color', this.currentConfig!.theme.background)}
          ${renderColorInput('cfg-theme-fg', 'Foreground Color', this.currentConfig!.theme.foreground)}
          ${renderColorInput('cfg-theme-highlight', 'Highlight Color', this.currentConfig!.theme.highlight)}
        `;

        const fontFamilySelect = content.querySelector('#cfg-font-family-select') as HTMLSelectElement | null;
        const fontFamilyCustom = content.querySelector('#cfg-font-family-custom') as HTMLInputElement | null;
        const fontSizeInput = content.querySelector('#cfg-font-size') as HTMLInputElement | null;
        const paddingInput = content.querySelector('#cfg-padding') as HTMLInputElement | null;

        const applyLivePreview = () => {
          if (!this.currentConfig) return;

          let fontVal = fontFamilySelect?.value || 'custom';
          if (fontVal === 'custom') {
            fontVal = fontFamilyCustom?.value.trim() || 'Consolas, monospace';
          }
          if (fontVal) this.currentConfig.font.family = fontVal;

          const sizeVal = parseInt(fontSizeInput?.value || '14', 10);
          if (!isNaN(sizeVal)) this.currentConfig.font.size = sizeVal;

          const padVal = parseInt(paddingInput?.value || '8', 10);
          if (!isNaN(padVal)) this.currentConfig.terminal_padding = padVal;

          const bg = (content.querySelector('#cfg-theme-bg') as HTMLInputElement | null)?.value;
          const fg = (content.querySelector('#cfg-theme-fg') as HTMLInputElement | null)?.value;
          const hl = (content.querySelector('#cfg-theme-highlight') as HTMLInputElement | null)?.value;
          const tb = (content.querySelector('#cfg-theme-title-bar') as HTMLInputElement | null)?.value;
          const at = (content.querySelector('#cfg-theme-active-tab') as HTMLInputElement | null)?.value;
          const it = (content.querySelector('#cfg-theme-inactive-tab') as HTMLInputElement | null)?.value;
          const th = (content.querySelector('#cfg-theme-tab-hover') as HTMLInputElement | null)?.value;
          const af = (content.querySelector('#cfg-theme-active-tab-fg') as HTMLInputElement | null)?.value;
          const inf = (content.querySelector('#cfg-theme-inactive-tab-fg') as HTMLInputElement | null)?.value;

          if (bg) this.currentConfig.theme.background = bg;
          if (fg) this.currentConfig.theme.foreground = fg;
          if (hl) this.currentConfig.theme.highlight = hl;
          if (tb) this.currentConfig.theme.title_bar = tb;
          if (at) this.currentConfig.theme.active_tab = at;
          if (it) this.currentConfig.theme.inactive_tab = it;
          if (th) this.currentConfig.theme.tab_hover = th;
          if (af) this.currentConfig.theme.active_tab_fg = af;
          if (inf) this.currentConfig.theme.inactive_tab_fg = inf;

          this.onSaveCallback(this.currentConfig);
        };

        if (fontFamilySelect && fontFamilyCustom) {
          fontFamilySelect.addEventListener('change', () => {
            if (fontFamilySelect.value === 'custom') {
              fontFamilyCustom.style.display = 'block';
            } else {
              fontFamilyCustom.style.display = 'none';
            }
            applyLivePreview();
          });
          fontFamilyCustom.addEventListener('input', applyLivePreview);
        }
        if (fontSizeInput) fontSizeInput.addEventListener('input', applyLivePreview);
        if (paddingInput) paddingInput.addEventListener('input', applyLivePreview);

        const colorIds = [
          'cfg-theme-bg',
          'cfg-theme-fg',
          'cfg-theme-highlight',
          'cfg-theme-title-bar',
          'cfg-theme-active-tab',
          'cfg-theme-active-tab-fg',
          'cfg-theme-inactive-tab',
          'cfg-theme-inactive-tab-fg',
          'cfg-theme-tab-hover',
        ];

        const baseColorMap = new Map<string, string>();
        colorIds.forEach((id) => {
          const textEl = content.querySelector(`#${id}`) as HTMLInputElement | null;
          const pickerEl = content.querySelector(`#${id}-picker`) as HTMLInputElement | null;
          const sliderEl = content.querySelector(`#${id}-brightness`) as HTMLInputElement | null;

          if (textEl && pickerEl && sliderEl) {
            baseColorMap.set(id, textEl.value);

            pickerEl.addEventListener('input', () => {
              baseColorMap.set(id, pickerEl.value);
              sliderEl.value = '100';
              textEl.value = pickerEl.value;
              applyLivePreview();
            });

            textEl.addEventListener('input', () => {
              const val = textEl.value.trim();
              if (/^#[0-9A-Fa-f]{6}$/.test(val)) {
                baseColorMap.set(id, val);
                sliderEl.value = '100';
                pickerEl.value = val;
              }
              applyLivePreview();
            });

            sliderEl.addEventListener('input', () => {
              const base = baseColorMap.get(id) || textEl.value;
              const factor = parseInt(sliderEl.value, 10) / 100;
              const adjusted = adjustHexBrightness(base, factor);
              textEl.value = adjusted;
              if (/^#[0-9A-Fa-f]{6}$/.test(adjusted)) {
                pickerEl.value = adjusted;
              }
              applyLivePreview();
            });
          }
        });
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
          <div class="settings-group">
            <label>Default Grid Columns (Width)</label>
            <input type="number" id="cfg-default-cols" value="${this.currentConfig!.default_cols || 120}" min="40" max="300" />
          </div>
          <div class="settings-group">
            <label>Default Grid Rows (Height)</label>
            <input type="number" id="cfg-default-rows" value="${this.currentConfig!.default_rows || 30}" min="10" max="150" />
          </div>
          <div class="settings-group">
            <label>Ring Buffer Capacity (KB per tab)</label>
            <input type="number" id="cfg-ring-buffer" value="${this.currentConfig!.ring_buffer_kb}" min="64" max="4096" step="64" />
          </div>
        `;
        const colsIn = content.querySelector('#cfg-default-cols') as HTMLInputElement | null;
        if (colsIn) {
          colsIn.addEventListener('input', () => {
            const v = parseInt(colsIn.value, 10);
            if (!isNaN(v) && v > 0) this.currentConfig!.default_cols = v;
          });
        }
        const rowsIn = content.querySelector('#cfg-default-rows') as HTMLInputElement | null;
        if (rowsIn) {
          rowsIn.addEventListener('input', () => {
            const v = parseInt(rowsIn.value, 10);
            if (!isNaN(v) && v > 0) this.currentConfig!.default_rows = v;
          });
        }
      } else if (tabName === 'keybindings') {
        content.innerHTML = `
          <div class="keybindings-list">
            <div class="keybinding-row"><span>New Tab (Default Shell)</span><span class="keybinding-key">Ctrl + Shift + +</span></div>
            <div class="keybinding-row"><span>Close Current Tab</span><span class="keybinding-key">Ctrl + Shift + -</span></div>
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

    const saveBtn = footer.querySelector('.settings-save-btn');
    const handleSave = (e: Event) => {
      e.stopPropagation();
      e.preventDefault();
      this.save();
    };
    if (saveBtn) {
      saveBtn.addEventListener('click', handleSave);
      saveBtn.addEventListener('mousedown', handleSave);
    }

    modal.appendChild(header);
    modal.appendChild(body);
    modal.appendChild(footer);
    this.overlay.appendChild(modal);

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

    const fontFamilySelect = document.getElementById('cfg-font-family-select') as HTMLSelectElement | null;
    const fontFamilyCustom = document.getElementById('cfg-font-family-custom') as HTMLInputElement | null;
    if (fontFamilySelect) {
      if (fontFamilySelect.value === 'custom' && fontFamilyCustom) {
        this.currentConfig.font.family = fontFamilyCustom.value.trim() || 'Consolas, monospace';
      } else if (fontFamilySelect.value !== 'custom') {
        this.currentConfig.font.family = fontFamilySelect.value;
      }
    }

    const fontSizeEl = document.getElementById('cfg-font-size') as HTMLInputElement | null;
    if (fontSizeEl) this.currentConfig.font.size = parseInt(fontSizeEl.value, 10) || 14;

    const paddingEl = document.getElementById('cfg-padding') as HTMLInputElement | null;
    if (paddingEl) this.currentConfig.terminal_padding = parseInt(paddingEl.value, 10) || 8;

    const themeBgEl = document.getElementById('cfg-theme-bg') as HTMLInputElement | null;
    if (themeBgEl) this.currentConfig.theme.background = themeBgEl.value;

    const themeFgEl = document.getElementById('cfg-theme-fg') as HTMLInputElement | null;
    if (themeFgEl) this.currentConfig.theme.foreground = themeFgEl.value;

    const themeHighlightEl = document.getElementById('cfg-theme-highlight') as HTMLInputElement | null;
    if (themeHighlightEl) this.currentConfig.theme.highlight = themeHighlightEl.value;

    const themeTitleBarEl = document.getElementById('cfg-theme-title-bar') as HTMLInputElement | null;
    if (themeTitleBarEl) this.currentConfig.theme.title_bar = themeTitleBarEl.value;

    const themeActiveTabEl = document.getElementById('cfg-theme-active-tab') as HTMLInputElement | null;
    if (themeActiveTabEl) this.currentConfig.theme.active_tab = themeActiveTabEl.value;

    const themeInactiveTabEl = document.getElementById('cfg-theme-inactive-tab') as HTMLInputElement | null;
    if (themeInactiveTabEl) this.currentConfig.theme.inactive_tab = themeInactiveTabEl.value;

    const profileEl = document.getElementById('cfg-default-profile') as HTMLSelectElement | null;
    if (profileEl) this.currentConfig.default_profile = profileEl.value;

    const colsEl = document.getElementById('cfg-default-cols') as HTMLInputElement | null;
    if (colsEl) this.currentConfig.default_cols = parseInt(colsEl.value, 10) || 120;

    const rowsEl = document.getElementById('cfg-default-rows') as HTMLInputElement | null;
    if (rowsEl) this.currentConfig.default_rows = parseInt(rowsEl.value, 10) || 30;

    const bufferEl = document.getElementById('cfg-ring-buffer') as HTMLInputElement | null;
    if (bufferEl) this.currentConfig.ring_buffer_kb = parseInt(bufferEl.value, 10) || 256;

    this.isSaved = true;
    try {
      this.onSaveCallback(this.currentConfig);
    } catch (e) {
      console.error('Error during onSaveCallback:', e);
    }
    this.close();

    try {
      await invoke('update_config', { config: this.currentConfig });
    } catch (e) {
      console.error('Failed to save config via IPC:', e);
    }
  }
}
