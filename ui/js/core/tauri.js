// The only place that touches window.__TAURI__ (injected by Tauri because withGlobalTauri = true).
const T = window.__TAURI__;
export const { invoke, convertFileSrc } = T.core;
export const { listen } = T.event;
export const dialogApi = T.dialog;
export const win = T.window.getCurrentWindow();
