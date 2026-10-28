//! Updating the app itself: check GitHub Releases, skip a release, download and run the installer.
//! The rules (what counts as an update) are in `domain::appupdate`.
use crate::domain::{
    appupdate::{
        download_allowed, fetch_releases, installer_asset, make_status, normalize_repo, parse_releases, pick_update, verify_digest, Asset, Release, UpdateStatus, API, DEFAULT_REPO,
    },
    download::{fetch_step, Ctl, Partial, Step},
    util::{e, R},
};
use crate::state::{log, persist, AppState, Settings};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering::SeqCst, Arc},
};
use tauri::{AppHandle, Emitter, Manager, State};

/// Where the installer is downloaded to (cleared at start-up).
pub fn update_dir(app: &AppHandle) -> PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("update")
}

fn repo_of(s: &Settings) -> String {
    normalize_repo(&s.update_repo).unwrap_or_else(|| DEFAULT_REPO.to_string())
}

/// The newest offered release (see `domain::appupdate` for the rules), or None.
async fn find_update(app: &AppHandle) -> R<(String, Option<Release>)> {
    let repo = repo_of(&app.state::<AppState>().settings.lock().unwrap());
    let rels = fetch_releases(API, &repo).await?;
    Ok((repo, pick_update(parse_releases(&rels), &app.package_info().version)))
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> R<UpdateStatus> {
    let skipped = app.state::<AppState>().settings.lock().unwrap().skipped_update.clone();
    let cur = app.package_info().version.clone();
    let (repo, latest) = match find_update(&app).await {
        Ok(x) => x,
        Err(er) => {
            log(&app, "WARN", format!("App update check failed: {er}"));
            return Err(er);
        }
    };
    log(&app, "INFO", format!("App update check ({repo}): installed {cur}, offered {}", latest.as_ref().map(|r| r.tag.as_str()).unwrap_or("nothing")));
    Ok(make_status(&cur, &repo, latest, &skipped))
}

/// "Skip": no popup / title bar button for this release. A newer release is offered again. `tag = ""` undoes it.
#[tauri::command]
pub fn skip_app_update(app: AppHandle, s: State<AppState>, tag: String) -> R<()> {
    let snapshot = {
        let mut g = s.settings.lock().unwrap();
        g.skipped_update = tag;
        g.clone()
    };
    persist(&app, &snapshot)
}

/// Change the repository the app updates from. Empty = back to the default. Returns the normalized "owner/repo".
#[tauri::command]
pub fn set_update_repo(app: AppHandle, s: State<AppState>, repo: String) -> R<String> {
    let new = if repo.trim().is_empty() { DEFAULT_REPO.to_string() } else { normalize_repo(&repo).ok_or("Enter the repository as owner/repo, or paste its GitHub link")? };
    let snapshot = {
        let mut g = s.settings.lock().unwrap();
        if g.update_repo != new {
            g.update_repo = new.clone();
            g.skipped_update.clear(); // a skip belongs to the old repository
        }
        g.clone()
    };
    persist(&app, &snapshot)?;
    log(&app, "INFO", format!("Update repository: {new}"));
    Ok(new)
}

#[derive(Serialize, Clone)]
struct Progress {
    state: &'static str, // downloading | verifying | launching | cancelled | error
    received: u64,
    total: u64,
    error: Option<String>,
}

fn emit(app: &AppHandle, state: &'static str, received: u64, total: u64, error: Option<String>) {
    let _ = app.emit("app-update-progress", Progress { state, received, total, error });
}

/// Starts downloading the installer of the offered release; progress comes as `app-update-progress` events.
/// The download address is looked up again here and must belong to the update repository: it never comes from the UI.
#[tauri::command]
pub async fn install_app_update(app: AppHandle, s: State<'_, AppState>) -> R<()> {
    if s.updating.swap(true, SeqCst) {
        return Err("The update is already being downloaded".into());
    }
    let prepared: R<(Release, Asset)> = async {
        let (repo, latest) = find_update(&app).await?;
        let rel = latest.ok_or("There is no update to install")?;
        let asset = installer_asset(&rel).cloned().ok_or("This release has no installer file. Open the release page to download it.")?;
        if !download_allowed(&asset.url, &repo) {
            return Err("The installer is not hosted in the update repository, so it was not downloaded.".to_string());
        }
        Ok((rel, asset))
    }
    .await;
    let (rel, asset) = match prepared {
        Ok(x) => x,
        Err(er) => {
            s.updating.store(false, SeqCst);
            return Err(er);
        }
    };
    let ctl = Arc::new(Ctl::default());
    *s.update_ctl.lock().unwrap() = Some(ctl.clone());
    log(&app, "INFO", format!("Downloading app update {} ({})", rel.tag, asset.name));
    tauri::async_runtime::spawn(run_update(app, asset, ctl));
    Ok(())
}

#[tauri::command]
pub fn cancel_app_update(s: State<AppState>) {
    if let Some(c) = s.update_ctl.lock().unwrap().as_ref() {
        c.cancel();
    }
}

async fn run_update(app: AppHandle, asset: Asset, ctl: Arc<Ctl>) {
    let st = app.state::<AppState>();
    let result = download_and_launch(&app, &asset, &ctl).await;
    match result {
        Ok(true) => {
            emit(&app, "launching", asset.size, asset.size, None);
            let a2 = app.clone();
            // give the UI a moment to show "starting the installer", then leave so the installer can replace the files
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1200));
                a2.exit(0);
            });
            return; // `updating` stays true: the app is closing
        }
        Ok(false) => {
            log(&app, "INFO", "App update cancelled");
            emit(&app, "cancelled", 0, 0, None);
        }
        Err(er) => {
            log(&app, "ERROR", format!("App update failed: {er}"));
            emit(&app, "error", 0, 0, Some(er));
        }
    }
    *st.update_ctl.lock().unwrap() = None;
    st.updating.store(false, SeqCst);
}

/// Ok(true) = installer started, Ok(false) = cancelled.
async fn download_and_launch(app: &AppHandle, asset: &Asset, ctl: &Ctl) -> R<bool> {
    let dir = update_dir(app);
    let _ = std::fs::remove_dir_all(&dir);
    let client = reqwest::Client::builder().user_agent("yoshirian").build().map_err(e)?;
    let mut part = Partial::default();
    emit(app, "downloading", 0, asset.size, None);
    let step = fetch_step(&client, &asset.url, &dir, "update", &mut part, ctl, |got, total| emit(app, "downloading", got, if total > 0 { total } else { asset.size }, None)).await?;
    let path = match step {
        Step::Finished(p) => p,
        Step::Cancelled | Step::Paused => {
            if let Some(p) = &part.path {
                let _ = std::fs::remove_file(p);
            }
            return Ok(false);
        }
    };
    emit(app, "verifying", part.got, part.got, None);
    if let Err(er) = verify_digest(&path, &asset.digest) {
        let _ = std::fs::remove_file(&path);
        return Err(er);
    }
    launch_installer(&path)?;
    Ok(true)
}

#[cfg(windows)]
fn launch_installer(path: &Path) -> R<()> {
    std::process::Command::new(path).spawn().map(|_| ()).map_err(|er| format!("Could not start the installer: {er}"))
}

#[cfg(not(windows))]
fn launch_installer(_path: &Path) -> R<()> {
    Err("Installing the update automatically is only available on Windows. Open the release page to download it.".into())
}
