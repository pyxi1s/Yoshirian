//! Where things live on disk (game folder, addons folder) and automatic Steam detection.
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const APP_ID: &str = "1422450"; // Deadlock Steam app id

pub fn addons(root: &Path) -> PathBuf {
    root.join("game").join("citadel").join("addons")
}
pub fn gameinfo(root: &Path) -> PathBuf {
    root.join("game").join("citadel").join("gameinfo.gi")
}
pub fn valid_root(root: &Path) -> bool {
    gameinfo(root).is_file()
}

// ---------- auto game folder detection ----------
pub fn detect() -> Option<PathBuf> {
    let mut steams: Vec<PathBuf> = vec![];
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(v) = std::env::var(var) {
            steams.push(PathBuf::from(v).join("Steam"));
        }
    }
    steams.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    if let Some(h) = std::env::var_os("HOME") {
        steams.push(PathBuf::from(&h).join(".steam/steam"));
        steams.push(PathBuf::from(&h).join(".local/share/Steam"));
    }
    let mut libs = vec![];
    for s in &steams {
        libs.push(s.clone());
        if let Ok(t) = fs::read_to_string(s.join("steamapps").join("libraryfolders.vdf")) {
            for l in t.lines() {
                let l = l.trim();
                if l.starts_with("\"path\"") {
                    if let Some(v) = l.split('"').nth(3) {
                        libs.push(PathBuf::from(v.replace("\\\\", "\\")));
                    }
                }
            }
        }
    }
    libs.into_iter()
        .map(|l| l.join("steamapps").join("common").join("Deadlock"))
        .find(|g| valid_root(g))
}

/// Folder for thumbnails and other files Yoshirian keeps next to the mods.
pub fn assets(root: &Path) -> PathBuf {
    addons(root).join(".yoshirian_assets")
}

