//! Update checks: installed GameBanana mods (notify / re-download) and the app itself (GitHub releases).
use super::downloads::{meta_from, start_download};
use crate::domain::{
    gamebanana as gb,
    meta::{copy_asset, load, norm_model, save},
    util::{e, R},
};
use crate::state::{log, root, AppState};
use super::downloads::fetch_thumb;
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
pub struct ModUpdate {
    id: String,
    name: String,
    remote: i64,
}

pub struct Remote {
    updated: i64,
    audio: String,
    description: String,
    author: String,
    thumb: Option<String>,
}

#[tauri::command]
pub async fn check_mod_updates(app: AppHandle) -> R<Vec<ModUpdate>> {
    let r = {
        let st = app.state::<AppState>();
        root(&st)?
    };
    let mut meta = load(&r)?;
    let mut keys: Vec<(u64, String)> = vec![];
    for m in &meta.mods {
        if let Some(g) = m.gb_id {
            let k = (g, norm_model(&m.gb_model));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
    }
    let mut remote: std::collections::HashMap<(u64, String), Remote> = std::collections::HashMap::new();
    for (g, model) in &keys {
        if let Ok(x) = gb::fetch_item(model, *g).await {
            remote.insert((*g, model.clone()), Remote { updated: x.updated, audio: x.audio.clone().unwrap_or_default(), description: x.description.clone(), author: x.author.clone(), thumb: x.thumb.clone() });
        }
    }
    let (mut out, mut changed) = (vec![], false);
    for m in meta.mods.iter_mut() {
        let Some(g) = m.gb_id else { continue };
        let Some(rm) = remote.get(&(g, norm_model(&m.gb_model))) else { continue };
        if m.gb_audio.is_empty() && !rm.audio.is_empty() {
            m.gb_audio = rm.audio.clone(); // sound mods: remember the preview link
            changed = true;
        }
        // fill what is still missing (data from the .yoshirian file and the user's edits always come first)
        if !m.edited {
            if m.description.trim().is_empty() && !rm.description.is_empty() {
                m.description = rm.description.clone();
                changed = true;
            }
            if m.author.is_empty() && !rm.author.is_empty() {
                m.author = rm.author.clone();
                changed = true;
            }
            if m.image.is_none() {
                if let Some(t) = &rm.thumb {
                    if let Ok(p) = fetch_thumb(&app, &m.yoshirian_id, t).await {
                        if let Ok(f) = copy_asset(&r, &m.yoshirian_id, &p) {
                            m.image = Some(f);
                            changed = true;
                        }
                    }
                }
            }
        }
        if m.gb_updated == 0 {
            m.gb_updated = rm.updated; // first check: remember the current state as the baseline
            changed = true;
        } else if rm.updated > m.gb_updated {
            out.push(ModUpdate { id: m.yoshirian_id.clone(), name: m.name.clone(), remote: rm.updated });
        }
    }
    if changed {
        save(&r, &meta)?;
    }
    log(&app, "INFO", format!("Mod update check: {} update(s)", out.len()));
    Ok(out)
}

/// Re-downloads a GameBanana mod and replaces the installed file(s); name, order, enabled state stay.
#[tauri::command]
pub async fn update_mod(app: AppHandle, id: String) -> R<String> {
    let (gid, model) = {
        let st = app.state::<AppState>();
        let r = root(&st)?;
        let meta = load(&r)?;
        let m = meta.mods.iter().find(|m| m.yoshirian_id == id).ok_or("Mod not found")?;
        (m.gb_id.ok_or("This mod was not installed from GameBanana")?, norm_model(&m.gb_model))
    };
    let item = gb::fetch_item(&model, gid).await?;
    if item.files.len() > 1 {
        return Err("MULTI".into());
    }
    let f = item.files.first().cloned().ok_or("No downloadable file found")?;
    let mut meta = meta_from(item);
    meta.update = true;
    Ok(start_download(&app, f.url, Some(meta)))
}

pub fn newer(a: &str, b: &str) -> bool {
    let p = |s: &str| s.trim_start_matches('v').split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    p(a) > p(b)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> R<Option<String>> {
    let repo = app.state::<AppState>().settings.lock().unwrap().update_repo.trim().to_string();
    if repo.is_empty() {
        return Err("Set the update repository (owner/repo) in Settings → Updates".into());
    }
    let cur = app.package_info().version.to_string();
    let c = reqwest::Client::builder().user_agent("yoshirian").build().map_err(e)?;
    let v: serde_json::Value = c
        .get(format!("https://api.github.com/repos/{repo}/releases/latest"))
        .send()
        .await
        .map_err(e)?
        .json()
        .await
        .map_err(e)?;
    let tag = v["tag_name"].as_str().unwrap_or("").to_string();
    log(&app, "INFO", format!("Update check: current {cur}, latest \"{tag}\""));
    Ok(if !tag.is_empty() && newer(&tag, &cur) { Some(tag) } else { None })
}
