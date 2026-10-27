//! Installing mods: archives (see `archive`) -> .vpk files -> entries in the mod list (always added disabled).
use super::archive::{extract, sniff, Kind};
use super::meta::{apply, copy_asset, load, norm_model, Mod};
use super::paths::addons;
use super::util::{e, R};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InstallItem {
    pub source: String, // path to a .vpk file
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub image: Option<String>, // local image path to copy in
    #[serde(default)]
    pub gb_id: Option<u64>,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub embedded_name: bool, // name came from a .yoshirian file inside the vpk
    #[serde(default)]
    pub gb_updated: i64,
    #[serde(default)]
    pub gb_model: String,
    #[serde(default)]
    pub gb_part: u32,
    #[serde(default)]
    pub replace: bool, // update: overwrite the installed mod with the same gb_id/part instead of adding a new one
    #[serde(default)]
    pub gb_audio: String,
    #[serde(default)]
    pub embedded: bool, // the vpk has a .yoshirian file
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for en in rd.flatten() {
            let p = en.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().map(|x| x.eq_ignore_ascii_case("vpk")).unwrap_or(false) {
                out.push(p);
            }
        }
    }
}


/// Turn dropped/selected files into install items. Archives are extracted into `tmp`.
pub fn inspect(paths: &[String], tmp: &Path) -> R<Vec<InstallItem>> {
    let mut out = vec![];
    for p in paths {
        let pb = Path::new(p);
        let stem = pb.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or("mod".into());
        if sniff(pb) == Kind::Vpk {
            out.push(InstallItem { source: p.clone(), name: stem, ..Default::default() });
            continue;
        }
        if !pb.is_file() {
            continue;
        }
        let dest = tmp.join(uuid::Uuid::new_v4().to_string());
        extract(pb, &dest)?;
        let mut v = vec![];
        walk(&dest, &mut v);
        v.sort();
        let multi = v.len() > 1;
        for (i, f) in v.into_iter().enumerate() {
            let name = if multi { format!("{stem}{:02}", i + 1) } else { stem.clone() };
            out.push(InstallItem { source: f.to_string_lossy().to_string(), name, ..Default::default() });
        }
    }
    if out.is_empty() {
        return Err("No .vpk file found in the selected files".into());
    }
    for it in out.iter_mut() {
        apply_embedded(it, tmp);
    }
    Ok(out)
}

/// Read an optional ".yoshirian" file inside the vpk and use it for default name / description / thumbnail.
fn apply_embedded(it: &mut InstallItem, tmp: &Path) {
    if let Some(m) = super::vpk::read_meta(Path::new(&it.source)) {
        it.embedded = true;
        if let Some(n) = m.name.filter(|n| !n.trim().is_empty()) {
            it.name = n.trim().to_string();
            it.embedded_name = true;
        }
        if let Some(d) = m.description {
            it.description = d;
        }
        if let Some((ext, bytes)) = m.thumb {
            let _ = fs::create_dir_all(tmp);
            let p = tmp.join(format!("{}_thumb.{}", uuid::Uuid::new_v4(), ext));
            if fs::write(&p, bytes).is_ok() {
                it.image = Some(p.to_string_lossy().to_string());
            }
        }
    }
}

pub fn install(root: &Path, items: &[InstallItem]) -> R<()> {
    if items.iter().any(|i| i.name.trim().is_empty()) {
        return Err("Mod name is required".into());
    }
    let mut meta = load(root)?;
    let dir = addons(root);
    for it in items {
        if it.replace && it.gb_id.is_some() {
            let model = norm_model(&it.gb_model);
            if let Some(m) = meta.mods.iter_mut().find(|m| m.gb_id == it.gb_id && norm_model(&m.gb_model) == model && m.gb_part == it.gb_part) {
                fs::copy(&it.source, dir.join(&m.current_file)).map_err(|er| format!("Could not replace the installed file: {er}. Close the game and try again."))?;
                m.gb_updated = it.gb_updated;
                if !it.gb_audio.is_empty() {
                    m.gb_audio = it.gb_audio.clone();
                }
                m.snd = None; // file changed: rescan
                m.ys_scanned = false;
                continue;
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let rel = format!("disable/{id}.vpk");
        fs::copy(&it.source, dir.join(&rel)).map_err(e)?;
        let image = match it.image.as_deref() {
            Some(p) if !p.is_empty() => Some(copy_asset(root, &id, p)?),
            _ => None,
        };
        let order = meta.mods.iter().map(|m| m.order).max().unwrap_or(0) + 1;
        // new mods always start disabled
        meta.mods.push(Mod {
            yoshirian_id: id,
            name: it.name.trim().to_string(),
            order,
            current_file: rel,
            is_enable: false,
            description: it.description.clone(),
            image,
            gb_id: it.gb_id,
            author: it.author.clone(),
            gb_updated: it.gb_updated,
            gb_model: it.gb_model.clone(),
            gb_part: it.gb_part,
            gb_audio: it.gb_audio.clone(),
            ys_scanned: true, // embedded data was already applied while inspecting
            has_ys: it.embedded,
            ..Default::default()
        });
    }
    apply(root, &mut meta)
}

