//! Mod profiles: named sets of enabled mods and their load order.
use super::edit;
use crate::domain::{
    meta::{apply_profile, load, save_profile, Profile},
    util::R,
};
use crate::state::{log, root, AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_profiles(s: State<AppState>) -> R<Vec<Profile>> {
    Ok(load(&root(&s)?)?.profiles)
}

#[tauri::command]
pub fn save_profile_cmd(app: AppHandle, s: State<AppState>, name: String) -> R<()> {
    if name.trim().is_empty() {
        return Err("Profile name is required".into());
    }
    edit(&s, |m| {
        save_profile(m, name.trim());
        Ok(())
    })?;
    log(&app, "INFO", format!("Saved profile \"{}\"", name.trim()));
    Ok(())
}

#[tauri::command]
pub fn apply_profile_cmd(app: AppHandle, s: State<AppState>, name: String) -> R<()> {
    edit(&s, |m| apply_profile(m, &name))?;
    log(&app, "INFO", format!("Applied profile \"{name}\""));
    Ok(())
}

#[tauri::command]
pub fn delete_profile(s: State<AppState>, name: String) -> R<()> {
    edit(&s, |m| {
        m.profiles.retain(|p| p.name != name);
        Ok(())
    })
}
