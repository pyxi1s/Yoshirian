//! The mod list (`.yoshirian.json`): data model, loading, saving and renaming the vpk files on disk.
use super::paths::{addons, assets};
use super::util::{e, ext_of, sanitize, R};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

pub const META: &str = ".yoshirian.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Mod {
    pub yoshirian_id: String,
    pub name: String,
    pub order: u32,
    pub current_file: String, // relative to addons, e.g. "pak01_dir.vpk" or "disable/name.vpk"
    pub is_enable: bool,
    #[serde(default)]
    pub requires: Vec<String>, // optional: names of mods this one depends on
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub image: Option<String>, // file name inside .yoshirian_assets
    #[serde(default)]
    pub gb_id: Option<u64>, // GameBanana mod id, if installed from there
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub gb_updated: i64, // GameBanana "updated" timestamp seen at install time (0 = unknown)
    #[serde(default)]
    pub gb_model: String, // "Mod" or "Sound" ("" = Mod)
    #[serde(default)]
    pub gb_part: u32, // index of this vpk inside the downloaded archive
    #[serde(default)]
    pub gb_audio: String, // preview sound url (sound mods)
    #[serde(default)]
    pub snd: Option<u32>, // number of sound files inside the vpk (None = not scanned yet)
    #[serde(default)]
    pub ys_scanned: bool, // the .yoshirian file inside the vpk has been looked at
    #[serde(default)]
    pub has_ys: bool, // the vpk contains a .yoshirian file
    #[serde(default)]
    pub edited: bool, // the user changed name/description/image by hand
}

pub fn norm_model(s: &str) -> String {
    if s.is_empty() { "Mod".to_string() } else { s.to_string() }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub enabled: Vec<String>, // mod ids, in load order
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    pub version: String, // major.minor.hotfix
    #[serde(rename = "mod")]
    pub mods: Vec<Mod>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for Meta {
    fn default() -> Self {
        Meta { version: "1.0.0".into(), mods: vec![], profiles: vec![] }
    }
}


fn is_pak(name: &str) -> bool {
    let n = name.to_lowercase();
    n.len() == 13 && n.starts_with("pak") && n.ends_with("_dir.vpk") && n[3..5].chars().all(|c| c.is_ascii_digit())
}

// ---------- metadata ----------
pub fn load(root: &Path) -> R<Meta> {
    let dir = addons(root);
    fs::create_dir_all(dir.join("disable")).map_err(e)?;
    let p = dir.join(META);
    let mut meta: Meta = if p.is_file() {
        serde_json::from_str(&fs::read_to_string(&p).map_err(e)?).map_err(e)?
    } else {
        Meta::default()
    };
    // drop entries whose file vanished
    meta.mods.retain(|m| dir.join(&m.current_file).is_file());
    // adopt unknown files found on disk
    let known: HashSet<String> = meta.mods.iter().map(|m| m.current_file.to_lowercase()).collect();
    let mut found: Vec<(String, bool)> = vec![];
    for ent in fs::read_dir(&dir).map_err(e)?.flatten() {
        let n = ent.file_name().to_string_lossy().to_string();
        if ent.path().is_file() && is_pak(&n) && !known.contains(&n.to_lowercase()) {
            found.push((n, true));
        }
    }
    found.sort();
    for ent in fs::read_dir(dir.join("disable")).map_err(e)?.flatten() {
        let n = ent.file_name().to_string_lossy().to_string();
        let rel = format!("disable/{n}");
        if n.to_lowercase().ends_with(".vpk") && !known.contains(&rel.to_lowercase()) {
            found.push((rel, false));
        }
    }
    let mut next = meta.mods.iter().map(|m| m.order).max().unwrap_or(0);
    for (file, en) in found {
        next += 1;
        let stem = Path::new(&file).file_stem().unwrap().to_string_lossy().to_string();
        meta.mods.push(Mod {
            yoshirian_id: uuid::Uuid::new_v4().to_string(),
            name: stem,
            order: next,
            current_file: file,
            is_enable: en,
            ..Default::default()
        });
    }
    meta.mods.sort_by_key(|m| m.order);
    Ok(meta)
}

pub fn save(root: &Path, meta: &Meta) -> R<()> {
    let s = serde_json::to_string_pretty(meta).map_err(e)?;
    fs::write(addons(root).join(META), s).map_err(e)
}

/// Renumber orders, then rename files: enabled -> pakNN_dir.vpk, disabled -> disable/<name>.vpk
pub fn apply(root: &Path, meta: &mut Meta) -> R<()> {
    let dir = addons(root);
    meta.mods.sort_by_key(|m| m.order);
    for (i, m) in meta.mods.iter_mut().enumerate() {
        m.order = (i + 1) as u32;
    }
    if meta.mods.iter().filter(|m| m.is_enable).count() > 99 {
        return Err("At most 99 mods can be enabled at once".into());
    }
    // phase 1: move everything to temp names to avoid collisions
    let mut tmps = vec![];
    for m in &meta.mods {
        let t = dir.join(format!(".tmp_{}", m.yoshirian_id));
        fs::rename(dir.join(&m.current_file), &t).map_err(e)?;
        tmps.push(t);
    }
    // phase 2: final names
    let (mut idx, mut used) = (0, HashSet::new());
    for (m, t) in meta.mods.iter_mut().zip(tmps) {
        let rel = if m.is_enable {
            idx += 1;
            format!("pak{:02}_dir.vpk", idx)
        } else {
            let base = sanitize(&m.name);
            let (mut n, mut k) = (base.clone(), 1);
            while !used.insert(n.to_lowercase()) {
                k += 1;
                n = format!("{base}_{k}");
            }
            format!("disable/{n}.vpk")
        };
        fs::rename(&t, dir.join(&rel)).map_err(e)?;
        m.current_file = rel;
    }
    save(root, meta)
}

// ---------- thumbnails (assets) ----------
pub fn remove_asset(root: &Path, id: &str) {
    if let Ok(rd) = fs::read_dir(assets(root)) {
        for en in rd.flatten() {
            if en.file_name().to_string_lossy().starts_with(id) {
                let _ = fs::remove_file(en.path());
            }
        }
    }
}

pub fn save_asset_bytes(root: &Path, id: &str, ext: &str, bytes: &[u8]) -> R<String> {
    fs::create_dir_all(assets(root)).map_err(e)?;
    remove_asset(root, id);
    let file = format!("{id}.{}", if ext.is_empty() { "png" } else { ext });
    fs::write(assets(root).join(&file), bytes).map_err(e)?;
    Ok(file)
}

pub fn copy_asset(root: &Path, id: &str, src: &str) -> R<String> {
    let ext = match ext_of(Path::new(src)).as_str() {
        "" => "png".to_string(),
        x => x.to_string(),
    };
    fs::create_dir_all(assets(root)).map_err(e)?;
    remove_asset(root, id);
    let file = format!("{id}.{ext}");
    fs::copy(src, assets(root).join(&file)).map_err(e)?;
    Ok(file)
}

// ---------- profiles ----------
pub fn save_profile(meta: &mut Meta, name: &str) {
    let mut en: Vec<&Mod> = meta.mods.iter().filter(|m| m.is_enable).collect();
    en.sort_by_key(|m| m.order);
    let ids = en.iter().map(|m| m.yoshirian_id.clone()).collect();
    meta.profiles.retain(|p| p.name != name);
    meta.profiles.push(Profile { name: name.into(), enabled: ids });
}

pub fn apply_profile(meta: &mut Meta, name: &str) -> R<()> {
    let p = meta.profiles.iter().find(|p| p.name == name).cloned().ok_or("Profile not found")?;
    for m in meta.mods.iter_mut() {
        match p.enabled.iter().position(|i| *i == m.yoshirian_id) {
            Some(pos) => {
                m.is_enable = true;
                m.order = pos as u32 + 1;
            }
            None => {
                m.is_enable = false;
                m.order += 1000;
            }
        }
    }
    Ok(())
}

