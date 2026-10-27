//! Cursor changer commands (the logic lives in `domain::cursor`).
use crate::domain::{cursor, util::R};
use crate::state::{log, root, tmp_dir, AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn cursor_info(s: State<AppState>) -> R<cursor::CursorInfo> {
    cursor::info(&root(&s)?)
}

#[tauri::command]
pub fn cursor_inspect(app: AppHandle, s: State<AppState>, paths: Vec<String>) -> R<Vec<cursor::CItem>> {
    let auto = s.settings.lock().unwrap().auto_cursor_preset;
    cursor::inspect(&paths, &tmp_dir(&app), auto, None)
}

#[tauri::command]
pub fn cursor_add(app: AppHandle, s: State<AppState>, items: Vec<cursor::CItem>) -> R<()> {
    cursor::add(&root(&s)?, &items)?;
    log(&app, "INFO", format!("Added {} cursor(s)", items.len()));
    Ok(())
}

#[tauri::command]
pub fn cursor_rename(s: State<AppState>, id: String, name: String) -> R<()> {
    cursor::rename(&root(&s)?, &id, &name)
}

#[tauri::command]
pub fn cursor_delete(s: State<AppState>, id: String) -> R<()> {
    cursor::delete(&root(&s)?, &id)
}

#[tauri::command]
pub fn cursor_activate(app: AppHandle, s: State<AppState>, kind: String, id: Option<String>) -> R<()> {
    cursor::set_active(&root(&s)?, &kind, id.as_deref())?;
    log(&app, "INFO", format!("Cursor {kind}: {}", id.as_deref().unwrap_or("original")));
    Ok(())
}

#[tauri::command]
pub fn cursor_profile(app: AppHandle, s: State<AppState>, op: String, name: String) -> R<()> {
    cursor::profile_op(&root(&s)?, &op, &name)?;
    log(&app, "INFO", format!("Cursor preset {op}: {name}"));
    Ok(())
}

#[tauri::command]
pub fn cursor_restore_all(app: AppHandle, s: State<AppState>) -> R<()> {
    cursor::restore_all(&root(&s)?)?;
    log(&app, "INFO", "Original cursors restored");
    Ok(())
}

#[tauri::command]
pub fn cursor_reapply(s: State<AppState>) -> R<()> {
    cursor::reapply(&root(&s)?)
}
