//! Pausable / resumable / cancellable file download.
//!
//! `fetch_step` performs ONE http request and can be interrupted at any moment through a [`Ctl`]:
//! pausing keeps the part of the file that is already on disk; the next `fetch_step` continues with an
//! HTTP `Range` request (when the server ignores `Range` the download simply starts again from zero).
use super::{
    archive,
    util::{e, R},
};
use futures_util::{future::Either, StreamExt};
use std::{
    fs,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};
use tokio::sync::Notify;

/// Remote control of one download, shared between the download task and the UI commands.
#[derive(Default)]
pub struct Ctl {
    paused: AtomicBool,
    cancelled: AtomicBool,
    wake: Notify,
}

impl Ctl {
    pub fn pause(&self) {
        self.paused.store(true, SeqCst);
        self.wake.notify_one();
    }
    pub fn resume(&self) {
        self.paused.store(false, SeqCst);
        self.wake.notify_one();
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, SeqCst);
        self.paused.store(false, SeqCst);
        self.wake.notify_one();
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(SeqCst)
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(SeqCst)
    }
    /// Sleeps while paused; returns when resumed or cancelled.
    pub async fn wait_resumed(&self) {
        while self.is_paused() && !self.is_cancelled() {
            self.wake.notified().await;
        }
    }
}

/// Runs `fut`, but gives up (returns `None`) as soon as the download is paused or cancelled.
pub async fn interruptible<T>(ctl: &Ctl, fut: impl Future<Output = T>) -> Option<T> {
    let mut fut = Box::pin(fut);
    loop {
        if ctl.is_paused() || ctl.is_cancelled() {
            return None;
        }
        match futures_util::future::select(fut, Box::pin(ctl.wake.notified())).await {
            Either::Left((v, _)) => return Some(v),
            Either::Right((_, unfinished)) => fut = unfinished, // woken: re-check the flags
        }
    }
}

/// What is already on disk. Lives as long as the download job, across pauses.
#[derive(Default)]
pub struct Partial {
    pub path: Option<PathBuf>,
    pub got: u64,
    pub total: u64, // 0 = the server did not say
}

pub enum Step {
    Finished(PathBuf),
    Paused,
    Cancelled,
}

fn interrupted(ctl: &Ctl) -> Step {
    if ctl.is_cancelled() { Step::Cancelled } else { Step::Paused }
}

/// The real file name: Content-Disposition first, then the last part of the final URL.
fn file_name(resp: &reqwest::Response) -> String {
    let cd = resp
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split("filename=").nth(1))
        .map(|v| v.trim_matches(|c: char| c == '"' || c == ';' || c == ' ').to_string());
    let raw = cd.filter(|s| !s.is_empty()).unwrap_or_else(|| resp.url().path().rsplit('/').next().unwrap_or("mod").to_string());
    let safe: String = raw.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    if safe.is_empty() { "mod".into() } else { safe }
}

/// One request. `progress(received, total)` is called about every 64 KB.
pub async fn fetch_step(
    client: &reqwest::Client,
    url: &str,
    dir: &Path,
    id: &str,
    part: &mut Partial,
    ctl: &Ctl,
    mut progress: impl FnMut(u64, u64),
) -> R<Step> {
    fs::create_dir_all(dir).map_err(e)?;
    for _attempt in 0..2 {
        if ctl.is_cancelled() || ctl.is_paused() {
            return Ok(interrupted(ctl));
        }
        let resuming = part.got > 0 && part.path.as_ref().map(|p| p.exists()).unwrap_or(false);
        let mut req = client.get(url);
        if resuming {
            req = req.header("Range", format!("bytes={}-", part.got));
        }
        let Some(resp) = interruptible(ctl, req.send()).await else { return Ok(interrupted(ctl)) };
        let resp = resp.map_err(e)?;
        let status = resp.status().as_u16();
        if resuming && status == 416 {
            // the server does not like our range: forget the partial file and start over
            if let Some(p) = &part.path {
                let _ = fs::remove_file(p);
            }
            *part = Partial::default();
            continue;
        }
        let resp = resp.error_for_status().map_err(e)?;
        let append = resuming && status == 206;
        let len = resp.content_length().unwrap_or(0);
        let mut f = if append {
            part.total = if len > 0 { part.got + len } else { part.total };
            fs::OpenOptions::new().append(true).open(part.path.as_ref().unwrap()).map_err(e)?
        } else {
            if let Some(old) = &part.path {
                let _ = fs::remove_file(old);
            }
            let path = dir.join(format!("{}_{}", id, file_name(&resp)));
            part.got = 0;
            part.total = len;
            part.path = Some(path.clone());
            fs::File::create(&path).map_err(e)?
        };
        let mut last = part.got;
        let mut stream = resp.bytes_stream();
        loop {
            match interruptible(ctl, stream.next()).await {
                None => return Ok(interrupted(ctl)),
                Some(None) => break,
                Some(Some(chunk)) => {
                    let c = chunk.map_err(e)?;
                    f.write_all(&c).map_err(e)?;
                    part.got += c.len() as u64;
                    if part.got - last >= 64 * 1024 {
                        last = part.got;
                        progress(part.got, part.total);
                    }
                }
            }
        }
        drop(f);
        progress(part.got, part.total.max(part.got));
        return Ok(Step::Finished(part.path.clone().unwrap()));
    }
    Err("The server refused to resume the download".into())
}

/// The real type comes from the file's first bytes; a wrong or missing extension gets fixed here.
pub fn finalize(path: &Path) -> R<PathBuf> {
    let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    let want = archive::ext_for(archive::sniff(path));
    if ext == want {
        return Ok(path.to_path_buf());
    }
    let np = PathBuf::from(format!("{}.{}", path.display(), want));
    fs::rename(path, &np).map_err(e)?;
    Ok(np)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader},
        net::{TcpListener, TcpStream},
        sync::{atomic::AtomicUsize, Arc},
        thread,
        time::Duration,
    };

    /// A tiny http server: serves `body` in 16 KB pieces, optionally honouring `Range`.
    fn serve(body: Arc<Vec<u8>>, honor_range: bool) -> (String, Arc<AtomicUsize>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file", l.local_addr().unwrap());
        let ranges = Arc::new(AtomicUsize::new(0));
        let rc = ranges.clone();
        thread::spawn(move || {
            for s in l.incoming().flatten() {
                let (b, rc) = (body.clone(), rc.clone());
                thread::spawn(move || handle(s, b, honor_range, rc));
            }
        });
        (url, ranges)
    }

    fn handle(mut s: TcpStream, body: Arc<Vec<u8>>, honor: bool, ranges: Arc<AtomicUsize>) {
        let mut start = 0usize;
        let mut rd = BufReader::new(s.try_clone().unwrap());
        loop {
            let mut line = String::new();
            if rd.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
            if let Some(v) = line.to_lowercase().strip_prefix("range: bytes=") {
                ranges.fetch_add(1, SeqCst);
                if honor {
                    start = v.trim().trim_end_matches('-').parse().unwrap_or(0);
                }
            }
        }
        let len = body.len();
        let head = if start > 0 {
            format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\n", start, len - 1, len, len - start)
        } else {
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n", len)
        };
        let head = format!("{head}Content-Disposition: attachment; filename=\"pack.zip\"\r\nConnection: close\r\n\r\n");
        if s.write_all(head.as_bytes()).is_err() {
            return;
        }
        for piece in body[start..].chunks(16 * 1024) {
            if s.write_all(piece).is_err() {
                return;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn sample() -> Arc<Vec<u8>> {
        Arc::new((0..1_000_000u32).map(|i| (i % 251) as u8).collect())
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("yoshirian-dl-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn run<F: Future>(f: F) -> F::Output {
        tauri::async_runtime::block_on(f)
    }

    #[test]
    fn plain_download_is_complete() {
        let body = sample();
        let (url, _) = serve(body.clone(), true);
        let (dir, ctl, mut part) = (tmp(), Ctl::default(), Partial::default());
        let step = run(fetch_step(&reqwest::Client::new(), &url, &dir, "t1", &mut part, &ctl, |_, _| {})).unwrap();
        let Step::Finished(p) = step else { panic!("not finished") };
        assert_eq!(fs::read(p).unwrap(), *body);
        assert_eq!(part.total, body.len() as u64);
    }

    #[test]
    fn pause_then_resume_uses_a_range_request() {
        let body = sample();
        let (url, ranges) = serve(body.clone(), true);
        let (dir, ctl, mut part) = (tmp(), Ctl::default(), Partial::default());
        let c = reqwest::Client::new();
        let first = run(fetch_step(&c, &url, &dir, "t2", &mut part, &ctl, |got, _| if got >= 256 * 1024 { ctl.pause() })).unwrap();
        assert!(matches!(first, Step::Paused));
        assert!(part.got >= 256 * 1024 && part.got < body.len() as u64, "paused in the middle, got {}", part.got);
        ctl.resume();
        let second = run(fetch_step(&c, &url, &dir, "t2", &mut part, &ctl, |_, _| {})).unwrap();
        let Step::Finished(p) = second else { panic!("not finished") };
        assert_eq!(fs::read(p).unwrap(), *body, "resumed file must be identical");
        assert_eq!(ranges.load(SeqCst), 1);
    }

    #[test]
    fn server_without_range_support_restarts_cleanly() {
        let body = sample();
        let (url, _) = serve(body.clone(), false);
        let (dir, ctl, mut part) = (tmp(), Ctl::default(), Partial::default());
        let c = reqwest::Client::new();
        let first = run(fetch_step(&c, &url, &dir, "t3", &mut part, &ctl, |got, _| if got >= 256 * 1024 { ctl.pause() })).unwrap();
        assert!(matches!(first, Step::Paused));
        ctl.resume();
        let Step::Finished(p) = run(fetch_step(&c, &url, &dir, "t3", &mut part, &ctl, |_, _| {})).unwrap() else { panic!("not finished") };
        assert_eq!(fs::read(p).unwrap(), *body, "no duplicated or missing bytes");
    }

    #[test]
    fn cancel_stops_the_transfer() {
        let body = sample();
        let (url, _) = serve(body, true);
        let (dir, ctl, mut part) = (tmp(), Ctl::default(), Partial::default());
        let step = run(fetch_step(&reqwest::Client::new(), &url, &dir, "t4", &mut part, &ctl, |got, _| if got >= 128 * 1024 { ctl.cancel() })).unwrap();
        assert!(matches!(step, Step::Cancelled));
        assert!(part.got < 1_000_000);
    }

    #[test]
    fn wait_resumed_returns_after_resume_or_cancel() {
        let ctl = Arc::new(Ctl::default());
        ctl.pause();
        let c2 = ctl.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            c2.resume();
        });
        run(ctl.wait_resumed());
        assert!(!ctl.is_paused());
        ctl.pause();
        let c3 = ctl.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            c3.cancel();
        });
        run(ctl.wait_resumed());
        assert!(ctl.is_cancelled());
    }

    #[test]
    fn finalize_fixes_the_extension() {
        let d = tmp();
        let p = d.join("x_download");
        fs::write(&p, b"PK\x03\x04rest").unwrap();
        assert_eq!(finalize(&p).unwrap().extension().unwrap(), "zip");
    }
}
