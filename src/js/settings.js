import { invoke, openDialog } from './api.js';
import { $, showToast } from './utils.js';
import { refreshMods, checkLoader } from './mods.js';
import { refreshIntroStatus } from './intro.js';
import { saveConfig, showGamePath } from './config.js';
import { state } from './state.js';

async function useGamePath(path) {
  state.config = await invoke('set_game_path', { path });
  showGamePath();
  await refreshMods();
  await refreshIntroStatus();
  checkLoader();
}

export async function detectGame({ silent = false } = {}) {
  try {
    const found = await invoke('detect_game');
    if (found.length === 0) {
      if (!silent) showToast("Couldn't find Minecraft Dungeons II. Use Browse to pick the install folder.", 'error');
      return false;
    }
    await useGamePath(found[0]);
    const extra = found.length > 1 ? ` (${found.length} installs found, using the first)` : '';
    showToast(`Found game at ${found[0]}${extra}`, 'success');
    return true;
  } catch (err) {
    if (!silent) showToast(`Auto-detect failed: ${err}`, 'error');
    return false;
  }
}

export function setupConfigEvents() {
  $('game-detect-btn').addEventListener('click', () => detectGame());

  $('game-browse-btn').addEventListener('click', async () => {
    const folder = await openDialog({ directory: true, title: 'Select the Minecraft Dungeons II folder' });
    if (!folder) return;
    try {
      await useGamePath(folder);
      showToast('Game folder set', 'success');
    } catch (err) {
      showToast(`${err}`, 'error');
    }
  });

  $('theme-select').addEventListener('change', (e) => {
    const theme = e.target.value;
    document.documentElement.setAttribute('data-theme', theme);
    state.config.theme = theme;
    saveConfig();
  });

  const openWith = (id, command) => $(id)?.addEventListener('click', async () => {
    try {
      await invoke(command);
    } catch (err) {
      showToast(`Failed to open folder: ${err}`, 'error');
    }
  });
  openWith('btn-open-game-dir', 'open_game_folder');
  openWith('btn-open-config-dir', 'open_config_dir');
  openWith('btn-open-themes-dir', 'open_themes_dir');
}
