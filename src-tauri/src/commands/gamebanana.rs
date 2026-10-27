//! GameBanana browsing / lookup / one-click install.
use super::downloads::{meta_from, start_download};
use crate::domain::{
    gamebanana as gb,
    util::{b64, e, open_url, R},
};
use tauri::AppHandle;

#[tauri::command]
pub async fn gb_lookup(input: String) -> R<gb::Lookup> {
    gb::lookup(&input).await
}

#[tauri::command]
pub async fn gb_browse(page: u32) -> R<Vec<gb::GbMod>> {
    gb::browse(page).await
}

#[tauri::command]
pub async fn gb_mod(id: u64, model: String) -> R<gb::GbMod> {
    gb::fetch_detail(&model, id).await
}

#[tauri::command]
pub async fn gb_install(app: AppHandle, id: u64, model: String) -> R<String> {
    let m = gb::fetch_item(&model, id).await?;
    if m.files.len() > 1 {
        return Err("MULTI".into()); // UI opens the page so the user can pick a file
    }
    let f = m.files.first().cloned().ok_or("This item has no downloadable file")?;
    Ok(start_download(&app, f.url, Some(meta_from(m))))
}

/// Downloads a preview sound (GameBanana only) and returns it base64-encoded, so the UI can play and draw it.
#[tauri::command]
pub async fn fetch_audio(url: String) -> R<String> {
    if !url.starts_with("https://") || !url.contains("gamebanana.com/") {
        return Err("Only GameBanana previews can be loaded".into());
    }
    let bytes = reqwest::get(&url).await.map_err(e)?.error_for_status().map_err(e)?.bytes().await.map_err(e)?;
    if bytes.len() > 30 * 1024 * 1024 {
        return Err("The preview is too large".into());
    }
    Ok(b64(&bytes))
}

#[tauri::command]
pub fn open_link(url: String) -> R<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links can be opened".into());
    }
    open_url(&url)
}
