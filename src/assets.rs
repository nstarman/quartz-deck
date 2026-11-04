//! Pinned offline libraries (research.md R1, R2, R6): MathJax is downloaded
//! once into qd's cache and verified by SHA-256; plotly.js comes from the
//! user's Python plotly so it matches their figures.

use crate::error::{QdError, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A file qd downloads and pins by hash.
pub struct Pinned {
    pub name: &'static str,
    pub version: &'static str,
    pub url: &'static str,
    pub file: &'static str,
    pub sha256: &'static str,
}

pub const MATHJAX: Pinned = Pinned {
    name: "mathjax",
    version: "3.2.2",
    url: "https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-svg-full.js",
    file: "tex-svg-full.js",
    sha256: "a4354ff94fd868aea0cc6eaaa79a57fda0588646fc46ee3700a349ee0a11cbe6",
};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Asset {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub source: String,
    pub path: PathBuf,
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut h = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Download `url` to `dest`, keeping it only if its SHA-256 is `sha256`.
pub fn download_verified(url: &str, dest: &Path, sha256: &str) -> Result<()> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let part = dest.with_extension("part");
    let fetch = || -> std::result::Result<(), String> {
        let mut resp = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
        let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        std::io::copy(&mut resp.body_mut().as_reader(), &mut file)
            .map_err(|e| format!("{url}: {e}"))?;
        Ok(())
    };
    if let Err(e) = fetch() {
        let _ = std::fs::remove_file(&part);
        return Err(QdError::Download(e));
    }
    let got = sha256_file(&part)?;
    if got != sha256 {
        let _ = std::fs::remove_file(&part);
        return Err(QdError::Download(format!(
            "{url}: checksum mismatch (expected {sha256}, got {got})"
        )));
    }
    std::fs::rename(&part, dest)?;
    Ok(())
}

/// `<root>/lib/<name>-<version>/<file>`, downloaded and verified if needed.
pub fn ensure(root: &Path, p: &Pinned, allow_download: bool) -> Result<Asset> {
    let path = root
        .join("lib")
        .join(format!("{}-{}", p.name, p.version))
        .join(p.file);
    let asset = |source: &str| Asset {
        name: p.name.into(),
        version: p.version.into(),
        sha256: p.sha256.into(),
        source: source.into(),
        path: path.clone(),
    };
    if path.exists() {
        if sha256_file(&path)? == p.sha256 {
            return Ok(asset("cache"));
        }
        std::fs::remove_file(&path)?;
        if !allow_download {
            return Err(QdError::Download(format!(
                "cached {} {} failed its checksum and downloads are disabled",
                p.name, p.version
            )));
        }
    } else if !allow_download {
        return Err(QdError::Download(format!(
            "{} {} is not cached yet and downloads are disabled",
            p.name, p.version
        )));
    }
    download_verified(p.url, &path, p.sha256)?;
    Ok(asset("download"))
}

/// plotly.js shipped with the user's Python plotly, if importable.
pub fn plotly_from_python() -> Option<Asset> {
    let out = std::process::Command::new("python3")
        .args(["-I", "-c", "import plotly, pathlib; print(plotly.__version__); print(pathlib.Path(plotly.__file__).parent / 'package_data' / 'plotly.min.js')"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut lines = text.lines();
    let version = lines.next()?.trim().to_string();
    let path = PathBuf::from(lines.next()?.trim());
    let sha256 = sha256_file(&path).ok()?;
    Some(Asset {
        name: "plotly.js".into(),
        version: format!("python plotly {version}"),
        sha256,
        source: "python".into(),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn online() -> bool {
        use std::net::ToSocketAddrs;
        ("cdn.jsdelivr.net", 443)
            .to_socket_addrs()
            .ok()
            .and_then(|mut a| a.next())
            .is_some_and(|a| {
                std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(3)).is_ok()
            })
    }

    #[test]
    fn hashes_files() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("x");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn cached_file_with_good_hash_is_used_without_network() {
        let d = tempfile::tempdir().unwrap();
        let p = Pinned {
            name: "t",
            version: "1",
            url: "https://invalid.invalid/x",
            file: "x.js",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        };
        let dir = d.path().join("lib/t-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.js"), b"abc").unwrap();
        let a = ensure(d.path(), &p, false).unwrap();
        assert_eq!(a.source, "cache");
        assert_eq!(a.path, dir.join("x.js"));
    }

    #[test]
    fn tampered_cache_is_rejected_when_download_forbidden() {
        let d = tempfile::tempdir().unwrap();
        let p = Pinned {
            name: "t",
            version: "1",
            url: "https://invalid.invalid/x",
            file: "x.js",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        };
        let dir = d.path().join("lib/t-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.js"), b"tampered").unwrap();
        assert_eq!(
            ensure(d.path(), &p, false).unwrap_err().code(),
            "E_DOWNLOAD"
        );
        assert!(
            !dir.join("x.js").exists(),
            "a file failing its checksum must not stay in the cache"
        );
    }

    #[test]
    fn missing_and_forbidden_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            ensure(d.path(), &MATHJAX, false).unwrap_err().code(),
            "E_DOWNLOAD"
        );
    }

    #[test]
    fn download_with_wrong_hash_is_discarded() {
        if !online() {
            eprintln!("offline: skipping");
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let p = Pinned {
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            ..MATHJAX
        };
        assert_eq!(ensure(d.path(), &p, true).unwrap_err().code(), "E_DOWNLOAD");
        let left: Vec<_> = walk(d.path());
        assert!(
            left.iter().all(|p| p.is_dir()),
            "no files may remain: {left:?}"
        );
    }

    #[test]
    fn downloads_pinned_mathjax() {
        if !online() {
            eprintln!("offline: skipping");
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let a = ensure(d.path(), &MATHJAX, true).unwrap();
        assert_eq!(a.source, "download");
        assert_eq!(sha256_file(&a.path).unwrap(), MATHJAX.sha256);
        assert_eq!(ensure(d.path(), &MATHJAX, false).unwrap().source, "cache");
    }

    #[test]
    fn plotly_lookup_does_not_panic() {
        if let Some(a) = plotly_from_python() {
            assert!(a.path.ends_with("plotly.min.js") && a.path.exists());
        }
    }

    fn walk(p: &Path) -> Vec<PathBuf> {
        let mut out = vec![];
        for e in std::fs::read_dir(p).unwrap() {
            let e = e.unwrap().path();
            if e.is_dir() {
                out.extend(walk(&e));
            }
            out.push(e);
        }
        out
    }
}
