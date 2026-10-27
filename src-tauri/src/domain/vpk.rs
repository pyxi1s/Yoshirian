//! Minimal VPK (v1/v2) reader: the optional `.yoshirian` metadata file (+ thumbnail) and the sounds inside a mod.
use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub struct VpkMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub thumb: Option<(String, Vec<u8>)>, // (extension, bytes)
}

struct Entry {
    ext: String,
    dir: String,
    name: String,
    arch: u16,
    off: u32,
    len: u32,
    preload: Vec<u8>,
}

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn cstr(&mut self) -> Option<String> {
        let s = self.p;
        while self.p < self.b.len() && self.b[self.p] != 0 {
            self.p += 1;
        }
        if self.p >= self.b.len() {
            return None;
        }
        let r = String::from_utf8_lossy(&self.b[s..self.p]).to_string();
        self.p += 1;
        Some(r)
    }
    fn u16(&mut self) -> Option<u16> {
        let v = self.b.get(self.p..self.p + 2)?;
        self.p += 2;
        Some(u16::from_le_bytes([v[0], v[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        let v = self.b.get(self.p..self.p + 4)?;
        self.p += 4;
        Some(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    }
}

fn parse_tree(b: &[u8]) -> Option<Vec<Entry>> {
    let mut c = Cur { b, p: 0 };
    let mut out = vec![];
    loop {
        let ext = c.cstr()?;
        if ext.is_empty() {
            break;
        }
        loop {
            let dir = c.cstr()?;
            if dir.is_empty() {
                break;
            }
            loop {
                let name = c.cstr()?;
                if name.is_empty() {
                    break;
                }
                let _crc = c.u32()?;
                let pre = c.u16()? as usize;
                let arch = c.u16()?;
                let off = c.u32()?;
                let len = c.u32()?;
                let _term = c.u16()?;
                let preload = b.get(c.p..c.p + pre)?.to_vec();
                c.p += pre;
                out.push(Entry { ext: ext.clone(), dir: dir.clone(), name, arch, off, len, preload });
            }
        }
    }
    Some(out)
}

fn full_path(e: &Entry) -> String {
    let (ext, name, dir) = (e.ext.trim(), e.name.trim(), e.dir.trim().replace('\\', "/"));
    let file = if name.is_empty() {
        format!(".{ext}")
    } else if ext.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{ext}")
    };
    if dir.is_empty() { file } else { format!("{dir}/{file}") }
}

fn read_entry(path: &Path, data_start: u64, e: &Entry, max: u32) -> Option<Vec<u8>> {
    if e.len > max {
        return None;
    }
    let mut data = e.preload.clone();
    if e.len > 0 {
        let (mut f, pos) = if e.arch == 0x7FFF {
            (File::open(path).ok()?, data_start + e.off as u64)
        } else {
            let stem = path.file_stem()?.to_string_lossy().to_string();
            let base = stem.strip_suffix("_dir").unwrap_or(&stem).to_string();
            (File::open(path.with_file_name(format!("{base}_{:03}.vpk", e.arch))).ok()?, e.off as u64)
        };
        f.seek(SeekFrom::Start(pos)).ok()?;
        let mut buf = vec![0u8; e.len as usize];
        f.read_exact(&mut buf).ok()?;
        data.extend(buf);
    }
    Some(data)
}

fn unquote(v: &str) -> String {
    v.trim().trim_end_matches(',').trim().trim_matches(|c: char| c == '"' || c == '\'').trim().to_string()
}

/// Accepts JSON, or simple `key : "value"` / `key = "value"` lines.
fn parse_meta(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let t = text.trim_start_matches('\u{feff}').trim();
    let (mut name, mut desc, mut thumb) = (None, None, None);
    let mut put = |k: &str, v: String| {
        if v.is_empty() {
            return;
        }
        match k.trim().trim_matches('"').to_lowercase().as_str() {
            "name" | "title" => name = Some(v),
            "description" | "desc" => desc = Some(v),
            "thumb" | "thump" | "thumbnail" | "image" | "icon" => thumb = Some(v),
            _ => {}
        }
    };
    if t.starts_with('{') {
        if let Ok(serde_json::Value::Object(m)) = serde_json::from_str::<serde_json::Value>(t) {
            for (k, v) in m {
                if let Some(s) = v.as_str() {
                    put(&k, s.to_string());
                }
            }
        }
    } else {
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') || l.starts_with("//") {
                continue;
            }
            if let Some(i) = l.find(|c: char| c == ':' || c == '=') {
                put(&l[..i], unquote(&l[i + 1..]));
            }
        }
    }
    (name, desc, thumb)
}

/// Largest directory tree we are willing to read from a mod file.
const MAX_TREE: u64 = 64 * 1024 * 1024;

fn load(path: &Path) -> Option<(Vec<Entry>, u64)> {
    let mut f = File::open(path).ok()?;
    let mut h = [0u8; 12];
    f.read_exact(&mut h).ok()?;
    if u32::from_le_bytes([h[0], h[1], h[2], h[3]]) != 0x55AA1234 {
        return None;
    }
    let ver = u32::from_le_bytes([h[4], h[5], h[6], h[7]]);
    let tree_size = u32::from_le_bytes([h[8], h[9], h[10], h[11]]) as u64;
    let hdr = match ver {
        1 => 12,
        2 => 28,
        _ => return None,
    };
    if tree_size == 0 || tree_size > MAX_TREE {
        return None;
    }
    f.seek(SeekFrom::Start(hdr)).ok()?;
    let mut tree = vec![0u8; tree_size as usize];
    f.read_exact(&mut tree).ok()?;
    Some((parse_tree(&tree)?, hdr + tree_size))
}

// ---------- sounds inside a vpk ----------
#[derive(Serialize)]
pub struct SoundEntry {
    pub path: String,
    pub size: u32,
}

const SND_EXT: [&str; 6] = ["vsnd_c", "vsnd", "wav", "mp3", "ogg", "flac"];

pub fn list_sounds(path: &Path) -> Vec<SoundEntry> {
    let Some((entries, _)) = load(path) else { return vec![] };
    let mut v: Vec<SoundEntry> = entries
        .iter()
        .filter(|e| SND_EXT.contains(&e.ext.trim().to_lowercase().as_str()))
        .map(|e| SoundEntry { path: full_path(e), size: e.len.saturating_add(e.preload.len() as u32) })
        .collect();
    v.sort_by(|a, b| a.path.cmp(&b.path));
    v.truncate(500);
    v
}

fn detect(t: &[u8]) -> Option<(String, Vec<u8>)> {
    if t.len() > 12 && &t[0..4] == b"RIFF" && &t[8..12] == b"WAVE" {
        return Some(("audio/wav".into(), t.to_vec()));
    }
    if t.len() > 4 && &t[0..4] == b"OggS" {
        return Some(("audio/ogg".into(), t.to_vec()));
    }
    if (t.len() > 3 && &t[0..3] == b"ID3") || (t.len() > 2 && t[0] == 0xFF && (t[1] & 0xE0) == 0xE0) {
        return Some(("audio/mpeg".into(), t.to_vec()));
    }
    None
}

/// A compiled Source 2 sound (.vsnd_c) is a resource header followed by the raw wav/mp3/ogg stream.
fn vsnd_payload(b: &[u8]) -> Option<(String, Vec<u8>)> {
    if b.len() >= 16 {
        let rd = |o: usize| -> usize { u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize };
        let table = 8usize.saturating_add(rd(8));
        let count = rd(12).min(32);
        let mut end = 0usize;
        for i in 0..count {
            let p = table.saturating_add(i * 12);
            if p.saturating_add(12) > b.len() {
                break;
            }
            let f = p + 4;
            end = end.max(f.saturating_add(rd(f)).saturating_add(rd(f + 4)));
        }
        if end > 0 && end < b.len() {
            let t = &b[end..];
            if let Some(r) = detect(t) {
                return Some(r);
            }
            for i in 0..t.len().min(64) {
                if t[i] == 0xFF && i + 1 < t.len() && (t[i + 1] & 0xE0) == 0xE0 {
                    return Some(("audio/mpeg".into(), t[i..].to_vec()));
                }
            }
        }
    }
    // fallback: look for a known container signature anywhere in the file
    let pos = b.windows(4).position(|w| w == b"RIFF" || w == b"OggS").or_else(|| b.windows(3).position(|w| w == b"ID3"))?;
    detect(&b[pos..])
}

/// Returns (mime, audio bytes) for one sound entry of a vpk.
pub fn read_sound(path: &Path, entry_path: &str) -> Option<(String, Vec<u8>)> {
    let (entries, ds) = load(path)?;
    let e = entries.iter().find(|e| full_path(e) == entry_path)?;
    let data = read_entry(path, ds, e, 30 * 1024 * 1024)?;
    match e.ext.trim().to_lowercase().as_str() {
        "wav" => Some(("audio/wav".into(), data)),
        "mp3" => Some(("audio/mpeg".into(), data)),
        "ogg" => Some(("audio/ogg".into(), data)),
        "flac" => Some(("audio/flac".into(), data)),
        _ => vsnd_payload(&data),
    }
}

pub fn read_meta(path: &Path) -> Option<VpkMeta> {
    let (entries, data_start) = load(path)?;

    let me = entries.iter().find(|e| {
        let ext = e.ext.trim().to_lowercase();
        let name = e.name.trim().trim_start_matches('.').to_lowercase();
        ext == "yoshirian" || (matches!(ext.as_str(), "json" | "toml" | "txt" | "cfg") && name == "yoshirian")
    })?;
    let text = String::from_utf8_lossy(&read_entry(path, data_start, me, 256 * 1024)?).to_string();
    let (name, description, thumb_path) = parse_meta(&text);

    let thumb = thumb_path.and_then(|tp| {
        let want = tp.trim().replace('\\', "/").trim_start_matches("./").trim_start_matches('/').to_lowercase();
        let e = entries.iter().find(|e| {
            let p = full_path(e).to_lowercase();
            p == want || p.ends_with(&format!("/{want}"))
        })?;
        let bytes = read_entry(path, data_start, e, 16 * 1024 * 1024)?;
        let ext = e.ext.trim().to_lowercase();
        Some((if ext.is_empty() { "png".to_string() } else { ext }, bytes))
    });
    Some(VpkMeta { name, description, thumb })
}
