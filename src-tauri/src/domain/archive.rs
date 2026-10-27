//! Archive support. The kind is detected from the file's first bytes (not only the extension), so a
//! download or a drop with a wrong / missing extension still works.
//!
//! * built in (no external program): zip, 7z (LZMA / LZMA2 / Deflate / Copy), tar, tar.gz / tgz, single .gz
//! * everything else (rar, bz2, xz, zst, cab, 7z with other codecs ...) goes through `tar` = libarchive,
//!   which ships with Windows 10+ (and with macOS). If it is missing the error says so.
use super::util::{e, ext_of, hide, R};
use std::{
    fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Vpk,
    Zip,
    SevenZ,
    Rar,
    Tar,
    Gz,
    Bz2,
    Xz,
    Zst,
    Cab,
    Unknown,
}

/// File extensions the app accepts as archives (shown in file pickers and in the drop hint).
pub const EXTS: &[&str] = &["zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "tbz2", "xz", "txz", "zst", "tzst", "cab"];

/// Look at the first bytes (and the extension when the bytes say nothing).
pub fn sniff(p: &Path) -> Kind {
    let mut head = [0u8; 512];
    let n = fs::File::open(p).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    let h = &head[..n];
    if h.starts_with(&[0x34, 0x12, 0xAA, 0x55]) {
        return Kind::Vpk;
    }
    if h.starts_with(b"PK\x03\x04") || h.starts_with(b"PK\x05\x06") {
        return Kind::Zip;
    }
    if h.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        return Kind::SevenZ;
    }
    if h.starts_with(b"Rar!\x1A\x07") {
        return Kind::Rar;
    }
    if h.starts_with(&[0x1F, 0x8B]) {
        return Kind::Gz;
    }
    if h.starts_with(b"BZh") {
        return Kind::Bz2;
    }
    if h.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        return Kind::Xz;
    }
    if h.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return Kind::Zst;
    }
    if h.starts_with(b"MSCF") {
        return Kind::Cab;
    }
    if n >= 262 && &h[257..262] == b"ustar" {
        return Kind::Tar;
    }
    match ext_of(p).as_str() {
        "vpk" => Kind::Vpk,
        "zip" => Kind::Zip,
        "7z" => Kind::SevenZ,
        "rar" => Kind::Rar,
        "tar" => Kind::Tar,
        "gz" | "tgz" => Kind::Gz,
        "bz2" | "tbz2" => Kind::Bz2,
        "xz" | "txz" => Kind::Xz,
        "zst" | "tzst" => Kind::Zst,
        "cab" => Kind::Cab,
        _ => Kind::Unknown,
    }
}

/// The extension to give a downloaded file of this kind.
pub fn ext_for(k: Kind) -> &'static str {
    match k {
        Kind::Zip => "zip",
        Kind::SevenZ => "7z",
        Kind::Rar => "rar",
        Kind::Tar => "tar",
        Kind::Gz => "gz",
        Kind::Bz2 => "bz2",
        Kind::Xz => "xz",
        Kind::Zst => "zst",
        Kind::Cab => "cab",
        Kind::Vpk | Kind::Unknown => "vpk",
    }
}

pub fn is_archive(p: &Path) -> bool {
    !matches!(sniff(p), Kind::Vpk | Kind::Unknown)
}

/// Extract `archive` into `dest`. Never writes outside `dest`.
pub fn extract(archive: &Path, dest: &Path) -> R<()> {
    fs::create_dir_all(dest).map_err(e)?;
    let kind = sniff(archive);
    let native = match kind {
        Kind::Zip => extract_zip(archive, dest),
        Kind::SevenZ => extract_7z(archive, dest),
        Kind::Tar => extract_tar(fs::File::open(archive).map_err(e)?, dest),
        Kind::Gz => extract_gz(archive, dest),
        _ => Err("no built-in reader".to_string()),
    };
    match native {
        Ok(()) => Ok(()),
        Err(native_err) => system_tar(archive, dest).map_err(|sys_err| {
            let name = archive.file_name().unwrap_or_default().to_string_lossy().to_string();
            if matches!(kind, Kind::Unknown) {
                format!("{name}: unsupported file type")
            } else if native_err == "no built-in reader" {
                format!("Could not extract {name} ({sys_err}). Try a .zip or .7z, or extract it manually.")
            } else {
                format!("Could not extract {name} ({native_err}; fallback: {sys_err})")
            }
        }),
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> R<()> {
    zip::ZipArchive::new(fs::File::open(archive).map_err(e)?).map_err(e)?.extract(dest).map_err(e)
}

/// Keep only the normal components of an entry name: no `..`, no drive letters, no leading slash.
fn safe_join(dest: &Path, name: &str) -> Option<PathBuf> {
    let mut out = dest.to_path_buf();
    let mut any = false;
    for c in Path::new(&name.replace('\\', "/")).components() {
        match c {
            Component::Normal(s) => {
                out.push(s);
                any = true;
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    any.then_some(out)
}

fn extract_7z(archive: &Path, dest: &Path) -> R<()> {
    let f = fs::File::open(archive).map_err(e)?;
    let mut r = sevenz_rust2::ArchiveReader::new(f, sevenz_rust2::Password::empty()).map_err(e)?;
    r.for_each_entries(|entry, data| {
        let Some(p) = safe_join(dest, entry.name()) else { return Ok(true) };
        if entry.is_directory() {
            fs::create_dir_all(&p)?;
        } else {
            if let Some(par) = p.parent() {
                fs::create_dir_all(par)?;
            }
            io::copy(data, &mut fs::File::create(&p)?)?;
        }
        Ok(true)
    })
    .map_err(e)
}

fn extract_tar<T: Read>(src: T, dest: &Path) -> R<()> {
    tar::Archive::new(src).unpack(dest).map_err(e) // the tar crate refuses paths that escape `dest`
}

/// .gz holds either a tar (tar.gz / tgz) or one single file.
fn extract_gz(archive: &Path, dest: &Path) -> R<()> {
    let mut dec = flate2::read::GzDecoder::new(io::BufReader::new(fs::File::open(archive).map_err(e)?));
    let mut head = Vec::with_capacity(512);
    (&mut dec).take(512).read_to_end(&mut head).map_err(e)?;
    let is_tar = head.len() >= 262 && &head[257..262] == b"ustar";
    let whole = io::Cursor::new(head).chain(dec);
    if is_tar {
        return extract_tar(whole, dest);
    }
    let stem = archive.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
    let mut whole = whole;
    io::copy(&mut whole, &mut fs::File::create(dest.join(stem)).map_err(e)?).map_err(e)?;
    Ok(())
}

/// libarchive's `tar`: rar, bz2, xz, zst, cab, other 7z codecs ...
fn system_tar(archive: &Path, dest: &Path) -> R<()> {
    let mut c = std::process::Command::new("tar");
    hide(&mut c);
    let o = c.arg("-xf").arg(archive).arg("-C").arg(dest).output().map_err(|er| format!("the system `tar` could not be started: {er}"))?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("yoshirian-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn tar_bytes() -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        let data = b"hello vpk";
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        b.append_data(&mut h, "sub/mod.vpk", &data[..]).unwrap();
        b.into_inner().unwrap()
    }

    fn gz(bytes: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(bytes).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn zip_is_detected_even_with_a_wrong_extension_and_extracts() {
        let d = tmp();
        let p = d.join("download.dat");
        let mut z = zip::ZipWriter::new(fs::File::create(&p).unwrap());
        z.start_file("a/mod.vpk", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b"data").unwrap();
        z.finish().unwrap();
        assert_eq!(sniff(&p), Kind::Zip);
        extract(&p, &d.join("out")).unwrap();
        assert_eq!(fs::read(d.join("out/a/mod.vpk")).unwrap(), b"data");
    }

    #[test]
    fn tar_and_tar_gz_extract() {
        let d = tmp();
        let t = d.join("x.tar");
        fs::write(&t, tar_bytes()).unwrap();
        assert_eq!(sniff(&t), Kind::Tar);
        extract(&t, &d.join("o1")).unwrap();
        assert_eq!(fs::read(d.join("o1/sub/mod.vpk")).unwrap(), b"hello vpk");

        let g = d.join("x.tgz");
        fs::write(&g, gz(&tar_bytes())).unwrap();
        assert_eq!(sniff(&g), Kind::Gz);
        extract(&g, &d.join("o2")).unwrap();
        assert_eq!(fs::read(d.join("o2/sub/mod.vpk")).unwrap(), b"hello vpk");
    }

    #[test]
    fn single_gz_file_extracts_under_its_inner_name() {
        let d = tmp();
        let g = d.join("pak01_dir.vpk.gz");
        fs::write(&g, gz(b"plain file")).unwrap();
        extract(&g, &d.join("o")).unwrap();
        assert_eq!(fs::read(d.join("o/pak01_dir.vpk")).unwrap(), b"plain file");
    }

    #[test]
    fn seven_z_extracts() {
        let d = tmp();
        let src = d.join("src");
        fs::create_dir_all(src.join("in")).unwrap();
        fs::write(src.join("in/mod.vpk"), b"seven").unwrap();
        let p = d.join("pack.bin"); // wrong extension on purpose
        sevenz_rust2::compress_to_path(&src, &p).unwrap();
        assert_eq!(sniff(&p), Kind::SevenZ);
        extract(&p, &d.join("out")).unwrap();
        assert_eq!(fs::read(d.join("out/in/mod.vpk")).unwrap(), b"seven");
    }

    #[test]
    fn vpk_and_unknown_are_not_archives() {
        let d = tmp();
        let v = d.join("a.vpk");
        fs::write(&v, [0x34, 0x12, 0xAA, 0x55, 0, 0, 0, 0]).unwrap();
        assert_eq!(sniff(&v), Kind::Vpk);
        assert!(!is_archive(&v));
        let u = d.join("notes.txt");
        fs::write(&u, b"just text").unwrap();
        assert!(!is_archive(&u));
    }

    #[test]
    fn entry_names_cannot_escape_the_destination() {
        let d = Path::new("/dest");
        assert!(safe_join(d, "../evil.txt").is_none());
        assert!(safe_join(d, "a/../../evil.txt").is_none());
        assert!(safe_join(d, "/etc/passwd").is_none());
        assert_eq!(safe_join(d, "a\\b.txt"), Some(PathBuf::from("/dest/a/b.txt")));
        assert_eq!(safe_join(d, "./x/y.vpk"), Some(PathBuf::from("/dest/x/y.vpk")));
    }
}
