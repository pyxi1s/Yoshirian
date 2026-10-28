//! Settings, game folder detection and "open folder" buttons.
use crate::domain::{
    meta::META,
    paths::{addons, detect, valid_root},
    util::{e, R},
};
use crate::state::{log, persist, root, settings_path, AppState, Settings};
use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(s: State<AppState>) -> Settings {
    s.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, s: State<AppState>, mut settings: Settings) -> R<()> {
    {
        // the update repository and the skipped release are changed by their own commands only
        let cur = s.settings.lock().unwrap();
        settings.update_repo = cur.update_repo.clone();
        settings.skipped_update = cur.skipped_update.clone();
    }
    persist(&app, &settings)?;
    *s.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn detect_root() -> Option<String> {
    detect().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_root(app: AppHandle, s: State<AppState>, path: String) -> R<()> {
    if !valid_root(&PathBuf::from(&path)) {
        return Err("That is not a Deadlock folder (game/citadel/gameinfo.gi not found).".into());
    }
    let snapshot = {
        let mut g = s.settings.lock().unwrap();
        g.game_root = Some(path.clone());
        g.clone()
    };
    persist(&app, &snapshot)?;
    log(&app, "INFO", format!("Game folder set to {path}"));
    Ok(())
}

#[derive(Serialize)]
pub struct Info {
    version: String,
    default_update_repo: &'static str,
    root: Option<String>,
    addons: Option<String>,
    meta_file: Option<String>,
    settings_file: String,
    log_dir: String,
}

#[tauri::command]
pub fn get_info(app: AppHandle, s: State<AppState>) -> Info {
    let r = root(&s).ok();
    Info {
        version: app.package_info().version.to_string(),
        default_update_repo: crate::domain::appupdate::DEFAULT_REPO,
        addons: r.as_ref().map(|r| addons(r).to_string_lossy().to_string()),
        meta_file: r.as_ref().map(|r| addons(r).join(META).to_string_lossy().to_string()),
        root: r.map(|r| r.to_string_lossy().to_string()),
        settings_file: settings_path(&app).map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        log_dir: app.path().app_log_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
    }
}

#[tauri::command]
pub fn open_folder(app: AppHandle, s: State<AppState>, kind: String) -> R<()> {
    let p = match kind.as_str() {
        "addons" => {
            let a = addons(&root(&s)?);
            fs::create_dir_all(&a).map_err(e)?;
            a
        }
        "game" => root(&s)?,
        "logs" => app.path().app_log_dir().map_err(e)?,
        "data" => app.path().app_config_dir().map_err(e)?,
        _ => return Err("Unknown folder".into()),
    };
    fs::create_dir_all(&p).map_err(e)?;
    #[cfg(windows)]
    let mut c = std::process::Command::new("explorer");
    #[cfg(not(windows))]
    let mut c = std::process::Command::new("xdg-open");
    c.arg(p);
    c.spawn().map(|_| ()).map_err(e)
}
