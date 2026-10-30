//! GameBanana (Deadlock) API client and `yoshirian://` link parsing.
use super::util::{e, R};
use serde::Serialize;
use serde_json::Value;

pub const GAME_ID: u64 = 20948;
pub const API: &str = "https://gamebanana.com/apiv11";

type LogFn = Box<dyn Fn(&str, &str) + Send + Sync>;
static LOGGER: std::sync::OnceLock<LogFn> = std::sync::OnceLock::new();
/// The app plugs its log in here (level, message), so requests to GameBanana show up in Settings -> Logs.
pub fn set_logger(f: impl Fn(&str, &str) + Send + Sync + 'static) {
    let _ = LOGGER.set(Box::new(f));
}
pub(super) fn note(level: &str, msg: &str) {
    if let Some(f) = LOGGER.get() {
        f(level, msg);
    }
}

#[derive(Serialize, Clone, Default)]
pub struct GbFile {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub url: String,
    pub description: String, // the uploader's short note for this file ("All playable files combined")
    pub added: i64,
}

/// One line of an update's change log, e.g. { cat: "Addition", text: "New blue dash trail remover" }.
#[derive(Serialize, Clone, Default)]
pub struct GbChange {
    pub cat: String,
    pub text: String,
}

/// An entry of the page's "Updates" list.
#[derive(Serialize, Clone, Default)]
pub struct GbUpdate {
    pub name: String,
    pub version: String,
    pub date: i64,
    pub changes: Vec<GbChange>,
    pub text: String, // optional free text (html)
    pub files: Vec<String>,
    #[serde(skip)]
    pub keys: Vec<String>, // field names GameBanana sent (for the log when the change log looks empty)
}

#[derive(Serialize, Clone, Default)]
pub struct GbMod {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub text: String, // html body, stripped by the UI
    pub thumb: Option<String>,
    pub images: Vec<String>,
    pub author: String,
    pub author_id: u64,
    pub author_avatar: Option<String>,
    pub added: i64,
    pub updated: i64,
    pub category: String,
    pub super_category: String,
    pub likes: u64,
    pub downloads: u64,
    pub views: u64,
    pub files: Vec<GbFile>,
    pub version: String,
    pub updates: Vec<GbUpdate>, // filled for detail pages only
    pub url: String,
    pub model: String, // "Mod" or "Sound"
    pub audio: Option<String>, // preview sound url, if any
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Lookup {
    Mod { item: GbMod },
    List {
        items: Vec<GbMod>,
        has_more: bool,          // "Load more" makes sense
        author: Option<String>,  // the author the results are limited to, if any
        note: Option<String>,    // e.g. "only the newest 600 mods were searched"
    },
    Direct { url: String },
}

pub(super) async fn get(url: &str, q: &[(&str, String)]) -> R<Value> {
    note("DEBUG", &format!("GameBanana GET {url} {}", q.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")));
    let c = reqwest::Client::builder().user_agent("yoshirian/0.1").build().map_err(e)?;
    let r = c.get(url).query(q).send().await.map_err(e)?;
    if !r.status().is_success() {
        return Err(format!("GameBanana returned {}", r.status()));
    }
    r.json::<Value>().await.map_err(e)
}

fn st(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn nu(v: &Value, k: &str) -> u64 {
    v[k].as_u64().unwrap_or(0)
}
fn ii(v: &Value, k: &str) -> i64 {
    v[k].as_i64().unwrap_or(0)
}
fn image_url(im: &Value, key: &str) -> Option<String> {
    let f = st(im, key);
    if f.is_empty() { None } else { Some(format!("{}/{}", st(im, "_sBaseUrl"), f)) }
}

fn is_audio_name(n: &str) -> bool {
    let l = n.to_lowercase();
    let base = l.split('?').next().unwrap_or("");
    [".mp3", ".ogg", ".wav", ".m4a", ".flac"].iter().any(|e| base.ends_with(e))
}

/// Looks anywhere inside the preview media for a link to an audio file.
fn find_audio(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if s.starts_with("http") && is_audio_name(s) => Some(s.clone()),
        Value::Array(a) => a.iter().find_map(find_audio),
        Value::Object(o) => o.values().find_map(find_audio),
        _ => None,
    }
}

fn parse(m: &Value) -> GbMod {
    let id = nu(m, "_idRow");
    let imgs = m["_aPreviewMedia"]["_aImages"].as_array().cloned().unwrap_or_default();
    let images: Vec<String> = imgs.iter().filter_map(|i| image_url(i, "_sFile")).collect();
    let thumb = imgs.first().and_then(|i| image_url(i, "_sFile530").or_else(|| image_url(i, "_sFile")));
    let files: Vec<GbFile> = m["_aFiles"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|f| {
                    let u = st(f, "_sDownloadUrl");
                    GbFile {
                        id: nu(f, "_idRow"),
                        name: st(f, "_sFile"),
                        size: nu(f, "_nFilesize"),
                        url: if u.is_empty() { format!("https://gamebanana.com/dl/{}", nu(f, "_idRow")) } else { u },
                        description: st(f, "_sDescription").trim().to_string(),
                        added: ii(f, "_tsDateAdded"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let audio = find_audio(&m["_aPreviewMedia"]).or_else(|| files.iter().find(|f| is_audio_name(&f.name)).map(|f| f.url.clone()));
    let sup = st(&m["_aSuperCategory"], "_sName");
    let url = st(m, "_sProfileUrl");
    let upd = ii(m, "_tsDateUpdated");
    GbMod {
        id,
        name: st(m, "_sName"),
        description: st(m, "_sDescription"),
        text: st(m, "_sText"),
        thumb,
        images,
        author: st(&m["_aSubmitter"], "_sName"),
        author_id: nu(&m["_aSubmitter"], "_idRow"),
        author_avatar: Some(st(&m["_aSubmitter"], "_sAvatarUrl")).filter(|s| !s.is_empty()),
        added: ii(m, "_tsDateAdded"),
        updated: if upd > 0 { upd } else { ii(m, "_tsDateModified") },
        category: st(&m["_aCategory"], "_sName"),
        super_category: if sup.is_empty() { st(&m["_aRootCategory"], "_sName") } else { sup },
        likes: nu(m, "_nLikeCount"),
        downloads: nu(m, "_nDownloadCount"),
        views: nu(m, "_nViewCount"),
        files,
        version: st(m, "_sVersion"),
        updates: vec![],
        url: if url.is_empty() { format!("https://gamebanana.com/mods/{id}") } else { url },
        model: { let m = st(m, "_sModelName"); if m.is_empty() { "Mod".to_string() } else { m } },
        audio,
    }
}

/// First string value of `o` whose key (lower case) contains one of `needles`, trying the needles in order.
fn pick_str(o: &serde_json::Map<String, Value>, needles: &[&str], skip: Option<&str>) -> String {
    for n in needles {
        for (k, v) in o {
            if Some(k.as_str()) != skip && k.to_lowercase().contains(n) {
                if let Some(s) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    return s.to_string();
                }
            }
        }
    }
    String::new()
}

/// One change log line. GameBanana calls the fields `_sCat` / `_sText`, but this also accepts other
/// spellings (`category`, `text`, `description` ...) and plain strings.
fn parse_change(c: &Value) -> Option<GbChange> {
    if let Some(s) = c.as_str() {
        return Some(GbChange { cat: String::new(), text: s.trim().to_string() }).filter(|c| !c.text.is_empty());
    }
    let o = c.as_object()?;
    let cat_key = o.iter().find(|(k, v)| v.is_string() && (k.to_lowercase().contains("cat") || k.to_lowercase().contains("type"))).map(|(k, _)| k.clone());
    let cat = cat_key.as_deref().and_then(|k| o.get(k)).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let mut text = pick_str(o, &["text", "desc", "change", "note", "title"], cat_key.as_deref());
    if text.is_empty() {
        text = o.iter().filter(|(k, _)| Some(k.as_str()) != cat_key.as_deref()).find_map(|(_, v)| v.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())).unwrap_or_default();
    }
    Some(GbChange { cat, text }).filter(|c| !c.text.is_empty())
}

fn parse_changes(u: &Value) -> Vec<GbChange> {
    let Some(o) = u.as_object() else { return vec![] };
    for (k, v) in o {
        if k.to_lowercase().contains("change") {
            if let Some(a) = v.as_array() {
                let list: Vec<GbChange> = a.iter().filter_map(parse_change).collect();
                if !list.is_empty() {
                    return list;
                }
            }
        }
    }
    vec![]
}

fn parse_update(u: &Value) -> GbUpdate {
    let files = u["_aFiles"]
        .as_array()
        .map(|a| a.iter().filter_map(|f| f.as_str().map(String::from).or_else(|| Some(st(f, "_sFile")).filter(|s| !s.is_empty()))).collect())
        .unwrap_or_default();
    GbUpdate {
        name: st(u, "_sName"),
        version: st(u, "_sVersion"),
        date: ii(u, "_tsDateAdded"),
        changes: parse_changes(u),
        text: st(u, "_sText"),
        files,
        keys: u.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default(),
    }
}

fn parse_updates(list: &Value) -> Vec<GbUpdate> {
    list.as_array().map(|a| a.iter().map(parse_update).filter(|u| !u.name.is_empty() || !u.changes.is_empty()).collect()).unwrap_or_default()
}

/// The "Updates" list of a page. Best effort: any failure just means "no update log".
async fn fetch_updates(model: &str, id: u64) -> Vec<GbUpdate> {
    match get(&format!("{API}/{model}/{id}/Updates"), &[("_nPage", "1".into())]).await {
        Ok(v) => parse_updates(&v["_aRecords"]),
        Err(_) => vec![],
    }
}

pub(super) fn records(v: &Value) -> Vec<GbMod> {
    v["_aRecords"]
        .as_array()
        .map(|a| a.iter().filter(|r| r["_sModelName"].as_str().map_or(true, |s| s == "Mod" || s == "Sound")).map(parse).collect())
        .unwrap_or_default()
}

/// Page data for installing / update checks (light: no update log).
pub async fn fetch_item(model: &str, id: u64) -> R<GbMod> {
    fetch_page(model, id, false).await
}

/// Page data for the detail view: same as `fetch_item` plus the update log.
pub async fn fetch_detail(model: &str, id: u64) -> R<GbMod> {
    fetch_page(model, id, true).await
}

async fn fetch_page(model: &str, id: u64, with_updates: bool) -> R<GbMod> {
    let model = if model.is_empty() { "Mod" } else { model };
    let profile = format!("{API}/{model}/{id}/ProfilePage");
    let (v, updates) = if with_updates {
        futures_util::future::join(get(&profile, &[]), fetch_updates(model, id)).await
    } else {
        (get(&profile, &[]).await, vec![])
    };
    let v = v?;
    if v["_idRow"].is_null() {
        return Err("Not found on GameBanana".into());
    }
    if let Some(g) = v["_aGame"]["_idRow"].as_u64() {
        if g != GAME_ID {
            return Err("This item is not for Deadlock".into());
        }
    }
    let mut m = parse(&v);
    m.model = model.to_string();
    if with_updates {
        m.updates = if updates.is_empty() { parse_updates(&v["_aLatestUpdates"]) } else { updates };
    }
    if m.files.is_empty() {
        if let Ok(f) = get(&format!("{API}/{model}/{id}"), &[("_csvProperties", "_aFiles".into())]).await {
            m.files = parse(&f).files;
        }
    }
    Ok(m)
}

/// When the model (Mod / Sound) is unknown: try Mod first, then Sound.
pub async fn fetch_any(id: u64) -> R<GbMod> {
    match fetch_item("Mod", id).await {
        Ok(m) => Ok(m),
        Err(_) => fetch_item("Sound", id).await,
    }
}

async fn fetch_detail_any(id: u64) -> R<GbMod> {
    match fetch_detail("Mod", id).await {
        Ok(m) => Ok(m),
        Err(_) => fetch_detail("Sound", id).await,
    }
}

/// "Latest updates" of the game (mods and sounds mixed).
pub async fn browse(page: u32) -> R<Vec<GbMod>> {
    subfeed_at(API, page).await
}

pub(super) async fn subfeed_at(base: &str, page: u32) -> R<Vec<GbMod>> {
    let v = get(&format!("{base}/Game/{GAME_ID}/Subfeed"), &[("_nPage", page.to_string()), ("_sSort", "updated".into())]).await?;
    Ok(records(&v))
}

/// Searches mods and sounds.
/// Name search in the given models ("Mod" / "Sound"). Returns the items and whether a model filled its page.
pub(super) async fn search_at(base: &str, q: &str, page: u32, models: &[&str]) -> R<(Vec<GbMod>, bool)> {
    let mut out: Vec<GbMod> = vec![];
    let mut last_err = None;
    let mut full = false;
    for model in models {
        let r = get(
            &format!("{base}/Util/Search/Results"),
            &[
                ("_sModelName", model.to_string()),
                ("_idGameRow", GAME_ID.to_string()),
                ("_sSearchString", q.into()),
                ("_sOrder", "best_match".into()),
                ("_nPage", page.to_string()),
                ("_nPerpage", "24".into()),
            ],
        )
        .await;
        match r {
            Ok(v) => {
                let got = records(&v);
                full |= got.len() >= 24;
                out.extend(got.into_iter().map(|mut m| {
                    if m.model.is_empty() {
                        m.model = model.to_string();
                    }
                    m
                }));
            }
            Err(er) => last_err = Some(er),
        }
    }
    if out.is_empty() {
        if let Some(er) = last_err {
            return Err(er);
        }
    }
    Ok((out, full))
}

fn digits(s: &str) -> Option<u64> {
    let d: String = s.trim().chars().take_while(|c| c.is_ascii_digit()).collect();
    d.parse().ok()
}

fn after(s: &str, pat: &str) -> Option<u64> {
    let i = s.find(pat)?;
    digits(&s[i + pat.len()..])
}

pub enum LinkKind {
    Open { id: u64, model: String },                                // show the page inside the app
    Install { file: u64, mod_id: Option<u64>, model: String }, // download + install one file (model "" = unknown)
    Invalid,
}

/// Classifies `yoshirian://https://gamebanana.com/...` links. The scheme separator can get mangled
/// on the way in (e.g. `https//`), so only the part after "gamebanana.com" is used.
pub fn classify(link: &str) -> (LinkKind, String) {
    let Some(i) = link.find("gamebanana.com") else { return (LinkKind::Invalid, String::new()) };
    let t = link[i + "gamebanana.com".len()..].trim_start_matches('/').to_string();
    let url = format!("https://gamebanana.com/{t}");
    let kind = if let Some(j) = t.find("mmdl/") {
        // GameBanana's own 1-click format: mmdl/<fileId>,Mod,<modId>
        let mut it = t[j + 5..].split(',');
        let file = it.next().and_then(digits);
        let model = it.next().unwrap_or("Mod").trim().to_string();
        let mid = it.next().and_then(digits);
        match file {
            Some(file) => LinkKind::Install { file, mod_id: mid, model },
            None => LinkKind::Invalid,
        }
    } else if let Some(rest) = t.strip_prefix("dl/") {
        match digits(rest) {
            Some(file) => LinkKind::Install { file, mod_id: None, model: String::new() },
            None => LinkKind::Invalid,
        }
    } else if let Some((model, rest)) = t.strip_prefix("mods/").map(|r| ("Mod", r)).or_else(|| t.strip_prefix("sounds/").map(|r| ("Sound", r))) {
        if let Some(r2) = rest.strip_prefix("download/") {
            match (after(&t, "FileInfo_"), digits(r2)) {
                (Some(file), m) => LinkKind::Install { file, mod_id: m, model: model.into() },
                (None, Some(m)) => LinkKind::Open { id: m, model: model.into() },
                _ => LinkKind::Invalid,
            }
        } else {
            match digits(rest) {
                Some(m) => LinkKind::Open { id: m, model: model.into() },
                None => LinkKind::Invalid,
            }
        }
    } else {
        LinkKind::Invalid
    };
    (kind, url)
}

pub async fn mod_of_file(file: u64) -> Option<u64> {
    get(&format!("{API}/File/{file}"), &[]).await.ok()?["_idModRow"].as_u64()
}

/// A link, a numeric id or a /dl/ link. `None` = the text is a search.
pub async fn lookup_link(input: &str) -> R<Option<Lookup>> {
    let t = input.trim();
    if let Some(id) = after(t, "/dl/") {
        if let Some(mid) = mod_of_file(id).await {
            if let Ok(m) = fetch_detail_any(mid).await {
                return Ok(Some(Lookup::Mod { item: m }));
            }
        }
        return Ok(Some(Lookup::Direct { url: format!("https://gamebanana.com/dl/{id}") }));
    }
    if let Some(id) = after(t, "/sounds/") {
        return Ok(Some(Lookup::Mod { item: fetch_detail("Sound", id).await? }));
    }
    if let Some(id) = after(t, "/mods/") {
        return Ok(Some(Lookup::Mod { item: fetch_detail("Mod", id).await? }));
    }
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        return Ok(Some(Lookup::Mod { item: fetch_detail_any(t.parse::<u64>().map_err(e)?).await? }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn changes(v: Value) -> Vec<(String, String)> {
        parse_update(&v).changes.into_iter().map(|c| (c.cat, c.text)).collect()
    }

    #[test]
    fn change_log_in_the_documented_shape() {
        let c = changes(json!({"_sName": "u", "_aChangeLog": [{"_sText": "All heroes cleaned up", "_sCat": "Addition"}, {"_sText": "Dust removed", "_sCat": "Improvement"}]}));
        assert_eq!(c, vec![("Addition".into(), "All heroes cleaned up".into()), ("Improvement".into(), "Dust removed".into())]);
    }

    #[test]
    fn change_log_with_other_spellings() {
        assert_eq!(changes(json!({"_aChangelog": [{"text": "B", "category": "Bugfix"}]})), vec![("Bugfix".into(), "B".into())]);
        assert_eq!(changes(json!({"_aChanges": [{"_sCategory": "Overhaul", "_sDescription": "C"}]})), vec![("Overhaul".into(), "C".into())]);
        assert_eq!(changes(json!({"_aChangeLog": ["plain line", ""]})), vec![("".into(), "plain line".into())]);
    }

    #[test]
    fn no_change_log_is_fine() {
        assert!(changes(json!({"_sName": "x", "_sText": "hello"})).is_empty());
        assert!(changes(json!({"_aChangeLog": null})).is_empty());
    }

    #[test]
    fn update_fields_are_read() {
        let u = parse_update(&json!({"_sName": "Abilities Cleanup", "_sVersion": "3.0", "_tsDateAdded": 1700, "_sText": "<p>t</p>", "_aFiles": [{"_sFile": "a.zip"}, "b.zip"]}));
        assert_eq!((u.name.as_str(), u.version.as_str(), u.date, u.files.len()), ("Abilities Cleanup", "3.0", 1700, 2));
        assert!(u.keys.contains(&"_sVersion".to_string()));
    }
}
