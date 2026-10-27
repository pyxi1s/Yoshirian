//! The mod list: listing, installing, enabling, ordering, editing and deleting mods.
use super::edit;
use crate::domain::{
    archive,
    install::{inspect, install, InstallItem},
    meta::{apply, load, remove_asset, copy_asset, save, save_asset_bytes, Mod},
    paths::{addons, assets},
    util::{b64, e, R},
    vpk,
};
use crate::state::{log, root, tmp_dir, AppState};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct ModView {
    #[serde(flatten)]
    m: Mod,
    size: u64,
    image_path: Option<String>,
}

/// Names that came from a file name (pak01_dir, mod) rather than from a person or a mod author.
pub fn looks_default(name: &str) -> bool {
    let n = name.trim().to_lowercase();
    n.is_empty() || (n.len() >= 7 && n.starts_with("pak") && n.ends_with("_dir") && n[3..n.len() - 4].chars().all(|c| c.is_ascii_digit()))
}

/// Forces name / description / image to come from the .yoshirian file inside the mod.
#[tauri::command]
pub fn reread_meta(s: State<AppState>, id: String) -> R<bool> {
    let r = root(&s)?;
    let mut meta = load(&r)?;
    let dir = addons(&r);
    {
        let m = meta.mods.iter_mut().find(|m| m.yoshirian_id == id).ok_or("Mod not found")?;
        let Some(em) = vpk::read_meta(&dir.join(&m.current_file)) else { return Ok(false) };
        m.has_ys = true;
        m.ys_scanned = true;
        m.edited = false;
        if let Some(n) = em.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            m.name = n;
        }
        if let Some(d) = em.description {
            m.description = d;
        }
        if let Some((ext, bytes)) = em.thumb {
            m.image = Some(save_asset_bytes(&r, &m.yoshirian_id, &ext, &bytes)?);
        }
    }
    apply(&r, &mut meta)?;
    Ok(true)
}

#[tauri::command]
pub fn list_mods(s: State<AppState>) -> R<Vec<ModView>> {
    let r = root(&s)?;
    let mut meta = load(&r)?;
    let dir = addons(&r);
    // detect sound mods once: count the audio files inside each vpk
    let mut dirty = false;
    for m in meta.mods.iter_mut() {
        if m.snd.is_none() {
            m.snd = Some(vpk::list_sounds(&dir.join(&m.current_file)).len() as u32);
            dirty = true;
        }
    }
    // read the optional .yoshirian file of mods installed before: it comes first, the rest is filled in later
    for m in meta.mods.iter_mut() {
        if m.ys_scanned {
            continue;
        }
        m.ys_scanned = true;
        dirty = true;
        let Some(em) = vpk::read_meta(&dir.join(&m.current_file)) else { continue };
        m.has_ys = true;
        if m.edited {
            continue;
        }
        if let Some(n) = em.name.as_ref().map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            if looks_default(&m.name) {
                m.name = n;
            }
        }
        if m.description.trim().is_empty() {
            if let Some(d) = em.description.clone() {
                m.description = d;
            }
        }
        if m.image.is_none() {
            if let Some((ext, bytes)) = em.thumb {
                if let Ok(f) = save_asset_bytes(&r, &m.yoshirian_id, &ext, &bytes) {
                    m.image = Some(f);
                }
            }
        }
    }
    let _ = dirty;
    save(&r, &meta)?;
    let assets = assets(&r);
    Ok(meta
        .mods
        .into_iter()
        .map(|m| {
            let size = fs::metadata(dir.join(&m.current_file)).map(|x| x.len()).unwrap_or(0);
            let image_path = m.image.as_ref().map(|f| assets.join(f).to_string_lossy().to_string());
            ModView { m, size, image_path }
        })
        .collect())
}

#[derive(Serialize)]
pub struct SoundData {
    mime: String,
    data: String,
}

pub fn mod_file(s: &State<AppState>, id: &str) -> R<PathBuf> {
    let r = root(s)?;
    let meta = load(&r)?;
    let m = meta.mods.iter().find(|m| m.yoshirian_id == id).ok_or("Mod not found")?;
    Ok(addons(&r).join(&m.current_file))
}

#[tauri::command]
pub fn vpk_sounds(s: State<AppState>, id: String) -> R<Vec<vpk::SoundEntry>> {
    Ok(vpk::list_sounds(&mod_file(&s, &id)?))
}

#[tauri::command]
pub fn vpk_sound_data(s: State<AppState>, id: String, path: String) -> R<SoundData> {
    let (mime, bytes) = vpk::read_sound(&mod_file(&s, &id)?, &path).ok_or("Could not read this sound from the file")?;
    Ok(SoundData { mime, data: b64(&bytes) })
}

#[tauri::command]
pub fn inspect_files(app: AppHandle, paths: Vec<String>) -> R<Vec<InstallItem>> {
    log(&app, "INFO", format!("Inspecting {} file(s)", paths.len()));
    inspect(&paths, &tmp_dir(&app)).map_err(|er| {
        log(&app, "ERROR", &er);
        er
    })
}

#[tauri::command]
pub fn install_mods(app: AppHandle, s: State<AppState>, items: Vec<InstallItem>) -> R<()> {
    install(&root(&s)?, &items)?;
    for i in &items {
        log(&app, "INFO", format!("Installed \"{}\" (disabled)", i.name));
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_mod(app: AppHandle, s: State<AppState>, id: String, enable: bool) -> R<()> {
    edit(&s, |m| {
        let x = m.mods.iter_mut().find(|x| x.yoshirian_id == id).ok_or("Mod not found")?;
        x.is_enable = enable;
        log(&app, "INFO", format!("{} \"{}\"", if enable { "Enabled" } else { "Disabled" }, x.name));
        Ok(())
    })
}

#[tauri::command]
pub fn reorder_mods(s: State<AppState>, ids: Vec<String>) -> R<()> {
    edit(&s, |m| {
        for x in m.mods.iter_mut() {
            if let Some(p) = ids.iter().position(|i| *i == x.yoshirian_id) {
                x.order = p as u32 + 1;
            }
        }
        Ok(())
    })
}

#[derive(Deserialize)]
pub struct EditArgs {
    id: String,
    name: String,
    description: String,
    requires: Vec<String>,
    image: Option<String>, // new local image path
    clear_image: bool,
}

#[tauri::command]
pub fn edit_mod(app: AppHandle, s: State<AppState>, args: EditArgs) -> R<()> {
    if args.name.trim().is_empty() {
        return Err("Mod name is required".into());
    }
    let r = root(&s)?;
    let mut meta = load(&r)?;
    {
        let m = meta.mods.iter_mut().find(|x| x.yoshirian_id == args.id).ok_or("Mod not found")?;
        let new_name = args.name.trim().to_string();
        if m.name != new_name || m.description != args.description || args.image.is_some() || args.clear_image {
            m.edited = true; // from now on automatic data never overwrites the user's choice
        }
        m.name = new_name;
        m.description = args.description;
        m.requires = args.requires;
        if args.clear_image {
            remove_asset(&r, &m.yoshirian_id);
            m.image = None;
        }
        if let Some(p) = args.image.filter(|p| !p.is_empty()) {
            m.image = Some(copy_asset(&r, &m.yoshirian_id, &p)?);
        }
        log(&app, "INFO", format!("Edited \"{}\"", m.name));
    }
    apply(&r, &mut meta)
}

#[tauri::command]
pub fn delete_mod(app: AppHandle, s: State<AppState>, id: String) -> R<()> {
    let r = root(&s)?;
    let mut meta = load(&r)?;
    if let Some(x) = meta.mods.iter().find(|x| x.yoshirian_id == id) {
        fs::remove_file(addons(&r).join(&x.current_file)).map_err(e)?;
        log(&app, "INFO", format!("Deleted \"{}\"", x.name));
    }
    remove_asset(&r, &id);
    meta.mods.retain(|x| x.yoshirian_id != id);
    apply(&r, &mut meta)
}

/// Archive extensions the app accepts (the UI uses this for its file pickers and drop hints).
#[tauri::command]
pub fn archive_exts() -> Vec<&'static str> {
    archive::EXTS.to_vec()
}
