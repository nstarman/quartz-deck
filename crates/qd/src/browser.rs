//! Finding a browser for the offline check and PDF (spec FR-042, research R5):
//! `$QD_CHROME`, an installed Chrome/Chromium, or qd's verified download of a
//! pinned Chrome for Testing `chrome-headless-shell`.

use crate::error::{QdError, Result};
use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Pinned Chrome for Testing build (hashes of the official zips, 2026-11-04).
pub const CFT_VERSION: &str = "157.0.8103.38";
pub const CFT_BUILDS: &[(&str, &str)] = &[
    (
        "mac-arm64",
        "e6a6b13551aa12f951d0891d79282606b13742837c8fa0505e1272bae116ce82",
    ),
    (
        "mac-x64",
        "79821745d0a72d2ba752d981284ed0a9e6fa320dad133596941a17dba6186de3",
    ),
    (
        "linux64",
        "deafffa74660b34853a4119c361fd427ac8c9fcb62db51ddb8a797cd83a4ba2d",
    ),
    (
        "linux-arm64",
        "b9b910a57b2a15eb3e55000a2339f93a973934a413ebfc6e585a50ec6c42fd6c",
    ),
];

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Browser {
    pub path: PathBuf,
    /// `env` ($QD_CHROME), `installed`, or `download` (qd's cache)
    pub source: String,
    pub version: Option<String>,
}

/// Chrome for Testing platform name for this machine.
pub fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("mac-arm64"),
        ("macos", "x86_64") => Some("mac-x64"),
        ("linux", "x86_64") => Some("linux64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        _ => None,
    }
}

/// Standard install locations checked after `$QD_CHROME`.
fn standard_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(target_os = "macos") {
        let apps = [
            "Google Chrome.app/Contents/MacOS/Google Chrome",
            "Chromium.app/Contents/MacOS/Chromium",
            "Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        ];
        let home = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications"));
        for base in [Some(PathBuf::from("/Applications")), home]
            .into_iter()
            .flatten()
        {
            v.extend(apps.iter().map(|a| base.join(a)));
        }
    }
    v
}

const PATH_NAMES: &[&str] = &[
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "chrome",
];

fn cache_dir(root: &Path) -> PathBuf {
    root.join("browser")
}

/// The cached `chrome-headless-shell`, if qd downloaded one.
pub fn cached(root: &Path) -> Option<PathBuf> {
    let plat = platform()?;
    let p = cache_dir(root)
        .join(CFT_VERSION)
        .join(format!("chrome-headless-shell-{plat}"))
        .join("chrome-headless-shell");
    p.is_file().then_some(p)
}

fn installed(path: PathBuf) -> Browser {
    Browser {
        path,
        source: "installed".into(),
        version: None,
    }
}

fn find_in(
    env: Option<OsString>,
    standard: &[PathBuf],
    path_env: Option<OsString>,
    root: &Path,
) -> Result<Option<Browser>> {
    if let Some(v) = env.filter(|v| !v.is_empty()) {
        let p = PathBuf::from(v);
        return if p.is_file() {
            Ok(Some(Browser {
                path: p,
                source: "env".into(),
                version: None,
            }))
        } else {
            Err(QdError::NoBrowser(format!(
                "QD_CHROME points to {}, which is not a file",
                p.display()
            )))
        };
    }
    if let Some(p) = standard.iter().find(|p| p.is_file()) {
        return Ok(Some(installed(p.clone())));
    }
    for dir in path_env.iter().flat_map(std::env::split_paths) {
        if let Some(p) = PATH_NAMES.iter().map(|n| dir.join(n)).find(|p| p.is_file()) {
            return Ok(Some(installed(p)));
        }
    }
    Ok(cached(root).map(|path| Browser {
        path,
        source: "download".into(),
        version: Some(CFT_VERSION.into()),
    }))
}

/// Find a browser; download the pinned one if none is found and allowed.
pub fn find(root: &Path, allow_download: bool) -> Result<Browser> {
    match find_in(
        std::env::var_os("QD_CHROME"),
        &standard_paths(),
        std::env::var_os("PATH"),
        root,
    )? {
        Some(b) => Ok(b),
        None if allow_download => install(root),
        None => Err(QdError::NoBrowser(
            "no Chrome/Chromium found and downloads are disabled".into(),
        )),
    }
}

/// Verify `zip` against `sha256`, then unpack it into `dest`.
fn install_zip(zip: &Path, sha256: &str, dest: &Path) -> Result<()> {
    let got = crate::assets::sha256_file(zip)?;
    if got != sha256 {
        return Err(QdError::Download(format!(
            "browser archive checksum mismatch (expected {sha256}, got {got})"
        )));
    }
    let tmp = dest.with_extension("unpacking");
    let _ = std::fs::remove_dir_all(&tmp);
    let unpack = || -> std::result::Result<(), String> {
        let mut archive =
            zip::ZipArchive::new(std::fs::File::open(zip).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        archive.extract(&tmp).map_err(|e| e.to_string())
    };
    if let Err(e) = unpack() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(QdError::Download(format!(
            "could not unpack the browser: {e}"
        )));
    }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

/// Download, verify and unpack the pinned Chrome for Testing headless shell.
pub fn install(root: &Path) -> Result<Browser> {
    let plat = platform().ok_or_else(|| {
        QdError::NoBrowser("no pinned Chrome for Testing build for this platform".into())
    })?;
    let sha = CFT_BUILDS
        .iter()
        .find(|(p, _)| *p == plat)
        .map(|(_, s)| *s)
        .unwrap();
    let url = format!(
        "https://storage.googleapis.com/chrome-for-testing-public/{CFT_VERSION}/{plat}/chrome-headless-shell-{plat}.zip"
    );
    let zip = cache_dir(root).join(format!("chrome-headless-shell-{plat}-{CFT_VERSION}.zip"));
    crate::assets::download_verified(&url, &zip, sha)?;
    let dest = cache_dir(root).join(CFT_VERSION);
    let res = install_zip(&zip, sha, &dest);
    let _ = std::fs::remove_file(&zip);
    res?;
    cached(root)
        .map(|path| Browser {
            path,
            source: "download".into(),
            version: Some(CFT_VERSION.into()),
        })
        .ok_or_else(|| {
            QdError::Download("the downloaded archive did not contain chrome-headless-shell".into())
        })
}

/// Delete qd's downloaded browser (never an installed one). Returns whether anything was removed.
pub fn remove(root: &Path) -> Result<bool> {
    let dir = cache_dir(root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn pins_cover_platforms() {
        for p in ["mac-arm64", "mac-x64", "linux64", "linux-arm64"] {
            let (_, h) = CFT_BUILDS.iter().find(|(x, _)| *x == p).expect(p);
            assert_eq!(h.len(), 64);
            assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        }
        assert!(
            platform().is_some(),
            "dev/CI platforms must have a pinned build"
        );
    }

    #[test]
    fn env_wins_and_must_exist() {
        let d = tempfile::tempdir().unwrap();
        let chrome = d.path().join("my-chrome");
        exe(&chrome);
        let other = d.path().join("std-chrome");
        exe(&other);
        let b = find_in(
            Some(chrome.clone().into()),
            std::slice::from_ref(&other),
            None,
            d.path(),
        )
        .unwrap()
        .unwrap();
        assert_eq!((b.path, b.source.as_str()), (chrome, "env"));
        let err =
            find_in(Some(d.path().join("nope").into()), &[other], None, d.path()).unwrap_err();
        assert_eq!(err.code(), "E_NO_BROWSER");
    }

    #[test]
    fn standard_then_path_then_cache() {
        let d = tempfile::tempdir().unwrap();
        let std1 = d.path().join("apps/Chrome");
        exe(&std1);
        let b = find_in(
            None,
            &[d.path().join("missing"), std1.clone()],
            None,
            d.path(),
        )
        .unwrap()
        .unwrap();
        assert_eq!((b.path, b.source.as_str()), (std1, "installed"));

        let bin = d.path().join("bin");
        exe(&bin.join("chromium"));
        let b = find_in(None, &[], Some(bin.clone().into_os_string()), d.path())
            .unwrap()
            .unwrap();
        assert_eq!(
            (b.path, b.source.as_str()),
            (bin.join("chromium"), "installed")
        );

        let root = d.path().join("cache");
        let plat = platform().unwrap();
        let shell = root
            .join("browser")
            .join(CFT_VERSION)
            .join(format!("chrome-headless-shell-{plat}"))
            .join("chrome-headless-shell");
        exe(&shell);
        let b = find_in(None, &[], None, &root).unwrap().unwrap();
        assert_eq!(
            (b.path, b.source.as_str(), b.version.as_deref()),
            (shell, "download", Some(CFT_VERSION))
        );

        assert!(
            find_in(None, &[], None, &d.path().join("empty"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn install_zip_verifies_and_unpacks() {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let zpath = d.path().join("b.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zpath).unwrap());
            let opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
            z.start_file("chrome-headless-shell-x/chrome-headless-shell", opts)
                .unwrap();
            z.write_all(b"#!/bin/sh\n").unwrap();
            z.finish().unwrap();
        }
        let good = crate::assets::sha256_file(&zpath).unwrap();
        let dest = d.path().join("out");
        assert_eq!(
            install_zip(&zpath, &"0".repeat(64), &dest)
                .unwrap_err()
                .code(),
            "E_DOWNLOAD"
        );
        assert!(!dest.exists(), "nothing unpacked from a bad archive");
        install_zip(&zpath, &good, &dest).unwrap();
        let bin = dest.join("chrome-headless-shell-x/chrome-headless-shell");
        assert!(bin.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                std::fs::metadata(&bin).unwrap().permissions().mode() & 0o111,
                0,
                "must be executable"
            );
        }
    }

    #[test]
    fn remove_only_touches_qd_cache() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("browser/x")).unwrap();
        std::fs::create_dir_all(d.path().join("lib")).unwrap();
        assert!(remove(d.path()).unwrap());
        assert!(!d.path().join("browser").exists());
        assert!(d.path().join("lib").exists());
        assert!(!remove(d.path()).unwrap());
    }

    /// Real 100+ MB download: opt in with QD_TEST_BROWSER_DOWNLOAD=1.
    #[test]
    fn real_install() {
        if std::env::var_os("QD_TEST_BROWSER_DOWNLOAD").is_none() {
            eprintln!("set QD_TEST_BROWSER_DOWNLOAD=1 to run");
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let b = install(d.path()).unwrap();
        assert_eq!(b.source, "download");
        let out = std::process::Command::new(&b.path)
            .arg("--version")
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("157."),
            "{out:?}"
        );
    }
}
