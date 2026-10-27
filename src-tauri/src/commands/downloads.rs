//! Download queue (download -> extract -> install as disabled) and `yoshirian://` deep links.
use crate::domain::{
    archive, cursor,
    gamebanana as gb,
    install::{inspect, install},
    util::{e, R},
};
use crate::state::{log, root, tmp_dir, AppState};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};
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
    pub state: &'static str, // queued | downloading | installing | done | error
    pub error: Option<String>,
    pub installed: usize,
    pub kind: &'static str, // mod | cursor
}

pub fn emit(app: &AppHandle, p: &Progress) {
    let _ = app.emit("dl-progress", p);
}

pub async fn run_download(app: &AppHandle, base: &Progress, url: &str) -> R<String> {
    let resp = reqwest::get(url).await.map_err(e)?.error_for_status().map_err(e)?;
    let total = resp.content_length().unwrap_or(0);
    // keep the real file name (needed to know zip/rar/vpk) from the final URL or Content-Disposition
    let cd = resp
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split("filename=").nth(1))
        .map(|v| v.trim_matches(|c: char| c == '"' || c == ';' || c == ' ').to_string());
    let fname = cd.unwrap_or_else(|| resp.url().path().rsplit('/').next().unwrap_or("mod").to_string());
    let dir = app.path().app_cache_dir().map_err(e)?.join("downloads");
    fs::create_dir_all(&dir).map_err(e)?;
    let safe: String = fname.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    let mut path = dir.join(format!("{}_{}", base.id, safe));
    let mut f = fs::File::create(&path).map_err(e)?;
    let (mut got, mut last) = (0u64, 0u64);
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let c = chunk.map_err(e)?;
        f.write_all(&c).map_err(e)?;
        got += c.len() as u64;
        if got - last > 128 * 1024 {
            last = got;
            emit(app, &Progress { received: got, total, state: "downloading", ..base.clone() });
        }
    }
    drop(f);
    // the real type comes from the file's first bytes; a wrong or missing extension gets fixed here
    let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    let kind = archive::sniff(&path);
    let want = archive::ext_for(kind);
    if ext != want {
        let np = PathBuf::from(format!("{}.{}", path.display(), want));
        fs::rename(&path, &np).map_err(e)?;
        path = np;
    }
    Ok(path.to_string_lossy().to_string())
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

pub async fn pipeline(app: &AppHandle, base: &Progress, url: &str, meta: &Option<DlMeta>) -> R<(usize, &'static str)> {
    let path = run_download(app, base, url).await?;
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

pub fn start_download(app: &AppHandle, url: String, meta: Option<DlMeta>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let fallback = url.split('?').next().unwrap_or("mod").rsplit('/').next().unwrap_or("mod").to_string();
    let name = meta.as_ref().map(|m| m.name.clone()).unwrap_or(fallback);
    let thumb = meta.as_ref().and_then(|m| m.thumb.clone());
    let base = Progress { id: id.clone(), name, thumb, received: 0, total: 0, state: "queued", error: None, installed: 0, kind: "mod" };
    emit(app, &base);
    log(app, "INFO", format!("Queued download: {} ({url})", base.name));
    let a = app.clone();
    tauri::async_runtime::spawn(async move {
        let st = a.state::<AppState>();
        let _permit = st.dl.acquire().await;
        emit(&a, &Progress { state: "downloading", ..base.clone() });
        match pipeline(&a, &base, &url, &meta).await {
            Ok((n, kind)) => {
                log(&a, "INFO", format!("Added {n} {kind}(s) from \"{}\"", base.name));
                emit(&a, &Progress { state: "done", installed: n, kind, ..base.clone() });
            }
            Err(er) => {
                log(&a, "ERROR", format!("Download failed for \"{}\": {er}", base.name));
                emit(&a, &Progress { state: "error", error: Some(er), ..base.clone() });
            }
        }
    });
    id
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
