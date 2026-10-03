import { invoke, ask, openDialog } from './api.js';
import { $, formatDate, formatSize, showToast, setStatus, showModal, closeModal } from './utils.js';
import { saveConfig } from './config.js';
import { state } from './state.js';

const escapeHtml = (s) => String(s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

export async function refreshMods(silent = false) {
  if (!state.config.paksPath) return;
  try {
    const newMods = await invoke('get_mods');

    if (silent && JSON.stringify(newMods) === JSON.stringify(state.availableMods)) return;

    state.availableMods = newMods;
    renderMods();
    if (!silent) setStatus('● ' + "System ready");
  } catch (err) {
    console.error(err);
    if (!silent) setStatus('● ' + err, true);
  }
}

export function renderMods() {
  const container = $('mods-list');
  const emptyState = $('mods-empty');
  const searchTerm = ($('mods-search')?.value || '').toLowerCase();
  const sortBy = $('mods-sort')?.value || 'name';
  const sortOrder = $('mods-sort-order')?.classList.contains('desc') ? 'desc' : 'asc';
  const statusFilter = $('mods-status-filter')?.value || 'all';

  if (!container) return;
  container.innerHTML = '';
  const stats = $('mods-stats');
  if (stats) {
    const enabled = state.availableMods.filter(m => m.enabled && !m.isLoader).length;
    const total = state.availableMods.filter(m => !m.isLoader).length;
    stats.innerText = `${total} mods • ${enabled} enabled`;
  }

  let filtered = state.availableMods.filter(mod => {
    const matchesSearch = mod.folderName.toLowerCase().includes(searchTerm);
    const matchesStatus = statusFilter === 'all' ||
      (statusFilter === 'enabled' && mod.enabled) ||
      (statusFilter === 'disabled' && !mod.enabled);
    return matchesSearch && matchesStatus;
  });

  filtered.sort((a, b) => {
    if (a.isLoader !== b.isLoader) return a.isLoader ? -1 : 1;
    let res = 0;
    if (sortBy === 'date') res = (a.createdAt || 0) - (b.createdAt || 0);
    else if (sortBy === 'size') res = (a.size || 0) - (b.size || 0);
    else res = a.folderName.localeCompare(b.folderName);
    return sortOrder === 'desc' ? -res : res;
  });

  if (filtered.length === 0) {
    emptyState.style.display = 'block';
    emptyState.innerHTML = searchTerm
      ? `<p>No mods matching "${escapeHtml(searchTerm)}"</p>`
      : state.config.paksPath
        ? `<p>No mods found. Click "Install Mod" to add some!</p>`
        : `<p>Game folder not set. Go to Settings to pick it.</p>`;
    return;
  }

  emptyState.style.display = 'none';

  filtered.forEach((mod) => {
    const id = escapeHtml(mod.folderName);
    const locked = mod.isLoader ? 'disabled' : '';
    const modEl = document.createElement('div');
    modEl.className = `mod-item ${mod.enabled ? 'active-mod' : ''} ${mod.isLoader ? 'loader-mod' : ''}`;
    modEl.title = `Added: ${formatDate(mod.createdAt)}`;

    modEl.innerHTML = `
      <div class="col-select">
        <label class="checkbox-container">
          <input type="checkbox" class="mod-item-check" data-id="${id}" ${locked}>
          <span class="checkmark"></span>
        </label>
      </div>
      <div class="mod-info">
        <div class="mod-name">${id}${mod.isLoader ? '<span class="loader-badge">Loader</span>' : ''}</div>
        <div class="mod-meta">${mod.fileCount} file${mod.fileCount === 1 ? '' : 's'} • ${formatSize(mod.size)} • Added: ${formatDate(mod.createdAt)}</div>
      </div>
      <button class="icon-btn btn-rename-mod" data-id="${id}" title="Rename Mod" ${locked}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
      </button>
      <button class="icon-btn danger btn-delete-mod" data-id="${id}" title="Delete Mod" ${locked}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/></svg>
      </button>
      <div class="toggle-wrap" title="${mod.isLoader ? 'The Blueprint Loader is always on' : ''}">
        <label class="toggle">
          <input type="checkbox" class="mod-toggle" data-id="${id}" ${mod.enabled ? 'checked' : ''} ${locked}>
          <span class="toggle-slider"></span>
        </label>
      </div>
    `;
    container.appendChild(modEl);
  });

  attachModEvents();
  updateModsBulkUI();
}

async function setEnabled(folderNames, enabled) {
  try {
    await invoke('set_mods_enabled', { folderNames, enabled });
    return true;
  } catch (err) {
    showToast(`${err}`, 'error');
    return false;
  } finally {
    await refreshMods();
  }
}

async function deleteMods(folderNames) {
  showModal('modal-loading');
  try {
    for (const folderName of folderNames) {
      await invoke('delete_mod', { folderName });
    }
    showToast(`Deleted ${folderNames.length === 1 ? folderNames[0] : `${folderNames.length} mods`}`, 'success');
  } catch (err) {
    showToast(`Error deleting: ${err}`, 'error');
  } finally {
    closeModal('modal-loading');
    await refreshMods();
  }
}

const togglableNames = () => state.availableMods.filter(m => !m.isLoader).map(m => m.folderName);
const selectedNames = () => Array.from(document.querySelectorAll('.mod-item-check:checked')).map(cb => cb.dataset.id);

function attachModEvents() {
  document.querySelectorAll('.mod-toggle').forEach(el => {
    el.addEventListener('change', (e) => setEnabled([e.target.dataset.id], e.target.checked));
  });

  document.querySelectorAll('.btn-rename-mod').forEach(el => {
    el.addEventListener('click', (e) => {
      const folderName = e.currentTarget.dataset.id;
      $('rename-input').value = folderName;
      $('rename-input').dataset.oldName = folderName;
      showModal('modal-rename');
    });
  });

  document.querySelectorAll('.btn-delete-mod').forEach(el => {
    el.addEventListener('click', async (e) => {
      const folderName = e.currentTarget.dataset.id;
      const yes = await ask(`Are you sure you want to permanently delete '${escapeHtml(folderName)}'?`, { title: "Confirm Deletion", kind: 'warning' });
      if (yes) await deleteMods([folderName]);
    });
  });

  document.querySelectorAll('.mod-item-check').forEach(el => {
    el.addEventListener('change', updateModsBulkUI);
  });
}

function updateModsBulkUI() {
  const checks = document.querySelectorAll('.mod-item-check:checked');
  const bar = $('mods-bulk-actions');
  const count = $('mods-selected-count');
  const selectAll = $('mods-select-all');

  if (checks.length > 0) {
    bar.style.display = 'flex';
    count.innerText = `${checks.length} item${checks.length === 1 ? '' : 's'} selected`;
  } else {
    bar.style.display = 'none';
  }

  const allChecks = document.querySelectorAll('.mod-item-check:not(:disabled)');
  if (selectAll) {
    selectAll.checked = allChecks.length > 0 && checks.length === allChecks.length;
  }
}

async function installFile(path) {
  try {
    return await invoke('install_mod', { path, overwrite: false });
  } catch (err) {
    const msg = `${err}`;
    if (!msg.startsWith('EXISTS:')) throw err;
    closeModal('modal-loading');
    const yes = await ask(`'${escapeHtml(msg.slice(7))}' is already installed. Replace it?`, { title: "Replace Mod?", kind: 'warning' });
    showModal('modal-loading');
    return yes ? await invoke('install_mod', { path, overwrite: true }) : [];
  }
}

export function setupModsEvents() {
  $('mods-search')?.addEventListener('input', renderMods);
  $('mods-sort')?.addEventListener('change', () => {
    state.config.modsSort = $('mods-sort').value;
    renderMods();
    saveConfig();
  });
  $('mods-sort-order')?.addEventListener('click', () => {
    $('mods-sort-order').classList.toggle('desc');
    state.config.modsSortOrder = $('mods-sort-order').classList.contains('desc') ? 'desc' : 'asc';
    renderMods();
    saveConfig();
  });
  $('mods-status-filter')?.addEventListener('change', () => {
    state.config.modsStatusFilter = $('mods-status-filter').value;
    renderMods();
    saveConfig();
  });

  $('btn-enable-all-mods')?.addEventListener('click', async () => {
    if (await setEnabled(togglableNames(), true)) showToast("All mods enabled", 'success');
  });
  $('btn-disable-all-mods')?.addEventListener('click', async () => {
    if (await setEnabled(togglableNames(), false)) showToast("All mods disabled", 'success');
  });

  $('btn-open-mods-dir')?.addEventListener('click', async () => {
    try { await invoke('open_mods_folder'); } catch (e) { showToast(`${e}`, 'error'); }
  });

  $('btn-add-mod')?.addEventListener('click', async () => {
    if (!state.config.paksPath) return showToast("Game folder not set! Go to Settings.", 'error');

    const files = await openDialog({
      multiple: true,
      filters: [{ name: 'Mods', extensions: ['zip', 'pak', 'utoc', 'ucas'] }]
    });
    if (!files || files.length === 0) return;

    showModal('modal-loading');
    const installed = new Set();
    const failed = [];
    for (const file of files) {
      setStatus('● ' + `Installing ${file.split(/[\\/]/).pop()}...`);
      try {
        (await installFile(file)).forEach(name => installed.add(name));
      } catch (err) {
        failed.push(`${file.split(/[\\/]/).pop()}: ${err}`);
      }
    }
    closeModal('modal-loading');

    if (installed.size > 0) showToast(`Installed ${[...installed].join(', ')}`, 'success');
    failed.forEach(f => showToast(f, 'error'));
    await refreshMods();
  });

  $('mods-select-all')?.addEventListener('change', (e) => {
    document.querySelectorAll('.mod-item-check:not(:disabled)').forEach(cb => {
      cb.checked = e.target.checked;
    });
    updateModsBulkUI();
  });

  $('btn-bulk-enable-mods')?.addEventListener('click', async () => {
    const selected = selectedNames();
    if (selected.length && await setEnabled(selected, true)) showToast(`Enabled ${selected.length} mods`, 'success');
  });

  $('btn-bulk-disable-mods')?.addEventListener('click', async () => {
    const selected = selectedNames();
    if (selected.length && await setEnabled(selected, false)) showToast(`Disabled ${selected.length} mods`, 'success');
  });

  $('btn-bulk-delete-mods')?.addEventListener('click', async () => {
    const selected = selectedNames();
    if (selected.length === 0) return;
    const yes = await ask(`Are you sure you want to permanently delete ${selected.length} selected mod${selected.length === 1 ? '' : 's'}?`, { title: "Confirm Bulk Deletion", kind: 'warning' });
    if (yes) await deleteMods(selected);
  });
}
