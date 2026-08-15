import { PROFILES } from '../config';

export const profileDropdownEl = document.createElement('div');
profileDropdownEl.className = 'profile-dropdown-menu';

export const profileSubMenuEl = document.createElement('div');
profileSubMenuEl.className = 'context-menu profile-submenu';
profileSubMenuEl.style.display = 'none';
document.body.appendChild(profileSubMenuEl);

export function closeProfileSubMenu() {
  profileSubMenuEl.style.display = 'none';
}

export interface ProfileDropdownActions {
  spawnTabWithProfile: (profile: string, elevated: boolean) => Promise<void>;
  triggerExportSave: () => Promise<void>;
  openSettingsModal: () => void;
}

let dropdownActions: ProfileDropdownActions | null = null;

export function registerProfileDropdownActions(actions: ProfileDropdownActions) {
  dropdownActions = actions;
}

export function openProfileSubMenu(profileId: string, clientX: number, clientY: number) {
  profileSubMenuEl.innerHTML = `
    <div class="context-menu-item" id="ctx-profile-admin">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="margin-right:6px; color:#ff9800;">
        <path d="M8 0c-.26 0-.51.1-.7.28L2.28 5.29A1 1 0 0 0 2 6v4c0 3.5 3.5 5.8 5.7 6a.98.98 0 0 0 .6 0C10.5 15.8 14 13.5 14 10V6a1 1 0 0 0-.28-.71L8.7 1.28A.99.99 0 0 0 8 0z"/>
      </svg>
      Run as Admin
    </div>
  `;

  profileSubMenuEl.style.top = `${clientY}px`;
  profileSubMenuEl.style.left = `${clientX}px`;
  profileSubMenuEl.style.display = 'block';

  document.getElementById('ctx-profile-admin')?.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileSubMenu();
    profileDropdownEl.style.display = 'none';
    if (dropdownActions) {
      dropdownActions.spawnTabWithProfile(profileId, true);
    }
  });
}

export function renderProfileDropdownMenu() {
  profileDropdownEl.innerHTML = '';

  PROFILES.forEach(({ id, label }) => {
    const item = document.createElement('div');
    item.className = 'profile-dropdown-item';
    item.textContent = label;

    item.addEventListener('click', (e) => {
      e.stopPropagation();
      closeProfileSubMenu();
      profileDropdownEl.style.display = 'none';
      if (dropdownActions) {
        dropdownActions.spawnTabWithProfile(id, false);
      }
    });

    item.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      e.stopPropagation();
      openProfileSubMenu(id, e.clientX, e.clientY);
    });

    profileDropdownEl.appendChild(item);
  });

  const divider = document.createElement('div');
  divider.className = 'context-menu-divider';
  profileDropdownEl.appendChild(divider);

  const exportItem = document.createElement('div');
  exportItem.className = 'profile-dropdown-item';
  exportItem.innerHTML = `Export Layout...`;
  exportItem.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileSubMenu();
    profileDropdownEl.style.display = 'none';
    if (dropdownActions) {
      dropdownActions.triggerExportSave();
    }
  });
  profileDropdownEl.appendChild(exportItem);

  const settingsItem = document.createElement('div');
  settingsItem.className = 'profile-dropdown-item';
  settingsItem.innerHTML = `Settings <span class="context-menu-shortcut">Ctrl+,</span>`;
  settingsItem.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileSubMenu();
    profileDropdownEl.style.display = 'none';
    if (dropdownActions) {
      dropdownActions.openSettingsModal();
    }
  });
  profileDropdownEl.appendChild(settingsItem);
}
