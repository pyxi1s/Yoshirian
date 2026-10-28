//! Application state shared by all commands: user settings, download queue, logs.
use crate::domain::{
    appupdate::{normalize_repo, DEFAULT_REPO},
    download::Ctl,
    paths::valid_root,
    util::{e, R},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::Write, path::PathBuf, sync::{Arc, Mutex}};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub game_root: Option<String>,
    pub auto_detect: bool,
    pub gameinfo_auto: bool,
    pub external_editor: String,
    pub launch_options: String,
    pub update_repo: String,     // "owner/repo" the app updates from (default: DEFAULT_REPO)
    pub skipped_update: String,  // tag of the app release the user chose to skip ("" = none)
    pub auto_update_check: bool,
    pub auto_mod_update_check: bool,
    pub auto_cursor_preset: bool,
    pub debug: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_root: None,
            auto_detect: true,
            gameinfo_auto: true,
            external_editor: String::new(),
            launch_options: String::new(),
            update_repo: DEFAULT_REPO.to_string(),
            skipped_update: String::new(),
            auto_update_check: true,
            auto_mod_update_check: true,
            auto_cursor_preset: true,
            debug: false,
        }
    }
}

impl Settings {
    /// Repair values read from settings.json (an empty or invalid update repository means "use the default").
    pub fn sanitize(&mut self) {
        self.update_repo = normalize_repo(&self.update_repo).unwrap_or_else(|| DEFAULT_REPO.to_string());
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub dl: tokio::sync::Semaphore,   // download queue: one at a time
    pub ctls: Mutex<HashMap<String, Arc<Ctl>>>, // pause / resume / cancel handles of unfinished downloads
    pub updating: std::sync::atomic::AtomicBool,      // the app installer is being downloaded
    pub update_ctl: Mutex<Option<Arc<Ctl>>>,         // cancel handle of that download
    pub pending: Mutex<Vec<String>>, // deep links waiting for the UI
    pub logs: Mutex<Vec<String>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            settings: Mutex::new(Settings::default()),
            dl: tokio::sync::Semaphore::new(1),
            ctls: Mutex::new(HashMap::new()),
            updating: std::sync::atomic::AtomicBool::new(false),
            update_ctl: Mutex::new(None),
            pending: Mutex::new(vec![]),
            logs: Mutex::new(vec![]),
        }
    }
}

/// Writes a line to the in-memory log (shown in Settings → Logs) and to `yoshirian.log`.
pub fn log(app: &AppHandle, level: &str, msg: impl std::fmt::Display) {
    let st = app.state::<AppState>();
    if level == "DEBUG" && !st.settings.lock().unwrap().debug {
        return;
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let line = format!("[{:02}:{:02}:{:02} UTC] {:<5} {}", (secs / 3600) % 24, (secs / 60) % 60, secs % 60, level, msg);
    {
        let mut l = st.logs.lock().unwrap();
        l.push(line.clone());
        if l.len() > 2000 {
            l.drain(..500);
        }
    }
    if let Ok(dir) = app.path().app_log_dir() {
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(dir.join("yoshirian.log")) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The configured game folder, or an error the UI shows to the user.
pub fn root(s: &State<AppState>) -> R<PathBuf> {
    let p = s.settings.lock().unwrap().game_root.clone().map(PathBuf::from);
    match p {
        Some(p) if valid_root(&p) => Ok(p),
        _ => Err("Game folder is not set. Choose it in Settings → Game folder.".to_string()),
    }
}

pub fn settings_path(app: &AppHandle) -> R<PathBuf> {
    Ok(app.path().app_config_dir().map_err(e)?.join("settings.json"))
}

pub fn persist(app: &AppHandle, s: &Settings) -> R<()> {
    let sp = settings_path(app)?;
    fs::create_dir_all(sp.parent().unwrap()).map_err(e)?;
    fs::write(sp, serde_json::to_string_pretty(s).map_err(e)?).map_err(e)
}

/// Scratch folder for extracted archives (cleared at startup).
pub fn tmp_dir(app: &AppHandle) -> PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("extract")
}
