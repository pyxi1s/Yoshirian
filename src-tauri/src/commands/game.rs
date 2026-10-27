//! gameinfo.gi patching, the built-in text editors (gameinfo.gi / autoexec.cfg) and launching the game.
use crate::domain::gameinfo::{
    file_path, force_patch, is_patched, launch, patch_gameinfo, read_text, restore_vanilla, write_text,
};
use crate::state::{log, root, AppState};
use crate::domain::util::R;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn gameinfo_status(s: State<AppState>) -> R<bool> {
    Ok(is_patched(&root(&s)?))
}

#[tauri::command]
pub fn patch_gameinfo_cmd(app: AppHandle, s: State<AppState>, force: bool) -> R<bool> {
    let r = root(&s)?;
    let res = if force { force_patch(&r).map(|_| true) } else { patch_gameinfo(&r) };
    match &res {
        Ok(_) => log(&app, "INFO", if force { "gameinfo.gi force-patched" } else { "gameinfo.gi patched" }),
        Err(er) => log(&app, "ERROR", format!("Patch failed: {er}")),
    }
    res
}

#[tauri::command]
pub fn restore_gameinfo(app: AppHandle, s: State<AppState>) -> R<()> {
    restore_vanilla(&root(&s)?)?;
    log(&app, "INFO", "gameinfo.gi restored to vanilla");
    Ok(())
}

#[tauri::command]
pub fn read_text_file(s: State<AppState>, kind: String) -> R<String> {
    read_text(&root(&s)?, &kind)
}

#[tauri::command]
pub fn write_text_file(app: AppHandle, s: State<AppState>, kind: String, content: String) -> R<()> {
    write_text(&root(&s)?, &kind, &content)?;
    log(&app, "INFO", format!("Saved {kind}"));
    Ok(())
}

#[tauri::command]
pub fn open_external(s: State<AppState>, kind: String) -> R<()> {
    let p = file_path(&root(&s)?, &kind)?;
    if kind == "autoexec" && !p.exists() {
        write_text(&root(&s)?, &kind, "")?;
    }
    let editor = s.settings.lock().unwrap().external_editor.trim().to_string();
    #[cfg(windows)]
    let default_editor = "notepad.exe";
    #[cfg(not(windows))]
    let default_editor = "xdg-open";
    let mut c = std::process::Command::new(if editor.is_empty() { default_editor } else { editor.as_str() });
    c.arg(p);
    c.spawn().map(|_| ()).map_err(|er| format!("Could not start editor: {er}"))
}

#[tauri::command]
pub fn launch_game(app: AppHandle, s: State<AppState>, vanilla: bool) -> R<()> {
    let r = root(&s)?;
    let (opts, auto) = {
        let g = s.settings.lock().unwrap();
        (g.launch_options.clone(), g.gameinfo_auto)
    };
    log(&app, "INFO", format!("Launching {} (options: \"{}\")", if vanilla { "vanilla" } else { "with mods" }, opts));
    launch(&r, vanilla, &opts, auto).map_err(|er| {
        log(&app, "ERROR", format!("Launch failed: {er}"));
        er
    })
}
