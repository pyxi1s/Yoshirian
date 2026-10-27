//! In-app log viewer.
use crate::state::{log, AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_logs(s: State<AppState>) -> Vec<String> {
    let l = s.logs.lock().unwrap();
    l[l.len().saturating_sub(500)..].to_vec()
}

#[tauri::command]
pub fn clear_logs(s: State<AppState>) {
    s.logs.lock().unwrap().clear();
}

#[tauri::command]
pub fn log_ui(app: AppHandle, level: String, msg: String) {
    log(&app, if level == "error" { "ERROR" } else { "DEBUG" }, msg);
}
