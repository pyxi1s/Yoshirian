//! Updating the app itself from GitHub Releases: which release counts as an update, version comparison,
//! choosing the installer and checking its checksum. No Tauri types in here, so all of it is unit tested.
//!
//! What the app offers (rules for the person who publishes releases):
//! * a normal published release that is newer than the installed version  -> offered ("Release")
//! * a draft                                                              -> never offered
//! * a pre-release                                                        -> not offered ...
//! * ... unless it is a hotfix: the tag or title contains "hotfix", or the notes contain `[critical]` / `[hotfix]`.
//!   A hotfix is offered even when it is marked as a pre-release, and the popup shows a "Critical fix" badge.
use super::util::{e, R};
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

/// The repository the app updates from unless the user types another one in Settings → Updates.
pub const DEFAULT_REPO: &str = "pyxi1s/Yoshirian";
pub const API: &str = "https://api.github.com";

/// "owner/repo", "https://github.com/owner/repo", "github.com/owner/repo.git" ... -> "owner/repo".
/// `None` when the text is not a repository.
pub fn normalize_repo(s: &str) -> Option<String> {
    let mut t = s.trim().trim_end_matches('/');
    let had_scheme = t.starts_with("http://") || t.starts_with("https://");
    for p in ["https://", "http://"] {
        t = t.strip_prefix(p).unwrap_or(t);
    }
    t = t.strip_prefix("www.").unwrap_or(t);
    match t.strip_prefix("github.com/") {
        Some(rest) => t = rest,
        None if had_scheme => return None, // some other website
        None => {}
    }
    let t = t.trim_end_matches(".git").trim_end_matches('/');
    let mut it = t.split('/');
    let (owner, repo) = (it.next()?, it.next()?);
    // GitHub user names have no dots; repository names may. Anything after owner/repo (/releases/tag/v1 ...) is ignored.
    let name_ok = |p: &str, dots: bool| !p.is_empty() && p != "." && p != ".." && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || (dots && c == '.'));
    (name_ok(owner, false) && name_ok(repo, true)).then(|| format!("{owner}/{repo}"))
}

/// "v0.2.0", "0.2.0-alpha.1", "V1.2" -> Version. Two-part versions get ".0".
pub fn parse_version(tag: &str) -> Option<Version> {
    let t = tag.trim().trim_start_matches(['v', 'V']);
    if let Ok(v) = Version::parse(t) {
        return Some(v);
    }
    let core_len = t.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(t.len());
    let (core, rest) = t.split_at(core_len);
    let core = core.trim_end_matches('.');
    let parts = core.split('.').count();
    if core.is_empty() || parts > 3 {
        return None;
    }
    let padded = format!("{core}{}{rest}", ".0".repeat(3 - parts));
    Version::parse(&padded).ok()
}

#[derive(Clone)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
    pub digest: Option<String>, // "sha256:<hex>" when GitHub provides it
}

#[derive(Serialize, Clone)]
pub struct Release {
    pub tag: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub prerelease: bool,
    pub critical: bool,
    pub url: String,
    #[serde(skip)]
    pub assets: Vec<Asset>,
}

/// Hotfix marker (see the module docs). Deliberately explicit so a sentence like "not critical" does not match.
pub fn is_critical(tag: &str, name: &str, body: &str) -> bool {
    let head = format!("{tag} {name}").to_lowercase();
    let b = body.to_lowercase();
    ["hotfix", "hot-fix", "hot fix"].iter().any(|k| head.contains(k)) || b.contains("[critical]") || b.contains("[hotfix]")
}

/// Releases from the GitHub API (drafts and tags that are not versions are dropped).
pub fn parse_releases(v: &Value) -> Vec<(Version, Release)> {
    let Some(list) = v.as_array() else { return vec![] };
    let s = |x: &Value, k: &str| x[k].as_str().unwrap_or("").to_string();
    list.iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let (tag, name, body) = (s(r, "tag_name"), s(r, "name"), s(r, "body"));
            let ver = parse_version(&tag)?;
            let assets = r["assets"]
                .as_array()
                .map(|a| a.iter().map(|x| Asset { name: s(x, "name"), size: x["size"].as_u64().unwrap_or(0), url: s(x, "browser_download_url"), digest: x["digest"].as_str().map(String::from) }).collect())
                .unwrap_or_default();
            let rel = Release { critical: is_critical(&tag, &name, &body), version: ver.to_string(), prerelease: r["prerelease"].as_bool().unwrap_or(false), url: s(r, "html_url"), name, body, tag, assets };
            Some((ver, rel))
        })
        .collect()
}

/// The newest offered release that is newer than `current` (None = nothing to offer).
pub fn pick_update(releases: Vec<(Version, Release)>, current: &Version) -> Option<Release> {
    releases.into_iter().filter(|(v, r)| v > current && (!r.prerelease || r.critical)).max_by(|a, b| a.0.cmp(&b.0)).map(|(_, r)| r)
}

/// The Windows installer among the assets (Tauri names it `<name>_<version>_x64-setup.exe`).
pub fn installer_asset(r: &Release) -> Option<&Asset> {
    let exe = |a: &&Asset| a.name.to_lowercase().ends_with(".exe");
    r.assets.iter().find(|a| a.name.to_lowercase().ends_with("-setup.exe")).or_else(|| r.assets.iter().find(exe))
}

/// Only files from this repository's release downloads over https are ever downloaded and run.
pub fn download_allowed(url: &str, repo: &str) -> bool {
    url.starts_with(&format!("https://github.com/{repo}/releases/download/"))
}

/// Check `path` against GitHub's `sha256:<hex>` digest. No digest = nothing to check (returns Ok).
pub fn verify_digest(path: &Path, digest: &Option<String>) -> R<()> {
    let Some(want) = digest.as_ref().and_then(|d| d.strip_prefix("sha256:")) else { return Ok(()) };
    let mut f = fs::File::open(path).map_err(e)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(e)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let got: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got.eq_ignore_ascii_case(want) { Ok(()) } else { Err("The downloaded installer does not match the checksum published on GitHub. It was not run.".into()) }
}

/// What the UI is told after a check.
#[derive(Serialize)]
pub struct UpdateStatus {
    pub current: String,
    pub repo: String,
    pub latest: Option<Release>, // newest offered release, even when the user skipped it (Settings can still install it)
    pub skipped: bool,           // the user chose "Skip" for exactly this release
    pub installer: Option<String>, // file name of the installer, None = only a download page can be offered
}

/// `skipped_tag` is the release the user skipped. A newer release has another tag, so it is offered again.
pub fn make_status(current: &Version, repo: &str, latest: Option<Release>, skipped_tag: &str) -> UpdateStatus {
    let skipped = latest.as_ref().map(|r| !skipped_tag.is_empty() && r.tag == skipped_tag).unwrap_or(false);
    let installer = latest.as_ref().and_then(|r| installer_asset(r)).map(|a| a.name.clone());
    UpdateStatus { current: current.to_string(), repo: repo.to_string(), latest, skipped, installer }
}

/// Fetch the releases of `repo` from the GitHub API at `base`. Friendly messages for the usual failures.
pub async fn fetch_releases(base: &str, repo: &str) -> R<Value> {
    let c = reqwest::Client::builder().user_agent("yoshirian").timeout(std::time::Duration::from_secs(20)).build().map_err(e)?;
    let resp = c
        .get(format!("{base}/repos/{repo}/releases?per_page=20"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach GitHub. Check your internet connection.".to_string())?;
    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|_| "GitHub sent an answer the app could not read.".to_string()),
        404 => Err(format!("The repository \"{repo}\" was not found on GitHub.")),
        403 | 429 => Err("GitHub is limiting requests from this connection right now. Try again later.".into()),
        s => Err(format!("GitHub answered with an error ({s}).")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{io::Write, net::TcpListener, thread};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn rel(tag: &str, pre: bool, name: &str, body: &str) -> Value {
        json!({"tag_name": tag, "name": name, "body": body, "prerelease": pre, "draft": false, "html_url": format!("https://github.com/o/r/releases/tag/{tag}"),
               "assets": [{"name": "notes.txt", "size": 1, "browser_download_url": "x"}, {"name": format!("Yoshirian_{}_x64-setup.exe", tag.trim_start_matches('v')), "size": 9, "browser_download_url": format!("https://github.com/o/r/releases/download/{tag}/a-setup.exe"), "digest": "sha256:abc"}]})
    }

    #[test]
    fn repository_text_is_normalized() {
        for ok in ["pyxi1s/Yoshirian", " https://github.com/pyxi1s/Yoshirian ", "github.com/pyxi1s/Yoshirian.git", "https://www.github.com/pyxi1s/Yoshirian/", "https://github.com/pyxi1s/Yoshirian/releases/tag/v1"] {
            assert_eq!(normalize_repo(ok).as_deref(), Some("pyxi1s/Yoshirian"), "{ok}");
        }
        for bad in ["", "   ", "pyxi1s", "a b/c", "../x", "x/..", "a/b?c", "owner/repo?x=1", "https://example.com/a/b", "example.com/a"] {
            assert_eq!(normalize_repo(bad), None, "{bad:?} must be rejected");
        }
    }

    #[test]
    fn versions_are_read_from_tags() {
        assert_eq!(parse_version("v0.2.0"), Some(v("0.2.0")));
        assert_eq!(parse_version("0.1.1-alpha"), Some(v("0.1.1-alpha")));
        assert_eq!(parse_version("V1.2"), Some(v("1.2.0")));
        assert_eq!(parse_version("v3"), Some(v("3.0.0")));
        assert_eq!(parse_version("v1.2-hotfix"), Some(v("1.2.0-hotfix")));
        assert_eq!(parse_version("nightly"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn alpha_is_older_than_its_final_release() {
        assert!(v("0.1.1-alpha") < v("0.1.1"));
        let r = parse_releases(&json!([rel("v0.1.1", false, "v0.1.1", "")]));
        assert!(pick_update(r, &v("0.1.1-alpha")).is_some());
    }

    #[test]
    fn hotfix_marker_is_explicit() {
        assert!(is_critical("v1.0.1", "Hotfix: crash on start", ""));
        assert!(is_critical("v1.0.1-hotfix", "", ""));
        assert!(is_critical("v1.0.1", "", "Fixes data loss.\n[critical]"));
        assert!(!is_critical("v1.0.1", "Small changes", "This is not a critical change for most people"));
    }

    #[test]
    fn only_releases_and_hotfixes_are_offered() {
        let cur = v("0.1.0");
        let list = json!([
            rel("v0.4.0", false, "", "").as_object().map(|o| { let mut o = o.clone(); o.insert("draft".into(), json!(true)); Value::Object(o) }).unwrap(),
            rel("v0.3.0", true, "beta", "testing"),
            rel("v0.2.0", false, "v0.2.0", ""),
            rel("v0.1.0", false, "same", ""),
            rel("v0.0.9", false, "old", ""),
            rel("junk-tag", false, "x", ""),
        ]);
        let got = pick_update(parse_releases(&list), &cur).unwrap();
        assert_eq!(got.tag, "v0.2.0", "draft and pre-release are ignored");
        assert!(!got.critical);

        let with_hotfix = json!([rel("v0.2.0", false, "", ""), rel("v0.2.1", true, "v0.2.1 hotfix", "fixes a crash")]);
        let got = pick_update(parse_releases(&with_hotfix), &cur).unwrap();
        assert_eq!((got.tag.as_str(), got.critical), ("v0.2.1", true), "a hotfix wins even when marked pre-release");

        assert!(pick_update(parse_releases(&json!([])), &cur).is_none(), "a repository without releases");
        assert!(pick_update(parse_releases(&json!({"message": "x"})), &cur).is_none());
    }

    #[test]
    fn installer_is_found_and_downloads_are_restricted() {
        let r = parse_releases(&json!([rel("v0.2.0", false, "", "")])).remove(0).1;
        let a = installer_asset(&r).unwrap();
        assert_eq!(a.name, "Yoshirian_0.2.0_x64-setup.exe");
        assert!(download_allowed(&a.url, "o/r"));
        assert!(!download_allowed("https://evil.example/o/r/releases/download/v1/a.exe", "o/r"));
        assert!(!download_allowed("http://github.com/o/r/releases/download/v1/a.exe", "o/r"));
        assert!(!download_allowed("https://github.com/other/r/releases/download/v1/a.exe", "o/r"));
    }

    #[test]
    fn skip_only_hides_that_release() {
        let cur = v("0.1.0");
        let latest = |tag: &str| pick_update(parse_releases(&json!([rel(tag, false, "", "")])), &cur);
        let s = make_status(&cur, "o/r", latest("v0.2.0"), "v0.2.0");
        assert!(s.latest.is_some() && s.skipped, "skipped release is still known, but flagged");
        let s = make_status(&cur, "o/r", latest("v0.2.0"), "");
        assert!(!s.skipped);
        let s = make_status(&cur, "o/r", latest("v0.3.0"), "v0.2.0");
        assert!(!s.skipped, "a newer release than the skipped one is offered again");
        assert_eq!(s.installer.as_deref(), Some("Yoshirian_0.3.0_x64-setup.exe"));
        let s = make_status(&cur, "o/r", None, "v0.2.0");
        assert!(s.latest.is_none() && !s.skipped);
    }

    #[test]
    fn checksum_is_verified() {
        let p = std::env::temp_dir().join(format!("yoshirian-sha-{}", uuid::Uuid::new_v4()));
        fs::write(&p, b"abc").unwrap();
        let good = Some("sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_string());
        assert!(verify_digest(&p, &good).is_ok());
        assert!(verify_digest(&p, &Some("sha256:00".into())).is_err());
        assert!(verify_digest(&p, &None).is_ok(), "no digest published = nothing to compare");
    }

    /// One-shot http server answering every request with `status` and `body`.
    fn serve(status: &str, body: &str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let (status, body) = (status.to_string(), body.to_string());
        thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let _ = std::io::Read::read(&mut s, &mut buf);
                let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        base
    }

    #[test]
    fn http_failures_get_friendly_messages() {
        let get = |status: &str| {
            let base = serve(status, if status.starts_with("200") { "[]" } else { "{}" });
            tauri::async_runtime::block_on(fetch_releases(&base, "o/r"))
        };
        assert!(get("200 OK").unwrap().as_array().unwrap().is_empty());
        assert!(get("404 Not Found").unwrap_err().contains("was not found"));
        assert!(get("403 Forbidden").unwrap_err().contains("limiting requests"));
        assert!(get("500 Oops").unwrap_err().contains("500"));
        let down = tauri::async_runtime::block_on(fetch_releases("http://127.0.0.1:1", "o/r"));
        assert!(down.unwrap_err().contains("Could not reach GitHub"));
    }
}
