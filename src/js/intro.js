import { invoke, ask } from './api.js';
import { $, showToast } from './utils.js';
import { state } from './state.js';

let introStatus = null;

export async function refreshIntroStatus() {
  const btn = $('btn-no-intro');
  if (!btn) return;
  try {
    introStatus = state.config.paksPath ? await invoke('intro_status') : null;
  } catch (err) {
    console.error('Failed to read intro status:', err);
    introStatus = null;
  }

  btn.style.display = introStatus ? '' : 'none';
  if (!introStatus) return;
  btn.innerText = introStatus.applied ? 'Restore Intro' : 'Skip Intro';
  btn.title = introStatus.applied
    ? 'Put the original startup movies back'
    : 'Replace the startup movies with blank ones so the game boots straight in';
}

export function setupIntroEvents() {
  $('btn-no-intro')?.addEventListener('click', async () => {
    if (!introStatus) return;
    try {
      if (!introStatus.applied) {
        await invoke('apply_no_intro');
        showToast('Intro movies skipped', 'success');
      } else {
        if (!introStatus.canRestore) {
          return showToast('No backup of the original intro movies. Repair the game files to get them back.', 'error');
        }
        const yes = await ask('Restore the original intro movies?', { title: 'Restore Intro' });
        if (!yes) return;
        await invoke('restore_intro');
        showToast('Intro movies restored', 'success');
      }
    } catch (err) {
      showToast(`${err}`, 'error');
    }
    await refreshIntroStatus();
  });
}
