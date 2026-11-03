//! Application state shared by all commands: user settings, download queue, logs.
use crate::domain::{
    appupdate::{normalize_repo, DEFAULT_REPO},
    download::Ctl,
    paths::valid_root,
    util::{e, R},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::Write, path::{Path, PathBuf}, sync::{Arc, Mutex, OnceLock}};
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
    let dir = logs_dir(app);
    let _ = fs::create_dir_all(&dir);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(dir.join("yoshirian.log")) {
        let _ = writeln!(f, "{line}");
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

// ---------------------------------------------------------------- where the app keeps its files
//
// EVERYTHING the app writes lives in ONE folder, the folder of the program itself (the one with Yoshirian.exe):
//
//   Yoshirian.exe  uninstall.exe  settings.json  logs\  cache\ (extract, downloads, update)  webview\ (theme, switches)
//
// That is %LOCALAPPDATA%\Yoshirian for the normal installer. If the program sits in a folder it cannot write to
// (Program Files) or runs from a build folder (`cargo run`, `tauri dev`), %LOCALAPPDATA%\Yoshirian is used instead.
//
// Use `app_home()` and the functions below. Never ask Tauri for app_config_dir / app_cache_dir / app_log_dir /
// app_local_data_dir (they point to two other folders named after the app identifier); a test fails if any other file does.
// Never delete `app_home()` itself: it contains the installed program.

/// Can files be created in `dir` (it is created when missing)? Leaves nothing behind.
pub fn writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    let ok = fs::write(&probe, b"x").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// `target\debug` / `target\release` of a cargo build: not a place for user data.
fn is_build_output(dir: &Path) -> bool {
    let last = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let parent = dir.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    (last == "debug" || last == "release") && parent == "target"
}

/// `%LOCALAPPDATA%\Yoshirian`: where the installer puts the program, and the fallback home.
fn local_home(app: &AppHandle) -> PathBuf {
    let base = app.path().local_data_dir().unwrap_or_else(|_| std::env::temp_dir());
    base.join(app.config().product_name.clone().unwrap_or_else(|| "Yoshirian".to_string()))
}

/// The one folder for everything the program writes (see above). Decided once: the write test runs a single time.
pub fn app_home(app: &AppHandle) -> PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
            if !is_build_output(&dir) && writable(&dir) {
                return dir;
            }
        }
        local_home(app)
    })
    .clone()
}
pub fn logs_dir(app: &AppHandle) -> PathBuf {
    app_home(app).join("logs")
}
pub fn cache_dir(app: &AppHandle) -> PathBuf {
    app_home(app).join("cache")
}
/// The web view's user data folder (the browser profile that holds the UI preferences).
pub fn webview_dir(app: &AppHandle) -> PathBuf {
    app_home(app).join("webview")
}
/// Make the folders of the layout above.
pub fn ensure_layout(home: &Path) {
    for d in ["logs", "cache", "webview"] {
        let _ = fs::create_dir_all(home.join(d));
    }
}

pub fn settings_path(app: &AppHandle) -> R<PathBuf> {
    Ok(app_home(app).join("settings.json"))
}

pub fn persist(app: &AppHandle, s: &Settings) -> R<()> {
    let sp = settings_path(app)?;
    fs::create_dir_all(sp.parent().unwrap()).map_err(e)?;
    fs::write(sp, serde_json::to_string_pretty(s).map_err(e)?).map_err(e)
}
/// Scratch folder for extracted archives (cleared at startup).
pub fn tmp_dir(app: &AppHandle) -> PathBuf {
    cache_dir(app).join("extract")
}

/// Move `from` to `to` unless `to` already exists (never overwrites); records what happened.
fn move_into(done: &mut Vec<String>, from: PathBuf, to: PathBuf, what: &str) {
    if !from.exists() || to.exists() {
        return;
    }
    if let Some(p) = to.parent() {
        let _ = fs::create_dir_all(p);
    }
    done.push(match move_path(&from, &to) {
        Ok(()) => format!("moved {what}"),
        Err(er) => format!("could not move {what}: {er}"),
    });
}

/// Older versions kept files elsewhere: in `<root>\data` (0.1.5) and in two folders named after the app identifier.
/// Move what is worth keeping into `root` (settings, logs, the web view profile), drop the temporary files, and remove
/// the old folders when nothing else is in them. Returns a line for every thing that was done.
pub fn migrate(old_config: &Path, old_local: &Path, root: &Path) -> Vec<String> {
    let mut done = vec![];
    let _ = fs::create_dir_all(root);
    // 0.1.5 kept everything in <root>\data: move it up one level (the newest of the old layouts, so it goes first)
    let old_data = root.join("data");
    if old_data.is_dir() {
        move_into(&mut done, old_data.join("settings.json"), root.join("settings.json"), "settings.json (from data\\)");
        move_into(&mut done, old_data.join("logs"), root.join("logs"), "the log folder (from data\\)");
        move_into(&mut done, old_data.join("webview"), root.join("webview"), "the web view profile (from data\\)");
        let _ = fs::remove_dir_all(old_data.join("cache"));
    }
    // the two folders named after the app identifier
    move_into(&mut done, old_config.join("settings.json"), root.join("settings.json"), "settings.json");
    move_into(&mut done, old_local.join("logs"), root.join("logs"), "the log folder");
    // WebView2 keeps its profile in "EBWebView" inside the user data folder it is given
    move_into(&mut done, old_local.join("EBWebView"), root.join("webview").join("EBWebView"), "the web view profile (theme and switches)");
    for tmp in ["extract", "downloads", "update"] {
        let p = old_local.join(tmp);
        if p.exists() && fs::remove_dir_all(&p).is_ok() {
            done.push(format!("deleted old temporary folder {tmp}"));
        }
    }
    for old in [old_data.as_path(), old_config, old_local] {
        // not recursive on purpose: a folder with anything unknown in it is left alone
        if old != root && old.exists() && fs::remove_dir(old).is_ok() {
            done.push(format!("removed the empty folder {}", old.display()));
        }
    }
    done
}

fn move_path(from: &Path, to: &Path) -> std::io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_all(from, to)?; // another drive: copy, and only then delete the original
    if from.is_dir() { fs::remove_dir_all(from) } else { fs::remove_file(from) }
}

fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_all(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        fs::copy(from, to)?;
    }
    Ok(())
}

/// Run once at start-up, before anything reads the folders.
pub fn migrate_old_data(app: &AppHandle) -> Vec<String> {
    match (app.path().app_config_dir(), app.path().app_local_data_dir()) {
        (Ok(c), Ok(l)) => migrate(&c, &l, &app_home(app)),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("yoshirian-data-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn old_layout(d: &Path) -> (PathBuf, PathBuf) {
        let (cfg, loc) = (d.join("Roaming/dev.app"), d.join("Local/dev.app"));
        fs::create_dir_all(&cfg).unwrap();
        fs::write(cfg.join("settings.json"), "{\"debug\":true}").unwrap();
        fs::create_dir_all(loc.join("logs")).unwrap();
        fs::write(loc.join("logs/yoshirian.log"), "old log").unwrap();
        fs::create_dir_all(loc.join("EBWebView/Default/Local Storage")).unwrap();
        fs::write(loc.join("EBWebView/Default/Local Storage/leveldb"), "theme").unwrap();
        for t in ["extract", "downloads", "update"] {
            fs::create_dir_all(loc.join(t)).unwrap();
            fs::write(loc.join(t).join("x.tmp"), "tmp").unwrap();
        }
        (cfg, loc)
    }

    #[test]
    fn old_folders_are_merged_into_the_one_data_folder() {
        let d = tmp();
        let (cfg, loc) = old_layout(&d);
        let root = d.join("Local/Yoshirian");
        let done = migrate(&cfg, &loc, &root);
        assert_eq!(fs::read_to_string(root.join("settings.json")).unwrap(), "{\"debug\":true}");
        assert_eq!(fs::read_to_string(root.join("logs/yoshirian.log")).unwrap(), "old log");
        assert_eq!(fs::read_to_string(root.join("webview/EBWebView/Default/Local Storage/leveldb")).unwrap(), "theme", "the UI preferences survive");
        assert!(!cfg.exists() && !loc.exists(), "the two old folders are gone: {done:?}");
        assert!(done.iter().any(|l| l.contains("web view profile")));
    }

    #[test]
    fn version_0_1_5_data_folder_is_moved_up() {
        let d = tmp();
        let root = d.join("Local/Yoshirian");
        fs::create_dir_all(root.join("data/logs")).unwrap();
        fs::create_dir_all(root.join("data/cache/extract")).unwrap();
        fs::create_dir_all(root.join("data/webview/EBWebView/Default")).unwrap();
        fs::write(root.join("data/settings.json"), "S").unwrap();
        fs::write(root.join("data/logs/yoshirian.log"), "L").unwrap();
        fs::write(root.join("data/webview/EBWebView/Default/prefs"), "P").unwrap();
        fs::write(root.join("Yoshirian.exe"), "program").unwrap();
        migrate(&d.join("nope1"), &d.join("nope2"), &root);
        assert_eq!(fs::read_to_string(root.join("settings.json")).unwrap(), "S");
        assert_eq!(fs::read_to_string(root.join("logs/yoshirian.log")).unwrap(), "L");
        assert_eq!(fs::read_to_string(root.join("webview/EBWebView/Default/prefs")).unwrap(), "P");
        assert!(!root.join("data").exists(), "the data folder is gone");
        assert_eq!(fs::read_to_string(root.join("Yoshirian.exe")).unwrap(), "program", "the program's files are never touched");
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        let d = tmp();
        let (cfg, loc) = old_layout(&d);
        let root = d.join("Local/Yoshirian");
        migrate(&cfg, &loc, &root);
        assert!(migrate(&cfg, &loc, &root).is_empty());
        assert_eq!(fs::read_to_string(root.join("settings.json")).unwrap(), "{\"debug\":true}");
    }

    #[test]
    fn existing_new_files_are_never_overwritten_and_unknown_files_are_kept() {
        let d = tmp();
        let (cfg, loc) = old_layout(&d);
        let root = d.join("Local/Yoshirian");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("settings.json"), "NEW").unwrap();
        fs::write(loc.join("something-else.txt"), "not ours").unwrap();
        migrate(&cfg, &loc, &root);
        assert_eq!(fs::read_to_string(root.join("settings.json")).unwrap(), "NEW");
        assert!(cfg.join("settings.json").exists(), "the older settings stay where they are");
        assert!(loc.join("something-else.txt").exists() && loc.exists(), "a folder with unknown files is not deleted");
    }

    #[test]
    fn the_layout_is_created_and_writability_is_checked() {
        let d = tmp();
        ensure_layout(&d.join("Yoshirian"));
        ensure_layout(&d.join("Yoshirian")); // twice is fine
        for f in ["logs", "cache", "webview"] {
            assert!(d.join("Yoshirian").join(f).is_dir(), "{f}");
        }
        assert!(writable(&d.join("new/sub")) && d.join("new/sub").is_dir());
        assert!(!d.join("new/sub/.write-test").exists(), "the probe file is removed");
        let file = d.join("a-file");
        fs::write(&file, "x").unwrap();
        assert!(!writable(&file.join("inside")), "a path below a file cannot be written to");
    }

    #[test]
    fn cargo_build_folders_are_not_used_for_user_data() {
        assert!(is_build_output(Path::new("C:/src/yoshirian/src-tauri/target/debug")));
        assert!(is_build_output(Path::new("/x/target/release")));
        assert!(!is_build_output(Path::new("C:/Users/me/AppData/Local/Yoshirian")));
        assert!(!is_build_output(Path::new("D:/Tools/Yoshirian")));
        assert!(!is_build_output(Path::new("/x/target")), "only debug / release below target");
    }

    #[test]
    fn copy_fallback_copies_whole_trees() {
        let d = tmp();
        let from = d.join("a");
        fs::create_dir_all(from.join("b/c")).unwrap();
        fs::write(from.join("b/c/f.txt"), "x").unwrap();
        copy_all(&from, &d.join("out")).unwrap();
        assert_eq!(fs::read_to_string(d.join("out/b/c/f.txt")).unwrap(), "x");
    }

    /// The whole point of the one data folder: no other file may use Tauri's own app folders.
    #[test]
    fn nobody_else_asks_tauri_for_app_folders() {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, out) } else if p.extension().map_or(false, |x| x == "rs") { out.push(p) }
            }
        }
        let mut files = vec![];
        walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let forbidden = ["app_config_dir", "app_cache_dir", "app_log_dir", "app_local_data_dir", "app_data_dir"];
        for f in files.iter().filter(|f| f.file_name().unwrap() != "state.rs") {
            let text = fs::read_to_string(f).unwrap();
            for word in forbidden {
                assert!(!text.contains(word), "{} uses {word}: use state::app_home() and friends instead", f.display());
            }
        }
    }
}
