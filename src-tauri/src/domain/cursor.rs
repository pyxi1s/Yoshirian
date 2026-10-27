//! Cursor changer: library of cursor bmp files, one active per type, backups of the originals, presets.
use super::archive::{extract, is_archive};
use super::paths::assets;
use super::util::{e, ext_of, R};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

// (kind, label, game file, library folder, width, height)
pub static KINDS: [(&str, &str, &str, &str, u32, u32); 4] = [
    ("cursor", "Default cursor", "cursor.bmp", "cursor_bmp", 48, 58),
    ("ping", "Ping", "cursor_ping.bmp", "cursor_ping", 48, 58),
    ("shop", "Shop", "cursor_shop.bmp", "cursor_shop", 60, 60),
    ("commend", "Commend", "cursor_commend.bmp", "cursor_commend", 80, 80),
];
const IMG: [&str; 10] = ["bmp", "png", "jpg", "jpeg", "gif", "webp", "ico", "cur", "tif", "tiff"];

fn row(kind: &str) -> R<&'static (&'static str, &'static str, &'static str, &'static str, u32, u32)> {
    KINDS.iter().find(|r| r.0 == kind).ok_or_else(|| "Unknown cursor type".to_string())
}
pub fn game_dir(root: &Path) -> PathBuf {
    root.join("game").join("citadel").join("resource").join("cursors")
}
pub fn cdir(root: &Path) -> PathBuf {
    assets(root).join("cursor")
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CMod {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file: String, // relative to cdir, e.g. cursor_ping/<id>.bmp
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CProfile {
    pub name: String,
    #[serde(default)]
    pub map: HashMap<String, Option<String>>,
}
#[derive(Serialize, Deserialize, Default)]
pub struct CState {
    #[serde(default)]
    pub mods: Vec<CMod>,
    #[serde(default)]
    pub active: HashMap<String, Option<String>>,
    #[serde(default)]
    pub profiles: Vec<CProfile>,
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CItem {
    pub source: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub preset: String, // preset to create/extend with this cursor ("" = none)
}

pub fn load(root: &Path) -> CState {
    let mut st: CState = fs::read_to_string(cdir(root).join("cursors.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let d = cdir(root);
    st.mods.retain(|m| d.join(&m.file).is_file());
    let ids: Vec<String> = st.mods.iter().map(|m| m.id.clone()).collect();
    for v in st.active.values_mut() {
        if v.as_ref().map_or(false, |i| !ids.contains(i)) {
            *v = None;
        }
    }
    st
}
pub fn save(root: &Path, st: &CState) -> R<()> {
    fs::create_dir_all(cdir(root)).map_err(e)?;
    fs::write(cdir(root).join("cursors.json"), serde_json::to_string_pretty(st).map_err(e)?).map_err(e)
}

/// Copy the game's current cursor files to original/ — only while no mod is active for that type.
pub fn ensure_backup(root: &Path, st: &CState) {
    let od = cdir(root).join("original");
    let _ = fs::create_dir_all(&od);
    for r in KINDS.iter() {
        let (dst, src) = (od.join(r.2), game_dir(root).join(r.2));
        if !dst.exists() && src.is_file() && st.active.get(r.0).map_or(true, |a| a.is_none()) {
            let _ = fs::copy(&src, &dst);
        }
    }
}

pub fn activate(root: &Path, st: &mut CState, kind: &str, id: Option<&str>) -> R<()> {
    let r = row(kind)?;
    ensure_backup(root, st);
    let target = game_dir(root).join(r.2);
    let werr = |er: std::io::Error| format!("Could not write {}: {er}. Close the game and try again.", r.2);
    match id {
        Some(id) => {
            let src = cdir(root).join(&st.mods.iter().find(|m| m.id == id && m.kind == kind).ok_or("Cursor not found")?.file);
            fs::copy(&src, &target).map_err(werr)?;
            st.active.insert(kind.into(), Some(id.into()));
        }
        None => {
            let o = cdir(root).join("original").join(r.2);
            if o.is_file() {
                fs::copy(&o, &target).map_err(werr)?;
            } else if st.active.get(kind).map_or(false, |a| a.is_some()) {
                return Err(format!("The backup of {} is missing, cannot restore it", r.2));
            }
            st.active.insert(kind.into(), None);
        }
    }
    Ok(())
}

pub fn detect_kind(stem: &str, dims: Option<(u32, u32)>) -> &'static str {
    let s = stem.to_lowercase();
    if s.contains("commend") {
        "commend"
    } else if s.contains("ping") {
        "ping"
    } else if s.contains("shop") {
        "shop"
    } else {
        match dims {
            Some((60, 60)) => "shop",
            Some((80, 80)) => "commend",
            _ => "cursor",
        }
    }
}

fn walk_imgs(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for en in rd.flatten() {
            let p = en.path();
            if p.is_dir() {
                walk_imgs(&p, out);
            } else if IMG.contains(&ext_of(&p).as_str()) {
                out.push(p);
            }
        }
    }
}

fn is_cursor_name(stem: &str) -> bool {
    let s = stem.to_lowercase();
    ["cursor", "ping", "shop", "commend"].iter().any(|k| s.contains(k))
}

/// Images are used directly. Archives and folders are scanned for cursor images, grouped by folder:
/// one group  -> names "<archive>_<file>" and (optionally) one preset named "<archive>";
/// many groups (e.g. Player 1..4) -> names "<archive>_<folder>_<file>" and one preset per folder.
pub fn inspect(paths: &[String], tmp: &Path, auto_preset: bool, base: Option<&str>) -> R<Vec<CItem>> {
    let mut out = vec![];
    let stem_of = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    for p in paths {
        let pb = Path::new(p);
        let ext = ext_of(pb);
        if pb.is_file() && IMG.contains(&ext.as_str()) {
            let stem = stem_of(pb);
            let kind = detect_kind(&stem, image::image_dimensions(pb).ok()).to_string();
            out.push(CItem { source: p.clone(), name: stem, kind, preset: String::new() });
            continue;
        }
        let (dir, arch) = if pb.is_dir() {
            (pb.to_path_buf(), pb.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        } else if is_archive(pb) {
            let dest = tmp.join(uuid::Uuid::new_v4().to_string());
            extract(pb, &dest)?;
            (dest, stem_of(pb))
        } else {
            continue;
        };
        // when the pack comes from a GameBanana page, the page name is used instead of the archive name
        let arch = base.map(|b| b.trim().to_string()).filter(|b| !b.is_empty()).unwrap_or(arch);
        let mut all = vec![];
        walk_imgs(&dir, &mut all);
        all.sort();
        let mut v: Vec<PathBuf> = all.iter().filter(|f| is_cursor_name(&stem_of(f))).cloned().collect();
        if v.is_empty() {
            v = all; // nothing is named like a cursor: take everything the user gave us
        }
        // group by folder (relative to the extraction root)
        let mut groups: Vec<(Vec<String>, Vec<PathBuf>)> = vec![];
        for f in v {
            let rel: Vec<String> = f
                .parent()
                .and_then(|d| d.strip_prefix(&dir).ok())
                .map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            match groups.iter_mut().find(|g| g.0 == rel) {
                Some(g) => g.1.push(f),
                None => groups.push((rel, vec![f])),
            }
        }
        let multi = groups.len() > 1;
        let mut common = 0;
        if multi {
            let min = groups.iter().map(|g| g.0.len()).min().unwrap_or(0);
            while common < min && groups.iter().all(|g| g.0[common] == groups[0].0[common]) {
                common += 1;
            }
        }
        for (rel, files) in groups {
            let label = if multi { let r = rel[common..].join("_").replace(char::is_whitespace, ""); if r.is_empty() { "main".to_string() } else { r } } else { String::new() };
            let preset = match (auto_preset, multi) {
                (false, _) => String::new(),
                (true, true) => format!("{arch}_{label}"),
                (true, false) => arch.clone(),
            };
            for f in files {
                let stem = stem_of(&f);
                let kind = detect_kind(&stem, image::image_dimensions(&f).ok()).to_string();
                let name = if multi { format!("{arch}_{label}_{stem}") } else { format!("{arch}_{stem}") };
                out.push(CItem { source: f.to_string_lossy().to_string(), name, kind, preset: preset.clone() });
            }
        }
    }
    if out.is_empty() {
        return Err("No cursor images found (bmp, png, jpg, gif, webp, ico, or an archive / folder containing them)".into());
    }
    Ok(out)
}

fn bmp_bpp(p: &Path) -> Option<u16> {
    let mut b = [0u8; 30];
    fs::File::open(p).ok()?.read_exact(&mut b).ok()?;
    if &b[0..2] != b"BM" {
        return None;
    }
    Some(u16::from_le_bytes([b[28], b[29]]))
}

/// Make a bmp of exactly w x h. A bmp that already has the right size is copied untouched.
/// Anything else is scaled to fit (anchored top-left, where cursor tips are) and written like the original (24/32 bit).
fn convert(src: &Path, dst: &Path, w: u32, h: u32, bpp: Option<u16>) -> R<()> {
    if ext_of(src) == "bmp" {
        if let Ok((iw, ih)) = image::image_dimensions(src) {
            if iw == w && ih == h {
                fs::copy(src, dst).map_err(e)?;
                return Ok(());
            }
        }
    }
    let img = image::open(src).map_err(|er| format!("Could not read {}: {er}", src.file_name().unwrap_or_default().to_string_lossy()))?;
    let rgba = img.to_rgba8();
    let (iw, ih) = rgba.dimensions();
    let scaled = if iw == w && ih == h {
        rgba
    } else {
        let sc = f64::min(w as f64 / iw as f64, h as f64 / ih as f64);
        let (nw, nh) = (((iw as f64 * sc).round() as u32).max(1), ((ih as f64 * sc).round() as u32).max(1));
        image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Lanczos3)
    };
    let mut canvas = image::RgbaImage::new(w, h);
    image::imageops::overlay(&mut canvas, &scaled, 0, 0);
    let res = if bpp.map_or(true, |b| b == 32) {
        canvas.save_with_format(dst, image::ImageFormat::Bmp)
    } else {
        image::DynamicImage::ImageRgba8(canvas).to_rgb8().save_with_format(dst, image::ImageFormat::Bmp)
    };
    res.map_err(e)
}

pub fn add(root: &Path, items: &[CItem]) -> R<()> {
    if items.iter().any(|i| i.name.trim().is_empty()) {
        return Err("Every cursor needs a name".into());
    }
    let mut st = load(root);
    let mut made: Vec<(String, String, String)> = vec![]; // (preset, kind, id)
    for it in items {
        let r = row(&it.kind)?;
        let id = uuid::Uuid::new_v4().to_string();
        let file = format!("{}/{}.bmp", r.3, id);
        let dst = cdir(root).join(&file);
        fs::create_dir_all(dst.parent().unwrap()).map_err(e)?;
        convert(Path::new(&it.source), &dst, r.4, r.5, bmp_bpp(&cdir(root).join("original").join(r.2)))?;
        if !it.preset.trim().is_empty() {
            made.push((it.preset.trim().to_string(), it.kind.clone(), id.clone()));
        }
        st.mods.push(CMod { id, name: it.name.trim().to_string(), kind: it.kind.clone(), file });
    }
    // auto presets: one per preset name, missing types stay "Original"
    let mut names: Vec<String> = vec![];
    for (p, _, _) in &made {
        if !names.contains(p) {
            names.push(p.clone());
        }
    }
    for p in names {
        let mut map: HashMap<String, Option<String>> = KINDS.iter().map(|r| (r.0.to_string(), None)).collect();
        for (pp, k, id) in &made {
            if *pp == p && map.get(k).map_or(false, |v| v.is_none()) {
                map.insert(k.clone(), Some(id.clone()));
            }
        }
        st.profiles.retain(|x| x.name != p);
        st.profiles.push(CProfile { name: p, map });
    }
    save(root, &st)
}

pub fn rename(root: &Path, id: &str, name: &str) -> R<()> {
    if name.trim().is_empty() {
        return Err("Name is required".into());
    }
    let mut st = load(root);
    st.mods.iter_mut().find(|m| m.id == id).ok_or("Cursor not found")?.name = name.trim().to_string();
    save(root, &st)
}

pub fn delete(root: &Path, id: &str) -> R<()> {
    let mut st = load(root);
    let Some(m) = st.mods.iter().find(|m| m.id == id).cloned() else { return Ok(()) };
    if st.active.get(&m.kind).map_or(false, |a| a.as_deref() == Some(id)) {
        activate(root, &mut st, &m.kind, None)?;
    }
    let _ = fs::remove_file(cdir(root).join(&m.file));
    st.mods.retain(|x| x.id != id);
    save(root, &st)
}

pub fn set_active(root: &Path, kind: &str, id: Option<&str>) -> R<()> {
    let mut st = load(root);
    activate(root, &mut st, kind, id)?;
    save(root, &st)
}

pub fn restore_all(root: &Path) -> R<()> {
    let mut st = load(root);
    for r in KINDS.iter() {
        activate(root, &mut st, r.0, None)?;
    }
    save(root, &st)
}

pub fn reapply(root: &Path) -> R<()> {
    let mut st = load(root);
    for r in KINDS.iter() {
        if let Some(Some(id)) = st.active.get(r.0).cloned() {
            activate(root, &mut st, r.0, Some(&id))?;
        }
    }
    save(root, &st)
}

pub fn profile_op(root: &Path, op: &str, name: &str) -> R<()> {
    let mut st = load(root);
    match op {
        "save" => {
            if name.trim().is_empty() {
                return Err("Preset name is required".into());
            }
            let map: HashMap<String, Option<String>> = KINDS.iter().map(|r| (r.0.to_string(), st.active.get(r.0).cloned().flatten())).collect();
            st.profiles.retain(|p| p.name != name);
            st.profiles.push(CProfile { name: name.trim().to_string(), map });
        }
        "delete" => st.profiles.retain(|p| p.name != name),
        "apply" => {
            let p = st.profiles.iter().find(|p| p.name == name).cloned().ok_or("Preset not found")?;
            for r in KINDS.iter() {
                let want = p.map.get(r.0).cloned().flatten().filter(|i| st.mods.iter().any(|m| &m.id == i));
                activate(root, &mut st, r.0, want.as_deref())?;
            }
        }
        _ => return Err("Unknown operation".into()),
    }
    save(root, &st)
}

#[derive(Serialize)]
pub struct KindInfo {
    pub kind: String,
    pub label: String,
    pub file: String,
    pub w: u32,
    pub h: u32,
    pub game_path: Option<String>,
    pub original_path: Option<String>,
}
#[derive(Serialize)]
pub struct CModView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: String,
}
#[derive(Serialize)]
pub struct CursorInfo {
    pub dir: String,
    pub kinds: Vec<KindInfo>,
    pub mods: Vec<CModView>,
    pub active: HashMap<String, Option<String>>,
    pub profiles: Vec<CProfile>,
}

pub fn info(root: &Path) -> R<CursorInfo> {
    let gd = game_dir(root);
    if !gd.is_dir() {
        return Err(format!("Cursor folder not found: {}", gd.display()));
    }
    let st = load(root);
    ensure_backup(root, &st);
    let od = cdir(root).join("original");
    let s = |p: PathBuf| if p.is_file() { Some(p.to_string_lossy().to_string()) } else { None };
    Ok(CursorInfo {
        dir: gd.to_string_lossy().to_string(),
        kinds: KINDS.iter().map(|r| KindInfo { kind: r.0.into(), label: r.1.into(), file: r.2.into(), w: r.4, h: r.5, game_path: s(gd.join(r.2)), original_path: s(od.join(r.2)) }).collect(),
        mods: st.mods.iter().map(|m| CModView { id: m.id.clone(), name: m.name.clone(), kind: m.kind.clone(), path: cdir(root).join(&m.file).to_string_lossy().to_string() }).collect(),
        active: KINDS.iter().map(|r| (r.0.to_string(), st.active.get(r.0).cloned().flatten())).collect(),
        profiles: st.profiles,
    })
}
