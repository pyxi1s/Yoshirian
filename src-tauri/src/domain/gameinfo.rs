//! `gameinfo.gi` patching, the editable text files (gameinfo.gi / autoexec.cfg) and launching the game.
use super::paths::{gameinfo, APP_ID};
use super::util::{e, open_url, urlenc, R};
use std::{
    fs,
    path::{Path, PathBuf},
};

const BAK: &str = "gameinfo.gi.yoshirian.bak";

pub fn is_patched(root: &Path) -> bool {
    fs::read_to_string(gameinfo(root)).map(|t| t.contains("citadel/addons")).unwrap_or(false)
}

/// Insert `Game citadel/addons` at the top of SearchPaths. Returns false if already patched.
pub fn patch_gameinfo(root: &Path) -> R<bool> {
    let p = gameinfo(root);
    let text = fs::read_to_string(&p).map_err(e)?;
    if text.contains("citadel/addons") {
        return Ok(false);
    }
    fs::copy(&p, p.with_file_name(BAK)).map_err(e)?; // file is pristine here
    let (mut out, mut state) = (String::new(), 0);
    for line in text.split_inclusive('\n') {
        out.push_str(line);
        if state == 0 && line.trim() == "SearchPaths" {
            state = 1;
        } else if state == 1 && line.trim() == "{" {
            out.push_str("\t\t\tGame\t\t\t\tcitadel/addons\n");
            state = 2;
        }
    }
    if state != 2 {
        return Err("SearchPaths was not found in gameinfo.gi".into());
    }
    fs::write(&p, out).map_err(e)?;
    Ok(true)
}

fn strip_patch(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|l| !(l.contains("citadel/addons") && l.trim_start().starts_with("Game")))
        .collect()
}

/// Remove our SearchPaths line (safer than restoring an old backup after a game update)
pub fn restore_vanilla(root: &Path) -> R<()> {
    let p = gameinfo(root);
    let text = fs::read_to_string(&p).map_err(e)?;
    if text.contains("citadel/addons") {
        fs::write(&p, strip_patch(&text)).map_err(e)?;
    }
    Ok(())
}

pub fn force_patch(root: &Path) -> R<()> {
    restore_vanilla(root)?;
    patch_gameinfo(root)?;
    Ok(())
}

// ---------- text files (gameinfo.gi / autoexec.cfg) ----------
fn text_path(root: &Path, kind: &str) -> R<PathBuf> {
    match kind {
        "gameinfo" => Ok(gameinfo(root)),
        "autoexec" => Ok(root.join("game").join("citadel").join("cfg").join("autoexec.cfg")),
        _ => Err("Unknown file".into()),
    }
}

pub fn read_text(root: &Path, kind: &str) -> R<String> {
    let p = text_path(root, kind)?;
    if !p.exists() && kind == "autoexec" {
        return Ok(String::new());
    }
    fs::read_to_string(p).map_err(e)
}

pub fn write_text(root: &Path, kind: &str, content: &str) -> R<()> {
    let p = text_path(root, kind)?;
    if kind == "gameinfo" && p.is_file() {
        fs::copy(&p, p.with_file_name("gameinfo.gi.yoshirian.edit.bak")).map_err(e)?;
    }
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(e)?;
    }
    fs::write(p, content).map_err(e)
}

pub fn file_path(root: &Path, kind: &str) -> R<PathBuf> {
    text_path(root, kind)
}

pub fn launch(root: &Path, vanilla: bool, opts: &str, auto_patch: bool) -> R<()> {
    if vanilla {
        restore_vanilla(root)?;
    } else if auto_patch {
        patch_gameinfo(root)?; // game updates overwrite gameinfo.gi, so re-check every launch
    }
    let args = opts.trim();
    let url = if args.is_empty() {
        format!("steam://rungameid/{APP_ID}")
    } else {
        format!("steam://run/{APP_ID}//{}/", urlenc(args))
    };
    open_url(&url)
}
