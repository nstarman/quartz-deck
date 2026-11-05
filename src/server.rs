//! `qd serve` (contracts/http.md): localhost-only server with the editor
//! shell, the latest good Quarto render, a read API, and SSE live updates.

use crate::cli::{run_op, show_json, slides_json};
use crate::deck::{BlockKind, Deck, version_of};
use crate::error::{QdError, Result};
use crate::kernel::{Session as Kernel, cell_code};
use crate::outputs::Outputs;
use crate::render::{preview_copy, render};
use crate::store::Store;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use notify::Watcher;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Notify, broadcast};
use tower_http::services::ServeDir;

const INDEX: &str = include_str!("ui/index.html");
const EDITOR_JS: &str = include_str!("ui/editor.js");
const EDITOR_CSS: &str = include_str!("ui/editor.css");
/// Editor ES modules imported by `editor.js`, served at `/ui/<name>`.
const UI_MODULES: &[(&str, &str)] = &[
    ("inline.js", include_str!("ui/inline.js")),
    ("toolbar.js", include_str!("ui/toolbar.js")),
    ("source.js", include_str!("ui/source.js")),
    ("actions.js", include_str!("ui/actions.js")),
    ("menu.js", include_str!("ui/menu.js")),
    ("pane.js", include_str!("ui/pane.js")),
    ("run.js", include_str!("ui/run.js")),
];

#[derive(Clone, Debug)]
enum Ev {
    Changed(String),
    Rendering,
    Rendered(String),
    RenderError(String),
    /// Code runs (contracts/http.md): `kernel`, `run_started`, `cell_output`, `run_done`.
    Run(&'static str, Value),
}

impl Ev {
    fn to_sse(&self, html: &str) -> Event {
        let (name, data) = match self {
            Ev::Changed(v) => ("changed", json!({"version": v})),
            Ev::Rendering => ("rendering", json!({})),
            Ev::Rendered(v) => ("rendered", json!({"version": v, "html": html})),
            Ev::RenderError(m) => ("render_error", json!({"message": m})),
            Ev::Run(name, data) => (*name, data.clone()),
        };
        Event::default().event(name).data(data.to_string())
    }
}

#[derive(Default)]
struct St {
    version: String,
    rendered: Option<String>,
    error: Option<String>,
}

struct App {
    store: Store,
    out: PathBuf,
    /// URL path of the preview HTML, e.g. `/preview/deck.qd-preview.html`.
    html: String,
    tx: broadcast::Sender<Ev>,
    st: Mutex<St>,
    kick: Notify,
    /// Required (`X-QD-Token`) to run code: only the editor page knows it (FR-015).
    token: String,
    /// The deck's live session, started on the first run.
    kernel: Mutex<Option<Arc<Kernel>>>,
    runs: AtomicU64,
}

impl App {
    fn dir(&self) -> PathBuf {
        self.store
            .path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default()
    }

    fn emit(&self, name: &'static str, data: Value) {
        let _ = self.tx.send(Ev::Run(name, data));
    }
}

type Shared = Arc<App>;

fn err_response(e: QdError) -> Response {
    let status = match e {
        QdError::NotFound(_) => StatusCode::NOT_FOUND,
        QdError::Ambiguous(_) | QdError::InvalidArg(_) | QdError::Usage(_) => {
            StatusCode::BAD_REQUEST
        }
        QdError::Conflict(_) | QdError::StaleElement { .. } | QdError::NoUndo(_) => {
            StatusCode::CONFLICT
        }
        QdError::UnsafeEdit(_) | QdError::OfflineIncompatible(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        QdError::NotCode(_) | QdError::Engine(_) => StatusCode::BAD_REQUEST,
        QdError::NoEnv(_) => StatusCode::NOT_FOUND,
        QdError::NoBrowser(_) | QdError::Download(_) | QdError::NoQuarto | QdError::EnvTool(_) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(e.to_json())).into_response()
}

/// Editor assets: never cached, so a new `qd` build is picked up on reload.
fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
}

/// The editor page, carrying this session's run token.
async fn index(State(app): State<Shared>) -> Response {
    let meta = format!("<head>\n<meta name=\"qd-token\" content=\"{}\">", app.token);
    let page = INDEX.replacen("<head>", &meta, 1);
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        page,
    )
        .into_response()
}

async fn editor_js() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", EDITOR_JS)
}

async fn editor_css() -> impl IntoResponse {
    asset("text/css; charset=utf-8", EDITOR_CSS)
}

async fn ui_module(UrlPath(name): UrlPath<String>) -> Response {
    match UI_MODULES.iter().find(|(n, _)| *n == name) {
        Some((_, body)) => asset("text/javascript; charset=utf-8", body).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn present(State(app): State<Shared>) -> Redirect {
    Redirect::to(&app.html)
}

/// The whole deck text (spec 004 source pane).
async fn api_source(State(app): State<Shared>) -> Response {
    match app.store.read() {
        Ok(d) => Json(json!({"text": d.text, "version": d.version})).into_response(),
        Err(e) => err_response(e),
    }
}

async fn api_deck(State(app): State<Shared>) -> Response {
    match app.store.read() {
        Ok(d) => Json(slides_json(&d)).into_response(),
        Err(e) => err_response(e),
    }
}

async fn api_slide(State(app): State<Shared>, UrlPath(r): UrlPath<String>) -> Response {
    let res = app
        .store
        .read()
        .and_then(|d| d.slide(&r).map(|s| show_json(&d, s)));
    match res {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(serde::Deserialize)]
struct OpBody {
    op: String,
    #[serde(default)]
    args: Value,
    expect: Option<String>,
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn local_host(h: &str) -> bool {
    let host = h.rsplit_once(':').map_or(h, |(h, _)| h);
    matches!(host, "127.0.0.1" | "localhost")
}

/// Only pages served by this server may edit: the Host must be loopback
/// (blocks DNS rebinding) and any Origin must be too. The `Json` extractor
/// already refuses non-JSON bodies, which browsers cannot send cross-site
/// without a CORS preflight we never grant.
fn same_origin(headers: &HeaderMap) -> bool {
    let get = |n| headers.get(n).and_then(|v| v.to_str().ok());
    let host_ok = get(header::HOST).is_some_and(local_host);
    let origin_ok =
        get(header::ORIGIN).is_none_or(|o| o.strip_prefix("http://").is_some_and(local_host));
    host_ok && origin_ok
}

async fn api_op(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<OpBody>,
) -> Response {
    if !same_origin(&headers) {
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }
    let store = app.store.clone();
    let res = tokio::task::spawn_blocking(move || {
        run_op(&store, &body.op, &body.args, body.expect.as_deref())
    })
    .await;
    match res {
        Ok(Ok(mut v)) => {
            // outputs inside the deck folder can be opened through /preview/'s fallback
            let deck_dir = app
                .store
                .path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_default();
            let href = |p: &Value| {
                let path = std::path::Path::new(p.as_str()?);
                if !matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("html" | "pdf")
                ) {
                    return None;
                }
                let rel = path.strip_prefix(&deck_dir).ok()?;
                let parts: Vec<String> = rel
                    .iter()
                    .map(|p| url_encode(&p.to_string_lossy()))
                    .collect();
                Some(json!(format!("/preview/{}", parts.join("/"))))
            };
            for (key, from) in [
                ("href", v["output"].clone()),
                ("html_href", v["html"].clone()),
                ("pdf_href", v["pdf"]["path"].clone()),
            ] {
                if let Some(h) = href(&from) {
                    v[key] = h;
                }
            }
            Json(v).into_response()
        }
        Ok(Err(e)) => err_response(e),
        Err(e) => err_response(QdError::Io(std::io::Error::other(e))),
    }
}

/// Same origin *and* the session token: required for anything that runs code.
fn authorized(app: &App, headers: &HeaderMap) -> bool {
    same_origin(headers)
        && headers.get("x-qd-token").and_then(|v| v.to_str().ok()) == Some(app.token.as_str())
}

/// 128 random bits from the OS, hex.
fn token() -> Result<String> {
    let mut b = [0u8; 16];
    std::io::Read::read_exact(&mut std::fs::File::open("/dev/urandom")?, &mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, "forbidden").into_response()
}

/// Stored figure/full-text files get URLs (`/outputs/<file>`).
fn with_urls(mut rec: Value) -> Value {
    for item in rec["items"].as_array_mut().into_iter().flatten() {
        for (key, url) in [("file", "url"), ("full", "full_url")] {
            if let Some(f) = item[key].as_str() {
                item[url] = json!(format!("/outputs/{f}"));
            }
        }
    }
    rec
}

#[derive(serde::Deserialize)]
struct RunBody {
    #[serde(default)]
    cells: Vec<String>,
    #[serde(default)]
    all: bool,
    /// Restart and run from the top through the last requested cell (FR-008b).
    #[serde(default)]
    fresh: bool,
}

/// `POST /api/run`: 202 at once; progress and outputs arrive as events.
async fn api_run(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<RunBody>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let plan = app.store.read().and_then(|d| {
        // (ref, fingerprint, code) of each cell to run, in order
        let mut picked = vec![];
        for r in &body.cells {
            let (_, b) = d.element(r)?;
            cell_code(&d, b)?;
            picked.push(b.id.clone());
        }
        let last = d.blocks().position(|b| picked.last() == Some(&b.id));
        let mut plan = vec![];
        for (i, b) in d.blocks().enumerate() {
            let wanted =
                body.all || picked.contains(&b.id) || (body.fresh && last.is_some_and(|l| i <= l));
            if b.kind != BlockKind::Cell || !wanted {
                continue;
            }
            // Run all / fresh skip what can't run here; picked cells were checked above
            if let Ok(Some(code)) = cell_code(&d, b) {
                plan.push((b.id.clone(), version_of(d.slice(b.span).as_bytes()), code));
            }
        }
        if !body.all && !body.fresh {
            // explicitly picked cells run in the order given
            plan.sort_by_key(|(id, ..)| picked.iter().position(|p| p == id));
        }
        Ok(plan)
    });
    let plan = match plan {
        Ok(p) => p,
        Err(e) => return err_response(e),
    };
    let run = app.runs.fetch_add(1, Ordering::Relaxed) + 1;
    let cells: Vec<&String> = plan.iter().map(|(id, ..)| id).collect();
    app.emit("run_started", json!({"run": run, "cells": cells}));
    let a = app.clone();
    tokio::task::spawn_blocking(move || run_cells(&a, run, plan, body.fresh));
    (StatusCode::ACCEPTED, Json(json!({"run": run}))).into_response()
}

/// The live session, started (in the deck's environment) if there is none.
fn kernel(app: &App) -> Result<Arc<Kernel>> {
    let mut k = app.kernel.lock().unwrap();
    if let Some(s) = k.as_ref() {
        return Ok(s.clone());
    }
    app.emit("kernel", json!({"state": "starting"}));
    let dir = app.dir();
    let s = crate::env::require(&dir).and_then(|env| Kernel::start(&env, &dir));
    let s = Arc::new(s.inspect_err(|_| app.emit("kernel", json!({"state": "stopped"})))?);
    *k = Some(s.clone());
    app.emit("kernel", json!({"state": "idle"}));
    Ok(s)
}

/// Run `plan` in order, storing outputs; stops at the first cell that fails
/// or is stopped (FR-010). The preview re-renders afterwards (R7).
fn run_cells(app: &App, run: u64, plan: Vec<(String, String, String)>, fresh: bool) {
    let outputs = Outputs::new(&app.store);
    let res = (|| -> Result<&'static str> {
        let k = kernel(app)?;
        if fresh {
            app.emit("kernel", json!({"state": "starting"}));
            k.restart()?;
        }
        for (cell, fp, code) in plan {
            app.emit("kernel", json!({"state": "busy"}));
            let reply = k.execute(&code, None);
            app.emit("kernel", json!({"state": "idle"}));
            let reply = reply?;
            let rec = with_urls(outputs.save(&cell, &fp, &reply)?);
            app.emit(
                "cell_output",
                json!({"run": run, "cell": cell, "counter": rec["counter"],
                "status": rec["status"], "items": rec["items"]}),
            );
            match reply["status"].as_str() {
                Some("ok") => {}
                Some("stopped") => return Ok("stopped"),
                _ => return Ok("error"),
            }
        }
        Ok("done")
    })();
    let done = match res {
        Ok(status) => json!({"run": run, "status": status}),
        Err(e) => {
            // the session is gone or never started: the next run starts a new one
            app.kernel.lock().unwrap().take();
            app.emit("kernel", json!({"state": "stopped"}));
            json!({"run": run, "status": "failed", "error": e.to_json()["error"]})
        }
    };
    app.emit("run_done", done);
    app.kick.notify_one();
}

/// `POST /api/run/interrupt`: stop the running cell (no-op when idle or starting).
async fn api_interrupt(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let k = app.kernel.try_lock().ok().and_then(|k| k.clone());
    match k.map_or(Ok(()), |k| k.interrupt()) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => err_response(e),
    }
}

/// `POST /api/run/restart`: fresh kernel state (stops a running cell first).
async fn api_restart(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let Some(k) = app.kernel.try_lock().ok().and_then(|k| k.clone()) else {
        return Json(json!({"ok": true})).into_response();
    };
    let a = app.clone();
    let res = tokio::task::spawn_blocking(move || {
        let _ = k.interrupt();
        a.emit("kernel", json!({"state": "starting"}));
        let r = k.restart();
        a.emit(
            "kernel",
            json!({"state": if r.is_ok() { "idle" } else { "stopped" }}),
        );
        r
    })
    .await;
    match res {
        Ok(Ok(())) => Json(json!({"ok": true})).into_response(),
        Ok(Err(e)) => err_response(e),
        Err(e) => err_response(QdError::Io(std::io::Error::other(e))),
    }
}

/// `GET /api/outputs`: stored outputs per cell, stale ones marked, and every
/// code cell in order with its lines (the source pane's Run buttons).
async fn api_outputs(State(app): State<Shared>) -> Response {
    match app.store.read() {
        Ok(d) => {
            let mut r = Outputs::new(&app.store).report(&d);
            let order: Vec<Value> = d
                .blocks()
                .filter(|b| b.kind == BlockKind::Cell)
                .map(|b| {
                    json!({"cell": b.id, "lines": d.lines(b.span),
                    "runnable": matches!(cell_code(&d, b), Ok(Some(_)))})
                })
                .collect();
            r["order"] = json!(order);
            for rec in r["cells"]
                .as_object_mut()
                .into_iter()
                .flat_map(|m| m.values_mut())
            {
                *rec = with_urls(rec.take());
            }
            Json(r).into_response()
        }
        Err(e) => err_response(e),
    }
}

/// `GET /outputs/<file>`: a stored figure or full text.
async fn output_file(State(app): State<Shared>, UrlPath(name): UrlPath<String>) -> Response {
    let Some(path) = Outputs::new(&app.store).file(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "text/plain; charset=utf-8",
    };
    match std::fs::read(&path) {
        Ok(b) => ([(header::CONTENT_TYPE, mime)], b).into_response(),
        Err(e) => err_response(e.into()),
    }
}

/// `GET /api/env`: as `qd env`.
async fn api_env(State(app): State<Shared>) -> Response {
    let dir = app.dir();
    match tokio::task::spawn_blocking(move || crate::env::require(&dir).map(|e| e.report())).await {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => err_response(e),
        Err(e) => err_response(QdError::Io(std::io::Error::other(e))),
    }
}

#[derive(serde::Deserialize)]
struct EnvBody {
    #[serde(default)]
    packages: Vec<String>,
}

/// `POST /api/env/init` and `/api/env/add` (token): create the environment / add packages.
async fn api_env_change(
    State(app): State<Shared>,
    UrlPath(action): UrlPath<String>,
    headers: HeaderMap,
    Json(body): Json<EnvBody>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let dir = app.dir();
    let res = tokio::task::spawn_blocking(move || match action.as_str() {
        "init" => crate::env::init(&dir, crate::env::Tool::Uv).map(|e| e.report()),
        "add" => crate::env::require(&dir).and_then(|e| e.add(&body.packages).map(|_| e.report())),
        _ => Err(QdError::NotFound(action)),
    })
    .await;
    // a session started before the change runs the old packages: start afresh next time
    app.kernel.lock().unwrap().take();
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => err_response(e),
        Err(e) => err_response(QdError::Io(std::io::Error::other(e))),
    }
}

async fn events(
    State(app): State<Shared>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let rx = app.tx.subscribe();
    let initial: Vec<Ev> = {
        let st = app.st.lock().unwrap();
        let mut v = Vec::new();
        if let Some(r) = &st.rendered {
            v.push(Ev::Rendered(r.clone()));
        }
        if let Some(e) = &st.error {
            v.push(Ev::RenderError(e.clone()));
        }
        v
    };
    let html = app.html.clone();
    let h2 = html.clone();
    let first = stream::iter(initial.into_iter().map(move |e| Ok(e.to_sse(&html))));
    let live = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(e) => return Some((e, rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
    .map(move |e| Ok(e.to_sse(&h2)));
    Sse::new(first.chain(live)).keep_alive(KeepAlive::default())
}

/// One render at a time; triggers that arrive meanwhile coalesce into one more.
async fn render_loop(app: Shared) {
    loop {
        app.kick.notified().await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let Ok(d) = app.store.read() else { continue };
        let _ = app.tx.send(Ev::Rendering);
        let outputs = Outputs::new(&app.store).report(&d);
        let copy = preview_copy(&d, &outputs["cells"]);
        let (path, out) = (app.store.path.clone(), app.out.clone());
        let res =
            tokio::task::spawn_blocking(move || render(&path, &copy, "preview", &out, true)).await;
        let ev = match res {
            Ok(Ok(_)) => {
                let mut st = app.st.lock().unwrap();
                st.rendered = Some(d.version.clone());
                st.error = None;
                Ev::Rendered(d.version)
            }
            Ok(Err(e)) => {
                let msg = match &e {
                    QdError::Render(log) => log.clone(),
                    other => other.to_string(),
                };
                app.st.lock().unwrap().error = Some(msg.clone());
                Ev::RenderError(msg)
            }
            Err(join) => Ev::RenderError(join.to_string()),
        };
        let _ = app.tx.send(ev);
    }
}

/// Watch the deck's directory (atomic renames replace the file's inode).
fn watch(app: Shared) -> Result<notify::RecommendedWatcher> {
    let deck = app.store.path.clone();
    let name = deck.file_name().map(|n| n.to_os_string());
    let rt = tokio::runtime::Handle::current();
    let a = app.clone();
    let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        if !ev
            .paths
            .iter()
            .any(|p| p.file_name().map(|n| n.to_os_string()) == name)
        {
            return;
        }
        let Ok(bytes) = std::fs::read(&a.store.path) else {
            return;
        };
        let v = version_of(&bytes);
        let mut st = a.st.lock().unwrap();
        if st.version != v {
            st.version = v.clone();
            drop(st);
            let _ = a.tx.send(Ev::Changed(v));
            let a = a.clone();
            rt.spawn(async move { a.kick.notify_one() });
        }
    })
    .map_err(|e| QdError::Io(std::io::Error::other(e)))?;
    w.watch(deck.parent().unwrap(), notify::RecursiveMode::NonRecursive)
        .map_err(|e| QdError::Io(std::io::Error::other(e)))?;
    Ok(w)
}

pub struct Session {
    pub url: String,
    pub port: u16,
}

/// Bind, then call `on_ready` (to print the URL) and serve until Ctrl-C.
pub fn serve(store: Store, port: u16, on_ready: impl FnOnce(&Session)) -> Result<()> {
    crate::render::quarto()?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let d: Deck = store.read()?;
        let out = store.cache_dir().join("preview");
        let stem = store
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let deck_dir = store.path.parent().unwrap().to_path_buf();
        let (tx, _) = broadcast::channel(64);
        let session_file = store.cache_dir().join("session.json");
        let app = Arc::new(App {
            html: format!("/preview/{stem}.qd-preview.html"),
            out: out.clone(),
            tx,
            st: Mutex::new(St {
                version: d.version.clone(),
                ..Default::default()
            }),
            kick: Notify::new(),
            token: token()?,
            kernel: Mutex::new(None),
            runs: AtomicU64::new(0),
            store,
        });
        let router = Router::new()
            .route("/", get(index))
            .route("/editor.js", get(editor_js))
            .route("/editor.css", get(editor_css))
            .route("/ui/{name}", get(ui_module))
            .route("/present/", get(present))
            .route("/api/deck", get(api_deck))
            .route("/api/source", get(api_source))
            .route("/api/slide/{r}", get(api_slide))
            .route("/api/op", post(api_op))
            .route("/api/run", post(api_run))
            .route("/api/run/interrupt", post(api_interrupt))
            .route("/api/run/restart", post(api_restart))
            .route("/api/outputs", get(api_outputs))
            .route("/outputs/{name}", get(output_file))
            .route("/api/env", get(api_env))
            .route("/api/env/{action}", post(api_env_change))
            .route("/events", get(events))
            .nest_service(
                "/preview",
                ServeDir::new(&out).fallback(ServeDir::new(&deck_dir)),
            )
            .with_state(app.clone());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        let port = listener.local_addr()?.port();
        let session = Session {
            url: format!("http://127.0.0.1:{port}/"),
            port,
        };
        std::fs::create_dir_all(app.store.cache_dir())?;
        let info = json!({"deck_path": app.store.path, "url": session.url, "port": port,
            "pid": std::process::id(), "token": app.token});
        // user-only: the token lets the CLI run code through this server
        let mut f = std::fs::OpenOptions::new();
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut f, 0o600);
        let _ = std::fs::remove_file(&session_file);
        std::io::Write::write_all(
            &mut f.write(true).create_new(true).open(&session_file)?,
            info.to_string().as_bytes(),
        )?;
        let _watcher = watch(app.clone())?;
        tokio::spawn(render_loop(app.clone()));
        app.kick.notify_one();
        on_ready(&session);
        let res = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await;
        app.kernel.lock().unwrap().take(); // shuts the kernel down
        let _ = std::fs::remove_file(&session_file);
        res.map_err(QdError::Io)
    })
}

/// URL of the running server for this deck, if it is still answering.
pub fn running_url(store: &Store) -> Result<String> {
    let text = std::fs::read_to_string(store.cache_dir().join("session.json"))
        .map_err(|_| QdError::NoServer)?;
    let v: Value = serde_json::from_str(&text).map_err(|_| QdError::NoServer)?;
    let port = v["port"].as_u64().ok_or(QdError::NoServer)? as u16;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300))
        .map_err(|_| QdError::NoServer)?;
    Ok(v["url"].as_str().ok_or(QdError::NoServer)?.to_string())
}

/// pid of the running server for this deck.
pub fn running_pid(store: &Store) -> Result<u32> {
    running_url(store)?;
    let text = std::fs::read_to_string(store.cache_dir().join("session.json"))?;
    let v: Value = serde_json::from_str(&text).map_err(|_| QdError::NoServer)?;
    v["pid"].as_u64().map(|p| p as u32).ok_or(QdError::NoServer)
}
