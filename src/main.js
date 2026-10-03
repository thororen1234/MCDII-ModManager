import { invoke, getCurrentWindow, checkUpdate } from './js/api.js';
import { $, showToast, showModal, closeModal } from './js/utils.js';
import { state } from './js/state.js';
import { loadConfig, saveConfig, loadCustomThemes } from './js/config.js';
import { setupModsEvents, refreshMods } from './js/mods.js';
import { setupConfigEvents, detectGame } from './js/settings.js';
import { setupIntroEvents, refreshIntroStatus } from './js/intro.js';

async function init() {
  await setupWindowControls();
  await loadConfig();

  try {
    const isDev = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';
    const branch = isDev ? 'Development' : 'Stable';
    const version = window.__TAURI__?.app ? await window.__TAURI__.app.getVersion() : 'Dev';
    $('title-bar-text').innerText = `MCDII MOD MANAGER // ${branch} v${version}`;
  } catch (e) {
    console.error("Failed to load version info:", e);
  }

  setupNavigation();
  setupModsEvents();
  setupModals();
  setupUtilityEvents();
  setupCustomSelects();
  setupConfigEvents();
  setupIntroEvents();

  if (!state.config.paksPath || !(await refreshModsOk())) {
    if (!(await detectGame({ silent: true }))) {
      showToast("Couldn't find Minecraft Dungeons II automatically. Pick the game folder in Settings.", 'error');
      switchPage('settings');
    }
  }
  refreshIntroStatus();

  checkUpdates(true);
  loadCustomThemes();
  startAutoRefresh();
}

async function refreshModsOk() {
  try {
    await invoke('get_mods');
    await refreshMods();
    return true;
  } catch {
    return false;
  }
}

let refreshInterval = null;
function startAutoRefresh() {
  if (refreshInterval) clearInterval(refreshInterval);
  refreshInterval = setInterval(async () => {
    if (document.hasFocus()) {
      await refreshMods(true);
      await loadCustomThemes(true);
    }
  }, 10000);
}

document.addEventListener('DOMContentLoaded', init);

async function setupWindowControls() {
  const appWindow = getCurrentWindow();

  const setupBtn = (id, action) => {
    const el = $(id);
    if (el) el.addEventListener('click', action);
  };

  setupBtn('btn-minimize', () => appWindow.minimize());
  setupBtn('btn-maximize', () => appWindow.toggleMaximize());
  setupBtn('btn-close', () => appWindow.close());
}

function switchPage(pageId) {
  document.querySelectorAll('.page').forEach(p => p.classList.toggle('active', p.id === `page-${pageId}`));
  document.querySelectorAll('.title-tab').forEach(t => t.classList.toggle('active', t.dataset.page === pageId));
}

function setupNavigation() {
  document.querySelectorAll('.title-tab').forEach(tab => {
    tab.addEventListener('click', () => switchPage(tab.dataset.page));
  });
}

function setupModals() {
  document.querySelectorAll('.modal-overlay').forEach(overlay => {
    overlay.addEventListener('mousedown', (e) => {
      if (e.target === overlay && overlay.id !== 'modal-loading') {
        closeModal(overlay.id);
      }
    });
  });

  $('modal-rename-cancel').addEventListener('click', () => closeModal('modal-rename'));
  $('modal-rename-confirm').addEventListener('click', async () => {
    const newName = $('rename-input').value.trim();
    const oldName = $('rename-input').dataset.oldName;
    if (!newName || newName === oldName) return closeModal('modal-rename');

    try {
      await invoke('rename_mod', { oldName, newName });
      showToast(`Renamed to ${newName}`, 'success');
      refreshMods();
    } catch (err) {
      showToast(`Rename failed: ${err}`, 'error');
    }
    closeModal('modal-rename');
  });
  $('rename-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') $('modal-rename-confirm').click();
  });
}

function setupUtilityEvents() {
  $('btn-launch')?.addEventListener('click', async () => {
    try {
      await invoke('launch_game');
    } catch (err) {
      showToast(`Launch error: ${err}`, 'error');
    }
  });
}

function setupCustomSelects() {
  const customSelect = $('theme-custom-select');
  const trigger = $('theme-trigger');
  const label = $('theme-current-label');
  const optionsPanel = $('theme-options');
  const nativeSelect = $('theme-select');

  if (!customSelect || !trigger) return;

  trigger.addEventListener('click', (e) => {
    e.stopPropagation();
    customSelect.classList.toggle('open');
  });

  optionsPanel.addEventListener('click', (e) => {
    const opt = e.target.closest('.custom-option');
    if (!opt) return;

    const value = opt.dataset.value;
    const text = opt.innerText;

    label.innerText = text;
    nativeSelect.value = value;
    customSelect.classList.remove('open');

    state.config.theme = value;
    document.documentElement.setAttribute('data-theme', value);
    saveConfig();

    document.querySelectorAll('.custom-option').forEach(o => o.classList.toggle('selected', o === opt));
  });

  document.addEventListener('click', () => {
    customSelect.classList.remove('open');
  });
}

let updateManifest = null;

async function checkUpdates(silent = false) {
  const btn = $('btn-check-updates');
  if (btn && !silent) {
    btn.innerText = "Checking...";
    btn.disabled = true;
  }

  try {
    const update = await checkUpdate();
    if (update) {
      updateManifest = update;
      if (btn) {
        btn.innerText = "Update Now";
        btn.classList.add('update-ready');
        btn.disabled = false;
      }
      if (!silent) showUpdateModal();
    } else {
      if (btn && !silent) {
        btn.innerText = "Up to Date";
        setTimeout(() => {
          btn.innerText = "Check for Updates";
          btn.disabled = false;
        }, 3000);
      }
      if (!silent) showToast("You are already using the latest version.", 'info');
    }
  } catch (err) {
    console.error("Update check failed:", err);
    if (btn && !silent) {
      btn.innerText = "Check Failed";
      btn.disabled = false;
      setTimeout(() => {
        btn.innerText = "Check for Updates";
      }, 3000);
    }
    if (!silent) showToast(`Update check failed: ${err}`, 'error');
  }
}

function showUpdateModal() {
  if (!updateManifest) return;

  $('modal-update-msg').innerText = `A new version (${updateManifest.version}) is available.`;
  $('modal-update-changelog').innerText = updateManifest.body || "No changelog provided.";

  $('modal-update-now').onclick = async () => {
    try {
      $('modal-update-now').disabled = true;
      $('modal-update-now').innerText = "Downloading...";
      await updateManifest.downloadAndInstall();
      showToast("Update installed! Restarting...", 'success');
    } catch (err) {
      console.error("Update failed:", err);
      showToast(`Update failed: ${err}`, 'error');
      $('modal-update-now').disabled = false;
      $('modal-update-now').innerText = "Try Again";
    }
  };

  $('modal-update-later').onclick = () => closeModal('modal-update');
  showModal('modal-update');
}

$('btn-check-updates')?.addEventListener('click', () => {
  if (updateManifest) {
    showUpdateModal();
  } else {
    checkUpdates(false);
  }
});
