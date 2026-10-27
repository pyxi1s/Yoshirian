//! Download queue (download -> extract -> install as disabled) and `yoshirian://` deep links.
use crate::domain::{
    cursor,
    download::{fetch_step, finalize, interruptible, Ctl, Partial, Step},
    gamebanana as gb,
    install::{inspect, install},
    util::{e, R},
};
use crate::state::{log, root, tmp_dir, AppState};
use serde::{Deserialize, Serialize};
use std::{fs, sync::Arc};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Deserialize, Clone, Default)]
pub struct DlMeta {
    pub name: String,
    pub description: String,
    pub thumb: Option<String>,
    pub gb_id: Option<u64>,
    pub author: String,
    #[serde(default)]
    pub updated: i64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub update: bool, // replace the installed mod instead of adding a new one
    #[serde(default)]
    pub audio: String,
}

pub fn meta_from(m: gb::GbMod) -> DlMeta {
    DlMeta { name: m.name, description: m.description, thumb: m.thumb, gb_id: Some(m.id), author: m.author, updated: m.updated, model: m.model, update: false, audio: m.audio.unwrap_or_default() }
}

#[derive(Serialize, Clone)]
pub struct Progress {
    pub id: String,
    pub name: String,
    pub thumb: Option<String>,
    pub received: u64,
    pub total: u64,
    pub state: &'static str, // queued | downloading | paused | installing | done | error | cancelled
    pub error: Option<String>,
    pub installed: usize,
    pub kind: &'static str, // mod | cursor
}

pub fn emit(app: &AppHandle, p: &Progress) {
    let _ = app.emit("dl-progress", p);
}

pub async fn fetch_thumb(app: &AppHandle, id: &str, url: &str) -> R<String> {
    let bytes = reqwest::get(url).await.map_err(e)?.error_for_status().map_err(e)?.bytes().await.map_err(e)?;
    let ext = url.split('?').next().unwrap_or("").rsplit('.').next().filter(|x| x.len() <= 4).unwrap_or("jpg");
    let dir = app.path().app_cache_dir().map_err(e)?.join("downloads");
    fs::create_dir_all(&dir).map_err(e)?;
    let p = dir.join(format!("{id}_thumb.{ext}"));
    fs::write(&p, &bytes).map_err(e)?;
    Ok(p.to_string_lossy().to_string())
}

/// Everything after the file is on disk: extract, detect mod / cursor pack, install (disabled).
pub async fn pipeline(app: &AppHandle, base: &Progress, path: String, meta: &Option<DlMeta>) -> R<(usize, &'static str)> {
    emit(app, &Progress { state: "installing", ..base.clone() });
    let tmp = tmp_dir(app);
    let mut items = match inspect(&[path.clone()], &tmp) {
        Ok(i) => i,
        Err(er) => {
            // no .vpk inside: maybe it is a cursor pack
            let auto = app.state::<AppState>().settings.lock().unwrap().auto_cursor_preset;
            return match cursor::inspect(&[path.clone()], &tmp, auto, meta.as_ref().map(|m| m.name.as_str())) {
                Ok(ci) => {
                    let st = app.state::<AppState>();
                    cursor::add(&root(&st)?, &ci)?;
                    let _ = fs::remove_file(&path);
                    Ok((ci.len(), "cursor"))
                }
                Err(_) => Err(er),
            };
        }
    };
    let thumb = match meta.as_ref().and_then(|m| m.thumb.clone()) {
        Some(t) => fetch_thumb(app, &base.id, &t).await.ok(),
        None => None,
    };
    if let Some(m) = meta {
        let n = items.len();
        for (i, it) in items.iter_mut().enumerate() {
            // a .yoshirian file inside the vpk wins over GameBanana data
            if !it.embedded_name {
                it.name = if n > 1 { format!("{} {:02}", m.name, i + 1) } else { m.name.clone() };
            }
            if it.description.trim().is_empty() {
                it.description = m.description.clone();
            }
            if it.image.is_none() {
                it.image = thumb.clone();
            }
            it.gb_id = m.gb_id;
            it.author = m.author.clone();
            it.gb_updated = m.updated;
            it.gb_model = m.model.clone();
            it.gb_part = i as u32;
            it.replace = m.update;
            it.gb_audio = m.audio.clone();
        }
    }
    let st = app.state::<AppState>();
    install(&root(&st)?, &items)?;
    let _ = fs::remove_file(&path);
    Ok((items.len(), "mod"))
}

/// One queued download. Lifecycle: queued -> downloading <-> paused -> installing -> done | error | cancelled.
/// The queue slot (semaphore) is released while paused, so other downloads can go on.
async fn run_job(app: AppHandle, base: Progress, url: String, meta: Option<DlMeta>, ctl: Arc<Ctl>) {
    let st = app.state::<AppState>();
    let client = reqwest::Client::new();
    let dir = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("downloads");
    let mut part = Partial::default();
    let result: R<Option<(usize, &'static str)>> = async {
        loop {
            if ctl.is_cancelled() {
                return Ok(None);
            }
            if ctl.is_paused() {
                emit(&app, &Progress { received: part.got, total: part.total, state: "paused", ..base.clone() });
                ctl.wait_resumed().await;
                continue;
            }
            // wait for our turn in the queue; pause / cancel also work while waiting
            let Some(permit) = interruptible(&ctl, st.dl.acquire()).await else { continue };
            let _permit = permit.map_err(e)?;
            emit(&app, &Progress { received: part.got, total: part.total, state: "downloading", ..base.clone() });
            let step = fetch_step(&client, &url, &dir, &base.id, &mut part, &ctl, |got, total| {
                emit(&app, &Progress { received: got, total, state: "downloading", ..base.clone() });
            })
            .await?;
            match step {
                Step::Cancelled => return Ok(None),
                Step::Paused => continue, // the permit is dropped here; the top of the loop reports "paused"
                Step::Finished(p) => {
                    let p = finalize(&p)?;
                    part.path = Some(p.clone());
                    emit(&app, &Progress { received: part.got, total: part.total.max(part.got), state: "installing", ..base.clone() });
                    return pipeline(&app, &base, p.to_string_lossy().to_string(), &meta).await.map(Some);
                }
            }
        }
    }
    .await;
    match result {
        Ok(Some((n, kind))) => {
            log(&app, "INFO", format!("Added {n} {kind}(s) from \"{}\"", base.name));
            emit(&app, &Progress { state: "done", installed: n, kind, received: part.got, total: part.total.max(part.got), ..base.clone() });
        }
        Ok(None) => {
            log(&app, "INFO", format!("Download cancelled: \"{}\"", base.name));
            emit(&app, &Progress { state: "cancelled", received: part.got, total: part.total, ..base.clone() });
        }
        Err(er) => {
            log(&app, "ERROR", format!("Download failed for \"{}\": {er}", base.name));
            emit(&app, &Progress { state: "error", error: Some(er), received: part.got, total: part.total, ..base.clone() });
        }
    }
    if let Some(p) = &part.path {
        let _ = fs::remove_file(p); // leftovers of a cancelled / failed download (a successful install already removed it)
    }
    st.ctls.lock().unwrap().remove(&base.id);
}

pub fn start_download(app: &AppHandle, url: String, meta: Option<DlMeta>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let fallback = url.split('?').next().unwrap_or("mod").rsplit('/').next().unwrap_or("mod").to_string();
    let name = meta.as_ref().map(|m| m.name.clone()).unwrap_or(fallback);
    let thumb = meta.as_ref().and_then(|m| m.thumb.clone());
    let base = Progress { id: id.clone(), name, thumb, received: 0, total: 0, state: "queued", error: None, installed: 0, kind: "mod" };
    emit(app, &base);
    log(app, "INFO", format!("Queued download: {} ({url})", base.name));
    let ctl = Arc::new(Ctl::default());
    app.state::<AppState>().ctls.lock().unwrap().insert(id.clone(), ctl.clone());
    tauri::async_runtime::spawn(run_job(app.clone(), base, url, meta, ctl));
    id
}

fn ctl_of(s: &State<AppState>, id: &str) -> R<Arc<Ctl>> {
    s.ctls.lock().unwrap().get(id).cloned().ok_or_else(|| "This download is not running any more".to_string())
}

#[tauri::command]
pub fn dl_pause(s: State<AppState>, id: String) -> R<()> {
    ctl_of(&s, &id)?.pause();
    Ok(())
}

#[tauri::command]
pub fn dl_resume(s: State<AppState>, id: String) -> R<()> {
    ctl_of(&s, &id)?.resume();
    Ok(())
}

#[tauri::command]
pub fn dl_cancel(s: State<AppState>, id: String) -> R<()> {
    ctl_of(&s, &id)?.cancel();
    Ok(())
}

#[tauri::command]
pub fn download(app: AppHandle, url: String, meta: Option<DlMeta>) -> R<String> {
    if !url.starts_with("https://") || !url.contains("gamebanana.com/") {
        return Err("Only GameBanana links are supported".into());
    }
    Ok(start_download(&app, url, meta))
}

#[tauri::command]
pub fn take_links(s: State<AppState>) -> Vec<String> {
    std::mem::take(&mut *s.pending.lock().unwrap())
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LinkInfo {
    Open { id: u64, model: String, url: String },
    Install { url: String },
    Invalid,
}

/// Pure parsing, no network: tells the UI whether a link opens a store page or installs a file.
#[tauri::command]
pub fn link_kind(link: String) -> LinkInfo {
    match gb::classify(&link) {
        (gb::LinkKind::Open { id, model }, url) => LinkInfo::Open { id, model, url },
        (gb::LinkKind::Install { .. }, url) => LinkInfo::Install { url },
        _ => LinkInfo::Invalid,
    }
}

/// yoshirian://https://gamebanana.com/dl/<file> or .../mods/download/<mod>#FileInfo_<file>
#[tauri::command]
pub async fn process_link(app: AppHandle, link: String) -> R<String> {
    match gb::classify(&link).0 {
        gb::LinkKind::Install { file, mod_id, model } => {
            let mid = match mod_id {
                Some(m) => Some(m),
                None => gb::mod_of_file(file).await,
            };
            let meta = match mid {
                Some(m) => {
                    let r = if model.is_empty() { gb::fetch_any(m).await } else { gb::fetch_item(&model, m).await };
                    r.ok().map(meta_from)
                }
                None => None,
            };
            Ok(start_download(&app, format!("https://gamebanana.com/dl/{file}"), meta))
        }
        _ => Err("Only GameBanana download links can be installed this way".into()),
    }
}
