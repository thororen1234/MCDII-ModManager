const { invoke } = window.__TAURI__.core;
const { open: openDialog } = window.__TAURI__.dialog;
const { openUrl } = window.__TAURI__.opener;
const { getCurrentWindow } = window.__TAURI__.window;
const { check: checkUpdate } = window.__TAURI__.updater;

import { ask, message } from './utils.js';

export { invoke, openDialog, message, ask, openUrl, getCurrentWindow, checkUpdate };
