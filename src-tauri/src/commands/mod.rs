//! Thin `#[tauri::command]` layer: each file serves one area of the UI and only glues the
//! frontend to `domain` (core logic) and `state` (settings, logs).
pub mod cursor;
pub mod downloads;
pub mod game;
pub mod gamebanana;
pub mod logs;
pub mod mods;
pub mod profiles;
pub mod settings;
pub mod updates;

use crate::domain::{
    meta::{apply, load, Meta},
    util::R,
};
use crate::state::{root, AppState};
use tauri::State;

/// Load the mod list, let `f` change it, then write it back (renames the vpk files accordingly).
pub(crate) fn edit<F: FnOnce(&mut Meta) -> R<()>>(s: &State<AppState>, f: F) -> R<()> {
    let r = root(s)?;
    let mut meta = load(&r)?;
    f(&mut meta)?;
    apply(&r, &mut meta)
}
