//! `qd serve` (contracts/http.md): localhost-only server with the editor
//! shell, the latest good Quarto render, a read API, and SSE live updates.

use crate::cli::{run_op, show_json, slides_json};
use crate::deck::{BlockKind, Deck, version_of};
use crate::error::{QdError, Result};
use crate::focus::Snapshot;
use crate::kernel::{Session as Kernel, cell_code};
use crate::outputs::Outputs;
use crate::render::{preview_copy, render};
use crate::store::Store;
use axum::extract::{DefaultBodyLimit, Path as UrlPath, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use notify::Watcher;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, broadcast};
use tower_http::services::ServeDir;

const INDEX: &str = include_str!("ui/index.html");
const EDITOR_JS: &str = include_str!("ui/editor.js");
const EDITOR_CSS: &str = include_str!("ui/editor.css");
/// The editor's one action list (spec 007): menus and keys read it, as `/ui/action-list.js`.
const ACTION_LIST_JS: &str = concat!("export default ", include_str!("ui/actions.json"), ";\n");
/// Editor ES modules imported by `editor.js`, served at `/ui/<name>`.
const UI_MODULES: &[(&str, &str)] = &[
    ("inline.js", include_str!("ui/inline.js")),
    ("toolbar.js", include_str!("ui/toolbar.js")),
    ("source.js", include_str!("ui/source.js")),
    ("actions.js", include_str!("ui/actions.js")),
    ("menu.js", include_str!("ui/menu.js")),
    ("pane.js", include_str!("ui/pane.js")),
    ("run.js", include_str!("ui/run.js")),
    ("focus.js", include_str!("ui/focus.js")),
    ("keys.js", include_str!("ui/keys.js")),
    ("menubar.js", include_str!("ui/menubar.js")),
    ("list.js", include_str!("ui/list.js")),
    ("settings.js", include_str!("ui/settings.js")),
    ("sheet.js", include_str!("ui/sheet.js")),
    ("action-list.js", ACTION_LIST_JS),
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
    /// `POST /api/shutdown` (`qd stop`).
    stop: Notify,
    /// Shared focus (spec 005).
    focus: Mutex<Tabs>,
    /// Spec 006: the VS Code buffer's text as the extension last knows it
    /// (None = no buffer attached); overlay changes beyond it go to VS Code.
    buffer: Mutex<Option<String>>,
    /// Held while Quarto renders; `true` once shutting down (no new renders).
    /// Shutdown waits for a render in progress: Quarto left running would
    /// clean up the deck folder under the next server's render.
    rendering: Mutex<bool>,
    /// The person's settings file (spec 007), shared by every qd.
    settings: PathBuf,
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

    fn state(&self, s: &str) {
        self.emit("kernel", json!({ "state": s }));
    }
}

type Shared = Arc<App>;

fn err_response(e: QdError) -> Response {
    let status = match e {
        QdError::NotFound(_) => StatusCode::NOT_FOUND,
        QdError::Ambiguous(_)
        | QdError::InvalidArg(_)
        | QdError::InvalidSetting(_)
        | QdError::Usage(_) => StatusCode::BAD_REQUEST,
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
/// The editor page. It never holds the run token: that comes in the link's
/// fragment (`#t=…`, from `qd serve` / `qd url`), which browsers don't send.
async fn index() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        INDEX,
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
        Ok(d) => {
            // the editor's top bar and tab title (spec 008 US6)
            let mut v = slides_json(&d);
            v["title"] = json!(d.meta.get("title").and_then(|t| t.as_str()));
            v["file"] = json!(app.store.path.file_name().map(|f| f.to_string_lossy()));
            Json(v).into_response()
        }
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

/// Every route answers only to a loopback Host: a DNS-rebound page on
/// another site can read nothing (source, outputs, the editor's token).
async fn local_only(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok());
    if host.is_some_and(local_host) {
        next.run(req).await
    } else {
        forbidden()
    }
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
        return forbidden();
    }
    let store = app.store.clone();
    let res = tokio::task::spawn_blocking(move || {
        run_op(&store, &body.op, &body.args, body.expect.as_deref())
    })
    .await;
    match res {
        Ok(Ok(mut v)) => {
            // outputs inside the deck folder can be opened through /preview/'s fallback
            let deck_dir = app.dir();
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
    /// Answer when the run ends, with its outputs (`qd run`), instead of 202 at once.
    #[serde(default)]
    wait: bool,
    /// Seconds before a cell is interrupted.
    timeout: Option<u64>,
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
    let plan = app
        .store
        .read()
        .and_then(|d| crate::kernel::plan(&d, &body.cells, body.all, body.fresh));
    let plan = match plan {
        Ok(p) => p,
        Err(e) => return err_response(e),
    };
    let run = app.runs.fetch_add(1, Ordering::Relaxed) + 1;
    let cells: Vec<&String> = plan.iter().map(|(id, ..)| id).collect();
    app.emit("run_started", json!({"run": run, "cells": cells}));
    let a = app.clone();
    let timeout = body.timeout.map(Duration::from_secs);
    let go = move || run_cells(&a, run, plan, body.fresh, timeout);
    if body.wait {
        return blocking(move || Ok(go())).await;
    }
    tokio::task::spawn_blocking(go);
    (StatusCode::ACCEPTED, Json(json!({"run": run}))).into_response()
}

/// The live session, started (in the deck's environment) if there is none.
fn kernel(app: &App) -> Result<Arc<Kernel>> {
    let mut k = app.kernel.lock().unwrap();
    if let Some(s) = k.as_ref() {
        return Ok(s.clone());
    }
    app.state("starting");
    let dir = app.dir();
    let s = crate::env::require(&dir).and_then(|env| Kernel::start(&env, &dir));
    let s = Arc::new(s.inspect_err(|_| app.state("stopped"))?);
    *k = Some(s.clone());
    app.state("idle");
    Ok(s)
}

/// Run `plan` (`kernel::run_plan`) in the live session, with events for the
/// editor; the preview re-renders afterwards (R7). Returns `run_done`'s
/// `{run, status, error?}` plus `runs`: each cell's entry as `qd run` reports it.
fn run_cells(
    app: &App,
    run: u64,
    plan: Vec<crate::kernel::Planned>,
    fresh: bool,
    timeout: Option<Duration>,
) -> Value {
    let outputs = Outputs::new(&app.store);
    let mut runs = vec![];
    let go = || -> Result<&'static str> {
        if plan.is_empty() {
            return Ok("done"); // nothing to run needs no kernel (nor an environment)
        }
        let k = kernel(app)?;
        if fresh {
            app.state("starting");
            k.restart()?;
        }
        crate::kernel::run_plan(&k, &outputs, plan, timeout, |cell, rec| {
            let Some(rec) = rec else {
                return app.state("busy");
            };
            app.state("idle");
            runs.push(outputs.entry(cell, rec));
            let rec = with_urls(rec.clone());
            app.emit(
                "cell_output",
                json!({"run": run, "cell": cell, "counter": rec["counter"],
                "status": rec["status"], "items": rec["items"]}),
            );
        })
    };
    let mut done = match go() {
        Ok(status) => json!({"run": run, "status": status}),
        Err(e) => {
            // the session is gone or never started: the next run starts a new one
            app.kernel.lock().unwrap().take();
            app.state("stopped");
            json!({"run": run, "status": "failed", "error": e.to_json()["error"]})
        }
    };
    app.emit("run_done", done.clone());
    app.kick.notify_one();
    done["runs"] = json!(runs);
    done
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
    blocking(move || {
        let _ = k.interrupt();
        a.state("starting");
        let r = k.restart();
        a.state(if r.is_ok() { "idle" } else { "stopped" });
        r.map(|_| json!({"ok": true}))
    })
    .await
}

/// Run `f` off the async runtime; its JSON or error as the response.
async fn blocking(f: impl FnOnce() -> Result<Value> + Send + 'static) -> Response {
    match tokio::task::spawn_blocking(f).await {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => err_response(e),
        Err(e) => err_response(QdError::Io(std::io::Error::other(e))),
    }
}

/// `GET /api/images`: the images beside the deck (spec 008 US5).
async fn api_images(State(app): State<Shared>) -> Response {
    let dir = app.dir();
    blocking(move || crate::images::list(&dir)).await
}

/// `POST /api/images?name=<file name>` (token): the body's bytes as a new file
/// under `images/` (checked type and bytes, a free name). The JSON writes' CORS
/// defence holds here too: neither octet-stream nor the token header can be sent
/// cross-origin without a preflight, which qd never grants.
async fn api_image_add(
    State(app): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let dir = app.dir();
    let name = q.get("name").cloned().unwrap_or_default();
    blocking(move || crate::images::add(&dir, &name, &body).map(|p| json!({ "path": p }))).await
}

/// `POST /api/shutdown` (token): stop serving, as Ctrl-C does (`qd stop`).
async fn api_shutdown(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    app.stop.notify_one();
    Json(json!({"ok": true})).into_response()
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
async fn output_file(
    State(app): State<Shared>,
    UrlPath(name): UrlPath<String>,
    req: axum::extract::Request,
) -> Response {
    let Some(path) = Outputs::new(&app.store).file(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match tower_http::services::ServeFile::new(path)
        .try_call(req)
        .await
    {
        Ok(r) => r.into_response(),
        Err(e) => err_response(e.into()),
    }
}

/// `GET /api/env`: as `qd env`.
async fn api_env(State(app): State<Shared>) -> Response {
    let dir = app.dir();
    blocking(move || crate::env::require(&dir).map(|e| e.report())).await
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
    let res = blocking(move || match action.as_str() {
        "init" => crate::env::init(&dir, crate::env::Tool::Uv).map(|e| e.report()),
        "add" => crate::env::require(&dir).and_then(|e| e.add(&body.packages).map(|_| e.report())),
        _ => Err(QdError::NotFound(action)),
    })
    .await;
    // a session started before the change runs the old packages: start afresh next time
    app.kernel.lock().unwrap().take();
    res
}

/// Per open editor tab: how many event streams it has open, and its last focus report.
type Tabs = HashMap<String, (usize, Option<(Snapshot, Instant)>)>;

/// The most recently reporting open tab: (id, its focus, when it arrived).
fn latest(t: &Tabs) -> Option<(String, Snapshot, Instant)> {
    t.iter()
        .filter_map(|(tab, (_, f))| f.clone().map(|(s, at)| (tab.clone(), s, at)))
        .max_by_key(|(.., at)| *at)
}

/// A tab counts as open while its event stream lasts.
struct TabGuard(Shared, String);

impl Drop for TabGuard {
    fn drop(&mut self) {
        let mut t = self.0.focus.lock().unwrap();
        let n = &mut t.get_mut(&self.1).unwrap().0;
        *n -= 1;
        if *n == 0 {
            t.remove(&self.1);
        }
    }
}

/// `POST /api/focus` (token): an editor tab's focus snapshot (data-model.md).
async fn api_focus_report(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(snap): Json<Snapshot>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let snap = match snap.clamp() {
        Ok(s) => s,
        Err(e) => return err_response(e),
    };
    // a tab without an open event stream doesn't count (FR-006)
    if let Some(tab) = app.focus.lock().unwrap().get_mut(&snap.tab) {
        tab.1 = Some((snap, Instant::now()));
    }
    StatusCode::NO_CONTENT.into_response()
}

/// `GET /api/focus` (token): `qd focus` — the most recently reporting open tab's focus.
async fn api_focus(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let latest = {
        let t = app.focus.lock().unwrap();
        if t.is_empty() {
            return Json(json!({"open": false})).into_response();
        }
        latest(&t)
    };
    let Some((_, snap, at)) = latest else {
        return Json(json!({"open": true, "age": null,
            "hint": "open the editor with the link from `qd serve` / `qd url`"}))
        .into_response();
    };
    match app.store.read() {
        Ok(d) => Json(snap.report(&d, at.elapsed().as_secs_f64())).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(serde::Deserialize)]
struct BufferBody {
    #[serde(default)]
    base: String,
    text: String,
}

/// `POST /api/buffer/open` (token): a VS Code buffer attaches; from now on
/// it is the deck for every qd read and write (spec 006).
async fn api_buffer_open(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(b): Json<BufferBody>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    *app.buffer.lock().unwrap() = Some(b.text.clone());
    match app.store.attach(&b.text) {
        Ok(v) => Json(json!({ "version": v })).into_response(),
        Err(e) => err_response(e),
    }
}

/// `POST /api/buffer` (token): the person typed in VS Code. Written iff no qd
/// edit landed since `base`; else 409 with the buffer as qd has it, to merge.
async fn api_buffer(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(b): Json<BufferBody>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    // known first, so the watcher doesn't echo the person's own typing back
    let mut known = app.buffer.lock().unwrap();
    let prev = known.replace(b.text.clone());
    match app.store.set_buffer(&b.base, &b.text) {
        Ok(v) => Json(json!({ "version": v })).into_response(),
        Err(e) => {
            *known = prev;
            match (e, app.store.read()) {
                (QdError::Conflict(_), Ok(d)) => (
                    StatusCode::CONFLICT,
                    Json(json!({"version": d.version, "text": d.text})),
                )
                    .into_response(),
                (e, _) => err_response(e),
            }
        }
    }
}

/// `POST /api/buffer/close` (token): the VS Code panel closed; back to the file.
async fn api_buffer_close(State(app): State<Shared>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    *app.buffer.lock().unwrap() = None;
    app.store.detach();
    StatusCode::NO_CONTENT.into_response()
}

/// `GET /api/settings` (spec 007): values, the window's definitions, notices.
async fn api_settings(State(app): State<Shared>) -> Response {
    Json(crate::settings::report(&app.settings)).into_response()
}

/// `POST /api/settings` (token): `{key: value, …}` or `{"reset": true}`. Open
/// editors hear of it from the file watch (as of a change by `qd settings`).
async fn api_settings_set(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Map<String, Value>>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let r = if body.get("reset") == Some(&Value::Bool(true)) {
        crate::settings::reset(&app.settings)
    } else {
        crate::settings::set(&app.settings, &body)
    };
    match r {
        Ok(values) => Json(json!({ "values": values })).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(serde::Deserialize)]
struct PointBody {
    #[serde(rename = "ref")]
    target: String,
}

/// `POST /api/focus/set` (token): point the most recently used tab at a slide
/// or element (US2). It goes there, or — while the person types — offers to.
async fn api_focus_set(
    State(app): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<PointBody>,
) -> Response {
    if !authorized(&app, &headers) {
        return forbidden();
    }
    let (anchor, target) = match app
        .store
        .read()
        .and_then(|d| crate::focus::resolve(&d, &body.target))
    {
        Ok(x) => x,
        Err(e) => return err_response(e),
    };
    let latest = latest(&app.focus.lock().unwrap());
    let result = match latest {
        None => "no_editor",
        Some((tab, snap, _)) => {
            app.emit(
                "point",
                json!({"tab": tab, "anchor": anchor, "ref": target}),
            );
            // as of the tab's last report; the page decides again when it gets it
            if snap.editing { "deferred" } else { "moved" }
        }
    };
    Json(json!({"ok": true, "result": result, "ref": target})).into_response()
}

async fn events(
    State(app): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let guard = q.get("tab").map(|tab| {
        app.focus.lock().unwrap().entry(tab.clone()).or_default().0 += 1;
        TabGuard(app.clone(), tab.clone())
    });
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
    .map(move |e| {
        let _tab = &guard; // dropped with the stream: the tab is gone
        Ok(e.to_sse(&h2))
    });
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
        let (path, out, a) = (app.store.path.clone(), app.out.clone(), app.clone());
        let res = tokio::task::spawn_blocking(move || {
            let closing = a.rendering.lock().unwrap();
            if *closing {
                return Err(QdError::NoServer);
            }
            render(&path, &copy, "preview", &out, true)
        })
        .await;
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
    let names = [
        deck.file_name().map(|n| n.to_os_string()),
        app.store.overlay().file_name().map(|n| n.to_os_string()),
    ];
    let rt = tokio::runtime::Handle::current();
    let a = app.clone();
    let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        // the person's settings changed (here, in another qd, or by `qd settings`)
        if ev.paths.iter().any(|p| p == &a.settings) {
            a.emit(
                "settings",
                json!({ "values": crate::settings::load(&a.settings).0 }),
            );
            return;
        }
        if !ev
            .paths
            .iter()
            .any(|p| names.contains(&p.file_name().map(|n| n.to_os_string())))
        {
            return;
        }
        let d = {
            // read under the lock: a read from before the person's typing was
            // stored would be compared with it and sent back as undoing it
            let mut known = a.buffer.lock().unwrap();
            // the deck as qd sees it: the VS Code buffer overlay when attached
            let Ok(d) = a.store.read() else { return };
            // a qd edit to the attached buffer (panel op, CLI, undo): tell VS Code
            if let Some(old) = known
                .as_ref()
                .filter(|old| a.store.attached() && **old != d.text)
            {
                let edits = crate::ops::line_edits(old, &d.text);
                let base = version_of(old.as_bytes());
                a.emit(
                    "buffer_edit",
                    json!({"base": base, "version": d.version, "edits": edits}),
                );
                *known = Some(d.text.clone());
            }
            d
        };
        let v = d.version;
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
    w.watch(app.store.cache_dir(), notify::RecursiveMode::NonRecursive)
        .map_err(|e| QdError::Io(std::io::Error::other(e)))?;
    w.watch(
        app.settings.parent().unwrap(),
        notify::RecursiveMode::NonRecursive,
    )
    .map_err(|e| QdError::Io(std::io::Error::other(e)))?;
    Ok(w)
}

pub struct Session {
    pub url: String,
    pub port: u16,
    /// The link to open: `url` plus the run token in its fragment.
    pub editor: String,
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
            stop: Notify::new(),
            focus: Mutex::default(),
            buffer: Mutex::default(),
            rendering: Mutex::new(false),
            settings: {
                // canonical, as file events report it (macOS: /var → /private/var)
                let p = crate::settings::path();
                let dir = p.parent().unwrap();
                std::fs::create_dir_all(dir)?;
                dir.canonicalize()?.join("settings.json")
            },
            store,
        });
        // an image upload: the images module refuses over 20 MB itself
        let limit = DefaultBodyLimit::max(crate::images::MAX_BYTES + 1);
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
            .route("/api/shutdown", post(api_shutdown))
            .route("/api/outputs", get(api_outputs))
            .route("/outputs/{name}", get(output_file))
            .route("/api/env", get(api_env))
            .route("/api/env/{action}", post(api_env_change))
            .route("/events", get(events))
            .route("/api/focus", get(api_focus).post(api_focus_report))
            .route("/api/focus/set", post(api_focus_set))
            .route("/api/buffer/open", post(api_buffer_open))
            .route("/api/buffer", post(api_buffer))
            .route("/api/buffer/close", post(api_buffer_close))
            .route("/api/settings", get(api_settings).post(api_settings_set))
            .route(
                "/api/images",
                get(api_images).post(api_image_add).layer(limit),
            )
            .nest_service(
                "/preview",
                ServeDir::new(&out).fallback(ServeDir::new(&deck_dir)),
            )
            .layer(axum::middleware::from_fn(local_only))
            .with_state(app.clone());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        let port = listener.local_addr()?.port();
        let session = Session {
            url: format!("http://127.0.0.1:{port}/"),
            port,
            editor: format!("http://127.0.0.1:{port}/#t={}", app.token),
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
        let stopping = Arc::new(Notify::new());
        let server = axum::serve(listener, router).with_graceful_shutdown({
            let (app, stopping) = (app.clone(), stopping.clone());
            async move {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = app.stop.notified() => {}
                }
                stopping.notify_one();
            }
        });
        // an open editor's event stream never ends by itself: don't wait on it
        let grace = async {
            stopping.notified().await;
            tokio::time::sleep(Duration::from_secs(2)).await;
        };
        let res = tokio::select! {
            r = server => r,
            _ = grace => Ok(()),
        };
        *app.rendering.lock().unwrap_or_else(|e| e.into_inner()) = true; // waits for a render in progress
        app.store.detach(); // the file is whatever VS Code last saved
        app.kernel.lock().unwrap().take(); // shuts the kernel down
        let _ = std::fs::remove_file(&session_file);
        res.map_err(QdError::Io)
    })
}

/// The session file of the running server for this deck (`url`, `port`,
/// `pid`, `token`), if it is still answering.
pub fn running(store: &Store) -> Result<Value> {
    let text = std::fs::read_to_string(store.cache_dir().join("session.json"))
        .map_err(|_| QdError::NoServer)?;
    let v: Value = serde_json::from_str(&text).map_err(|_| QdError::NoServer)?;
    let port = v["port"].as_u64().ok_or(QdError::NoServer)? as u16;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300))
        .map_err(|_| QdError::NoServer)?;
    Ok(v)
}
