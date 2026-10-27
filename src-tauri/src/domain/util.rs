//! Small helpers shared by every module: error type, process spawning, file names, base64.
use std::path::Path;

pub type R<T> = Result<T, String>;

/// Turn any error into the `String` the frontend receives.
pub fn e<E: ToString>(x: E) -> String {
    x.to_string()
}

pub fn ext_of(p: &Path) -> String {
    p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Replace characters that are not allowed in Windows file names.
pub fn sanitize(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let t = t.trim().to_string();
    if t.is_empty() { "mod".into() } else { t }
}

/// Hide the console window of child processes on Windows.
pub fn hide(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

pub fn open_url(url: &str) -> R<()> {
    #[cfg(windows)]
    let mut c = {
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    };
    #[cfg(not(windows))]
    let mut c = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    hide(&mut c);
    c.spawn().map(|_| ()).map_err(e)
}

pub fn urlenc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) })
        .collect()
}

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}
