//! Offline check of a rendered reveal.js deck in headless Chrome with the
//! network blocked (spec 002 US1; research.md R3). Shared launcher is reused
//! by PDF printing (US3).

use crate::browser::Browser as Found;
use crate::error::{QdError, Result};
use chromiumoxide::cdp::browser_protocol::fetch::{
    ContinueRequestParams, EnableParams, EventRequestPaused, FailRequestParams, RequestPattern,
};
use chromiumoxide::cdp::browser_protocol::network::ErrorReason;
use chromiumoxide::cdp::js_protocol::runtime::{EvaluateParams, EventExceptionThrown};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Problem {
    pub code: &'static str,
    pub severity: &'static str,
    pub slide: Option<String>,
    pub message: String,
    pub hint: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Problem {
    fn error(
        code: &'static str,
        slide: Option<String>,
        message: String,
        url: Option<String>,
    ) -> Problem {
        Problem {
            code,
            severity: "error",
            slide,
            message,
            hint: hint(code),
            url,
        }
    }
}

fn hint(code: &str) -> &'static str {
    match code {
        "P_NETWORK" => {
            "the deck loads this from the internet; export it with `qd export` (bundles MathJax/plotly.js) or remove the remote resource"
        }
        "P_MISSING_FILE" => {
            "ship the file beside the HTML at this relative path, or fix the path in the deck"
        }
        "P_PAGE_ERROR" => "a script on the page failed; check {=html} blocks and embedded apps",
        "P_RAW_TEX" => {
            "math is not typeset offline; use MathJax via `qd export` (KaTeX is never embedded)"
        }
        "P_OFFLINE_INCOMPATIBLE" => "this feature needs a server; remove it for an offline deck",
        "P_NOTES_VISIBLE" => {
            "`show-notes` in the YAML shows speaker notes to the audience; remove it (qd pdf --notes prints notes)"
        }
        _ => "",
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CheckReport {
    /// Marks this JSON as an offline check report (the CLI exits 1 when `ok` is false).
    pub kind: &'static str,
    pub ok: bool,
    pub html: PathBuf,
    pub slides: usize,
    pub steps: usize,
    pub problems: Vec<Problem>,
}

/// Requests that load nothing from the network by design (FR-003).
fn allowed(url: &str) -> bool {
    ["file:", "data:", "blob:", "about:"]
        .iter()
        .any(|s| url.starts_with(s))
}

fn other(e: impl std::fmt::Display) -> QdError {
    QdError::Io(std::io::Error::other(e.to_string()))
}

/// (what happened, slide anchor shown at the time), collected by event tasks.
type Seen = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A headless page that cannot reach the network; records what it tried.
pub(crate) struct OfflinePage {
    pub browser: Browser,
    pub page: Page,
    pub blocked: Seen,
    pub errors: Seen,
    /// Slide anchor being shown, for attributing requests/errors.
    pub current: Arc<Mutex<Option<String>>>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    profile: PathBuf,
}

impl OfflinePage {
    pub async fn open(chrome: &Found) -> Result<OfflinePage> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let profile =
            std::env::temp_dir().join(format!("qd-chrome-{}-{nanos}", std::process::id()));
        let config = BrowserConfig::builder()
            .chrome_executable(&chrome.path)
            .user_data_dir(&profile)
            .window_size(1280, 800)
            .build()
            .map_err(other)?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| QdError::NoBrowser(format!("{}: {e}", chrome.path.display())))?;
        let mut tasks = vec![tokio::spawn(async move {
            while handler.next().await.is_some() {}
        })];
        let page = browser.new_page("about:blank").await.map_err(other)?;
        let blocked = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let current = Arc::new(Mutex::new(None));

        page.execute(
            EnableParams::builder()
                .pattern(RequestPattern::builder().url_pattern("*").build())
                .build(),
        )
        .await
        .map_err(other)?;
        let mut paused = page
            .event_listener::<EventRequestPaused>()
            .await
            .map_err(other)?;
        let (p, b, c) = (page.clone(), blocked.clone(), current.clone());
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = paused.next().await {
                let url = ev.request.url.clone();
                if allowed(&url) {
                    let _ = p
                        .execute(ContinueRequestParams::new(ev.request_id.clone()))
                        .await;
                } else {
                    b.lock().unwrap().push((url, c.lock().unwrap().clone()));
                    let _ = p
                        .execute(FailRequestParams::new(
                            ev.request_id.clone(),
                            ErrorReason::InternetDisconnected,
                        ))
                        .await;
                }
            }
        }));
        let mut thrown = page
            .event_listener::<EventExceptionThrown>()
            .await
            .map_err(other)?;
        let (e, c) = (errors.clone(), current.clone());
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = thrown.next().await {
                let d = &ev.exception_details;
                let msg = d
                    .exception
                    .as_ref()
                    .and_then(|x| x.description.clone())
                    .unwrap_or_else(|| d.text.clone());
                e.lock().unwrap().push((
                    msg.lines().next().unwrap_or("").to_string(),
                    c.lock().unwrap().clone(),
                ));
            }
        }));
        Ok(OfflinePage {
            browser,
            page,
            blocked,
            errors,
            current,
            tasks,
            profile,
        })
    }

    /// Navigate and wait until reveal.js is ready and MathJax (if any) has typeset.
    pub async fn load(&self, url: &str) -> Result<()> {
        self.page.goto(url).await.map_err(other)?;
        self.eval(
            r#"new Promise(res => { const go = async () => {
                 if (window.MathJax?.startup?.promise) { try { await MathJax.startup.promise; } catch (e) {} }
                 res(true);
               };
               if (!window.Reveal) return res(false);
               Reveal.isReady() ? go() : Reveal.on('ready', go); })"#,
        )
        .await?;
        Ok(())
    }

    pub async fn eval(&self, js: &str) -> Result<Value> {
        let params = EvaluateParams::builder()
            .expression(js)
            .await_promise(true)
            .return_by_value(true)
            .build()
            .map_err(other)?;
        let r = self.page.evaluate(params).await.map_err(other)?;
        Ok(r.value().cloned().unwrap_or(Value::Null))
    }

    pub async fn close(mut self) {
        let _ = self.browser.close().await;
        let _ = self.browser.wait().await;
        for t in self.tasks.drain(..) {
            t.abort();
        }
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

pub(crate) fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

pub(crate) fn file_url(html: &Path) -> Result<String> {
    let abs = std::fs::canonicalize(html)
        .map_err(|e| QdError::NotFound(format!("{}: {e}", html.display())))?;
    let mut s = String::from("file://");
    for b in abs.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                s.push(b as char)
            }
            _ => s.push_str(&format!("%{b:02X}")),
        }
    }
    Ok(s)
}

/// Per-slide info gathered in the page.
const SLIDES_JS: &str = r#"Reveal.getSlides().map(s => {
  const i = Reveal.getIndices(s);
  const frags = new Set([...s.querySelectorAll('.fragment')].map(f => f.getAttribute('data-fragment-index')));
  const bg = ['data-background-video', 'data-background-iframe', 'data-background-image']
    .map(a => s.getAttribute(a)).filter(Boolean);
  return { id: s.id || null, h: i.h, v: i.v, fragments: frags.size, backgrounds: bg };
})"#;

/// After showing a slide: is any math left untypeset?
const RAW_TEX_JS: &str = r#"(() => {
  const s = Reveal.getCurrentSlide();
  return [...s.querySelectorAll('.math')].some(m => !m.querySelector('mjx-container, .katex, svg'));
})()"#;

const CONFIG_JS: &str = r#"(() => { const c = Reveal.getConfig();
  return { showNotes: !!c.showNotes, multiplex: !!c.multiplex, parallax: c.parallaxBackgroundImage || null }; })()"#;

/// Is `p` a local path (not a URL or data URI)?
fn local(p: &str) -> bool {
    !(p.contains("://") || p.starts_with("data:") || p.starts_with('#') || p.starts_with("//"))
}

/// Run the offline check on a rendered deck.
pub fn check(html: &Path, chrome: &Found) -> Result<CheckReport> {
    let url = file_url(html)?;
    let dir = html.parent().unwrap_or(Path::new(".")).to_path_buf();
    runtime()?.block_on(async {
        let op = OfflinePage::open(chrome).await?;
        let res = check_page(&op, &url, &dir).await;
        op.close().await;
        let (slides, steps, mut problems) = res?;
        problems.sort_by(|a, b| {
            (a.slide.is_some(), &a.slide, a.code).cmp(&(b.slide.is_some(), &b.slide, b.code))
        });
        let ok = !problems.iter().any(|p| p.severity == "error");
        Ok(CheckReport {
            kind: "offline_check",
            ok,
            html: html.to_path_buf(),
            slides,
            steps,
            problems,
        })
    })
}

async fn check_page(
    op: &OfflinePage,
    url: &str,
    dir: &Path,
) -> Result<(usize, usize, Vec<Problem>)> {
    op.load(url).await?;
    if op.eval("!!window.Reveal").await? != Value::Bool(true) {
        return Err(QdError::InvalidArg(format!(
            "{url} is not a reveal.js presentation"
        )));
    }
    let mut problems = Vec::new();
    let slides: Vec<Value> = serde_json::from_value(op.eval(SLIDES_JS).await?).unwrap_or_default();
    let mut steps = 0;
    for s in &slides {
        let anchor = s["id"].as_str().map(String::from);
        *op.current.lock().unwrap() = anchor.clone();
        let (h, v) = (s["h"].as_u64().unwrap_or(0), s["v"].as_u64().unwrap_or(0));
        let frags = s["fragments"].as_u64().unwrap_or(0);
        steps += 1 + frags as usize;
        op.eval(&format!("Reveal.slide({h}, {v}, -1)")).await?;
        for f in 0..frags {
            op.eval(&format!("Reveal.slide({h}, {v}, {f})")).await?;
        }
        tokio::time::sleep(Duration::from_millis(120)).await; // let lazy media/iframes request
        if op.eval(RAW_TEX_JS).await? == Value::Bool(true) {
            problems.push(Problem::error(
                "P_RAW_TEX",
                anchor.clone(),
                "math on this slide is shown as raw TeX".into(),
                None,
            ));
        }
        for bg in s["backgrounds"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if local(bg) && !dir.join(bg).exists() {
                problems.push(Problem::error(
                    "P_MISSING_FILE",
                    anchor.clone(),
                    format!("background file '{bg}' is missing"),
                    Some(bg.into()),
                ));
            }
        }
    }
    *op.current.lock().unwrap() = None;
    let cfg = op.eval(CONFIG_JS).await?;
    if cfg["showNotes"] == true {
        problems.push(Problem {
            code: "P_NOTES_VISIBLE",
            severity: "warning",
            slide: None,
            message: "speaker notes are shown to the audience (show-notes)".into(),
            hint: hint("P_NOTES_VISIBLE"),
            url: None,
        });
    }
    if cfg["multiplex"] == true {
        problems.push(Problem::error(
            "P_OFFLINE_INCOMPATIBLE",
            None,
            "multiplex needs a server".into(),
            None,
        ));
    }
    if let Some(p) = cfg["parallax"]
        .as_str()
        .filter(|p| local(p) && !dir.join(p).exists())
    {
        problems.push(Problem::error(
            "P_MISSING_FILE",
            None,
            format!("parallax background '{p}' is missing"),
            Some(p.into()),
        ));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut seen = std::collections::HashSet::new();
    for (u, slide) in op.blocked.lock().unwrap().iter() {
        if seen.insert((u.clone(), slide.clone())) {
            problems.push(Problem::error(
                "P_NETWORK",
                slide.clone(),
                format!("tried to load {u}"),
                Some(u.clone()),
            ));
        }
    }
    for (m, slide) in op.errors.lock().unwrap().iter() {
        problems.push(Problem::error(
            "P_PAGE_ERROR",
            slide.clone(),
            m.clone(),
            None,
        ));
    }
    Ok((slides.len(), steps, problems))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_pdf_pages_not_page_trees() {
        let pdf = b"<< /Type /Pages /Count 2 >> << /Type /Page /Parent 1 0 R >> << /Type/Page>> << /Type /Page\n>>";
        assert_eq!(count_pdf_pages(pdf), 3);
        assert_eq!(count_pdf_pages(b"%PDF-1.4"), 0);
    }

    #[test]
    fn allowlist() {
        assert!(allowed("about:blank?config=TeX-AMS_HTML-full"));
        assert!(
            allowed("file:///x/y.js")
                && allowed("data:image/png;base64,AA")
                && allowed("blob:file:///1")
        );
        assert!(!allowed(
            "https://cdn.jsdelivr.net/npm/mathjax@2.7.9/MathJax.js"
        ));
        assert!(!allowed("http://example.com"));
    }

    #[test]
    fn local_paths_and_urls() {
        assert!(local("media/clip.mp4") && local("../a b.png"));
        assert!(
            !local("https://x/y.mp4")
                && !local("data:video/mp4;base64,AA")
                && !local("//cdn/x.png")
        );
    }

    #[test]
    fn file_urls_are_escaped() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a deck#1.html");
        std::fs::write(&f, "").unwrap();
        let u = file_url(&f).unwrap();
        assert!(
            u.starts_with("file:///") && u.ends_with("a%20deck%231.html"),
            "{u}"
        );
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PdfResult {
    pub path: PathBuf,
    pub pages: usize,
    pub per_slide: bool,
    pub notes: bool,
}

/// Number of pages in a PDF (page objects, not the `/Pages` tree nodes).
pub fn count_pdf_pages(pdf: &[u8]) -> usize {
    let mut n = 0;
    for pat in [&b"/Type /Page"[..], &b"/Type/Page"[..]] {
        n += pdf
            .windows(pat.len() + 1)
            .filter(|w| w.starts_with(pat) && !w[pat.len()].is_ascii_alphanumeric())
            .count();
    }
    n
}

/// Print a deck to PDF through reveal.js's own print mode (research R4).
pub fn print_pdf(
    html: &Path,
    out: &Path,
    notes: bool,
    per_slide: bool,
    chrome: &Found,
) -> Result<PdfResult> {
    use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;
    // URL options override the deck's config (reveal.js): fragments as pages unless
    // per-slide; notes only when asked (even if the YAML sets show-notes)
    let url = format!(
        "{}?print-pdf&pdfSeparateFragments={}&showNotes={}",
        file_url(html)?,
        !per_slide,
        if notes { "separate-page" } else { "false" }
    );
    let bytes = runtime()?.block_on(async {
        let op = OfflinePage::open(chrome).await?;
        let res = async {
            op.load(&url).await?;
            tokio::time::sleep(Duration::from_millis(300)).await; // print layout settles after MathJax
            let params = PrintToPdfParams::builder()
                .prefer_css_page_size(true)
                .print_background(true)
                .build();
            op.page.pdf(params).await.map_err(other)
        }
        .await;
        op.close().await;
        res
    })?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, &bytes)?;
    Ok(PdfResult {
        path: out.to_path_buf(),
        pages: count_pdf_pages(&bytes),
        per_slide,
        notes,
    })
}
