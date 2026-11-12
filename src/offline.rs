//! Offline check of a rendered reveal.js deck in headless Chrome with the
//! network blocked (spec 002 US1; research.md R3). Shared launcher is reused
//! by PDF printing (US3).

use crate::browser::Browser as Found;
use crate::error::{QdError, Result};
use chromiumoxide::cdp::browser_protocol::fetch::{
    ContinueRequestParams, EnableParams, EventRequestPaused, FailRequestParams, RequestPattern,
};
use chromiumoxide::cdp::browser_protocol::network::ErrorReason;
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::cdp::browser_protocol::target::GetTargetsParams;
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
    pub(crate) fn error(
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
        "P_OVERFLOW" => {
            "this element sticks out of the slide (cut off or over the edge when presented): move or shrink it"
        }
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
    /// Where the file's bytes go (spec 010 FR-008).
    pub size: Size,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Size {
    pub bytes: usize,
    pub largest: Vec<crate::inline::Part>,
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
    pub browser: Arc<Browser>,
    pub page: Page,
    pub blocked: Seen,
    pub errors: Seen,
    /// `file:` URLs of files that do not exist.
    pub missing: Seen,
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
        let config = || {
            BrowserConfig::builder()
                .chrome_executable(&chrome.path)
                .user_data_dir(&profile)
                // the size check_overflow.py measures at (not chromiumoxide's 800x600):
                // a deck sized from the viewport (a full-width footer) fits as on a 16:9 screen
                .window_size(1600, 900)
                .viewport(chromiumoxide::handler::viewport::Viewport {
                    width: 1600,
                    height: 900,
                    ..Default::default()
                })
                // the 20 s default is too short on a loaded machine (CI runs Quarto alongside)
                .launch_timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(other)
        };
        // one retry: a cold start can still miss the timeout
        let launched = match Browser::launch(config()?).await {
            Ok(b) => Ok(b),
            Err(_) => Browser::launch(config()?).await,
        };
        let (browser, mut handler) =
            launched.map_err(|e| QdError::NoBrowser(format!("{}: {e}", chrome.path.display())))?;
        let mut tasks = vec![tokio::spawn(async move {
            while handler.next().await.is_some() {}
        })];
        let browser = Arc::new(browser);
        let page = browser.new_page("about:blank").await.map_err(other)?;
        let blocked = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let missing: Seen = Arc::new(Mutex::new(Vec::new()));
        let current = Arc::new(Mutex::new(None));

        // at the browser level: an app's iframe runs in its own process (target),
        // which a page-level interception never sees
        browser
            .execute(
                EnableParams::builder()
                    .pattern(RequestPattern::builder().url_pattern("*").build())
                    .build(),
            )
            .await
            .map_err(other)?;
        let mut paused = browser
            .event_listener::<EventRequestPaused>()
            .await
            .map_err(other)?;
        let (p, pg, b, m, c) = (
            browser.clone(),
            page.clone(),
            blocked.clone(),
            missing.clone(),
            current.clone(),
        );
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = paused.next().await {
                let url = ev.request.url.clone();
                if allowed(&url) {
                    if file_path(&url).is_some_and(|f| !f.exists()) {
                        m.lock().unwrap().push((url, c.lock().unwrap().clone()));
                    }
                    let _ = p
                        .execute(ContinueRequestParams::new(ev.request_id.clone()))
                        .await;
                } else {
                    // a frame of the deck, or an app's own iframe target under one;
                    // Chrome's own (headless UI) requests are blocked too but not reported
                    if in_deck(&p, &pg, &ev.frame_id).await {
                        b.lock().unwrap().push((url, c.lock().unwrap().clone()));
                    }
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
            missing,
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
        // the event tasks hold the browser; the first task drives it, so it stops last
        for t in self.tasks.drain(1..) {
            t.abort();
            let _ = t.await;
        }
        if let Some(b) = Arc::get_mut(&mut self.browser) {
            let _ = b.close().await;
            let _ = b.wait().await;
        }
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

/// Is `frame` the deck's page, one of its frames, or an out-of-process iframe below them?
async fn in_deck(browser: &Browser, page: &Page, frame: &FrameId) -> bool {
    let frames = page.frames().await.unwrap_or_default();
    let targets = browser
        .execute(GetTargetsParams::default())
        .await
        .map(|t| t.result.target_infos)
        .unwrap_or_default();
    let mut id = frame.clone();
    for _ in 0..8 {
        if frames.contains(&id) {
            return true;
        }
        let parent = targets
            .iter()
            .find(|t| t.r#type == "iframe" && t.target_id.inner() == id.inner())
            .and_then(|t| t.parent_frame_id.clone());
        match parent {
            Some(p) => id = p,
            None => return false,
        }
    }
    false
}

/// The file a `file:` URL names (without `?query`/`#hash`, `%`-decoded).
fn file_path(url: &str) -> Option<PathBuf> {
    let p = url.strip_prefix("file://")?.split(['?', '#']).next()?;
    Some(PathBuf::from(
        &*percent_encoding::percent_decode_str(p).decode_utf8_lossy(),
    ))
}

/// `qd pdf` / `qd export --pdf` file name: `<stem>.pdf`, or `<stem>-notes.pdf` with notes.
pub fn pdf_name(stem: &str, notes: bool) -> String {
    format!("{stem}{}.pdf", if notes { "-notes" } else { "" })
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
  const apps = s.querySelectorAll('iframe').length + s.hasAttribute('data-background-iframe');
  return { id: s.id || null, h: i.h, v: i.v, fragments: frags.size, backgrounds: bg, apps };
})"#;

/// After showing a slide: is any math left untypeset?
const RAW_TEX_JS: &str = r#"(() => {
  const s = Reveal.getCurrentSlide();
  return [...s.querySelectorAll('.math')].some(m => !m.querySelector('mjx-container, .katex, svg'));
})()"#;

const CONFIG_JS: &str = r#"(() => { const c = Reveal.getConfig();
  return { showNotes: !!c.showNotes, multiplex: !!c.multiplex, parallax: c.parallaxBackgroundImage || null }; })()"#;

/// Elements outside the current slide (port of the quarto-revealjs skill's
/// `check_overflow.py`), in slide coordinates with 2 px of slack. Not counted
/// (false alarms there): speaker notes, anything marked `.overflow-ok` (a
/// footer meant to bleed off the edge), SVG and MathJax internals, anything
/// inside a clipping container, invisible or empty elements.
const OVERFLOW_JS: &str = r#"(() => {
  const tol = 2, box = document.querySelector('.reveal .slides').getBoundingClientRect();
  const s = Reveal.getScale(), W = Reveal.getConfig().width, H = Reveal.getConfig().height;
  const slide = Reveal.getCurrentSlide(), out = [];
  const skip = (e) => e.closest('aside.notes, .notes, .overflow-ok, svg *, mjx-container *');
  const clipped = (e) => {
    for (let a = e.parentElement; a && a !== slide; a = a.parentElement) {
      const o = getComputedStyle(a);
      if (o.overflowX !== 'visible' || o.overflowY !== 'visible') return true;
    }
    return false;
  };
  for (const e of slide.querySelectorAll('*')) {
    if (e.tagName === 'SECTION' || skip(e)) continue;
    const st = getComputedStyle(e);
    if (st.display === 'none' || st.visibility === 'hidden') continue;
    const r = e.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    const x = (r.left - box.left) / s, y = (r.top - box.top) / s, w = r.width / s, h = r.height / s;
    if (x < -tol || y < -tol || x + w > W + tol || y + h > H + tol) {
      if (clipped(e)) continue;
      const name = e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') +
        (typeof e.className === 'string' && e.className.trim() ? '.' + e.className.trim().split(/\s+/).join('.') : '');
      const text = (e.innerText || '').trim().replace(/\s+/g, ' ').slice(0, 40);
      out.push(`${name} at ${Math.round(x)},${Math.round(y)} size ${Math.round(w)}x${Math.round(h)}` +
               ` (slide ${W}x${H})${text ? ' "' + text + '"' : ''}`);
    }
  }
  return out;
})()"#;

use crate::inline::local;

/// Run the offline check on a rendered deck.
pub fn check(html: &Path, chrome: &Found) -> Result<CheckReport> {
    let url = file_url(html)?;
    // canonical, like the URLs the page requests
    let dir = std::fs::canonicalize(html)?
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    runtime()?.block_on(async {
        let op = OfflinePage::open(chrome).await?;
        let res = check_page(&op, &url, &dir).await;
        op.close().await;
        let (slides, steps, mut problems) = res?;
        let text = std::fs::read_to_string(html)?;
        let size = Size {
            bytes: text.len(),
            largest: crate::inline::largest(&text, 8),
        };
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
            size,
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
    // no slide animation: measured boxes are where they stay
    op.eval("Reveal.configure({ transition: 'none', backgroundTransition: 'none' })")
        .await?;
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
        // let lazy media request; an app (an iframe) gets time to load and run on its own slide
        let apps = s["apps"].as_u64().unwrap_or(0) > 0;
        tokio::time::sleep(Duration::from_millis(if apps { 700 } else { 120 })).await;
        if op.eval(RAW_TEX_JS).await? == Value::Bool(true) {
            problems.push(Problem::error(
                "P_RAW_TEX",
                anchor.clone(),
                "math on this slide is shown as raw TeX".into(),
                None,
            ));
        }
        // the slide at its fullest: every fragment is shown now
        for m in op.eval(OVERFLOW_JS).await?.as_array().into_iter().flatten() {
            problems.push(Problem {
                code: "P_OVERFLOW",
                severity: "warning",
                slide: anchor.clone(),
                message: format!("outside the slide: {}", m.as_str().unwrap_or("")),
                hint: hint("P_OVERFLOW"),
                url: None,
            });
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
    // any other local file that failed to load (an <object>, an image in an app…)
    for (u, slide) in op.missing.lock().unwrap().iter() {
        let f = file_path(u).unwrap_or_default();
        let rel = f
            .strip_prefix(dir)
            .unwrap_or(&f)
            .to_string_lossy()
            .into_owned();
        if !problems.iter().any(|p| p.url.as_ref() == Some(&rel)) {
            problems.push(Problem::error(
                "P_MISSING_FILE",
                slide.clone(),
                format!("'{rel}' is missing"),
                Some(rel),
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
    /// Images recompressed by ghostscript (200 dpi JPEG).
    pub compressed: bool,
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
    let compressed = recompress(out)?;
    Ok(PdfResult {
        path: out.to_path_buf(),
        pages: count_pdf_pages(&std::fs::read(out)?),
        per_slide,
        notes,
        compressed,
    })
}

/// Chrome stores images losslessly (a WebP becomes a large Flate image):
/// when ghostscript is installed, rewrite them as 200 dpi JPEG, in RGB
/// (the skill's `make_pdf.py`; about 50 MB -> 12 MB on a real talk).
fn recompress(pdf: &Path) -> Result<bool> {
    let Some(gs) = crate::render::which("gs", std::env::var_os("PATH")) else {
        return Ok(false);
    };
    let small = pdf.with_extension("small.pdf");
    let mut out = std::ffi::OsString::from("-sOutputFile=");
    out.push(&small);
    let st = std::process::Command::new(gs)
        .args([
            "-q",
            "-dNOPAUSE",
            "-dBATCH",
            "-dSAFER",
            "-sDEVICE=pdfwrite",
            "-dCompatibilityLevel=1.7",
            "-sColorConversionStrategy=RGB",
            "-dDownsampleColorImages=true",
            "-dColorImageResolution=200",
            "-dColorImageDownsampleThreshold=1.2",
            "-dColorImageDownsampleType=/Bicubic",
            "-dAutoFilterColorImages=false",
            "-dColorImageFilter=/DCTEncode",
            "-dJPEGQ=88",
            "-dDownsampleGrayImages=true",
            "-dGrayImageResolution=200",
            "-dAutoFilterGrayImages=false",
            "-dGrayImageFilter=/DCTEncode",
        ])
        .arg(out)
        .arg(pdf)
        .output()?;
    if !st.status.success() {
        let _ = std::fs::remove_file(&small);
        return Err(QdError::Io(std::io::Error::other(format!(
            "ghostscript could not recompress {}: {}",
            pdf.display(),
            String::from_utf8_lossy(&st.stderr).trim()
        ))));
    }
    std::fs::rename(&small, pdf)?;
    Ok(true)
}
