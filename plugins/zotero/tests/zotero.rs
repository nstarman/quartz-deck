//! `qd-zotero COLLECTION` (spec 014 US3, research R8): a Zotero collection
//! as CSL-JSON on stdout, against a mock of Zotero 7's local API.

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// A mock local API serving `items` 100 per page, by the request's `start`;
/// its URL, for `QD_ZOTERO_URL`.
fn mock(items: Vec<Value>) -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).into_owned();
            let line = req.lines().next().unwrap_or("");
            assert!(
                line.starts_with("GET /api/users/0/collections/ABCD1234/items/top?")
                    && line.contains("include=data,csljson")
                    && line.contains("limit=100"),
                "{line}"
            );
            let start: usize = line
                .split(['?', '&', ' '])
                .find_map(|p| p.strip_prefix("start="))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let page: Vec<&Value> = items.iter().skip(start).take(100).collect();
            let body = serde_json::to_string(&page).unwrap();
            let _ = write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    url
}

/// One Zotero item as `include=data,csljson` returns it.
fn item(n: usize, data: Value, mut csl: Value) -> Value {
    csl["id"] = json!(format!("1234/ITEM{n}"));
    json!({"key": format!("ITEM{n}"), "data": data, "csljson": csl})
}

fn run(url: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qd-zotero"))
        .env("QD_ZOTERO_URL", url)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn prints_the_collection_as_csl_json() {
    let mut items = vec![
        // Zotero 7's own citation key
        item(
            0,
            json!({"itemType": "journalArticle", "citationKey": "bonaca2020spur"}),
            json!({"type": "article-journal", "title": "Spur", "DOI": "10.1/x"}),
        ),
        // Better BibTeX's key, in Extra
        item(
            1,
            json!({"itemType": "journalArticle", "extra": "tex.x: y\nCitation Key: koposov2010"}),
            json!({"type": "article-journal", "title": "GD-1"}),
        ),
        // no key: qd makes one; arXiv's own DOI yields to the URL
        item(
            2,
            json!({"itemType": "preprint"}),
            json!({"type": "article", "title": "Pre", "DOI": "10.48550/arXiv.1", "URL": "https://arxiv.org/abs/1"}),
        ),
        item(
            3,
            json!({"itemType": "attachment"}),
            json!({"title": "PDF"}),
        ),
        item(4, json!({"itemType": "note"}), json!({"title": "note"})),
    ];
    // past one page of 100
    items.extend((5..106).map(|n| {
        item(
            n,
            json!({"itemType": "book"}),
            json!({"type": "book", "title": format!("B{n}")}),
        )
    }));
    // Zotero 7's local API sends csljson as a string holding a one-entry array
    for i in [1, 2] {
        let csl = items[i]["csljson"].clone();
        items[i]["csljson"] = json!(serde_json::to_string(&[csl]).unwrap());
    }
    let url = mock(items);
    let out = run(&url, &["ABCD1234"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v.len(),
        104,
        "attachments and notes skipped, both pages read"
    );
    assert_eq!(v[0]["id"], "bonaca2020spur");
    assert_eq!(v[1]["id"], "koposov2010");
    assert!(
        v[2].get("id").is_none(),
        "Zotero's own item ids are not keys: {}",
        v[2]
    );
    assert!(v[2].get("DOI").is_none() && v[2]["URL"] == "https://arxiv.org/abs/1");
    assert_eq!(v[0]["DOI"], "10.1/x", "other DOIs stay");
}

#[test]
fn refuses_a_bad_key_without_connecting() {
    for k in ["ABC", "ABCD/EFG", "ABCDEFGHI"] {
        let out = run("http://127.0.0.1:9", &[k]);
        assert_eq!(out.status.code(), Some(2), "{k}");
        assert!(
            String::from_utf8_lossy(&out.stderr).starts_with("E_INVALID_ARG"),
            "{k}"
        );
    }
    let out = run("http://127.0.0.1:9", &[]);
    assert_eq!(out.status.code(), Some(2), "no collection");
}

#[test]
fn zotero_not_running_fails_fast_with_the_hint() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let t = Instant::now();
    let out = run(&format!("http://{closed}"), &["ABCD1234"]);
    assert!(t.elapsed() < Duration::from_secs(5), "{:?}", t.elapsed());
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("E_ZOTERO") && err.contains("Allow other applications"),
        "{err}"
    );
    assert!(out.stdout.is_empty());
}

/// Spec 014 (T025): `qd bib pull --from zotero KEY` runs this binary from
/// `PATH` and fills a deck's references. Needs `qd` built beside it (as
/// `cargo test --workspace` does) and Quarto; skips otherwise.
#[test]
fn qd_pulls_from_zotero() {
    let plugin = std::path::Path::new(env!("CARGO_BIN_EXE_qd-zotero"));
    let qd = plugin.with_file_name("qd");
    let quarto = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !qd.is_file() || !quarto {
        eprintln!("needs qd beside qd-zotero, and quarto: skipping");
        return;
    }
    let url = mock(vec![item(
        0,
        json!({"itemType": "journalArticle", "citationKey": "bonaca2020"}),
        json!({"type": "article-journal", "title": "Spur", "DOI": "10.1/x",
               "author": [{"family": "Bonaca", "given": "Ana"}], "issued": {"date-parts": [[2020]]}}),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let t = dir.path();
    std::fs::write(
        t.join("talk.qmd"),
        "---\ntitle: T\n---\n\n## One\n\nSee @bonaca2020.\n",
    )
    .unwrap();
    let path = format!(
        "{}:{}",
        plugin.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );
    let out = Command::new(&qd)
        .current_dir(t)
        .env("PATH", path)
        .env("QD_ZOTERO_URL", &url)
        .env("QD_CACHE_DIR", t.join(".cache"))
        .env("QD_CONFIG_DIR", t.join(".config"))
        .env_remove("QD_DECK")
        .args(["bib", "pull", "--from", "zotero", "ABCD1234"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["source"], "zotero", "{v}");
    assert_eq!(v["added"], json!(["bonaca2020"]), "{v}");
    let bib = std::fs::read_to_string(t.join("references.bib")).unwrap();
    assert!(
        bib.contains("@article{bonaca2020,") && bib.contains("10.1/x"),
        "{bib}"
    );
}
