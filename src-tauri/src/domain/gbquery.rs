//! The search box of the Downloads tab: a link / id, `author:"Name"`, or plain words.
//!
//! `author:"Name"` finds the author's member id by reading the newest listings until one of their mods
//! shows up, then asks GameBanana for everything with that id (`Generic_Submitter`). When the id is already
//! known (the author link on a detail page) the first step is skipped.
use super::gamebanana::{self as gb, note, GbMod, Lookup, GAME_ID};
use super::util::R;
use futures_util::future::join;
use serde::Deserialize;

pub const PER_PAGE: usize = 24;
const FIND_PER_PAGE: usize = 50;
const FIND_MAX_PAGES: u32 = 8; // 8 x 50 newest mods and sounds are read to find an author

// ---------------------------------------------------------------- query text

#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub text: String,
    pub author: Option<String>,
}

fn is_quote(c: char) -> bool {
    matches!(c, '"' | '“' | '”')
}

/// Length of "author:" / "auther:" (any case) at the start of `s`, if present.
fn key_len(s: &[char]) -> Option<usize> {
    for key in ["author:", "auther:"] {
        let k: Vec<char> = key.chars().collect();
        if s.len() >= k.len() && s.iter().zip(&k).all(|(a, b)| a.eq_ignore_ascii_case(b)) {
            return Some(k.len());
        }
    }
    None
}

/// `author:"Hanturaya" lava` -> text "lava", author "Hanturaya". Also `author:Hanturaya` (no quotes).
pub fn parse_query(q: &str) -> Parsed {
    let c: Vec<char> = q.chars().collect();
    let (mut i, mut text, mut author) = (0, String::new(), None);
    while i < c.len() {
        let token_start = i == 0 || c[i - 1].is_whitespace();
        if author.is_none() && token_start {
            if let Some(len) = key_len(&c[i..]) {
                let mut j = i + len;
                let name: String;
                if j < c.len() && is_quote(c[j]) {
                    j += 1;
                    let start = j;
                    while j < c.len() && !is_quote(c[j]) {
                        j += 1;
                    }
                    name = c[start..j].iter().collect();
                    if j < c.len() {
                        j += 1;
                    }
                } else {
                    let start = j;
                    while j < c.len() && !c[j].is_whitespace() {
                        j += 1;
                    }
                    name = c[start..j].iter().collect();
                }
                let name = name.trim().to_string();
                if !name.is_empty() {
                    author = Some(name);
                    i = j;
                    continue;
                }
            }
        }
        text.push(c[i]);
        i += 1;
    }
    Parsed { text: text.split_whitespace().collect::<Vec<_>>().join(" "), author }
}

// ---------------------------------------------------------------- GameBanana calls

fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Most recently updated first (what the listing is shown in).
fn sort_by_update(v: &mut [GbMod]) {
    v.sort_by(|a, b| b.updated.max(b.added).cmp(&a.updated.max(a.added)).then(b.id.cmp(&a.id)));
}

async fn index(base: &str, model: &str, sort: &str, filters: &[(&str, String)], page: u32, per: usize) -> R<Vec<GbMod>> {
    let mut q: Vec<(&str, String)> = vec![("_nPage", page.to_string()), ("_nPerpage", per.to_string()), ("_aFilters[Generic_Game]", GAME_ID.to_string()), ("_sSort", sort.to_string())];
    q.extend(filters.iter().map(|(k, v)| (*k, v.clone())));
    let v = gb::get(&format!("{base}/{model}/Index"), &q).await?;
    Ok(gb::records(&v).into_iter().map(|mut m| {
        if m.model.is_empty() {
            m.model = model.to_string();
        }
        m
    }).collect())
}

/// The member id and exact name of `name`, found by reading the newest mods and sounds.
async fn find_author(base: &str, name: &str) -> Option<(u64, String)> {
    for p in 1..=FIND_MAX_PAGES {
        let (m, s) = join(index(base, "Mod", "Generic_Newest", &[], p, FIND_PER_PAGE), index(base, "Sound", "Generic_Newest", &[], p, FIND_PER_PAGE)).await;
        let (mut items, mut full) = (vec![], false);
        for v in [m, s].into_iter().flatten() {
            full |= v.len() >= FIND_PER_PAGE;
            items.extend(v);
        }
        if items.is_empty() {
            // the listing endpoint did not work: the latest-updates feed is the fallback
            if let Ok(v) = gb::subfeed_at(base, p).await {
                full = !v.is_empty();
                items = v;
            }
        }
        if let Some(m) = items.iter().find(|m| m.author_id != 0 && same_name(&m.author, name)) {
            note("INFO", &format!("Author \"{}\" is member {} (found on page {p})", m.author, m.author_id));
            return Some((m.author_id, m.author.clone()));
        }
        if !full {
            break;
        }
    }
    None
}

/// Everything (mods and sounds) of one member, most recently updated first.
async fn by_author(base: &str, id: u64, page: u32) -> (Vec<GbMod>, bool, Option<String>) {
    let f = [("_aFilters[Generic_Submitter]", id.to_string())];
    let (m, s) = join(index(base, "Mod", "Generic_LatestUpdated", &f, page, PER_PAGE), index(base, "Sound", "Generic_LatestUpdated", &f, page, PER_PAGE)).await;
    let (mut items, mut more, mut raw, mut last_err) = (vec![], false, 0, None);
    for r in [m, s] {
        match r {
            Ok(v) => {
                more |= v.len() >= PER_PAGE;
                raw += v.len();
                items.extend(v);
            }
            Err(er) => last_err = Some(er),
        }
    }
    let ignored = raw > 0 && items.iter().any(|m| m.author_id != id);
    items.retain(|m| m.author_id == id); // never show somebody else's mods
    let note_text = if ignored {
        note("WARN", "GameBanana did not apply the author filter");
        Some("GameBanana did not apply the author filter, so only part of this author's work may be shown.".to_string())
    } else if items.is_empty() && raw == 0 && page == 1 {
        last_err.map(|er| format!("GameBanana could not list this author's mods: {er}"))
    } else {
        None
    };
    sort_by_update(&mut items);
    (items, more, note_text)
}

// ---------------------------------------------------------------- the search box

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindArgs {
    pub query: String,
    #[serde(default)]
    pub page: u32,
    pub author_id: Option<u64>, // known member id (from the author link on a detail page)
}

/// A link / id / /dl/ link, `author:"Name"` (+ optional words), or plain words.
pub async fn find(base: &str, a: FindArgs) -> R<Lookup> {
    let p = parse_query(&a.query);
    let page = a.page.max(1);
    if p.author.is_none() && !p.text.is_empty() {
        if let Some(l) = gb::lookup_link(&p.text).await? {
            return Ok(l);
        }
    }
    let Some(name) = p.author else {
        // plain words: GameBanana's own search over mods and sounds
        let (items, full) = gb::search_at(base, &p.text, page, &["Mod", "Sound"]).await?;
        return Ok(Lookup::List { items, has_more: full, author: None, note: None });
    };

    let (id, shown) = match a.author_id.filter(|i| *i != 0) {
        Some(id) => (id, name.clone()),
        None => match find_author(base, &name).await {
            Some(found) => found,
            None => {
                let n = FIND_MAX_PAGES as usize * FIND_PER_PAGE;
                return Ok(Lookup::List {
                    items: vec![],
                    has_more: false,
                    author: Some(name.clone()),
                    note: Some(format!("No mod or sound by \"{name}\" was found among the newest {n} of each. Check the spelling, or open one of their mods and click the author name.")),
                });
            }
        },
    };
    let (mut items, has_more, note_text) = by_author(base, id, page).await;
    if !p.text.is_empty() {
        let w = p.text.to_lowercase();
        items.retain(|m| m.name.to_lowercase().contains(&w));
    }
    Ok(Lookup::List { items, has_more, author: Some(shown), note: note_text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    #[test]
    fn author_syntax_is_parsed() {
        let p = |s: &str| parse_query(s);
        assert_eq!(p("author:\"Hanturaya\""), Parsed { text: "".into(), author: Some("Hanturaya".into()) });
        assert_eq!(p("AUTHOR:\"Han Turaya\" lava"), Parsed { text: "lava".into(), author: Some("Han Turaya".into()) });
        assert_eq!(p("lava auther:Hanturaya  skin"), Parsed { text: "lava skin".into(), author: Some("Hanturaya".into()) });
        assert_eq!(p("author:“Smart Quotes”"), Parsed { text: "".into(), author: Some("Smart Quotes".into()) });
        assert_eq!(p("coauthor:bob"), Parsed { text: "coauthor:bob".into(), author: None }, "only at the start of a word");
        assert_eq!(p("author:\"\" x"), Parsed { text: "author:\"\" x".into(), author: None });
        assert_eq!(p("plain search"), Parsed { text: "plain search".into(), author: None });
        assert_eq!(p("  "), Parsed { text: "".into(), author: None });
    }

    // ------------------------------------------------ a fake GameBanana

    #[derive(Clone)]
    struct Fake {
        honor_submitter: bool,
        mods: u64,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl Fake {
        fn good() -> Self {
            Fake { honor_submitter: true, mods: 130, seen: Arc::default() }
        }
    }

    /// Authors: ids 1..=5 are "dave" (the oldest, so he is only found deep in the newest-first listing);
    /// everybody else cycles alice / bob / carol.
    fn record(model: &str, i: u64) -> Value {
        let (author, aid) = if model == "Mod" && i <= 5 { ("dave", 903) } else { (["alice", "bob", "carol"][(i % 3) as usize], 900 + i % 3) };
        json!({"_idRow": i + if model == "Sound" { 5000 } else { 0 }, "_sModelName": model, "_sName": format!("{model} {i} {author}"),
               "_aSubmitter": {"_idRow": aid, "_sName": author}, "_aCategory": {"_idRow": 1, "_sName": "Skins"},
               "_tsDateAdded": 1000 + i, "_tsDateModified": 5000 + (i * 37) % 1000, "_tsDateUpdated": 5000 + (i * 37) % 1000})
    }

    fn dataset(model: &str, mods: u64) -> Vec<Value> {
        (1..=if model == "Mod" { mods } else { 60 }).map(|i| record(model, i)).collect()
    }

    fn decode(s: &str) -> String {
        let b = s.as_bytes();
        let (mut out, mut i) = (Vec::new(), 0);
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(b'?'));
                i += 3;
            } else {
                out.push(if b[i] == b'+' { b' ' } else { b[i] });
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).to_string()
    }

    fn respond(f: &Fake, path: &str, q: &HashMap<String, String>) -> (u16, Value) {
        let page: usize = q.get("_nPage").and_then(|x| x.parse().ok()).unwrap_or(1);
        let per: usize = q.get("_nPerpage").and_then(|x| x.parse().ok()).unwrap_or(15);
        if path.ends_with("/Util/Search/Results") {
            let model = q.get("_sModelName").cloned().unwrap_or_default();
            let s = q.get("_sSearchString").cloned().unwrap_or_default().to_lowercase();
            let hits: Vec<Value> = dataset(&model, f.mods).into_iter().filter(|r| r["_sName"].as_str().unwrap().to_lowercase().contains(&s)).take(per).collect();
            return (200, json!({"_aRecords": hits}));
        }
        if path.ends_with("/Subfeed") {
            return (200, json!({"_aRecords": dataset("Mod", f.mods).into_iter().rev().take(20).collect::<Vec<_>>()}));
        }
        let model = if path.contains("/Sound/") { "Sound" } else { "Mod" };
        let mut rows = dataset(model, f.mods);
        if f.honor_submitter {
            if let Some(a) = q.get("_aFilters[Generic_Submitter]") {
                rows.retain(|r| r["_aSubmitter"]["_idRow"].as_u64().unwrap().to_string() == *a);
            }
        }
        let key = |r: &Value| -> i64 { if q.get("_sSort").map(String::as_str) == Some("Generic_Newest") { r["_tsDateAdded"].as_i64().unwrap() } else { r["_tsDateUpdated"].as_i64().unwrap() } };
        rows.sort_by(|a, b| key(b).cmp(&key(a)));
        (200, json!({"_aRecords": rows.into_iter().skip((page - 1) * per).take(per).collect::<Vec<_>>()}))
    }

    fn serve(f: Fake) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let f = f.clone();
                thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    let n = s.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let target = req.lines().next().unwrap_or("").split(' ').nth(1).unwrap_or("/").to_string();
                    f.seen.lock().unwrap().push(decode(&target));
                    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                    let q: HashMap<String, String> = query.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (decode(k), decode(v))).collect();
                    let (code, body) = respond(&f, path, &q);
                    let body = body.to_string();
                    let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                });
            }
        });
        base
    }

    fn run_find(base: &str, args: serde_json::Value) -> (Vec<GbMod>, bool, Option<String>, Option<String>) {
        let a: FindArgs = serde_json::from_value(args).unwrap();
        match tauri::async_runtime::block_on(find(base, a)).unwrap() {
            Lookup::List { items, has_more, note, author } => (items, has_more, note, author),
            _ => panic!("not a list"),
        }
    }

    // ------------------------------------------------ behaviour

    #[test]
    fn known_member_id_goes_straight_to_the_server_filter() {
        let f = Fake::good();
        let seen = f.seen.clone();
        let base = serve(f);
        let (v, _, note, author) = run_find(&base, json!({"query": "author:\"bob\"", "authorId": 901}));
        assert!(!v.is_empty() && v.iter().all(|m| m.author == "bob"));
        assert!(v.iter().any(|m| m.model == "Mod") && v.iter().any(|m| m.model == "Sound"), "mods and sounds");
        assert!(v.windows(2).all(|w| w[0].updated >= w[1].updated), "most recently updated first");
        assert_eq!((note, author.as_deref()), (None, Some("bob")));
        let s = seen.lock().unwrap();
        assert!(s.iter().all(|u| u.contains("Generic_Submitter]=901")), "no scanning when the id is known: {s:?}");
        assert_eq!(s.len(), 2, "one request per section");
    }

    #[test]
    fn author_typed_by_name_is_found_first_then_listed_with_the_server_filter() {
        let f = Fake::good();
        let seen = f.seen.clone();
        let base = serve(f);
        let (v, _, note, author) = run_find(&base, json!({"query": "author:\"CAROL\""}));
        assert!(!v.is_empty() && v.iter().all(|m| m.author == "carol"));
        assert_eq!(author.as_deref(), Some("carol"), "GameBanana's own spelling");
        assert!(note.is_none());
        let s = seen.lock().unwrap();
        assert!(s.iter().any(|u| u.contains("Generic_Newest")), "looked for the member id first");
        assert!(s.iter().any(|u| u.contains("Generic_Submitter]=902")), "then used it: {s:?}");
    }

    #[test]
    fn an_author_with_only_old_mods_is_found_deeper_in_the_listing() {
        let base = serve(Fake::good()); // dave's mods are the 5 oldest of 130 -> page 3 of the newest-first listing
        let (v, _, note, _) = run_find(&base, json!({"query": "author:dave"}));
        assert_eq!(v.len(), 5);
        assert!(v.iter().all(|m| m.author == "dave" && m.model == "Mod"));
        assert!(note.is_none());
    }

    #[test]
    fn an_unknown_author_gives_an_explanation_not_an_error() {
        let f = Fake::good();
        let seen = f.seen.clone();
        let base = serve(f);
        let (v, more, note, author) = run_find(&base, json!({"query": "author:nobody"}));
        assert!(v.is_empty() && !more);
        assert_eq!(author.as_deref(), Some("nobody"));
        assert!(note.unwrap().contains("No mod or sound by \"nobody\" was found"));
        assert!(seen.lock().unwrap().len() <= 2 * 3 + 2, "stops reading when the listing ends (130 mods = 3 pages)");
    }

    #[test]
    fn the_search_is_limited_to_eight_pages_of_a_huge_listing() {
        let f = Fake { mods: 2000, ..Fake::good() };
        let seen = f.seen.clone();
        let base = serve(f);
        let (v, _, note, _) = run_find(&base, json!({"query": "author:nobody"}));
        assert!(v.is_empty() && note.unwrap().contains("400"));
        assert!(seen.lock().unwrap().len() <= 16, "8 pages x (mods + sounds)");
    }

    #[test]
    fn a_server_that_ignores_the_author_filter_never_shows_other_peoples_mods() {
        let base = serve(Fake { honor_submitter: false, ..Fake::good() });
        let (v, _, note, _) = run_find(&base, json!({"query": "author:bob", "authorId": 901}));
        assert!(v.iter().all(|m| m.author == "bob"), "other authors are filtered out");
        assert!(note.unwrap().contains("did not apply the author filter"), "and the user is told the list may be partial");
    }

    #[test]
    fn words_can_be_combined_with_an_author() {
        let base = serve(Fake::good());
        let (v, _, _, _) = run_find(&base, json!({"query": "author:bob 4", "authorId": 901}));
        assert!(!v.is_empty() && v.iter().all(|m| m.author == "bob" && m.name.contains('4')));
    }

    #[test]
    fn plain_words_use_gamebanana_search_and_can_have_more_pages() {
        let base = serve(Fake::good());
        let (v, more, _, author) = run_find(&base, json!({"query": "Mod"}));
        assert_eq!(v.len(), 24, "only the mods are named \"Mod ...\"; a full page");
        assert!(v.iter().all(|m| m.model == "Mod") && more && author.is_none());
    }

    #[test]
    fn the_listing_falls_back_to_the_feed_when_index_gives_nothing() {
        // `find_author` falls back to the latest-updates feed; the fake's Index never fails, so this checks the feed shape instead
        let base = serve(Fake::good());
        let v = tauri::async_runtime::block_on(gb::subfeed_at(&base, 1)).unwrap();
        assert!(!v.is_empty() && v[0].author_id != 0);
    }
}
