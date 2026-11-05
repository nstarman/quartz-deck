//! US2 integration (T027, T032): `qd serve` renders via Quarto, serves the
//! preview, and pushes SSE events when the deck changes. Skipped (passes
//! trivially) when Quarto is not installed.

use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

mod common;
use common::{have, qd};

const BASIC: &str = include_str!("fixtures/basic.qmd");

fn qd_json(dir: &Path, args: &[&str]) -> (bool, Value) {
    let out = qd(dir).args(args).output().unwrap();
    let bytes = if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    };
    (
        out.status.success(),
        serde_json::from_slice(bytes).unwrap_or(Value::Null),
    )
}

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start(dir: &Path) -> Server {
    let mut child = qd(dir)
        .args(["serve", "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let v: Value =
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad serve output {line:?}: {e}"));
    let port = v["port"].as_u64().unwrap() as u16;
    // the editor link carries the run token in its fragment (never sent to the server)
    let url = v["url"].as_str().unwrap();
    let token = url
        .strip_prefix(&format!("http://127.0.0.1:{port}/#t="))
        .unwrap_or("");
    assert!(
        token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "{url}"
    );
    Server { child, port }
}

fn post(port: u16, path: &str, host: &str, ctype: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf).into_owned();
    let status = text.split_whitespace().nth(1).unwrap().parse().unwrap();
    (
        status,
        text.split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default(),
    )
}

fn get(port: u16, path: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf).into_owned();
    let status = text.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    (status, body)
}

/// Read SSE `event:` names (with data) until `want` arrives or `timeout` passes.
struct Events {
    r: BufReader<TcpStream>,
}

impl Events {
    fn open(port: u16) -> Events {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            s,
            "GET /events HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\n\r\n"
        )
        .unwrap();
        s.set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        Events {
            r: BufReader::new(s),
        }
    }

    fn wait(&mut self, want: &str, timeout: Duration) -> Option<String> {
        let end = Instant::now() + timeout;
        let mut event = String::new();
        while Instant::now() < end {
            let mut line = String::new();
            match self.r.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => {}
                Err(_) => continue, // read timeout: keep polling
            }
            let line = line.trim();
            if let Some(e) = line.strip_prefix("event:") {
                event = e.trim().to_string();
            } else if let Some(d) = line.strip_prefix("data:")
                && event == want
            {
                return Some(d.trim().to_string());
            }
        }
        None
    }
}

fn setup() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("deck.qmd"), BASIC).unwrap();
    let p = dir.path().to_path_buf();
    (dir, p)
}

#[test]
fn serve_renders_and_live_reloads() {
    if !have("quarto") {
        return;
    }
    let (_tmp, dir) = setup();
    let server = start(&dir);
    let mut ev = Events::open(server.port);
    let first = ev
        .wait("rendered", Duration::from_secs(60))
        .expect("initial render");
    let first: Value = serde_json::from_str(&first).unwrap();
    let html = first["html"].as_str().unwrap().to_string();

    // preview is served, carries element markers, slide ids, and assets
    let (status, body) = get(server.port, &html);
    assert_eq!(status, 200);
    assert!(
        body.contains(r#"data-qd="intro/0""#),
        "missing data-qd markers"
    );
    assert!(body.contains(r#"<section id="results""#));
    assert!(
        body.contains(r#"<section id="qd-3""#),
        "rule slide should get a preview id"
    );
    assert_eq!(get(server.port, "/").0, 200);
    assert!(
        (300..400).contains(&get(server.port, "/present/").0),
        "present/ should redirect"
    );

    // API mirrors the CLI
    let (_, api) = get(server.port, "/api/deck");
    let (_, cli) = qd_json(&dir, &["slides"]);
    assert_eq!(serde_json::from_str::<Value>(&api).unwrap(), cli);
    let (status, slide) = get(server.port, "/api/slide/results");
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&slide).unwrap()["slide"]["id"],
        "results"
    );
    assert_eq!(get(server.port, "/api/slide/nope").0, 404);

    // url command
    let (ok, url) = qd_json(&dir, &["url", "results"]);
    assert!(ok);
    let base = format!("http://127.0.0.1:{}/", server.port);
    let link = url["url"].as_str().unwrap();
    assert!(link.starts_with(&format!("{base}#/results&t=")), "{link}");
    let link = qd_json(&dir, &["url", "@3"]).1["url"].to_string();
    assert!(link.contains(&format!("{base}#/qd-3&t=")), "{link}");

    // an edit through the CLI -> changed, then rendered within 3 s (SC-005)
    let t = Instant::now();
    assert!(qd_json(&dir, &["title", "intro", "Live Title"]).0);
    ev.wait("changed", Duration::from_secs(3))
        .expect("changed event");
    let r = ev
        .wait("rendered", Duration::from_secs(10))
        .expect("rendered after edit");
    let elapsed = t.elapsed();
    eprintln!("edit -> rendered in {elapsed:?}");
    // SC-005 is "< 3 s" on a dev machine (measured 1.2 s). Shared CI runners
    // render several decks in parallel on 2 vCPUs, so CI sets a looser ceiling.
    let budget: f64 = std::env::var("QD_PREVIEW_BUDGET_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3.0);
    assert!(
        elapsed.as_secs_f64() < budget,
        "preview update took {elapsed:?} (budget {budget} s)"
    );
    let html2 = serde_json::from_str::<Value>(&r).unwrap()["html"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(get(server.port, &html2).1.contains("Live Title"));

    // an external editor writing a broken deck -> render_error, last good preview kept
    std::fs::write(dir.join("deck.qmd"), "---\ntitle: [unclosed\n---\n\n## X\n").unwrap();
    let err = ev
        .wait("render_error", Duration::from_secs(20))
        .expect("render_error event");
    assert!(!err.is_empty());
    let (status, body) = get(server.port, &html2);
    assert_eq!(status, 200);
    assert!(
        body.contains("Live Title"),
        "last good render must stay available"
    );

    drop(server);
    std::thread::sleep(Duration::from_millis(200));
    let (ok, e) = qd_json(&dir, &["url"]);
    assert!(!ok);
    assert_eq!(e["error"]["code"], "E_NO_SERVER");
}

#[test]
fn check_render_reports_quarto_errors() {
    if !have("quarto") {
        return;
    }
    let (_tmp, dir) = setup();
    let (ok, v) = qd_json(&dir, &["check", "--render"]);
    assert!(ok);
    assert_eq!(v["ok"], true, "{v}");
    std::fs::write(dir.join("deck.qmd"), "---\ntitle: [unclosed\n---\n\n## X\n").unwrap();
    let (_, v) = qd_json(&dir, &["check", "--render"]);
    assert_eq!(v["ok"], false);
    assert!(
        v["problems"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["code"] == "E_RENDER"),
        "{v}"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("qd-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp copies left behind: {leftovers:?}"
    );
}

#[test]
fn render_commands_without_quarto() {
    let (_tmp, dir) = setup();
    for args in [
        &["check", "--render"][..],
        &["render"][..],
        &["serve", "--port", "0"][..],
    ] {
        let out = qd(&dir)
            .env("QD_QUARTO", "/nonexistent/quarto")
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{args:?}");
        let err: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(err["error"]["code"], "E_NO_QUARTO", "{args:?}");
        assert!(
            err["error"]["hint"]
                .as_str()
                .unwrap()
                .contains("quarto.org")
        );
    }
    // pure edits still work (FR-031)
    for args in [
        &["slides"][..],
        &["title", "intro", "x"][..],
        &["add", "--title", "Y"][..],
    ] {
        let out = qd(&dir)
            .env("QD_QUARTO", "/nonexistent/quarto")
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?}");
    }
}

/// T033 over HTTP: POST /api/op edits the deck; non-JSON and foreign hosts are refused.
#[test]
fn api_op_over_http() {
    if !have("quarto") {
        return;
    }
    let (_tmp, dir) = setup();
    let server = start(&dir);
    let host = format!("127.0.0.1:{}", server.port);
    let body = r#"{"op":"title","args":{"slide":"intro","text":"From GUI"}}"#;

    let (status, _) = post(server.port, "/api/op", &host, "text/plain", body);
    assert_eq!(status, 415, "non-JSON must be refused (CSRF)");
    let (status, _) = post(
        server.port,
        "/api/op",
        "evil.example:80",
        "application/json",
        body,
    );
    assert_eq!(status, 403, "foreign Host must be refused (DNS rebinding)");
    let mut s = TcpStream::connect(("127.0.0.1", server.port)).unwrap();
    write!(
        s,
        "GET /api/source HTTP/1.1\r\nHost: evil.example\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut reply = String::new();
    s.read_to_string(&mut reply).unwrap();
    assert!(
        reply.starts_with("HTTP/1.1 403"),
        "reads refused too: {reply}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("deck.qmd")).unwrap(),
        BASIC
    );

    let (status, out) = post(server.port, "/api/op", &host, "application/json", body);
    assert_eq!(status, 200, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "title");
    assert!(
        std::fs::read_to_string(dir.join("deck.qmd"))
            .unwrap()
            .contains("## From GUI {#intro}")
    );

    let (status, out) = post(
        server.port,
        "/api/op",
        &host,
        "application/json",
        r#"{"op":"serve","args":{}}"#,
    );
    assert_eq!(status, 400);
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap()["error"]["code"],
        "E_INVALID_ARG"
    );
    let stale = r#"{"op":"title","args":{"slide":"intro","text":"x"},"expect":"0000000000000000"}"#;
    assert_eq!(
        post(server.port, "/api/op", &host, "application/json", stale).0,
        409
    );

    let (status, out) = post(
        server.port,
        "/api/op",
        &format!("localhost:{}", server.port),
        "application/json",
        r#"{"op":"undo"}"#,
    );
    assert_eq!(status, 200, "{out}");
    assert_eq!(
        std::fs::read_to_string(dir.join("deck.qmd")).unwrap(),
        BASIC
    );

    // render through the API: the reply links the final output for the browser
    let (status, out) = post(
        server.port,
        "/api/op",
        &host,
        "application/json",
        r#"{"op":"render"}"#,
    );
    assert_eq!(status, 200, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["href"], "/preview/deck.html");
    let (status, body) = get(server.port, "/preview/deck.html");
    assert_eq!(status, 200);
    assert!(body.contains("<section id=\"intro\"") && !body.contains("data-qd="));

    // export through the API (skips the check when no browser is available)
    let browser = qd(&dir)
        .args(["browser", "--no-download"])
        .output()
        .unwrap()
        .status
        .success();
    if browser {
        std::fs::write(dir.join("deck.qmd"), BASIC).unwrap();
        let (status, out) = post(
            server.port,
            "/api/op",
            &host,
            "application/json",
            r#"{"op":"export","args":{"pdf":true}}"#,
        );
        assert_eq!(status, 200, "{out}");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["html_href"], "/preview/export/deck.html", "{v}");
        assert_eq!(v["pdf_href"], "/preview/export/deck.pdf", "{v}");
        assert_eq!(get(server.port, "/preview/export/deck.pdf").0, 200);
    }
}

/// US5 (T046): `qd render` renders the deck itself with Quarto and reports
/// where the output went — next to the deck, or in a project's output-dir.
#[test]
fn render_reports_output() {
    if !have("quarto") {
        return;
    }
    let (_tmp, dir) = setup();
    let (ok, v) = qd_json(&dir, &["render"]);
    assert!(ok, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["format"], "revealjs");
    let out = PathBuf::from(v["output"].as_str().unwrap());
    assert!(out.is_absolute() && out.exists(), "{out:?}");
    assert_eq!(out.file_name().unwrap(), "deck.html");
    let html = std::fs::read_to_string(&out).unwrap();
    assert!(html.contains("<section id=\"intro\""));
    assert!(
        !html.contains("data-qd="),
        "final render must not carry editor markers"
    );
    assert!(v["log"].is_string());

    let (ok, v) = qd_json(&dir, &["render", "--to", "html"]);
    assert!(ok, "{v}");
    assert_eq!(v["format"], "html");
    let (ok, v) = qd_json(&dir, &["render", "--to", "--evil"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_USAGE");
    let (ok, v) = qd_json(&dir, &["render", "--to", "x;y"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_INVALID_ARG");

    // in a Quarto project the output goes to its output-dir
    std::fs::write(
        dir.join("_quarto.yml"),
        "project:\n  output-dir: _output\nformat: revealjs\n",
    )
    .unwrap();
    let (ok, v) = qd_json(&dir, &["render"]);
    assert!(ok, "{v}");
    let out = PathBuf::from(v["output"].as_str().unwrap());
    assert!(
        out.starts_with(std::fs::canonicalize(&dir).unwrap().join("_output")),
        "{out:?}"
    );
    assert!(out.exists());

    // a broken deck: E_RENDER with the Quarto log in details
    std::fs::remove_file(dir.join("_quarto.yml")).unwrap();
    std::fs::write(dir.join("deck.qmd"), "---\ntitle: [unclosed\n---\n\n## X\n").unwrap();
    let (ok, v) = qd_json(&dir, &["render"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_RENDER");
    assert!(
        v["error"]["details"].as_str().unwrap().contains("YAML"),
        "{v}"
    );
}

/// Spec 004 US3 (T023, T025): token-guarded runs, events, stored outputs, and
/// a preview that shows them without executing anything. Needs uv and Quarto.
#[test]
fn run_cells_over_http() {
    if !have("quarto") || !have("uv") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("deck.qmd");
    std::fs::write(&deck, include_str!("fixtures/run/basic.qmd")).unwrap();
    assert!(qd_json(dir.path(), &["env", "init"]).0);
    assert!(qd_json(dir.path(), &["env", "add", "matplotlib"]).0);
    let before = std::fs::read(&deck).unwrap();
    let server = start(dir.path());
    let p = server.port;

    // the token: in the editor page and the user-only session file, nowhere else
    let session = std::fs::read_dir(dir.path().join(".cache"))
        .unwrap()
        .map(|e| e.unwrap().path().join("session.json"))
        .find(|f| f.is_file())
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&session).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "session file is user-only");
    }
    let token = serde_json::from_str::<Value>(&std::fs::read_to_string(&session).unwrap()).unwrap()
        ["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        !get(p, "/").1.contains(&token),
        "the page never holds the token"
    );
    let auth = format!("application/json\r\nX-QD-Token: {token}");

    let run = |body: &str, ctype: &str| post(p, "/api/run", "127.0.0.1", ctype, body).0;
    let print = r#"{"cells":["print/0"]}"#;
    assert_eq!(run(print, "application/json"), 403, "no token");
    assert_eq!(
        run(print, "application/json\r\nX-QD-Token: wrong"),
        403,
        "wrong token"
    );
    let evil = format!("{auth}\r\nOrigin: http://evil.example");
    assert_eq!(run(print, &evil), 403, "right token, other origin");
    let op = r#"{"op":"run","args":{"cells":["print/0"]}}"#;
    assert_ne!(
        post(p, "/api/op", "127.0.0.1", "application/json", op).0,
        200,
        "run_op refuses run"
    );
    assert_eq!(run(r#"{"cells":["plain/0"]}"#, &auth), 400, "E_NOT_CODE");
    assert_eq!(run(r#"{"cells":["r/0"]}"#, &auth), 400, "E_ENGINE");

    let mut ev = Events::open(p);
    assert_eq!(run(print, &auth), 202);
    let wait = Duration::from_secs(120);
    assert!(ev.wait("kernel", wait).unwrap().contains("starting"));
    let out = ev.wait("cell_output", wait).expect("cell output");
    assert!(out.contains("42"), "{out}");
    assert!(ev.wait("run_done", wait).unwrap().contains("\"done\""));

    // Run all: in order, skipping eval:false and non-Python, stopping at the failure
    assert_eq!(run(r#"{"all":true}"#, &auth), 202);
    let started = ev.wait("run_started", wait).unwrap();
    assert!(
        started.contains("fails/0") && !started.contains("skipped/0") && !started.contains("r/0"),
        "{started}"
    );
    let done = ev.wait("run_done", wait).unwrap();
    assert!(done.contains("\"error\""), "{done}");

    let outputs: Value = serde_json::from_str(&get(p, "/api/outputs").1).unwrap();
    let cells = &outputs["cells"];
    assert_eq!(cells["state/0"]["items"][0]["text"], "43", "{outputs}");
    assert_eq!(cells["fails/0"]["items"][0]["ename"], "ZeroDivisionError");
    let img = cells["plot/0"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "image")
        .unwrap();
    assert_eq!(get(p, img["url"].as_str().unwrap()).0, 200);
    assert!(cells["print/0"]["stale"] == false && cells["print/0"]["counter"].is_number());
    assert_eq!(
        std::fs::read(&deck).unwrap(),
        before,
        "runs never touch the .qmd"
    );

    // the preview shows the stored outputs without running anything
    let end = Instant::now() + wait;
    let mut html = String::new();
    while !html.contains("ZeroDivisionError") && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(200)); // the render after run_done
        html = get(p, "/preview/deck.qd-preview.html").1;
    }
    assert!(
        html.contains("qd-output")
            && html.contains("/outputs/")
            && html.contains("ZeroDivisionError")
    );

    // an edited cell's output is marked stale
    assert!(
        qd_json(
            dir.path(),
            &["replace", "--old", "21 * 2", "--new", "21 * 3"]
        )
        .0
    );
    let outputs: Value = serde_json::from_str(&get(p, "/api/outputs").1).unwrap();
    assert_eq!(outputs["cells"]["print/0"]["stale"], true);

    assert_eq!(
        post(p, "/api/run/interrupt", "127.0.0.1", &auth, "{}").0,
        200
    );

    // shutdown needs the token too; `qd stop` uses it
    let shutdown = |ctype: &str| post(p, "/api/shutdown", "127.0.0.1", ctype, "{}").0;
    assert_eq!(shutdown("application/json"), 403);
    assert_eq!(get(p, "/api/deck").0, 200, "still serving");
    assert!(qd_json(dir.path(), &["stop"]).0);
    let mut server = server;
    let end = Instant::now() + Duration::from_secs(20);
    while server.child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < end, "server did not stop");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!session.exists(), "session file removed on shutdown");
}
