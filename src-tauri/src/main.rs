#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Yoshirian — Deadlock mod manager (Tauri v2).
//!
//! * `domain`   core logic (files, vpk, GameBanana, cursors) — no Tauri types
//! * `commands` thin `#[tauri::command]` layer, one file per UI area
//! * `state`    settings, logs and other app-wide state
//!
//! To add a command: write it in the matching `commands/*.rs` file, then list it in `generate_handler!` below.
mod commands;
mod domain;
mod state;

use commands::{appupdate, cursor, downloads, game, gamebanana, logs, mods, profiles, settings, updates};
use domain::paths::{detect, valid_root};
use state::{log, persist, settings_path, tmp_dir, AppState, Settings};
use std::{fs, path::PathBuf};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            let h = app.handle().clone();
            let st = app.state::<AppState>();
            // load settings
            if let Ok(sp) = settings_path(&h) {
                if let Ok(t) = fs::read_to_string(sp) {
                    if let Ok(mut v) = serde_json::from_str::<Settings>(&t) {
                        v.sanitize();
                        *st.settings.lock().unwrap() = v;
                    }
                }
            }
            // auto game folder detection
            {
                let mut g = st.settings.lock().unwrap();
                let ok = g.game_root.as_ref().map(|p| valid_root(&PathBuf::from(p))).unwrap_or(false);
                if g.auto_detect && !ok {
                    if let Some(p) = detect() {
                        g.game_root = Some(p.to_string_lossy().to_string());
                    }
                }
            }
            let snapshot = st.settings.lock().unwrap().clone();
            let _ = persist(&h, &snapshot);
            let _ = fs::remove_dir_all(tmp_dir(&h)); // clear old extractions
            let _ = fs::remove_dir_all(appupdate::update_dir(&h)); // a finished app update left its installer here
            log(&h, "INFO", format!("Yoshirian {} started", h.package_info().version));

            #[cfg(any(windows, target_os = "linux"))]
            {
                let _ = app.deep_link().register_all();
            }
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                st.pending.lock().unwrap().extend(urls.iter().map(|u| u.to_string()));
            }
            let h2 = h.clone();
            app.deep_link().on_open_url(move |ev| {
                h2.state::<AppState>().pending.lock().unwrap().extend(ev.urls().iter().map(|u| u.to_string()));
                let _ = h2.emit("deeplink", ());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings & folders
            settings::get_settings, settings::save_settings, settings::detect_root, settings::set_root,
            settings::get_info, settings::open_folder,
            // mods
            mods::list_mods, mods::archive_exts, mods::inspect_files, mods::install_mods, mods::toggle_mod, mods::reorder_mods,
            mods::edit_mod, mods::delete_mod, mods::reread_meta, mods::vpk_sounds, mods::vpk_sound_data,
            // profiles
            profiles::list_profiles, profiles::save_profile_cmd, profiles::apply_profile_cmd, profiles::delete_profile,
            // game files & launching
            game::gameinfo_status, game::patch_gameinfo_cmd, game::restore_gameinfo, game::read_text_file,
            game::write_text_file, game::open_external, game::launch_game,
            // logs
            logs::get_logs, logs::clear_logs, logs::log_ui,
            // GameBanana & downloads
            gamebanana::gb_lookup, gamebanana::gb_browse, gamebanana::gb_mod, gamebanana::gb_install,
            gamebanana::fetch_audio, gamebanana::open_link,
            downloads::download, downloads::dl_pause, downloads::dl_resume, downloads::dl_cancel, downloads::take_links, downloads::link_kind, downloads::process_link,
            // updates
            updates::check_mod_updates, updates::skip_mod_update, updates::update_mod,
            // app updates
            appupdate::check_app_update, appupdate::skip_app_update, appupdate::set_update_repo,
            appupdate::install_app_update, appupdate::cancel_app_update,
            // cursors
            cursor::cursor_info, cursor::cursor_inspect, cursor::cursor_add, cursor::cursor_rename,
            cursor::cursor_delete, cursor::cursor_activate, cursor::cursor_profile, cursor::cursor_restore_all,
            cursor::cursor_reapply
        ])
        .run(tauri::generate_context!())
        .expect("error while running Yoshirian");
}

#[cfg(test)]
mod tests {
    /// The app version has ONE source: package.json. `tauri.conf.json` points to it, and everything
    /// (window text, logs, update check, installer name) reads it from Tauri at run time.
    #[test]
    fn app_version_comes_from_package_json() {
        let pkg: serde_json::Value = serde_json::from_str(include_str!("../../package.json")).unwrap();
        let ctx: tauri::Context<tauri::Wry> = tauri::generate_context!();
        assert_eq!(ctx.package_info().version.to_string(), pkg["version"].as_str().unwrap());
    }
}
