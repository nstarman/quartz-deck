//! Spec 003 browser tests: drive the real editor (`qd serve`) in headless
//! Chrome and check the `.qmd` bytes (T003, T008, T009, T011, T013).
//! Skipped (with a message) when Quarto or a browser is missing; CI has both.

use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams, DispatchMouseEventType,
    InsertTextParams, MouseButton,
};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::layout::Point;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod common;
use common::{have, qd};

/// The selected preview element's ref, or null.
const SEL: &str = "(document.querySelector('#preview').contentDocument.querySelector('.qd-sel')?.dataset.qd ?? null)";
const SRC: &str = "document.querySelector('#srcedit textarea')";
/// Default wait: generous, since all browser tests (each with Quarto and Chrome) run in parallel.
const WAIT: u64 = 20;

const PANE: &str = "document.querySelector('#srcpane textarea')";
const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/corpus");

/// Chrome to use, or None (test skips) when Quarto or a browser is missing.
fn chrome() -> Option<PathBuf> {
    let quarto = have("quarto");
    let tmp = tempfile::tempdir().unwrap();
    let out = qd(tmp.path()).args(["browser"]).output().ok()?;
    let path = serde_json::from_slice::<Value>(&out.stdout).ok()?["path"]
        .as_str()
        .map(PathBuf::from);
    if !quarto || path.is_none() {
        eprintln!("skipping: quarto={quarto} browser={path:?}");
        return None;
    }
    path
}

/// Records the last pointer presses and focus changes (target and point), for a
/// failed wait's report: where a click really landed.
const EVENT_LOG: &str = "(() => { window.__ev = []; const name = (t) => t?.nodeType === 1 ? t.tagName.toLowerCase() + (t.id ? '#' + t.id : '') + (t.dataset?.menu ? `[${t.dataset.menu}]` : '') + (t.className && typeof t.className === 'string' ? '.' + t.className.trim().split(/ +/)[0] : '') : String(t); const log = (e) => { __ev.push(`${e.type} ${name(e.target)}${e.clientX !== undefined ? ` @${Math.round(e.clientX)},${Math.round(e.clientY)}` : ''}${e.defaultPrevented ? ' (prevented)' : ''}`); if (__ev.length > 10) __ev.shift(); }; for (const t of ['mousedown', 'pointerdown', 'focusin', 'focusout']) addEventListener(t, log, true); addEventListener('mousedown', (e) => setTimeout(() => __ev.push(`  after: prevented=${e.defaultPrevented}`)), true); addEventListener('blur', () => __ev.push('window blur')); })()";

/// A failed wait's report: the window, the top row and what has the keyboard.
const PAGE_STATE: &str = "(() => { try { const q = (s) => document.querySelector(s); const r = (s) => { const b = q(s)?.getBoundingClientRect(); return b && [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)]; }; return { window: [innerWidth, innerHeight], menus: q('#menubar')?.className, menubar: r('#menubar'), header: r('header'), notice: q('#notice')?.textContent, status: q('#status')?.textContent, list: getComputedStyle(q('#slides')).display, focus: document.activeElement?.outerHTML.slice(0, 80), menuOpen: !q('#ctxmenu')?.hidden, current: q('#slides li.current')?.dataset.id, currentBox: r('#slides li.current'), nav: [q('nav')?.scrollTop, q('nav')?.clientHeight, q('nav')?.scrollHeight], shown: q('#preview')?.contentWindow?.Reveal?.getCurrentSlide()?.id, events: window.__ev }; } catch (e) { return String(e); } })()";

/// The editor has loaded: preview ready, slide list drawn, session API up.
const READY: &str = "(() => { const f = document.querySelector('#preview'); return !!(f && f.contentWindow?.Reveal?.isReady?.() && document.querySelector('#slides li') && window.qdSession); })()";

/// Evaluate `js` in `page` (promises awaited); its JSON value.
async fn eval(page: &Page, js: &str) -> Value {
    let p = EvaluateParams::builder()
        .expression(js)
        .await_promise(true)
        .return_by_value(true)
        .build()
        .unwrap();
    let r = page
        .evaluate_expression(p)
        .await
        .unwrap_or_else(|e| panic!("eval failed: {e}\n{js}"));
    r.value().cloned().unwrap_or(Value::Null)
}

struct Ed {
    dir: tempfile::TempDir,
    page: Page,
    _browser: Browser,
    _profile: tempfile::TempDir,
    _slot: tokio::sync::OwnedSemaphorePermit,
    _watchdog: Watchdog,
}

/// A browser test that runs over 5 minutes is stuck, not slow: under heavy load
/// Chrome can go away, and a CDP call then waits forever. Name the test and
/// stop the run rather than hang it.
struct Watchdog(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Watchdog {
    fn start() -> Watchdog {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (flag, name) = (
            done.clone(),
            std::thread::current().name().unwrap_or("?").to_string(),
        );
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(300));
            if !flag.load(std::sync::atomic::Ordering::Relaxed) {
                // straight to stderr: the harness captures eprintln!, and exit() drops that
                let _ = writeln!(
                    std::io::stderr(),
                    "{name}: still running after 5 minutes (is Chrome gone?); stopping the run"
                );
                std::process::exit(101);
            }
        });
        Watchdog(done)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// At most half the CPUs' worth of editors (Chrome + Quarto each) at once:
/// on a 2-vCPU CI runner they otherwise starve each other past the waits.
async fn slot() -> tokio::sync::OwnedSemaphorePermit {
    static SLOTS: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> =
        std::sync::OnceLock::new();
    let n = std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1));
    let s = SLOTS.get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(n)));
    s.clone().acquire_owned().await.unwrap()
}

impl Drop for Ed {
    fn drop(&mut self) {
        let _ = qd(self.dir.path()).arg("stop").output();
    }
}

impl Ed {
    async fn open(deck: &str) -> Option<Ed> {
        let chrome = chrome()?;
        let slot = slot().await;
        let watchdog = Watchdog::start();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("deck.qmd"), deck).unwrap();
        let out = qd(dir.path()).args(["serve", "--detach"]).output().unwrap();
        let info: Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("serve: {}", String::from_utf8_lossy(&out.stderr)));
        let url = info["url"].as_str().unwrap().to_string();
        let profile = tempfile::tempdir().unwrap();
        let config = BrowserConfig::builder()
            .chrome_executable(chrome)
            .user_data_dir(profile.path())
            .window_size(1280, 800)
            .launch_timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        let (browser, mut handler) = Browser::launch(config).await.unwrap();
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        let page = browser.new_page(url.as_str()).await.unwrap();
        let ed = Ed {
            dir,
            page,
            _browser: browser,
            _profile: profile,
            _slot: slot,
            _watchdog: watchdog,
        };
        ed.wait_secs(READY, 60).await;
        ed.eval(EVENT_LOG).await;
        Some(ed)
    }

    fn file(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("deck.qmd")).unwrap()
    }

    /// Run qd in the deck folder; it must succeed. Returns its JSON output.
    fn cli(&self, args: &[&str]) -> Value {
        let out = qd(self.dir.path()).args(args).output().unwrap();
        assert!(
            out.status.success(),
            "qd {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null)
    }

    async fn eval(&self, js: &str) -> Value {
        eval(&self.page, js).await
    }

    /// Wait (up to `WAIT` s) until the JS condition is true.
    async fn wait(&self, cond: &str) {
        self.wait_secs(cond, WAIT).await;
    }

    async fn wait_secs(&self, cond: &str, secs: u64) {
        // a condition that throws (e.g. the preview is mid-reload after a
        // render) counts as "not yet", not as a failure
        let cond_safe = format!(
            "(async () => {{ try {{ return await ({cond}); }} catch {{ return false; }} }})()"
        );
        let start = Instant::now();
        while self.eval(&cond_safe).await != Value::Bool(true) {
            if start.elapsed() > Duration::from_secs(secs) {
                // what the page looked like, so a failure elsewhere (CI) can be read
                let state = self.eval(PAGE_STATE).await;
                panic!("timed out waiting for: {cond}\npage: {state}");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn wait_file(&self, pred: impl Fn(&str) -> bool, what: &str) {
        let start = Instant::now();
        while !pred(&self.file()) {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "file never {what}:\n{}",
                self.file()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Press at `from` and move to `to` in steps, the button still down.
    async fn press_and_move(&self, from: (f64, f64), to: (f64, f64)) {
        use DispatchMouseEventType::*;
        self.mouse(MousePressed, from, false).await;
        for i in 1..=8 {
            let t = i as f64 / 8.0;
            let at = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            self.mouse(MouseMoved, at, false).await;
        }
    }

    /// ⌘V on `target` (a JS expression), then the paste event a browser fires
    /// for it, carrying `text` (a JS expression). Devtools key presses don't
    /// run the browser's paste command on macOS and paste Chrome's own
    /// clipboard on Linux, so the test sends both itself.
    async fn paste(&self, target: &str, text: &str) {
        self.eval(&format!(
            "(() => {{ const t = {target}; t.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'v', code: 'KeyV', ctrlKey: true, bubbles: true, cancelable: true }}));
              const dt = new DataTransfer(); dt.setData('text/plain', {text});
              t.dispatchEvent(new ClipboardEvent('paste', {{ clipboardData: dt, bubbles: true, cancelable: true }})); }})()"
        ))
        .await;
    }

    /// Click slide `id` in the list and wait until it is on the canvas and has the
    /// keyboard: keys pressed straight after the click must reach that slide.
    async fn pick_slide(&self, id: &str) {
        // an edit still landing renumbers the items (a new item under the press loses the click)
        self.settle().await;
        self.click_editor(&format!("#slides li[data-id={id}] > button"))
            .await;
        self.wait(&format!("document.activeElement?.closest('#slides li')?.dataset.id === '{id}' && document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide()?.id === '{id}'")).await;
    }

    /// Show slide `id` on the canvas and wait until the list has scrolled it into view
    /// (the list scrolls to the shown slide; a slide change still landing would undo
    /// a scroll done by hand).
    async fn show_in_list(&self, id: &str) {
        self.go(id).await;
        self.wait(&format!("(() => {{ const n = document.querySelector('nav').getBoundingClientRect(); const r = document.querySelector('#slides li.current[data-id={id}]')?.getBoundingClientRect(); return !!r && r.top >= n.top - 1 && r.bottom <= n.bottom + 1; }})()")).await;
    }

    /// Edit ▸ Undo once; the file must be back to `before` (one edit, one undo step).
    async fn undo_to(&self, before: &str) {
        self.idle().await;
        self.menu("Edit", "Undo").await;
        self.wait_file(|f| f == before, "one undo step").await;
    }

    /// Poll `qd <args>` until `pred` holds on its JSON (10 s).
    async fn until_cli(&self, args: &[&str], pred: impl Fn(&Value) -> bool, what: &str) {
        let start = Instant::now();
        while !pred(&self.cli(args)) {
            assert!(start.elapsed() < Duration::from_secs(10), "{what}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Run `label` from the menu bar's `top` menu with the mouse (submenus: "Format/Bold").
    async fn menu(&self, top: &str, label: &str) {
        self.click_editor(&format!("#menubar [data-menu={top:?}]"))
            .await;
        for part in label.split('/') {
            self.click_menu(part).await;
        }
    }

    /// Show the slide with this preview anchor and wait for it.
    async fn go(&self, anchor: &str) {
        // re-set the hash while waiting: a preview reload in between drops it
        self.wait(&format!(
            "(w => w.Reveal?.getCurrentSlide()?.id === '{anchor}' || (w.location.hash = '#/{anchor}', false))(document.querySelector('#preview').contentWindow)"
        ))
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await; // slide transition
    }

    /// Page point `[x, y]` that `js` evaluates to.
    async fn point(&self, js: &str) -> (f64, f64) {
        let pt = self.eval(js).await;
        (pt[0].as_f64().unwrap(), pt[1].as_f64().unwrap())
    }

    /// Trusted click at the page point `js` evaluates to.
    async fn click_js(&self, js: &str) {
        let (x, y) = self.point(js).await;
        self.page.click(Point { x, y }).await.unwrap();
    }

    /// Page point of a preview element (`sel` in the iframe): `x` is a JS
    /// expression over its rect `r` (e.g. `r.width / 2`), y its middle.
    async fn slide_at(&self, sel: &str, x: &str) -> (f64, f64) {
        self.point(&format!("(() => {{ const f = document.querySelector('#preview'); const el = f.contentDocument.querySelector({sel:?}); const a = f.getBoundingClientRect(); const r = el.getBoundingClientRect(); return [a.left + r.left + {x}, a.top + r.top + r.height / 2]; }})()")).await
    }

    async fn slide_point(&self, sel: &str) -> (f64, f64) {
        self.slide_at(sel, "r.width / 2").await
    }

    /// Click an element of the preview near its left edge (where text starts).
    async fn click_in_slide(&self, sel: &str) {
        let (x, y) = self.slide_at(sel, "4").await;
        self.page.click(Point { x, y }).await.unwrap();
    }

    /// Centre of an element of the editor page (`dx` from its left edge instead, if given).
    async fn editor_point(&self, sel: &str, dx: Option<f64>) -> (f64, f64) {
        let x = dx.map_or("r.width / 2".to_string(), |d| d.to_string());
        self.point(&format!("(() => {{ const r = document.querySelector({sel:?}).getBoundingClientRect(); return [r.left + {x}, r.top + r.height / 2]; }})()")).await
    }

    async fn click_editor(&self, sel: &str) {
        let (x, y) = self.editor_point(sel, None).await;
        self.page.click(Point { x, y }).await.unwrap();
    }

    /// Click somewhere on the slide that is not an element (ends an edit).
    async fn click_blank(&self) {
        self.click_js("(() => { const r = document.querySelector('#preview').getBoundingClientRect(); return [r.left + r.width / 2, r.bottom - 30]; })()").await;
    }

    /// Put the caret (or a selection of `word`) inside the open edit.
    async fn caret(&self, sel: &str, word: Option<&str>) {
        // the edit opens once the block's source has been fetched
        self.wait("window.qdSession() !== null").await;
        let w = word.map_or("null".to_string(), |w| format!("{w:?}"));
        let js = format!(
            "(() => {{ const d = document.querySelector('#preview').contentDocument; const el = d.querySelector({sel:?});
              const w = d.createTreeWalker(el, NodeFilter.SHOW_TEXT); let n, last = null; const r = d.createRange();
              while ((n = w.nextNode())) {{ if (n.data.trim()) last = n; const i = {w} === null ? -1 : n.data.indexOf({w});
                if (i >= 0) {{ r.setStart(n, i); r.setEnd(n, i + {len}); break; }} }}
              if ({w} === null) {{ const t = last.data.replace(/\\s+$/, '').length; r.setStart(last, t); r.collapse(true); }}
              const s = d.getSelection(); s.removeAllRanges(); s.addRange(r); return d.activeElement?.isContentEditable === true; }})()",
            len = word.map_or(0, |w| w.chars().count()),
        );
        assert_eq!(
            self.eval(&js).await,
            Value::Bool(true),
            "no edit open on {sel}"
        );
    }

    async fn type_text(&self, text: &str) {
        self.page
            .execute(InsertTextParams::new(text))
            .await
            .unwrap();
    }

    async fn key(&self, key: &str, code: &str, vk: i64, modifiers: i64) {
        // a plain Enter must be a KeyDown carrying "\r" for Chrome to edit with it
        let enter = key == "Enter" && modifiers == 0;
        let down = if enter {
            DispatchKeyEventType::KeyDown
        } else {
            DispatchKeyEventType::RawKeyDown
        };
        for t in [down, DispatchKeyEventType::KeyUp] {
            let mut b = DispatchKeyEventParams::builder()
                .r#type(t.clone())
                .key(key)
                .code(code)
                .windows_virtual_key_code(vk)
                .modifiers(modifiers);
            if enter && t == DispatchKeyEventType::KeyDown {
                b = b.text("\r");
            }
            self.page.execute(b.build().unwrap()).await.unwrap();
        }
    }

    async fn session(&self) -> Value {
        self.eval("window.qdSession()").await
    }

    async fn set_source(&self, text: &str) {
        self.eval(&format!("(() => {{ const t = {SRC}; t.value = {text:?}; t.dispatchEvent(new Event('input')); t.focus(); }})()")).await;
    }
    async fn idle(&self) {
        self.wait_secs(
            "window.qdSession() === null && !document.querySelector('#pending')?.textContent",
            30,
        )
        .await;
    }

    /// Wait until the preview shows the file as it is now (render caught up, no edit open).
    async fn settle(&self) {
        self.wait_secs(
            "(async () => { const v = (await fetch('/api/deck').then((r) => r.json())).version; const st = document.querySelector('#status'); const f = document.querySelector('#preview').contentWindow; return window.qdSession() === null && st.textContent === 'Up to date' && st.title.endsWith(v.slice(0, 7)) && !!f.Reveal?.isReady?.(); })()",
            60,
        )
        .await;
    }
    /// Type `text` at the end of a paragraph-like element and leave it.
    async fn type_into(&self, anchor: &str, sel: &str, text: &str) {
        self.settle().await;
        self.go(anchor).await;
        self.click_in_slide(&format!("{sel} p")).await;
        self.caret(sel, None).await;
        self.type_text(text).await;
    }

    async fn mouse(&self, t: DispatchMouseEventType, (x, y): (f64, f64), right: bool) {
        let up = matches!(t, DispatchMouseEventType::MouseReleased);
        let p = DispatchMouseEventParams::builder()
            .r#type(t)
            .x(x)
            .y(y)
            .button(if right {
                MouseButton::Right
            } else {
                MouseButton::Left
            })
            .buttons(if up {
                0
            } else if right {
                2
            } else {
                1
            })
            .click_count(1)
            .build()
            .unwrap();
        self.page.execute(p).await.unwrap();
    }
    async fn right_click(&self, at: (f64, f64)) {
        self.mouse(DispatchMouseEventType::MousePressed, at, true)
            .await;
        self.mouse(DispatchMouseEventType::MouseReleased, at, true)
            .await;
    }
    async fn drag(&self, from: (f64, f64), to: (f64, f64)) {
        use DispatchMouseEventType::*;
        for (t, at) in [(MousePressed, from), (MouseMoved, to), (MouseReleased, to)] {
            self.mouse(t, at, false).await;
        }
    }
    /// Hover a preview element until its + / drag handles show.
    async fn hover(&self, sel: &str) {
        let (x, y) = self.slide_point(sel).await;
        self.page.move_mouse(Point { x, y }).await.unwrap();
        self.wait("!document.querySelector('#hoverbar').hidden")
            .await;
    }
    async fn src_open(&self) {
        self.wait("!document.querySelector('#srcedit').hidden")
            .await;
    }

    async fn close_menu(&self) {
        self.key("Escape", "Escape", 27, 0).await;
        self.wait("document.querySelector('#ctxmenu').hidden").await;
    }
    /// Select a preview element (right-click it, close the menu) so the element bar shows.
    async fn select(&self, sel: &str) {
        self.right_click(self.slide_point(sel).await).await;
        self.wait("!document.querySelector('#ctxmenu').hidden")
            .await;
        self.close_menu().await;
        self.wait("!document.querySelector('#elbar').hidden").await;
    }
    /// Labels of the open context menu's top level, with "(disabled)" marks.
    async fn menu_labels(&self) -> Vec<String> {
        self.wait("!document.querySelector('#ctxmenu').hidden")
            .await;
        let v = self.eval("[...document.querySelectorAll('#ctxmenu > ul > li[tabindex]')].map((li) => li.firstChild.textContent + (li.getAttribute('aria-disabled') ? ' (disabled)' : ''))").await;
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect()
    }
    /// Click the visible menu item labelled `label` (opening its submenu if it has one).
    async fn click_menu(&self, label: &str) {
        self.wait(&format!("[...document.querySelectorAll('#ctxmenu li')].some((li) => li.firstChild?.textContent === {label:?} && li.offsetParent)")).await;
        self.click_js(&format!("(() => {{ const li = [...document.querySelectorAll('#ctxmenu li')].find((li) => li.firstChild?.textContent === {label:?} && li.offsetParent); li.dispatchEvent(new MouseEvent('mouseenter')); const r = li.getBoundingClientRect(); return [r.left + 20, r.top + r.height / 2]; }})()")).await;
    }

    /// View ▸ Source pane, run directly (the same action in every layout,
    /// including VS Code's panel, where the menus are one ☰).
    async fn toggle_source(&self) {
        self.eval("import('/ui/keys.js').then((k) => k.run('toggle-source'))")
            .await;
    }

    async fn open_source(&self) {
        self.toggle_source().await;
        self.wait(&format!(
            "{PANE}.value.length > 0 && !document.querySelector('#srcpane').hidden"
        ))
        .await;
    }
    /// Put the caret right after `after` in the source pane (or select `after` itself).
    async fn pane_caret(&self, after: &str, select: bool) {
        let len = after.chars().count();
        let from = if select { 0 } else { len };
        let js = format!(
            "(() => {{ const t = {PANE}; const i = t.value.indexOf({after:?}); t.focus(); t.setSelectionRange(i + {from}, i + {len}); t.dispatchEvent(new KeyboardEvent('keyup')); return i >= 0; }})()"
        );
        assert_eq!(
            self.eval(&js).await,
            Value::Bool(true),
            "{after:?} not in the pane"
        );
    }
}

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Methods {.c}\n\nWe use **bold** words and a [link](https://example.org)\nacross two \"quoted\" lines -- here.\n\n- one\n- two\n\n## Results {#results}\n\nText.\n";

// ---------- T009: the round-trip gate over the corpus ----------

const GATE_JS: &str = r#"(async () => {
  const m = await import('/ui/inline.js');
  const doc = document.querySelector('#preview').contentDocument;
  const deck = await fetch('/api/deck').then((r) => r.json());
  const out = { typeable: [], other: [], broken: [], why: {} };
  for (const s of deck.slides) {
    const slide = (await fetch('/api/slide/' + encodeURIComponent(s.id)).then((r) => r.json())).slide;
    slide.blocks.forEach((b, i) => {
      const ref = `${s.id}/${i}`;
      const el = doc.querySelector(`[data-qd="${CSS.escape(ref)}"]`);
      const reg = el && m.region(b.kind, b.text);
      const g = reg && m.gate(el, reg, m.hintsFor(b.text));
      if (!g) {
        out.other.push(ref);
        try { out.why[ref] = el ? (m.serializeBlocks(el, m.hintsFor(b.text)), 'does not align') : 'not in preview'; } catch (e) { out.why[ref] = e.message; }
        return;
      }
      out.typeable.push(ref);
      // no-op and single-character edits must touch only the mapped source bytes
      if (m.splice(reg.inner, g.S0, g.S0, g.pos) !== reg.inner) out.broken.push(ref + ' (no-op)');
      for (let k = 0; k < g.S0.length; k++) {
        if (g.pos[k] < 0) continue;
        const got = m.splice(reg.inner, g.S0, g.S0.slice(0, k) + 'Z' + g.S0.slice(k), g.pos);
        if (got !== reg.inner.slice(0, g.pos[k]) + 'Z' + reg.inner.slice(g.pos[k])) { out.broken.push(`${ref} @${k}`); break; }
      }
      const end = m.splice(reg.inner, g.S0, g.S0 + 'Z', g.pos);
      if (end !== reg.inner.replace(/\s*$/, '') + 'Z' + reg.inner.slice(reg.inner.replace(/\s*$/, '').length)) out.broken.push(ref + ' (end)');
    });
  }
  return out;
})()"#;

async fn gate(name: &str) -> Option<Value> {
    let deck = std::fs::read_to_string(Path::new(CORPUS).join(name)).unwrap();
    let ed = Ed::open(&deck).await?;
    let r = ed.eval(GATE_JS).await;
    eprintln!(
        "{name}: typeable {}\n  not typeable {}",
        r["typeable"], r["why"]
    );
    assert_eq!(
        r["broken"],
        json!([]),
        "{name}: splice changed bytes outside the edit"
    );
    Some(r)
}

fn has(v: &Value, key: &str, refs: &[&str]) {
    let list: Vec<&str> = v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    for r in refs {
        assert!(list.contains(r), "{r} should be {key}: {list:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn gate_prose() {
    let Some(r) = gate("prose.qmd").await else {
        return;
    };
    has(
        &r,
        "typeable",
        &["quotes/0", "breaks/0", "escapes/0", "emoji/0"],
    );
    has(&r, "other", &["refs/0"]); // footnote and citation render differently from their source
}

#[tokio::test(flavor = "multi_thread")]
async fn gate_lists() {
    let Some(r) = gate("lists.qmd").await else {
        return;
    };
    has(
        &r,
        "typeable",
        &[
            "tight/0",
            "nested/0",
            "ordered/0",
            "start/0",
            "loose/0",
            "plus/0",
            "incr/0",
        ],
    );
    has(&r, "other", &["task/0"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn gate_inline() {
    let Some(r) = gate("inline.qmd").await else {
        return;
    };
    has(&r, "typeable", &["emph/0", "spans/0", "supsub/0", "hard/0"]);
    has(&r, "other", &["imath/0"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn gate_blocks() {
    let Some(r) = gate("blocks.qmd").await else {
        return;
    };
    has(
        &r,
        "typeable",
        &[
            "quote/0",
            "aligned/0",
            "box/0",
            "subhead/0",
            "subhead/1",
            "notes/0",
        ],
    );
    has(
        &r,
        "other",
        &["math/0", "code/0", "callout/0", "raw/0", "table/0"],
    );
}

// ---------- T011: typing on the slide ----------

#[tokio::test(flavor = "multi_thread")]
async fn typing_writes_only_the_element() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.go("methods").await;
    let p = "[data-qd=\"methods/0\"]";
    ed.click_in_slide(&format!("{p} p")).await;
    ed.caret(p, None).await;
    ed.type_text(" Truly \"new\".").await;
    ed.click_blank().await;
    ed.wait_file(|f| f.contains("Truly"), "saved").await;
    assert_eq!(
        ed.file(),
        DECK.replace("lines -- here.", "lines -- here. Truly \"new\".")
    );

    // Escape keeps an edit, like clicking elsewhere (spec 007); undo takes it back
    let before = ed.file();
    ed.idle().await;
    ed.click_in_slide("[data-qd=\"methods/1\"] li").await;
    ed.caret("[data-qd=\"methods/1\"]", None).await;
    ed.type_text(" kept").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait_file(|f| f.contains("two kept"), "kept by Escape")
        .await;
    ed.undo_to(&before).await;

    // undo restores the file byte-for-byte (shared history with the CLI)
    ed.menu("Edit", "Undo").await;
    ed.wait_file(|f| f == DECK, "restored by undo").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn enter_adds_list_items_and_saves_titles() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.go("methods").await;
    let list = "[data-qd=\"methods/1\"]";
    ed.click_in_slide(&format!("{list} li")).await;
    ed.caret(list, Some("one")).await;
    ed.eval("(() => { const d = document.querySelector('#preview').contentDocument; const s = d.getSelection(); s.collapseToEnd(); })()").await;
    ed.key("Enter", "Enter", 13, 0).await;
    ed.type_text("one and a half").await;
    ed.key("Enter", "Enter", 13, 2).await; // Ctrl+Enter saves
    ed.wait_file(|f| f.contains("half"), "saved").await;
    assert!(
        ed.file().contains("- one\n- one and a half\n- two\n"),
        "{}",
        ed.file()
    );

    // the slide title: Enter saves; attributes are kept
    ed.idle().await;
    ed.go("methods").await;
    ed.click_in_slide("section#methods > h2").await;
    ed.caret("section#methods > h2", None).await;
    ed.type_text(" and data").await;
    ed.key("Enter", "Enter", 13, 0).await;
    ed.wait_file(|f| f.contains("and data"), "saved").await;
    // retitling pins the old auto id so references keep working (spec 001)
    assert!(
        ed.file().contains("## Methods and data {#methods .c}"),
        "{}",
        ed.file()
    );
}

// ---------- T013: formatting toolbar ----------

#[tokio::test(flavor = "multi_thread")]
async fn toolbar_formats_and_unformats() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.go("methods").await;
    let list = "[data-qd=\"methods/1\"]";
    ed.click_in_slide(&format!("{list} li")).await;
    ed.caret(list, Some("two")).await;
    ed.click_editor("#fmtbar [data-fmt=bold]").await;
    ed.caret(list, Some("one")).await;
    ed.key("i", "KeyI", 73, 2).await; // Ctrl+I
    ed.caret(list, Some("two")).await;
    ed.click_editor("#fmtbar [data-fmt=color]").await; // the colour menu
    ed.click_editor("#fmtbar .swatch").await;
    assert_eq!(
        ed.session().await["draft"],
        "- *one*\n- **[two]{style=\"color:#c0392b\"}**"
    );
    // remove all three again: the draft is byte-identical to the source
    ed.click_editor("#fmtbar [data-fmt=color]").await;
    ed.click_editor("#fmtbar [data-fmt=nocolor]").await;
    ed.caret(list, Some("two")).await;
    ed.click_editor("#fmtbar [data-fmt=bold]").await;
    ed.caret(list, Some("one")).await;
    ed.key("i", "KeyI", 73, 2).await;
    assert_eq!(ed.session().await["draft"], "- one\n- two");
    // code and highlight
    ed.caret(list, Some("two")).await;
    ed.key("e", "KeyE", 69, 2).await; // Ctrl+E
    ed.caret(list, Some("one")).await;
    ed.click_editor("#fmtbar [data-fmt=more]").await; // "Aa"
    ed.click_editor("#fmtbar [data-fmt=mark]").await;
    ed.click_blank().await;
    ed.wait_file(|f| f.contains("`two`"), "saved").await;
    assert!(
        ed.file().contains("- [one]{.mark}\n- `two`\n"),
        "{}",
        ed.file()
    );
}

// ---------- T008: renders wait for an open edit ----------

#[tokio::test(flavor = "multi_thread")]
async fn render_waits_for_the_open_edit() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.go("methods").await;
    ed.eval("document.querySelector('#preview').contentWindow.__marker = 1")
        .await;
    ed.click_in_slide("[data-qd=\"methods/0\"] p").await;
    ed.caret("[data-qd=\"methods/0\"]", None).await;
    ed.type_text(" draft").await;
    // an agent edits another slide meanwhile
    ed.cli(&["title", "results", "Findings"]);
    ed.wait_secs(
        "(document.querySelector('#pending')?.textContent || '').includes('refresh')",
        30,
    )
    .await;
    assert_eq!(
        ed.eval("document.querySelector('#preview').contentWindow.__marker")
            .await,
        1,
        "preview reloaded during the edit"
    );
    // the person's save still succeeds (element-level check), then the render applies
    ed.click_blank().await;
    ed.wait_file(|f| f.contains("here. draft"), "saved").await;
    assert!(ed.file().contains("## Findings {#results}"));
    ed.wait_secs(
        "document.querySelector('#preview').contentWindow.__marker === undefined",
        30,
    )
    .await;
}

// ---------- US2: source editing in place (T016) ----------

const MATH: &str =
    "---\ntitle: T\nformat: revealjs\n---\n\n## Eq {#eq}\n\n$$\nE = mc^2\n$$\n\nPlain words.\n";

#[tokio::test(flavor = "multi_thread")]
async fn source_editor_opens_over_math_and_refuses_new_slides() {
    let Some(ed) = Ed::open(MATH).await else {
        return;
    };
    ed.go("eq").await;
    ed.click_in_slide("[data-qd=\"eq/0\"]").await;
    ed.wait(&format!("{SRC}?.value === '$$\\nE = mc^2\\n$$'"))
        .await;
    // drawn over the element it edits
    let over = ed
        .eval("(() => { const f = document.querySelector('#preview').getBoundingClientRect(); const e = document.querySelector('#preview').contentDocument.querySelector('[data-qd=\"eq/0\"]').getBoundingClientRect(); const b = document.querySelector('#srcedit').getBoundingClientRect(); return Math.abs(b.top - (f.top + e.top)) < 40; })()")
        .await;
    assert_eq!(over, Value::Bool(true));
    ed.set_source("$$\nE = m c^2\n$$").await;
    ed.key("Enter", "Enter", 13, 2).await; // Ctrl+Enter saves
    ed.wait_file(|f| f.contains("E = m c^2"), "saved").await;
    assert_eq!(ed.file(), MATH.replace("mc^2", "m c^2"));

    // a slide heading in the source is refused; the text stays for fixing
    ed.idle().await;
    ed.go("eq").await;
    ed.click_in_slide("[data-qd=\"eq/0\"]").await;
    ed.src_open().await;
    let before = ed.file();
    ed.set_source("Text.\n\n## Sneaky\n\nMore.").await;
    ed.key("Enter", "Enter", 13, 2).await;
    ed.wait("(document.querySelector('#srcedit .err')?.textContent || '').includes('slide')")
        .await;
    assert_eq!(
        ed.eval(&format!("{SRC}.value")).await,
        "Text.\n\n## Sneaky\n\nMore."
    );
    assert_eq!(ed.file(), before);
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait("document.querySelector('#srcedit').hidden && window.qdSession() === null")
        .await;
    assert_eq!(ed.file(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn alt_enter_switches_typing_to_source() {
    let Some(ed) = Ed::open(MATH).await else {
        return;
    };
    ed.go("eq").await;
    ed.click_in_slide("[data-qd=\"eq/1\"] p").await;
    ed.caret("[data-qd=\"eq/1\"]", None).await;
    ed.type_text(" more").await;
    ed.key("Enter", "Enter", 13, 1).await; // Alt+Enter
    ed.wait(&format!("{SRC}?.value === 'Plain words. more'"))
        .await;
    ed.set_source("Plain *words* more.").await;
    ed.key("Enter", "Enter", 13, 2).await;
    ed.wait_file(|f| f.contains("Plain *words* more."), "saved")
        .await;
    assert_eq!(
        ed.file(),
        MATH.replace("Plain words.", "Plain *words* more.")
    );
}

// ---------- US5: editing together with an agent (T019, T021) ----------

#[tokio::test(flavor = "multi_thread")]
async fn same_element_conflict_offers_both_versions() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    let para = "[data-qd=\"methods/0\"]";
    ed.type_into("methods", para, " mine").await;
    ed.cli(&["edit", "methods/0", "--md", "Agent text."]);
    ed.click_blank().await;
    ed.wait("!document.querySelector('#conflict').hidden").await;
    let bar = ed
        .eval("document.querySelector('#conflict').textContent")
        .await;
    let bar = bar.as_str().unwrap();
    assert!(
        bar.contains("Agent text.") && bar.contains("here. mine"),
        "{bar}"
    );
    assert!(
        ed.file().contains("\n\nAgent text.\n\n"),
        "agent's version stays until the person chooses"
    );
    assert_eq!(ed.session().await, Value::Null);

    // "Use mine…": the person's text opens as source over the element, based on the agent's version
    ed.settle().await;
    ed.go("methods").await;
    ed.click_editor("#conflict [data-act=mine]").await;
    ed.src_open().await;
    assert!(
        ed.eval(&format!("{SRC}.value"))
            .await
            .as_str()
            .unwrap()
            .ends_with("here. mine")
    );
    ed.eval(&format!("{SRC}.focus()")).await;
    ed.key("Enter", "Enter", 13, 2).await;
    ed.wait_file(
        |f| f.contains("here. mine"),
        "saved with the person's version",
    )
    .await;
    assert!(!ed.file().contains("Agent text."));
    ed.wait("document.querySelector('#conflict').hidden").await;

    // "Keep theirs": nothing is written
    ed.type_into("methods", para, " again").await;
    ed.cli(&["edit", "methods/0", "--md", "Theirs."]);
    ed.click_blank().await;
    ed.wait("!document.querySelector('#conflict').hidden").await;
    ed.click_editor("#conflict [data-act=theirs]").await;
    ed.wait("document.querySelector('#conflict').hidden").await;
    assert!(ed.file().contains("\n\nTheirs.\n\n"));

    // an on-slide save is in the same undo history as the CLI
    ed.type_into("methods", para, " last").await;
    ed.click_blank().await;
    ed.wait_file(|f| f.contains("Theirs. last"), "saved").await;
    ed.cli(&["undo"]);
    assert!(ed.file().contains("\n\nTheirs.\n\n"));
}

/// SC-005: 50 interleaved edits from the person (on slide A) and an agent (on
/// slides A and B); nothing is lost and every same-element collision is reported.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "slow (~1-2 min): run with --ignored; runs in CI"]
async fn fifty_interleaved_edits_lose_nothing() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    let para = "[data-qd=\"methods/0\"]";
    let mut collisions = 0;
    for i in 0..25 {
        ed.type_into("methods", para, &format!(" p{i}")).await;
        // the agent edits another slide every time, and the same slide's list often
        ed.cli(&["edit", "results/0", "--md", &format!("Agent {i}.")]);
        if i % 3 == 0 {
            ed.cli(&["edit", "methods/1", "--md", &format!("- one\n- two {i}")]);
        }
        let collide = i % 5 == 4;
        if collide {
            ed.cli(&["edit", "methods/0", "--md", &format!("Agent rewrote {i}.")]);
        }
        ed.click_blank().await;
        if collide {
            ed.wait("!document.querySelector('#conflict').hidden").await;
            collisions += 1;
            ed.settle().await;
            ed.go("methods").await;
            ed.click_editor("#conflict [data-act=mine]").await;
            ed.src_open().await;
            ed.eval(&format!("{SRC}.focus()")).await;
            ed.key("Enter", "Enter", 13, 2).await;
        }
        let want = format!(" p{i}");
        ed.wait_file(|f| f.contains(&want), "has the person's edit")
            .await;
        assert!(
            ed.file().contains(&format!("Agent {i}.")),
            "agent's edit {i} lost:\n{}",
            ed.file()
        );
    }
    assert_eq!(collisions, 5);
    let f = ed.file();
    assert!(f.contains("- two 24"), "{f}");
}

// ---------- P2: menus, bars, insert, drag, clipboard (T022, T027, T029, T031) ----------

const P2: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## A {#a}\n\nFirst para.\n\nSecond para.\n\n- x\n- y\n\n## B {#b}\n\nOther slide.\n\n## Empty {#empty}\n";

#[tokio::test(flavor = "multi_thread")]
async fn context_menus_list_and_run_actions() {
    let Some(ed) = Ed::open(P2).await else { return };
    ed.go("a").await;
    let first = ed.slide_point("[data-qd=\"a/0\"] p").await;
    ed.right_click(first).await;
    let labels = ed.menu_labels().await;
    for l in [
        "Edit text",
        "Edit markdown",
        "Cut",
        "Copy",
        "Paste after",
        "Duplicate",
        "Delete",
        "Move up (disabled)",
        "Move down",
        "Insert before",
        "Insert after",
        "Appears on click",
        "Appearance",
    ] {
        assert!(labels.iter().any(|x| x == l), "{l} missing from {labels:?}");
    }
    // keyboard: arrows move, Escape closes
    ed.key("ArrowDown", "ArrowDown", 40, 0).await;
    assert_eq!(
        ed.eval("document.activeElement.firstChild.textContent")
            .await,
        "Edit markdown"
    );
    ed.close_menu().await;
    // each action is the qd command: Duplicate = `qd block dup`
    ed.right_click(first).await;
    ed.click_menu("Duplicate").await;
    ed.wait_file(|f| f.matches("First para.").count() == 2, "duplicated")
        .await;
    assert!(
        ed.file()
            .contains("First para.\n\nFirst para.\n\nSecond para.")
    );

    // the slide list: Transition ▸ Fade = `qd set b --attr transition=fade`
    ed.settle().await;
    ed.right_click(ed.editor_point("#slides li[data-id=b]", Some(20.0)).await)
        .await;
    let labels = ed.menu_labels().await;
    assert!(
        labels.iter().any(|x| x == "Hidden in the presentation"),
        "{labels:?}"
    );
    ed.click_menu("Transition").await;
    ed.click_menu("Fade").await;
    ed.wait_file(
        |f| f.contains("## B {#b transition=fade}"),
        "transition set",
    )
    .await;

    // Shift+F10 opens the menu for the selected element
    ed.settle().await;
    ed.go("a").await;
    ed.select("[data-qd=\"a/3\"]").await; // the list, now last
    ed.key("F10", "F10", 121, 8).await;
    assert!(
        ed.menu_labels()
            .await
            .iter()
            .any(|x| x == "Move down (disabled)"),
        "last element"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bars_set_properties_and_source_panel_is_hidden() {
    let Some(ed) = Ed::open(P2).await else { return };
    ed.go("a").await;
    assert_eq!(
        ed.eval("document.querySelector('main').classList.contains('panel-open')")
            .await,
        false
    );
    assert_eq!(
        ed.eval("getComputedStyle(document.querySelector('#panel')).display")
            .await,
        "none"
    );
    // slide bar: hidden toggle = visibility attribute
    ed.wait("document.querySelector('#slidebar').style.visibility !== 'hidden'")
        .await;
    ed.click_editor("#slidebar [name=hidden]").await;
    ed.wait_file(|f| f.contains("## A {#a visibility=hidden}"), "hidden")
        .await;
    ed.settle().await;
    ed.wait("document.querySelector('#slidebar [name=hidden]').checked")
        .await;
    ed.click_editor("#slidebar [name=hidden]").await;
    ed.wait_file(|f| f == P2, "unhidden, bytes as before").await;
    // element bar: "On click" = .fragment, and back again byte-identical
    for (sel, want) in [
        ("[data-qd=\"a/1\"] p", "::: {.fragment}\nSecond para.\n:::"),
        ("[data-qd=\"a/1\"]", "\n\nSecond para.\n\n"),
    ] {
        ed.settle().await;
        ed.go("a").await;
        ed.select(sel).await;
        ed.click_editor("#elbar [data-act=fragment]").await;
        ed.wait_file(|f| f.contains(want), "fragment toggled").await;
    }
    assert_eq!(ed.file(), P2, "fragment removed, bytes as before");
    // the Source button shows the source pane; its Form button the old properties panel
    ed.toggle_source().await;
    ed.wait("!document.querySelector('#srcpane').hidden").await;
    // the element bar follows the element when the layout moves; it never sits over other controls
    ed.wait("(() => { const b = document.querySelector('#elbar'); if (b.hidden) return true; const r = b.getBoundingClientRect(), f = document.querySelector('#srcpane [data-act=form]').getBoundingClientRect(); return r.right < f.left || f.right < r.left || r.bottom < f.top || f.bottom < r.top; })()").await;
    ed.eval("window.__clicked = []; document.addEventListener('click', (e) => __clicked.push(e.target.outerHTML.slice(0, 60)), true)").await;
    ed.click_editor("#srcpane [data-act=form]").await;
    let opened = ed
        .eval("new Promise((r) => { const t = Date.now(); (function f() { if (getComputedStyle(document.querySelector('#panel')).display === 'block') r(true); else if (Date.now() - t > 5000) r(false); else setTimeout(f, 50); })(); })")
        .await;
    assert_eq!(
        opened,
        true,
        "the Form button did not open the panel: main={} clicked={} under the button now={}",
        ed.eval("document.querySelector('main').className").await,
        ed.eval("JSON.stringify(window.__clicked)").await,
        ed.eval("(() => { const r = document.querySelector('#srcpane [data-act=form]').getBoundingClientRect(); return document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)?.outerHTML.slice(0, 80); })()").await,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_drag_delete_and_empty_slides() {
    let Some(ed) = Ed::open(P2).await else { return };
    ed.go("a").await;
    // hover → + → List: a new list opens for typing with its placeholder selected
    // (between the paragraphs: a list right after a list would join it, as in markdown)
    ed.hover("[data-qd=\"a/0\"] p").await;
    ed.click_editor("#hoverbar [data-act=insert]").await;
    ed.click_menu("List").await;
    ed.wait_secs("window.qdSession()?.ref === 'a/1'", 30).await;
    ed.type_text("typed item").await;
    ed.key("Enter", "Enter", 13, 2).await;
    ed.wait_file(
        |f| f.contains("First para.\n\n- typed item\n\nSecond para."),
        "inserted",
    )
    .await;

    // drag the x/y list (a/3) above the first paragraph
    ed.settle().await;
    ed.go("a").await;
    ed.hover("[data-qd=\"a/3\"]").await;
    let handle = ed.editor_point("#hoverbar [data-act=drag]", None).await;
    let (fx, fy) = ed.slide_point("[data-qd=\"a/0\"]").await;
    ed.drag(handle, (fx, fy - 8.0)).await;
    ed.wait_file(
        |f| f.contains("## A {#a}\n\n- x\n- y\n\nFirst para."),
        "moved",
    )
    .await;

    // Delete key on the selected element = `qd block rm`
    ed.settle().await;
    ed.go("a").await;
    ed.select("[data-qd=\"a/0\"]").await;
    ed.key("Delete", "Delete", 46, 0).await;
    ed.wait_file(|f| !f.contains("- x\n- y"), "deleted").await;

    // an empty slide offers its first element
    ed.settle().await;
    ed.go("empty").await;
    ed.wait("!document.querySelector('#emptyadd').hidden").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_and_paste_carry_the_markdown() {
    let Some(ed) = Ed::open(P2).await else { return };
    ed.go("a").await;
    ed.right_click(ed.slide_point("[data-qd=\"a/2\"]").await)
        .await;
    ed.click_menu("Copy").await;
    ed.go("b").await;
    ed.right_click(ed.slide_point("[data-qd=\"b/0\"] p").await)
        .await;
    ed.click_menu("Paste after").await;
    ed.wait_file(
        |f| f.contains("Other slide.\n\n- x\n- y\n\n## Empty"),
        "pasted byte-for-byte",
    )
    .await;
    // cut = copy + remove
    ed.settle().await;
    ed.go("a").await;
    ed.right_click(ed.slide_point("[data-qd=\"a/0\"] p").await)
        .await;
    ed.click_menu("Cut").await;
    ed.wait_file(|f| !f.contains("First para."), "cut").await;
    let clip = ed
        .eval("navigator.clipboard.readText().catch(() => null)")
        .await;
    assert_eq!(clip.as_str().unwrap_or("First para."), "First para.");
}

// ---------- polish: fragments, notes, keyboard selection (T032, T033) ----------

#[tokio::test(flavor = "multi_thread")]
async fn fragments_notes_and_keyboard_selection() {
    let deck = "---\ntitle: T\nformat: revealjs\n---\n\n## F {#f}\n\n::: {.fragment}\nLater.\n:::\n\nNow.\n";
    let Some(ed) = Ed::open(deck).await else {
        return;
    };
    ed.go("f").await;
    // every fragment is visible (and clickable) in the editor
    let opacity = "getComputedStyle(document.querySelector('#preview').contentDocument.querySelector('.fragment')).opacity";
    assert_eq!(ed.eval(opacity).await, "1");

    // Tab / Shift+Tab move the selection; Enter edits; Escape leaves the edit, keeping the selection
    ed.click_blank().await;
    for (shift, want) in [(0, "f/0"), (0, "f/1"), (8, "f/0")] {
        ed.key("Tab", "Tab", 9, shift).await; // 8 = Shift+Tab
        ed.wait(&format!("{SEL} === '{want}'")).await;
    }
    ed.wait("document.querySelector('#notice').textContent.startsWith('Selected')")
        .await;
    ed.key("Enter", "Enter", 13, 0).await;
    ed.wait("window.qdSession()?.ref === 'f/0'").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!("window.qdSession() === null && {SEL} === 'f/0'"))
        .await;

    // Speaker notes…: created when missing, then edited in the source box
    ed.right_click(ed.editor_point("#slides li[data-id=f]", Some(20.0)).await)
        .await;
    ed.click_menu("Speaker notes…").await;
    ed.wait_file(
        |f| f.ends_with("Now.\n\n::: notes\nSpeaker notes.\n:::\n"),
        "notes added",
    )
    .await;
    ed.src_open().await;
    assert_eq!(
        ed.eval(&format!("{SRC}.value")).await,
        "::: notes\nSpeaker notes.\n:::"
    );
    ed.set_source("::: notes\nSay hello first.\n:::").await;
    ed.key("Enter", "Enter", 13, 2).await;
    ed.wait_file(|f| f.contains("Say hello first."), "notes saved")
        .await;
}

// ---------- spec 004: source pane and sync (T008, T011) ----------

#[tokio::test(flavor = "multi_thread")]
async fn source_pane_saves_minimal_changes() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.open_source().await;
    assert_eq!(
        ed.eval(&format!("{PANE}.value")).await.as_str().unwrap(),
        DECK
    );
    // a word in a paragraph and a value in the YAML, each saved after a pause
    ed.pane_caret("two", true).await;
    ed.type_text("2").await;
    ed.wait_file(|f| f.contains("across 2 \"quoted\""), "paragraph saved")
        .await;
    ed.pane_caret("title: T", false).await;
    ed.type_text("alk").await;
    ed.wait_file(|f| f.contains("title: Talk\n"), "yaml saved")
        .await;
    assert_eq!(
        ed.file(),
        DECK.replace("across two", "across 2")
            .replace("title: T\n", "title: Talk\n")
    );
    // undo restores the bytes, and the pane follows
    ed.menu("Edit", "Undo").await;
    ed.wait_file(|f| f.contains("title: T\n"), "undone").await;
    ed.wait(&format!("{PANE}.value.includes('title: T\\n')"))
        .await;
    // an agent's edit elsewhere merges into unsaved text without losing it
    ed.pane_caret("- one", false).await;
    ed.type_text(" (mine)").await;
    ed.cli(&["title", "results", "Findings"]);
    ed.wait_file(
        |f| f.contains("- one (mine)") && f.contains("## Findings {#results}"),
        "both kept",
    )
    .await;
    ed.wait(&format!(
        "{PANE}.value.includes('## Findings') && {PANE}.value.includes('- one (mine)')"
    ))
    .await;
    // the same line changed by both: the conflict bar shows both versions
    ed.settle().await;
    ed.pane_caret("- two", false).await;
    ed.type_text(" (pane)").await;
    ed.cli(&["replace", "--old", "- two\n", "--new", "- two (agent)\n"]);
    ed.wait("!document.querySelector('#conflict').hidden").await;
    let bar = ed
        .eval("document.querySelector('#conflict').textContent")
        .await;
    let bar = bar.as_str().unwrap();
    assert!(bar.contains("(agent)") && bar.contains("(pane)"), "{bar}");
    ed.click_editor("#conflict [data-act=mine]").await;
    ed.wait_file(
        |f| f.contains("- two (pane)\n") && !f.contains("(agent)"),
        "mine applied",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn source_and_slides_follow_each_other() {
    let Some(ed) = Ed::open(DECK).await else {
        return;
    };
    ed.open_source().await;
    // caret in the Results slide's text shows that slide; focus stays in the pane
    ed.pane_caret("Text.", false).await;
    ed.wait("document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide().id === 'results'").await;
    assert_eq!(
        ed.eval(&format!("document.activeElement === {PANE}")).await,
        true
    );
    // selecting an element on a slide highlights its lines in the pane
    ed.go("methods").await;
    let lines = ed.cli(&["show", "methods"])["slide"]["blocks"][1]["lines"].clone();
    ed.select("[data-qd=\"methods/1\"]").await;
    let want = format!("{}-{}", lines[0], lines[1]);
    ed.wait(&format!(
        "document.querySelector('#srcpane .band').dataset.lines === {want:?}"
    ))
    .await;
    // full width hides the slides
    ed.click_editor("#srcpane [data-act=full]").await;
    assert_eq!(
        ed.eval("getComputedStyle(document.querySelector('#stage')).display")
            .await,
        "none"
    );
}

const RUN_DECK: &str = "---\ntitle: R\nformat: revealjs\n---\n\n## Print {#print}\n\n```{python}\nprint(6 * 7)\n```\n\n## Fails {#fails}\n\n```{python}\n1 / 0\n```\n\n## Slow {#slow}\n\n```{python}\nimport time\ntime.sleep(600)\n```\n";

/// Spec 004 US3 (T027): Run from the source pane, outputs in its dock and on
/// the slide, ⇧Enter, Stop; the deck file never changes. Needs uv too.
#[tokio::test(flavor = "multi_thread")]
async fn run_cells_in_the_editor() {
    if !have("uv") {
        return;
    }
    let Some(ed) = Ed::open(RUN_DECK).await else {
        return;
    };
    ed.cli(&["env", "init"]);
    ed.open_source().await;
    let dock = "document.querySelector('#srcpane .outdock')";
    ed.wait("document.querySelectorAll('#srcpane .gutter button').length === 3")
        .await;

    // the token came in the link's fragment: gone from the address bar and the page
    assert_eq!(
        ed.eval("!location.href.includes('t=') && !document.querySelector('meta[name=qd-token]') && !!sessionStorage.getItem('qd-token')").await,
        true
    );
    ed.click_editor("#srcpane .gutter button[data-cell=\"print/0\"]")
        .await;
    ed.wait_secs(&format!("{dock}.textContent.includes('42')"), 120)
        .await;
    // and a reload in the same tab can still run code
    ed.page.reload().await.unwrap();
    ed.wait_secs(READY, 60).await;
    if ed.eval("document.querySelector('#srcpane').hidden").await == true {
        ed.open_source().await; // (the editor remembers an open pane across reloads)
    }
    ed.wait("document.querySelectorAll('#srcpane .gutter button').length === 3")
        .await;
    ed.click_editor("#srcpane .gutter button[data-cell=\"print/0\"]")
        .await;
    ed.wait_secs(
        &format!("{dock}.querySelector('.head')?.textContent.includes('[2]')"),
        60,
    )
    .await;
    // the slide shows the stored output, rendered without running anything
    ed.go("print").await;
    ed.wait_secs(
        "document.querySelector('#preview').contentDocument.querySelector('.qd-output')?.textContent.includes('42')",
        60,
    )
    .await;

    // ⇧Enter runs the cell under the caret (and types no newline)
    ed.pane_caret("1 / 0", false).await;
    ed.key("Enter", "Enter", 13, 8).await;
    ed.wait_secs(
        &format!("{dock}.querySelector('pre.error')?.textContent.includes('ZeroDivisionError')"),
        60,
    )
    .await;
    ed.wait("document.querySelector('#srcpane .gutter button[data-cell=\"fails/0\"]').classList.contains('err')")
        .await;

    // Stop interrupts a running cell
    ed.pane_caret("time.sleep", false).await;
    ed.key("Enter", "Enter", 13, 8).await;
    ed.wait("document.querySelector('#srcpane .kernel').dataset.state === 'busy'")
        .await;
    ed.click_editor("#srcpane [data-act=stop]").await;
    ed.wait_secs(
        &format!("{dock}.textContent.includes('KeyboardInterrupt')"),
        30,
    )
    .await;
    assert_eq!(ed.file(), RUN_DECK, "running never changes the deck");
}

/// Spec 004 US5 (T032): outputs survive a restart of `qd serve` and show in
/// the pane and on the slide without running anything; an edited cell's
/// output is marked stale in both. Needs uv too.
#[tokio::test(flavor = "multi_thread")]
async fn outputs_persist_across_restarts_and_go_stale() {
    if !have("uv") {
        return;
    }
    let Some(ed) = Ed::open(RUN_DECK).await else {
        return;
    };
    ed.cli(&["env", "init"]);
    let run = ed.cli(&["run", "print/0"]);
    assert_eq!(run["runs"][0]["outputs"][0]["text"], "42\n", "{run}");

    // a new server (new port, new token), as when the deck is reopened
    ed.cli(&["stop"]);
    let info = ed.cli(&["serve", "--detach"]);
    ed.page.goto(info["url"].as_str().unwrap()).await.unwrap();
    ed.wait_secs(READY, 60).await;
    ed.go("print").await;
    let frame = "document.querySelector('#preview').contentDocument";
    ed.wait_secs(
        &format!("{frame}.querySelector('.qd-output')?.textContent.includes('42')"),
        60,
    )
    .await;
    if ed.eval("document.querySelector('#srcpane').hidden").await == true {
        ed.open_source().await;
    }
    let print = "document.querySelector('#srcpane .gutter button[data-cell=\"print/0\"]')";
    ed.wait(&format!("{print}?.textContent === '▶ 1'")).await;
    ed.pane_caret("print(6", false).await;
    let dock = "document.querySelector('#srcpane .outdock')";
    ed.wait(&format!("{dock}.textContent.includes('42')")).await;
    assert_eq!(
        ed.eval("document.querySelector('#srcpane .kernel').dataset.state ?? 'none'")
            .await,
        "none",
        "nothing ran: no kernel was started"
    );

    // an edit marks the stored output stale on the slide and in the pane
    ed.cli(&["replace", "--old", "6 * 7", "--new", "6 * 8"]);
    ed.wait_secs(
        &format!("!!{frame}.querySelector('.qd-output.qd-stale')"),
        60,
    )
    .await;
    ed.wait(&format!("{print}.classList.contains('stale')"))
        .await;
    ed.pane_caret("print(6", false).await;
    ed.wait(&format!("{dock}.classList.contains('stale')"))
        .await;
}

/// Spec 004 SC-006 (T035): a page from another local origin, open in the same
/// browser as the editor, tries every way a page can reach the editor's
/// server — JSON fetch with a guessed token, no-cors posts, plain form posts —
/// against the code-running and editing endpoints, and to read the page for
/// the token. Nothing runs, nothing changes, the server stays up. Needs uv.
#[tokio::test(flavor = "multi_thread")]
async fn another_origin_cannot_run_code_or_edit() {
    if !have("uv") {
        return;
    }
    let Some(ed) = Ed::open(RUN_DECK).await else {
        return;
    };
    ed.cli(&["env", "init"]);
    let manifest = || std::fs::read(ed.dir.path().join("pyproject.toml")).unwrap();
    let before = manifest();
    let port = ed.eval("location.port").await;

    // the hostile page: same machine, another port = another origin
    let evil = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let evil_port = evil.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in evil.incoming() {
            let mut s = s.unwrap();
            let _ = s.read(&mut [0; 4096]);
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<iframe name=\"sink\"></iframe>");
        }
    });
    let attacker = ed
        ._browser
        .new_page(format!("http://127.0.0.1:{evil_port}/").as_str())
        .await
        .unwrap();
    let attack = format!(
        r#"(async () => {{
const qd = 'http://127.0.0.1:{port}';
const bodies = {{
  '/api/run': JSON.stringify({{all: true}}),
  '/api/env/init': '{{}}',
  '/api/env/add': JSON.stringify({{packages: ['six']}}),
  '/api/op': JSON.stringify({{op: 'replace', args: {{old: '6 * 7', new: '6 * 9'}}}}),
  '/api/shutdown': '{{}}',
  '/api/focus': JSON.stringify({{tab: 'evil', version: 'v', slide: 'slow', elements: [], selection: {{text: 'planted', where: 'slide'}}, line: null, editing: false}}),
  '/api/focus/set': JSON.stringify({{ref: 'slow'}}),
  '/api/buffer/open': JSON.stringify({{text: '## Pwned\\n'}}),
  '/api/buffer': JSON.stringify({{base: '', text: '## Pwned\\n'}}),
  '/api/buffer/close': '{{}}',
  '/api/settings': JSON.stringify({{theme: 'dark', 'source.side': 'right', 'confirm.slide_delete': false}}),
  '/api/images?name=evil.svg': '<svg xmlns="http://www.w3.org/2000/svg"><script>parent.pwned = 1</script></svg>',
}};
for (const [path, body] of Object.entries(bodies)) {{
  await fetch(qd + path, {{method: 'POST', headers: {{'Content-Type': 'application/json', 'X-QD-Token': 'guess'}}, body}}).catch(() => {{}});
  await fetch(qd + path, {{method: 'POST', mode: 'no-cors', headers: {{'Content-Type': 'text/plain'}}, body}}).catch(() => {{}});
  const f = Object.assign(document.createElement('form'), {{method: 'POST', action: qd + path, enctype: 'text/plain', target: 'sink'}});
  f.append(Object.assign(document.createElement('input'), {{name: body, value: ''}}));
  document.body.append(f);
  f.submit();
  await new Promise((r) => setTimeout(r, 300));
}}
// the editor page (and any token in it) and the focus must be unreadable from here
const unreadable = async (p) => (await fetch(qd + p).then((r) => r.text()).catch(() => null)) === null;
return (await unreadable('/')) && (await unreadable('/api/focus'));
}})()"#
    );
    assert_eq!(
        eval(&attacker, &attack).await,
        Value::Bool(true),
        "the editor page and the focus are unreadable cross-origin"
    );
    tokio::time::sleep(Duration::from_secs(3)).await; // anything that got through would show by now

    assert_eq!(ed.file(), RUN_DECK, "no edit landed");
    // spec 008: no file was written beside the deck
    assert!(!ed.dir.path().join("images").exists(), "no image added");
    // spec 007: the person's settings are unchanged
    let v = &ed.cli(&["settings"])["values"];
    assert_eq!(
        (&v["theme"], &v["source.side"], &v["confirm.slide_delete"]),
        (&json!("system"), &json!("left"), &json!(true)),
        "no setting changed"
    );
    // spec 006: no VS Code buffer was attached in the person's place
    assert!(
        !ed.cli(&["slides"]).to_string().contains("Pwned"),
        "qd still reads the file, not a planted buffer"
    );
    assert_eq!(manifest(), before, "no package added");
    assert_eq!(ed.cli(&["outputs"])["outputs"], json!([]), "nothing ran");
    assert_eq!(
        ed.eval("document.querySelector('#srcpane .kernel').dataset.state ?? 'none'")
            .await,
        "none",
        "no kernel started"
    );
    assert!(
        ed.cli(&["url"])["url"].is_string(),
        "the server is still up"
    );
    // spec 005: the focus can be neither planted nor used to move the editor
    assert_ne!(ed.cli(&["focus"])["selection"]["text"], "planted");
    assert_ne!(
        ed.eval("document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide().id")
            .await,
        "slow",
        "the editor was not pointed anywhere"
    );
}

/// Spec 005 US1 (T008): the focus follows the person — slide, clicked element,
/// text selected while typing, a source selection spanning blocks — and
/// `qd focus` reads it; a later edit shows in the versions; no editor = closed.
#[tokio::test(flavor = "multi_thread")]
async fn focus_follows_the_person() {
    let Some(ed) = Ed::open(P2).await else { return };
    // `qd focus` until `pred` holds; returns it and how long that took (SC-001)
    let until = |pred: &dyn Fn(&Value) -> bool| {
        let start = Instant::now();
        loop {
            let v = ed.cli(&["focus"]);
            if pred(&v) {
                return (v, start.elapsed());
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "focus never matched: {v}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    };

    ed.go("b").await;
    let (v, t) = until(&|v| v["slide"] == "b");
    assert!(v["open"] == true && v["elements"] == json!([]), "{v}");
    eprintln!("slide → qd focus in {t:?}");

    ed.select("[data-qd=\"b/0\"]").await;
    let (_, t) = until(&|v| v["elements"] == json!(["b/0"]));
    eprintln!("element → qd focus in {t:?}");

    // words selected while typing in a paragraph
    ed.go("a").await;
    ed.click_in_slide("[data-qd=\"a/0\"] p").await;
    ed.caret("[data-qd=\"a/0\"]", Some("First")).await;
    let (v, t) = until(&|v| v["selection"]["text"] == "First");
    assert!(
        v["selection"]["where"] == "slide"
            && v["elements"] == json!(["a/0"])
            && v["editing"] == true,
        "{v}"
    );
    eprintln!("slide selection → qd focus in {t:?}");
    ed.click_blank().await;
    until(&|v| v["editing"] == false && v["selection"].is_null());

    // a source selection across two blocks: both, where = source, the caret's line
    ed.open_source().await;
    ed.pane_caret("First para.\n\nSecond para.", true).await;
    let (v, t) = until(&|v| v["selection"]["where"] == "source");
    assert_eq!(v["elements"], json!(["a/0", "a/1"]), "{v}");
    assert_eq!(v["selection"]["text"], "First para.\n\nSecond para.");
    assert!(v["line"].as_u64().is_some_and(|l| l > 1), "{v}");
    eprintln!("source selection → qd focus in {t:?}");

    // the caret into another slide's text: the editor follows it, and the
    // focus stays the source pane's (line kept, that slide's block)
    ed.pane_caret("Other slide", false).await;
    let (v, _) = until(&|v| v["slide"] == "b" && v["line"].is_u64());
    assert_eq!(v["elements"], json!(["b/0"]), "{v}");
    ed.wait("document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide().id === 'b'")
        .await;
    std::thread::sleep(Duration::from_millis(400)); // the slide change must not reset it
    let v = ed.cli(&["focus"]);
    assert!(v["line"].is_u64() && v["elements"] == json!(["b/0"]), "{v}");
    ed.pane_caret("First para.\n\nSecond para.", true).await;
    until(&|v| v["selection"]["where"] == "source");

    // an edit elsewhere: the focus says which version it was taken against
    ed.cli(&["title", "b", "B two"]);
    let v = ed.cli(&["focus"]);
    assert_ne!(v["version"], v["deck_version"], "{v}");
    assert_eq!(v["elements"], json!(["a/0", "a/1"]), "refs still resolve");

    ed.cli(&["stop"]);
    assert_eq!(ed.cli(&["focus"]), json!({"open": false}));
}

/// Spec 005 US2 (T015): `qd focus --set` shows the slide and flashes the
/// element (and its source lines); while the person types, 20 requests are
/// deferred, nothing moves, nothing typed is lost, and a "Go to …" offer waits.
#[tokio::test(flavor = "multi_thread")]
async fn pointing_moves_or_offers() {
    let Some(ed) = Ed::open(P2).await else { return };
    let current = |id: &str| {
        format!(
            "document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide().id === '{id}'"
        )
    };
    ed.go("a").await;
    ed.open_source().await;
    // the server knows this tab once it has reported
    ed.until_cli(&["focus"], |v| v["age"].is_number(), "tab never reported")
        .await;

    let t = Instant::now();
    assert_eq!(ed.cli(&["focus", "--set", "b/0"])["result"], "moved");
    ed.wait(&current("b")).await;
    eprintln!("--set → slide shown in {:?}", t.elapsed()); // SC-002
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(
        ed.cli(&["focus"])["slide"],
        "a",
        "where an agent points isn't the person's focus"
    );
    ed.wait("document.querySelector('#preview').contentDocument.querySelector('[data-qd=\"b/0\"]').getAnimations().length > 0")
        .await;
    ed.wait("document.querySelector('#srcpane .band').dataset.lines === '17-17'")
        .await;

    // typing: every request is deferred, the view stays, the text arrives whole (SC-003)
    ed.go("a").await;
    ed.click_in_slide("[data-qd=\"a/0\"] p").await;
    ed.caret("[data-qd=\"a/0\"]", None).await;
    ed.type_text(" abc").await;
    ed.until_cli(
        &["focus"],
        |v| v["editing"] == true,
        "editing never reported",
    )
    .await;
    for _ in 0..20 {
        assert_eq!(ed.cli(&["focus", "--set", "b"])["result"], "deferred");
    }
    ed.type_text("def").await;
    assert_eq!(ed.eval(&current("a")).await, true, "the view never moved");
    ed.wait("!document.querySelector('#gooffer').hidden").await;
    ed.click_blank().await;
    ed.wait_file(|f| f.contains("First para. abcdef"), "typed in full")
        .await;
    ed.click_editor("#gooffer [data-act=go]").await;
    ed.wait(&current("b")).await;
    assert_eq!(
        ed.eval("document.querySelector('#gooffer').hidden").await,
        true
    );
}

/// Spec 006 (T013): the editor framed by VS Code takes its theme from the
/// query, then from messages — only its parent's — and hides Run when the
/// Quarto extension runs cells.
#[tokio::test(flavor = "multi_thread")]
async fn framed_editor_takes_theme_and_messages() {
    let Some(ed) = Ed::open(P2).await else { return };
    let link = ed.cli(&["url"])["url"].as_str().unwrap().to_string();
    let framed = |q: &str| link.replace("/#t=", &format!("/?host=vscode&{q}#t="));
    let theme = "document.documentElement.dataset.theme";
    let bg = "getComputedStyle(document.documentElement).getPropertyValue('--bg').trim()";

    ed.page.goto(framed("theme=dark").as_str()).await.unwrap();
    ed.wait_secs(READY, 60).await;
    assert_eq!(ed.eval(theme).await, "dark");
    assert_eq!(ed.eval(bg).await, "#18181a");

    // here the page is its own parent: its messages count, a child frame's don't
    ed.eval("window.postMessage({type: 'theme', kind: 'hc'}, '*')")
        .await;
    ed.wait(&format!("{theme} === 'hc'")).await;
    ed.eval("(() => { const f = document.createElement('iframe'); f.srcdoc = `<script>parent.postMessage({type: 'theme', kind: 'light'}, '*')</script>`; document.body.append(f); })()").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        ed.eval(theme).await,
        "hc",
        "a stranger's message is ignored"
    );
    assert_eq!(ed.eval(bg).await, "#000000");

    // US5: no menu bar, one ☰ holding the same menus; F10 reaches it
    let shown =
        |m: &str| format!("!!document.querySelector('#menubar [data-menu={m}]').offsetParent");
    assert_eq!(ed.eval(&shown("File")).await, false);
    assert_eq!(ed.eval(&shown("Menu")).await, true);
    ed.go("a").await;
    ed.click_editor("#menubar [data-menu=Menu]").await;
    assert_eq!(
        ed.menu_labels().await,
        ["File", "Edit", "Insert", "Slide", "View", "Run", "Help"]
    );
    ed.click_menu("Slide").await;
    ed.click_menu("Duplicate slide").await;
    ed.wait_file(|f| f.matches("First para.").count() == 2, "ran from ☰")
        .await;
    ed.cli(&["undo"]);
    ed.settle().await;
    ed.key("F10", "F10", 121, 0).await;
    ed.wait("document.activeElement?.dataset.menu === 'Menu'")
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    // VS Code's keys stay VS Code's: ⌘B is not Bold here, and the menus and the
    // reference show only the panel's keys
    let resolve = |k: &str, place: &str| format!("window.qdActions().resolve({k:?}, [{place:?}])");
    assert_eq!(ed.eval(&resolve("Mod+B", "text")).await, Value::Null);
    assert_eq!(ed.eval(&resolve("Mod+Shift+D", "slide")).await, Value::Null);
    assert_eq!(ed.eval(&resolve("Mod+I", "text")).await, "italic");
    let dup = ed
        .eval("qdMenus().then((m) => m.Slide.find((i) => i.label === 'Duplicate slide'))")
        .await;
    assert_eq!(dup["shortcut"], Value::Null, "{dup}");

    // VS Code's cursor in slide B's text shows B, and qd focus has it (US3)
    ed.go("a").await;
    ed.eval("window.postMessage({type: 'follow', line: 17, first: 17, last: 17, text: ''}, '*')")
        .await;
    ed.wait("document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide().id === 'b'")
        .await;
    ed.until_cli(&["focus"], |v| v["line"] == 17, "focus never got the line")
        .await;
    // an element chosen on a slide: its source lines go to VS Code
    // what the editor posts to its parent (here itself): reveal, focusText
    ed.eval("window.__msgs = []; addEventListener('message', (e) => __msgs.push(e.data))")
        .await;
    ed.go("a").await;
    ed.click_in_slide("[data-qd=\"a/0\"] p").await;
    ed.wait("JSON.stringify(__msgs.findLast((m) => m.type === 'reveal')?.lines) === '[8,8]'")
        .await;
    let focus_text = "__msgs.filter((m) => m.type === 'focusText').length";
    // Esc clears the selection first; Esc with nothing left hands the keyboard to the text
    let esc = || ed.key("Escape", "Escape", 27, 0);
    ed.eval(EVENT_LOG).await;
    ed.eval("(() => { const f = document.querySelector('#preview').contentWindow; const log = (w) => (e) => __ev.push(`${w} key ${e.key} -> ${e.target.tagName} sel=${document.querySelector('#preview').contentDocument.querySelector('.qd-sel')?.dataset.qd} src=${!document.querySelector('#srcpane').hidden}`); f.addEventListener('keydown', log('frame'), true); addEventListener('keydown', log('page'), true); f.addEventListener('unload', () => __ev.push('frame unload')); })()").await;
    ed.wait("window.qdSession() !== null").await;
    esc().await; // ends the edit the click started (the element stays selected)
    ed.wait(&format!("window.qdSession() === null && {SEL} === 'a/0'"))
        .await;
    esc().await; // clears the selection
    ed.wait(&format!("!{SEL}")).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        ed.eval(focus_text).await,
        0,
        "Esc ended the edit, then cleared the selection"
    );
    esc().await;
    ed.wait(&format!("{focus_text} === 1")).await;

    // the Quarto extension runs cells: no Run bar, no gutter
    ed.page
        .goto(framed("theme=light&run=external").as_str())
        .await
        .unwrap();
    ed.wait_secs(READY, 60).await;
    ed.open_source().await;
    assert_eq!(
        ed.eval("['.runbar', '.gutter'].map((s) => getComputedStyle(document.querySelector('#srcpane ' + s)).display)").await,
        json!(["none", "none"])
    );
    assert_eq!(ed.eval(theme).await, "light");
}

// ---------- spec 007: one action list, the menu bar ----------

/// Every listed key, in each place it works, resolves to its own action.
const KEYS_RESOLVE: &str = "(async () => { const { ACTIONS, keysOf } = await import('/ui/keys.js'); const bad = []; for (const a of ACTIONS) for (const k of keysOf(a)) for (const w of a.where) { const r = qdActions().resolve(k, [w]); if (r !== a.id) bad.push([a.id, k, w, r]); } return bad; })()";

/// Spec 007 US1 (T005, T007): the menu bar holds every action. Run from a menu
/// it does what its button, key or right-click item does; the menus work from
/// the keyboard alone and carry the menu roles screen readers need.
#[tokio::test(flavor = "multi_thread")]
async fn menu_bar_runs_every_action() {
    let Some(ed) = Ed::open(P2).await else { return };
    assert_eq!(
        ed.eval("qdActions().missing").await,
        json!([]),
        "every action has a handler"
    );
    assert_eq!(
        ed.eval(KEYS_RESOLVE).await,
        json!([]),
        "every key reaches its action"
    );
    assert_eq!(
        ed.eval("qdMenus().then(Object.keys)").await,
        json!(["File", "Edit", "Insert", "Slide", "View", "Run", "Help"])
    );
    assert_eq!(
        ed.eval("[...document.querySelectorAll('#menubar[role=menubar] > [role=menuitem][aria-haspopup=menu]')].filter((b) => b.offsetParent).length").await,
        7
    );
    // clicking an open menu's name again closes it
    let shown = "!document.querySelector('#ctxmenu').hidden";
    let insert = "#menubar [data-menu=Insert]";
    ed.click_editor(insert).await;
    ed.wait(shown).await;
    ed.click_editor(insert).await;
    ed.wait(&format!("!({shown})")).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        ed.eval(shown).await,
        false,
        "a second click closes the menu"
    );
    ed.click_editor(insert).await;
    ed.wait(shown).await;
    // moving onto another name while a menu is open switches to it (as on macOS)
    let (x, y) = ed.editor_point("#menubar [data-menu=View]", None).await;
    ed.page.move_mouse(Point { x, y }).await.unwrap();
    ed.wait("document.querySelector('#menubar [data-menu=View]').getAttribute('aria-expanded') === 'true'")
        .await;
    assert_eq!(ed.eval(shown).await, true, "View's menu is open");
    ed.close_menu().await;
    // a top row too narrow for the menus: they fold into ☰ (and come back when there is room)
    let folded = "document.querySelector('#menubar').classList.contains('folded')";
    ed.eval("document.querySelector('header').style.width = '260px'")
        .await;
    ed.wait(folded).await;
    ed.eval("document.querySelector('header').style.width = ''")
        .await;
    ed.wait(&format!("!{folded}")).await;

    // Slide → Duplicate slide, then Edit → Undo
    ed.go("a").await;
    ed.menu("Slide", "Duplicate slide").await;
    ed.wait_file(|f| f.matches("## A").count() == 2, "duplicated")
        .await;
    ed.menu("Edit", "Undo").await;
    ed.wait_file(|f| f == P2, "undone").await;

    // with an element selected: Paste shows disabled (nothing copied yet), Insert goes after it
    ed.go("a").await;
    ed.select("[data-qd=\"a/0\"]").await;
    let paste = ed
        .eval("qdMenus().then((m) => m.Edit.find((i) => i.label === 'Paste after'))")
        .await;
    assert_eq!(paste["disabled"], true, "{paste}");
    ed.menu("Insert", "Paragraph").await;
    ed.wait_file(
        |f| f.contains("First para.\n\nNew paragraph."),
        "inserted after the selection",
    )
    .await;

    ed.wait("window.qdSession()?.ref === 'a/1'").await; // it opens for typing
    ed.click_blank().await;
    ed.wait("window.qdSession() === null").await;
    ed.settle().await;
    ed.go("a").await;

    // while typing on the slide, Edit → Format → Bold applies and keeps the edit open
    ed.click_in_slide("[data-qd=\"a/0\"]").await;
    ed.caret("[data-qd=\"a/0\"]", Some("First")).await;
    ed.menu("Edit", "Format/Bold").await;
    ed.wait("window.qdSession()?.draft?.includes('**First**')")
        .await;
    ed.key("Enter", "Enter", 13, 2).await; // Ctrl+Enter saves
    ed.wait_file(|f| f.contains("**First** para."), "bold saved")
        .await;

    // View → Source pane is a toggle, checked once open
    ed.menu("View", "Source pane").await;
    ed.wait("!document.querySelector('#srcpane').hidden").await;
    let view = ed
        .eval("qdMenus().then((m) => m.View.find((i) => i.label === 'Source pane'))")
        .await;
    assert_eq!(view["checked"], true, "{view}");
    ed.menu("View", "Source pane").await;
    ed.wait("document.querySelector('#srcpane').hidden").await;

    // keyboard only: F10, arrows, Enter, Escape; focus comes back to where it was
    let active = "(document.activeElement.dataset.menu || document.activeElement.id || document.activeElement.textContent.trim().split('\\n')[0])";
    ed.go("a").await;
    ed.eval("document.querySelector('#add-slide').focus()")
        .await;
    ed.key("F10", "F10", 121, 0).await;
    ed.wait(&format!("{active} === 'File'")).await;
    for _ in 0..3 {
        ed.key("ArrowRight", "ArrowRight", 39, 0).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    ed.wait(&format!("{active} === 'Slide'")).await;
    ed.key("ArrowDown", "ArrowDown", 40, 0).await;
    ed.wait("document.activeElement.closest('#ctxmenu') && document.activeElement.firstChild.textContent === 'Duplicate slide'").await;
    assert!(
        ed.eval("document.activeElement.getAttribute('aria-keyshortcuts')")
            .await
            .as_str()
            .is_some_and(|k| k.ends_with("Shift+D")),
        "the shortcut is announced"
    );
    assert_eq!(
        ed.eval(
            "document.querySelector('#menubar [data-menu=Slide]').getAttribute('aria-expanded')"
        )
        .await,
        "true"
    );
    ed.key("Enter", "Enter", 13, 0).await;
    ed.wait_file(
        |f| f.matches("## A").count() == 2,
        "duplicated from the keyboard",
    )
    .await;
    ed.key("F10", "F10", 121, 0).await;
    ed.wait(&format!("{active} === 'File'")).await;
    ed.key("ArrowDown", "ArrowDown", 40, 0).await;
    ed.wait("!document.querySelector('#ctxmenu').hidden").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!(
        "document.querySelector('#ctxmenu').hidden && {active} === 'File'"
    ))
    .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait("document.activeElement.closest('#menubar') === null")
        .await;
}

/// Spec 007 US2 (T013): settings apply at once, are the person's (the CLI sees
/// them, and changes there reach the open editor), and stick: another deck on
/// a new server opens with the same settings and layout.
#[tokio::test(flavor = "multi_thread")]
async fn settings_apply_live_and_stick() {
    let Some(ed) = Ed::open(P2).await else { return };
    let main = "document.querySelector('main').classList";
    let theme = "document.documentElement.dataset.theme";
    ed.key(",", "Comma", 188, 2).await; // ⌘, (Ctrl+, works too)
    ed.wait("document.querySelector('#settings').open").await;
    let choose = |key: &str, v: &str| {
        format!(
            "(() => {{ const s = document.getElementById('set-{key}'); s.value = {v:?}; s.dispatchEvent(new Event('change')); }})()"
        )
    };
    ed.eval(&choose("source.side", "right")).await;
    ed.wait(&format!("{main}.contains('source-right')")).await;
    ed.eval(&choose("theme", "dark")).await;
    ed.wait(&format!("{theme} === 'dark'")).await;
    ed.eval(&choose("source.save", "explicit")).await;
    ed.wait("document.querySelector('#srcpane .hint').textContent.includes('leave the pane')")
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait("!document.querySelector('#settings').open").await;

    // they are the person's: the CLI sees them, and a change there applies at once
    assert_eq!(ed.cli(&["settings"])["values"]["theme"], "dark");
    ed.cli(&["settings", "set", "theme", "light"]);
    ed.wait(&format!("{theme} === 'light'")).await;

    // layout memory: the source pane opened here is open in another deck, on a new server
    ed.open_source().await;
    ed.until_cli(
        &["settings"],
        |v| v["values"]["ui.source_was_open"] == true,
        "the open source pane was not remembered",
    )
    .await;
    ed.cli(&["stop"]);
    let other = ed.dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let deck = other.join("talk.qmd");
    std::fs::write(&deck, P2).unwrap();
    let info = ed.cli(&["--deck", deck.to_str().unwrap(), "serve", "--detach"]);
    ed.page.goto(info["url"].as_str().unwrap()).await.unwrap();
    ed.wait_secs(READY, 60).await;
    ed.wait(&format!(
        "!document.querySelector('#srcpane').hidden && {main}.contains('source-right') && {theme} === 'light'"
    ))
    .await;

    // Reset to defaults asks first, then everything is back (theme follows the system)
    ed.key(",", "Comma", 188, 2).await;
    ed.wait("document.querySelector('#settings').open").await;
    ed.click_js("(r => [r.left + r.width / 2, r.top + r.height / 2])([...document.querySelectorAll('#settings button')].find((b) => b.textContent.startsWith('Reset')).getBoundingClientRect())").await;
    ed.wait("document.querySelector('dialog.sheet[open]')?.textContent.includes('default')")
        .await;
    ed.click_editor("dialog.sheet button[value=ok]").await;
    ed.wait(&format!(
        "{theme} === undefined && !{main}.contains('source-right')"
    ))
    .await;
    ed.cli(&["--deck", deck.to_str().unwrap(), "stop"]);
}

/// Spec 007 US3 (T015): ? opens the shortcut reference (not while typing),
/// built from the same list as the keys: every action with a key is in it,
/// grouped by menu; search filters it; Escape closes it.
#[tokio::test(flavor = "multi_thread")]
async fn shortcut_reference() {
    let Some(ed) = Ed::open(P2).await else { return };
    let open = "document.querySelector('#shortcuts').open";
    ed.go("a").await;
    ed.click_blank().await;
    ed.key("?", "Slash", 191, 8).await; // ⇧/
    ed.wait(open).await;
    let shown = "[...document.querySelectorAll('#shortcuts tr[data-id]')].filter((r) => !r.hidden).map((r) => r.dataset.id)";
    let missing = ed
        .eval(&format!("(async () => {{ const {{ ACTIONS, keysOf }} = await import('/ui/keys.js'); const s = new Set({shown}); return ACTIONS.filter((a) => keysOf(a).length && !s.has(a.id)).map((a) => a.id); }})()"))
        .await;
    assert_eq!(missing, json!([]), "every key is listed");
    assert!(
        ed.eval("[...document.querySelectorAll('#shortcuts h3')].map((h) => h.textContent).join()")
            .await
            .as_str()
            .unwrap()
            .starts_with("File,Edit"),
        "grouped like the menus"
    );
    ed.eval("(() => { const s = document.querySelector('#shortcuts input'); s.value = 'duplicate'; s.dispatchEvent(new Event('input')); })()").await;
    assert_eq!(
        ed.eval(shown).await,
        json!(["dup", "slide-dup"]),
        "search filters"
    );
    let close = || async {
        ed.key("Escape", "Escape", 27, 0).await;
        ed.wait(&format!("!{open}")).await;
    };
    close().await;

    // Help → Keyboard shortcuts opens it too; ? while typing on a slide just types
    ed.menu("Help", "Keyboard shortcuts").await;
    ed.wait(open).await;
    close().await;
    ed.click_in_slide("[data-qd=\"a/0\"]").await;
    ed.caret("[data-qd=\"a/0\"]", None).await;
    ed.type_text("?").await;
    ed.wait("window.qdSession()?.draft?.includes('para.?')")
        .await;
    assert_eq!(ed.eval(open).await, false);
}

/// T019 (spec 007 US4): Escape backs out one level and keeps the edit; slides
/// copy and paste with fresh ids; text from elsewhere is escaped; each edit is
/// one undo step; focus always shows.
#[tokio::test(flavor = "multi_thread")]
async fn same_rules_everywhere() {
    let Some(ed) = Ed::open(P2).await else { return };

    // Escape: menu over the edit → edit (saved) → selected element → nothing selected
    let before = ed.file();
    ed.type_into("a", "[data-qd=\"a/0\"]", " More.").await;
    ed.click_editor("#menubar [data-menu=Edit]").await;
    ed.wait("!document.querySelector('#ctxmenu').hidden").await;
    ed.close_menu().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        ed.session().await["draft"]
            .as_str()
            .is_some_and(|d| d.contains("More.")),
        "the first Esc closed only the menu"
    );
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait_file(|f| f.contains("First para. More."), "kept by Escape")
        .await;
    ed.wait(&format!("window.qdSession() === null && {SEL} === 'a/0'"))
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!("{SEL} === null")).await;
    ed.undo_to(&before).await; // on-slide typing

    // a menu and a shortcut are one undo step each
    ed.settle().await;
    ed.go("b").await;
    let before = ed.file();
    ed.menu("Slide", "Duplicate slide").await;
    ed.wait_file(
        |f| f.matches("Other slide.").count() == 2,
        "duplicated by the menu",
    )
    .await;
    ed.undo_to(&before).await;
    ed.settle().await;
    ed.go("b").await;
    ed.pick_slide("b").await;
    ed.key("d", "KeyD", 68, 2 | 8).await; // Ctrl+Shift+D
    ed.wait_file(
        |f| f.matches("Other slide.").count() == 2,
        "duplicated by the key",
    )
    .await;
    ed.undo_to(&before).await;

    // the source pane
    ed.open_source().await;
    ed.pane_caret("Other slide.", true).await;
    ed.type_text("Changed.").await;
    ed.wait_file(|f| f.contains("Changed."), "saved from the pane")
        .await;
    ed.undo_to(&before).await;
    ed.toggle_source().await;

    // slides on the clipboard: copy in the slide list, paste with fresh ids.
    // Copying writes the system clipboard (caught here); pasting reads it from
    // the paste event.
    ed.settle().await;
    ed.eval("window.__clip = null; navigator.clipboard.writeText = (t) => { __clip = t; return Promise.resolve(); }").await;
    ed.pick_slide("b").await;
    ed.key("c", "KeyC", 67, 2).await;
    ed.wait("window.__clip?.startsWith('## B {#b}')").await;
    ed.paste("document.activeElement", "__clip").await;
    ed.wait_file(|f| f.matches("Other slide.").count() == 2, "pasted")
        .await;
    let s = &ed.cli(&["slides"])["slides"];
    assert_eq!(s.as_array().unwrap().len(), 4, "{s}");
    assert_eq!(s[1]["id"], "b");
    assert_ne!(s[2]["id"], "b", "the copy has a fresh id: {s}");
    ed.undo_to(&before).await;

    // text from another application: a paragraph, never raw HTML
    ed.settle().await;
    ed.pick_slide("b").await;
    ed.paste("document.activeElement", "'1 < 2 & <b>x</b>'")
        .await;
    ed.wait_file(
        |f| f.contains("Other slide.\n\n1 &lt; 2 &amp; &lt;b>x&lt;/b>"),
        "escaped",
    )
    .await;
    ed.undo_to(&before).await;

    // focus shows in every theme: a control, a slide in the list, an element
    ed.settle().await;
    ed.go("a").await;
    ed.select("[data-qd=\"a/0\"]").await;
    for theme in ["light", "dark", "hc"] {
        ed.eval(&format!(
            "document.documentElement.dataset.theme = '{theme}'"
        ))
        .await;
        for target in ["#menubar [data-menu=File]", "#slides li button"] {
            ed.eval(&format!("document.querySelector({target:?}).focus()"))
                .await;
            ed.key("Shift", "ShiftLeft", 16, 8).await; // a key press: focus-visible
            let ring = ed.eval("(s => s.outlineStyle !== 'none' || s.boxShadow !== 'none')(getComputedStyle(document.activeElement))").await;
            assert_eq!(
                ring,
                Value::Bool(true),
                "{theme}: no focus ring on {target}"
            );
        }
        let ring = ed.eval("getComputedStyle(document.querySelector('#preview').contentDocument.querySelector('.qd-sel')).outlineStyle").await;
        assert_eq!(ring, "solid", "{theme}: selected element");
    }
}

/// T025 (spec 007 SC-008): on a 30-slide deck every menu and the shortcut
/// reference open in under 100 ms (median of 5, so one slow frame on a busy
/// machine doesn't decide it).
#[tokio::test(flavor = "multi_thread")]
async fn menus_open_fast() {
    let mut deck = String::from("---\ntitle: T\nformat: revealjs\n---\n");
    for i in 1..=30 {
        deck += &format!("\n## Slide {i} {{#s{i}}}\n\nSome text.\n\n- one\n- two\n");
    }
    let Some(ed) = Ed::open(&deck).await else {
        return;
    };
    ed.go("s15").await;
    let times = ed
        .eval(
            r#"(async () => {
  const until = async (f, what) => { const t = performance.now(); while (!f()) { if (performance.now() - t > 5000) throw new Error('stuck: ' + what); await new Promise(requestAnimationFrame); } };
  const median = (xs) => xs.sort((a, b) => a - b)[Math.floor(xs.length / 2)];
  const menu = document.querySelector('#ctxmenu');
  const out = {};
  for (const b of document.querySelectorAll('#menubar [role=menuitem]')) {
    if (!b.offsetParent) continue;
    const ts = [];
    for (let i = 0; i < 5; i++) {
      const t = performance.now();
      b.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      await until(() => !menu.hidden, 'open ' + b.dataset.menu);
      ts.push(performance.now() - t);
      b.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })); // closes it
      await until(() => menu.hidden, 'close ' + b.dataset.menu);
    }
    out[b.dataset.menu] = median(ts);
  }
  const ref = document.querySelector('#shortcuts');
  const keys = await import('/ui/keys.js');
  const ts = [];
  for (let i = 0; i < 5; i++) {
    const t = performance.now();
    keys.run('shortcuts'); // what ? runs
    await until(() => ref.open, 'reference');
    ts.push(performance.now() - t);
    ref.close();
  }
  out.reference = median(ts);
  return out;
})()"#,
        )
        .await;
    let times = times.as_object().unwrap();
    eprintln!("menu and reference open times (ms): {times:?}");
    assert_eq!(times.len(), 8, "7 menus and the reference: {times:?}");
    for (what, ms) in times {
        assert!(
            ms.as_f64().unwrap() < 100.0,
            "{what} took {ms} ms: {times:?}"
        );
    }
}

/// ⌘V takes the text from the browser's own paste event, which needs no
/// clipboard permission (pasting a slide copied in another deck failed in a
/// page without it). The clipboard API is never asked.
#[tokio::test(flavor = "multi_thread")]
async fn paste_key_uses_the_paste_event() {
    let Some(ed) = Ed::open(P2).await else { return };
    ed.go("b").await;
    ed.pick_slide("b").await;
    // the clipboard API refuses, as without permission, and records being asked
    let refuse = "window.__asked = false; navigator.clipboard.readText = () => { __asked = true; return Promise.reject(new Error('no permission')); }";
    ed.eval(refuse).await;
    ed.paste(
        "document.activeElement",
        "'## Pasted {#b}\\n\\nFrom another deck.\\n'",
    )
    .await;
    ed.wait_file(|f| f.contains("From another deck."), "slide pasted")
        .await;
    let s = &ed.cli(&["slides"])["slides"];
    assert_eq!(s[1]["id"], "b");
    assert_ne!(s[2]["id"], "b", "a fresh id: {s}");
    assert_eq!(
        ed.eval("__asked").await,
        false,
        "the clipboard API was not asked"
    );

    // an element selected: other text pastes after it, escaped
    ed.settle().await;
    ed.go("a").await;
    ed.select("[data-qd=\"a/0\"]").await;
    ed.eval("document.querySelector('#preview').contentWindow.focus()")
        .await;
    ed.paste(
        "document.querySelector('#preview').contentDocument.body",
        "'1 < 2'",
    )
    .await;
    ed.wait_file(
        |f| f.contains("First para.\n\n1 &lt; 2"),
        "pasted after the element",
    )
    .await;
    assert_eq!(ed.eval("__asked").await, false);

    // an empty paste event (Chrome on Linux fires one for a devtools ⌘V, with
    // its own empty clipboard) falls back to the clipboard API, not to nothing
    ed.settle().await;
    ed.pick_slide("b").await;
    ed.eval("navigator.clipboard.readText = () => Promise.resolve('## Again {#again}\\n\\nFrom the API.\\n')")
        .await;
    ed.paste("document.activeElement", "''").await;
    ed.wait_file(|f| f.contains("From the API."), "pasted from the API")
        .await;
}

// ---------- spec 008: usability ----------

/// The audit deck, the same file the audit's screenshots use.
const AUDIT: &str = include_str!("../specs/008-usability/audit/deck/talk.qmd");

/// The slide ids in `qd slides` output, in order.
fn slide_ids(v: &Value) -> Vec<String> {
    v["slides"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect()
}

/// In-page checks for 008's measurement test: contrast of every visible text
/// and form value, and the size of every interactive target, in the editor's
/// own document (the deck's slides are the deck's). Returns failures.
const MEASURE: &str = r#"(() => {
  const lum = (c) => { const [r, g, b] = c.match(/[\d.]+/g).slice(0, 3).map(Number).map((v) => v / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
  const opaque = (c) => !/rgba\([^)]*,\s*0\)|transparent/.test(c);
  const bgOf = (el) => { for (let e = el; e; e = e.parentElement) { const c = getComputedStyle(e).backgroundColor; if (opaque(c)) return c; } return 'rgb(255, 255, 255)'; };
  const shown = (el) => { const r = el.getBoundingClientRect(); const s = getComputedStyle(el); return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.opacity !== '0' && !el.closest('[hidden], dialog:not([open])'); };
  const name = (el) => el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') + (el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\s+/).join('.') : '') + ` "${(el.textContent || el.value || '').trim().slice(0, 20)}"`;
  const out = [];
  for (const el of document.querySelectorAll('body *')) {
    if (!shown(el) || el.closest('#preview, #slides .thumb') || el.matches('#srcpane textarea') || el.disabled || el.getAttribute('aria-disabled') === 'true') continue;
    const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.data.trim()) || ['SELECT', 'TEXTAREA'].includes(el.tagName) || (el.tagName === 'INPUT' && !['checkbox', 'color', 'radio', 'range'].includes(el.type)); // swatches and boxes show no text
    if (own) {
      const s = getComputedStyle(el);
      const a = lum(s.color); const b = lum(bgOf(el));
      const ratio = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      const px = parseFloat(s.fontSize); const large = px >= 24 || (px >= 18.66 && +s.fontWeight >= 700);
      if (ratio < (large ? 3 : 4.5)) out.push(`contrast ${ratio.toFixed(2)}: ${name(el)}`);
    }
    if (el.matches('button, select, input:not([type=hidden]), [role=menuitem], [role=menuitemcheckbox], a[href]')) {
      const r = el.getBoundingClientRect();
      if (r.width < 20 || r.height < 20) out.push(`size ${Math.round(r.width)}x${Math.round(r.height)}: ${name(el)}`);
    }
  }
  return out;
})()"#;

/// The element under the open element bar or formatting bar, if any (null when
/// none is shown or it covers nothing): a bar must never cover what the slide shows
/// (an element's contents, not the empty end of a block such as a short heading).
const OVERLAP: &str = "(() => { const f = document.querySelector('#preview'); const a = f.getBoundingClientRect(); const bar = [...document.querySelectorAll('#elbar, #fmtbar')].find((b) => !b.hidden && b.getClientRects().length); if (!bar) return null; const r = bar.getBoundingClientRect(); const hit = [...f.contentDocument.querySelectorAll('section.present [data-qd], section.present > h2')].filter((el) => !el.classList.contains('qd-sel') && !el.classList.contains('qd-editing')).find((el) => { const g = el.ownerDocument.createRange(); g.selectNodeContents(el); const c = g.getBoundingClientRect(); const e = c.width && c.height ? c : el.getBoundingClientRect(); const x = a.left + e.left, y = a.top + e.top; return x < r.right && x + e.width > r.left && y < r.bottom && y + e.height > r.top; }); return hit ? (hit.dataset.qd || hit.tagName) : null; })()";

/// T004 (spec 008 SC-004, SC-005, SC-003): in light, dark and high contrast,
/// every text and control in the editor meets the contrast and size minimums,
/// with the source pane open too, and the bars cover no element (the audit's
/// failures, fixed by 008 US2 and US3).
#[tokio::test(flavor = "multi_thread")]
async fn audit_measures() {
    let Some(ed) = Ed::open(AUDIT).await else {
        return;
    };
    let link = ed.cli(&["url"])["url"].as_str().unwrap().to_string();
    let mut failures = vec![];
    for theme in ["light", "dark", "hc"] {
        // through the link: settings (which opening the source pane writes) don't override it
        ed.page
            .goto(
                link.replace("/#t=", &format!("/?theme={theme}#t="))
                    .as_str(),
            )
            .await
            .unwrap();
        ed.wait_secs(READY, 60).await;
        ed.go("highlights").await;
        for source in [false, true] {
            if source {
                ed.open_source().await;
            }
            let found = ed.eval(MEASURE).await;
            for f in found.as_array().unwrap() {
                failures.push(format!(
                    "{theme}{}: {}",
                    if source { "+source" } else { "" },
                    f.as_str().unwrap()
                ));
            }
            if source {
                ed.toggle_source().await;
            }
        }
    }
    // the bars cover no element: select, then edit, the element under each title
    for (anchor, el) in [("highlights", "highlights/0"), ("model", "model/0")] {
        ed.go(anchor).await;
        ed.select(&format!("[data-qd=\"{el}\"]")).await;
        let hit = ed.eval(OVERLAP).await;
        if !hit.is_null() {
            failures.push(format!("element bar over {hit} when {el} is selected"));
        }
        ed.key("Escape", "Escape", 27, 0).await;
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// T005 (spec 008 US1): the slide list works like a slide app's: thumbnails,
/// the title slide first, a + in every gap, untitled new slides with an
/// editor-only placeholder, and a drag you can see (handle, drag image, gap,
/// Escape, one undo step), plus the keyboard.
#[tokio::test(flavor = "multi_thread")]
async fn slide_list_like_a_slide_app() {
    let Some(ed) = Ed::open(AUDIT).await else {
        return;
    };
    let ids = "[...document.querySelectorAll('#slides li[data-id]')].map((li) => li.dataset.id)";
    let order = || slide_ids(&ed.cli(&["slides"]));
    let built = |id: &str| {
        format!("+(document.querySelector('#slides li[data-id={id}] .thumb')?.dataset.built ?? 0)")
    };

    // the title slide first (unnumbered, not draggable), then every slide
    ed.wait(&format!("{ids}.length === 8")).await;
    assert_eq!(
        ed.eval(&format!("{ids}[0]")).await,
        "title-slide",
        "the title slide is listed first"
    );
    assert_eq!(
        ed.eval("(li => !li.querySelector('.idx')?.textContent.trim() && !li.querySelector('.handle'))(document.querySelector('#slides li[data-id=title-slide]'))").await,
        true,
        "unnumbered, no drag handle"
    );
    // thumbnails: a copy of the slide's section, for items in view; scrolled, all
    ed.wait(&format!("{} > 0 && document.querySelector('#slides li[data-id=highlights] .thumb').shadowRoot.querySelector('section')?.textContent.includes('Revenue grew')", built("highlights"))).await;
    assert_eq!(
        ed.eval("!!document.querySelector('#peek')").await,
        false,
        "no hover pop-up"
    );
    ed.eval("document.querySelector('nav').scrollTop = 1e6")
        .await;
    ed.wait(&format!("{} > 0", built("next"))).await;
    ed.eval("document.querySelector('nav').scrollTop = 0").await;
    // an edit rebuilds that slide's thumbnail only
    let (h, g) = (
        ed.eval(&built("highlights")).await,
        ed.eval(&built("growth")).await,
    );
    ed.cli(&["edit", "highlights/0", "--md", "- Changed bullet"]);
    ed.wait(&format!("document.querySelector('#slides li[data-id=highlights] .thumb').shadowRoot.textContent.includes('Changed bullet') && {} > {h}", built("highlights"))).await;
    assert_eq!(
        ed.eval(&built("growth")).await,
        g,
        "other thumbnails untouched"
    );

    // + in the gap before "columns": an untitled slide there, a placeholder on the canvas only
    ed.settle().await;
    ed.click_editor("#slides li[data-id=columns] .add-here")
        .await;
    ed.wait_file(|f| f.contains("## {#s-"), "untitled slide added")
        .await;
    let new = order()[2].clone();
    assert!(
        new.starts_with("s-"),
        "added between growth and columns: {:?}",
        order()
    );
    assert!(!ed.file().contains("Click to add title"));
    ed.settle().await;
    ed.wait(&format!(
        "document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide()?.id === '{new}'"
    ))
    .await;
    assert!(
        ed.eval(&format!("getComputedStyle(document.querySelector('#preview').contentDocument.querySelector('#{new} > h2'), '::before').content")).await
            .as_str().unwrap().contains("Click to add title"),
        "the placeholder shows on the canvas"
    );

    // Enter on a focused slide adds one after it
    ed.eval("document.querySelector('#slides li[data-id=model] button').focus()")
        .await;
    ed.key("Enter", "Enter", 13, 0).await;
    ed.until_cli(
        &["slides"],
        |v| v["slides"].as_array().unwrap().len() == 9,
        "Enter added a slide",
    )
    .await;
    let o = order();
    assert!(
        o[o.iter().position(|x| x == "model").unwrap() + 1].starts_with("s-"),
        "{o:?}"
    );

    // drag "code" by its handle above "model": a drag image and a gap; Escape cancels
    ed.settle().await;
    ed.show_in_list("code").await;
    let before = order();
    let from = ed
        .editor_point("#slides li[data-id=code] .handle", None)
        .await;
    let to = ed
        .editor_point("#slides li[data-id=model]", Some(20.0))
        .await;
    let to = (to.0, to.1 - 30.0);
    use DispatchMouseEventType::*;
    ed.press_and_move(from, to).await;
    ed.wait("!!document.querySelector('.drag-ghost') && [...document.querySelectorAll('#slides li')].some((li) => li.style.transform && li.style.transform !== 'none')").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.mouse(MouseReleased, to, false).await;
    ed.wait("!document.querySelector('.drag-ghost')").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(order(), before, "Escape cancelled the drag");
    // again, dropped: one move, one undo
    let file = ed.file();
    ed.press_and_move(from, to).await;
    ed.mouse(MouseReleased, to, false).await;
    ed.until_cli(
        &["slides"],
        |v| {
            let o = slide_ids(v);
            o.iter().position(|x| x == "code") < o.iter().position(|x| x == "model")
        },
        "code moved above model",
    )
    .await;
    ed.undo_to(&file).await;
    // held near the list's top edge, the list scrolls
    ed.settle().await;
    ed.show_in_list("code").await;
    let top = ed
        .point("(() => { const r = document.querySelector('nav').getBoundingClientRect(); return [r.left + 60, r.top + 10]; })()")
        .await;
    let scrolled = ed.eval("document.querySelector('nav').scrollTop").await;
    ed.press_and_move(from, top).await;
    ed.wait(&format!(
        "document.querySelector('nav').scrollTop < {scrolled}"
    ))
    .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.mouse(MouseReleased, top, false).await;

    // ⌥↑ on a focused slide moves it up
    ed.settle().await;
    ed.eval("document.querySelector('#slides li[data-id=columns] button').focus()")
        .await;
    ed.key("ArrowUp", "ArrowUp", 38, 1).await; // 1 = Alt
    ed.until_cli(
        &["slides"],
        |v| {
            let o = slide_ids(v);
            o.iter().position(|x| x == "columns") < o.iter().position(|x| x.starts_with("s-"))
        },
        "⌥↑ moved columns up",
    )
    .await;
}

/// T011 (spec 008 US2): nothing covers the slide. The bars for a selected or
/// edited element never sit on another element or the title; reveal.js's own
/// menu button, arrows and progress bar are hidden while editing; and the
/// canvas doesn't move between the title slide and a content slide.
#[tokio::test(flavor = "multi_thread")]
async fn nothing_covers_the_slide() {
    let Some(ed) = Ed::open(AUDIT).await else {
        return;
    };
    let canvas_top = "Math.round(document.querySelector('#preview').getBoundingClientRect().top)";
    ed.go("title-slide").await;
    let top = ed.eval(canvas_top).await;
    ed.go("highlights").await;
    assert_eq!(ed.eval(canvas_top).await, top, "the canvas stays put");
    assert_eq!(
        ed.eval("(d => [...d.querySelectorAll('.slide-menu-button, .reveal .controls, .reveal .progress')].filter((e) => e.checkVisibility({ visibilityProperty: true })).length)(document.querySelector('#preview').contentDocument)").await,
        0,
        "no reveal.js chrome while editing"
    );
    for (anchor, el) in [
        ("highlights", "highlights/0"),
        ("model", "model/0"),
        ("annotated", "annotated/0"),
    ] {
        ed.go(anchor).await;
        ed.select(&format!("[data-qd=\"{el}\"]")).await;
        assert_eq!(
            ed.eval(OVERLAP).await,
            Value::Null,
            "element bar over another element when {el} is selected"
        );
        ed.key("Enter", "Enter", 13, 0).await; // edit it
        // typed on the slide (the formatting bar) or in the markdown pop-up over it
        ed.wait("window.qdSession() !== null").await;
        assert_eq!(
            ed.eval(OVERLAP).await,
            Value::Null,
            "formatting bar over another element while editing {el}"
        );
        ed.key("Escape", "Escape", 27, 0).await;
        ed.key("Escape", "Escape", 27, 0).await;
        ed.idle().await;
    }
    // typing on the slide: the formatting bar shows and covers nothing
    ed.go("model").await;
    ed.click_in_slide("[data-qd=\"model/0\"]").await;
    ed.wait("window.qdSession() !== null && !document.querySelector('#fmtbar').hidden")
        .await;
    assert_eq!(
        ed.eval(OVERLAP).await,
        Value::Null,
        "formatting bar over another element while typing"
    );
}

/// T015 (spec 008 US4): Present puts the canvas's own frame full screen at the
/// slide on screen, with reveal.js's keys and the deck's own controls; leaving full screen
/// gives the editor its settings back. "From the beginning" starts at the title.
#[tokio::test(flavor = "multi_thread")]
async fn present_from_here_and_from_start() {
    let mut deck = String::from("---\ntitle: Talk\nformat: revealjs\n---\n");
    for i in 1..=5 {
        deck += &format!("\n## Slide {i} {{#s{i}}}\n\nSome text.\n");
    }
    let Some(ed) = Ed::open(&deck).await else {
        return;
    };
    // full screen needs a real screen: record what asked for it, and say it happened
    ed.eval(
        "window.__fs = []; Element.prototype.requestFullscreen = function () { __fs.push(this.id); \
         window.__full = this; document.dispatchEvent(new Event('fullscreenchange')); return Promise.resolve(); }; \
         Object.defineProperty(document, 'fullscreenElement', { configurable: true, get: () => window.__full ?? null })",
    )
    .await;
    let reveal = "document.querySelector('#preview').contentWindow.Reveal";
    let leave = "window.__full = null; document.dispatchEvent(new Event('fullscreenchange'))";
    let presenting =
        format!("{reveal}.getConfig().progress === true && {reveal}.getConfig().keyboard === true");
    let editing = format!(
        "{reveal}.getConfig().progress === false && {reveal}.getConfig().keyboard?.[27] === null"
    );

    ed.go("s4").await;
    ed.click_editor("#present").await;
    ed.wait(&format!("__fs.join() === 'preview' && {presenting}"))
        .await;
    assert_eq!(
        ed.eval(&format!("{reveal}.getCurrentSlide().id")).await,
        "s4"
    );
    ed.eval(leave).await;
    ed.wait(&editing).await;

    ed.click_editor("#present-more").await;
    ed.click_menu("Present from the beginning").await;
    ed.wait(&format!("__fs.length === 2 && {presenting}")).await;
    assert_eq!(
        ed.eval(&format!("{reveal}.getCurrentSlide().id")).await,
        "title-slide"
    );
    ed.eval(leave).await;
    ed.wait(&editing).await;
}

/// T020 (spec 008 US5): the editor asks with its own sheets, never the browser's
/// boxes; an image file dropped on the slide is copied beside the deck and lands
/// where it was dropped.
#[tokio::test(flavor = "multi_thread")]
async fn own_sheets_and_image_drop() {
    let Some(ed) = Ed::open(P2).await else {
        return;
    };
    std::fs::write(
        ed.dir.path().join("chart.svg"),
        "<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'/>",
    )
    .unwrap();
    ed.eval(
        "window.__native = []; for (const f of ['prompt', 'confirm', 'alert']) window[f] = (...a) => { __native.push(f); return null; }",
    )
    .await;
    let sheet = "document.querySelector('dialog.sheet[open]')";

    // Insert ▸ Image…: the images beside the deck, with previews
    ed.go("b").await;
    ed.click_editor("#menubar [data-menu=Insert]").await;
    ed.click_menu("Image…").await;
    ed.wait(&format!(
        "{sheet}?.querySelector('[data-path=\"chart.svg\"] img')?.complete === true"
    ))
    .await;
    ed.click_editor("dialog.sheet [data-path=\"chart.svg\"]")
        .await;
    ed.click_editor("dialog.sheet button[value=ok]").await;
    ed.wait_file(
        |f| f.contains("Other slide.\n\n![](chart.svg)"),
        "image inserted",
    )
    .await;
    ed.settle().await;

    // a PNG dropped on the lower half of the first paragraph lands after it
    ed.go("a").await;
    ed.eval(
        r#"(() => {
  const doc = document.querySelector('#preview').contentDocument;
  const p = doc.querySelector('[data-qd="a/0"]');
  const r = p.getBoundingClientRect();
  const bytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3]);
  const dt = new DataTransfer();
  dt.items.add(new File([bytes], 'drop.png', { type: 'image/png' }));
  const at = { bubbles: true, cancelable: true, dataTransfer: dt, clientX: r.left + 5, clientY: r.bottom - 2 };
  p.dispatchEvent(new DragEvent('dragover', at));
  p.dispatchEvent(new DragEvent('drop', at));
})()"#,
    )
    .await;
    ed.wait_file(
        |f| f.contains("First para.\n\n![](images/drop.png)\n\nSecond para."),
        "dropped image after the first paragraph",
    )
    .await;
    assert!(ed.dir.path().join("images/drop.png").exists());
    ed.settle().await;

    // the link sheet says what's wrong with an address, and OK waits for a good one
    ed.go("b").await;
    ed.click_in_slide("[data-qd=\"b/0\"]").await;
    ed.wait("window.qdSession() !== null").await;
    ed.click_editor("#fmtbar [data-fmt=link]").await;
    ed.wait(&format!(
        "{sheet}?.querySelector('input[name=href]') != null"
    ))
    .await;
    ed.eval(&format!(
        "(() => {{ const i = {sheet}.querySelector('input[name=href]'); i.value = 'not a url'; i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
    ))
    .await;
    ed.wait(&format!(
        "{sheet}.querySelector('.error').textContent !== '' && {sheet}.querySelector('button[value=ok]').disabled"
    ))
    .await;
    ed.key("Escape", "Escape", 27, 0).await;
    // Cancel puts the keyboard back in the text; Esc there ends the edit
    ed.wait(&format!("!{sheet} && document.activeElement?.id === 'preview' && document.querySelector('#preview').contentDocument.activeElement.isContentEditable"))
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.idle().await;

    // the background colour sheet offers None, which removes the colour
    ed.cli(&["set", "b", "--attr", "background-color=#ffeeaa"]);
    ed.settle().await;
    ed.go("b").await;
    ed.menu("Slide", "Background colour…").await;
    ed.wait(&format!(
        "{sheet}?.querySelector('[data-value=\"\"]') != null"
    ))
    .await;
    ed.click_editor("dialog.sheet [data-value=\"\"]").await;
    ed.click_editor("dialog.sheet button[value=ok]").await;
    ed.wait_file(|f| !f.contains("background-color"), "colour removed")
        .await;

    assert_eq!(ed.eval("__native.length").await, 0, "a native box was used");
}

/// T023 (spec 008 US6): one top row — menus, the deck's title and file, the
/// status, Present — the tab named after the deck, the status in words, and no
/// jargon left in the menus or bars.
#[tokio::test(flavor = "multi_thread")]
async fn one_top_bar_in_plain_words() {
    let Some(ed) = Ed::open(P2).await else {
        return;
    };
    let row = |sel: &str| format!("document.querySelector('{sel}').closest('header') !== null");
    for sel in ["#menubar", "#deck-name", "#status", "#present"] {
        assert_eq!(ed.eval(&row(sel)).await, true, "{sel} in the top row");
    }
    ed.wait("document.querySelector('#deck-name').textContent.includes('T') && document.querySelector('#deck-name .file').textContent === 'deck.qmd'")
        .await;
    assert_eq!(ed.eval("document.title").await, "T");

    // the status in words, the version as its tooltip (settle waits for both)
    ed.settle().await;
    ed.eval("window.__st = []; new MutationObserver(() => __st.push(document.querySelector('#status').textContent)).observe(document.querySelector('#status'), { childList: true, characterData: true, subtree: true })")
        .await;
    ed.type_into("a", "[data-qd=\"a/0\"]", "!").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait_file(|f| f.contains("First para.!"), "saved").await;
    ed.wait("__st.includes('Saving…') && __st.includes('Saved') && __st.at(-1) === 'Up to date'")
        .await;

    // no jargon in any menu or bar
    let jargon = r#"(async () => {
  const old = ['Render', 'Open preview', 'Export offline HTML', 'Export with PDF', 'Form', 'Form panel', 'On click', 'Edit source', '+ Package', 'Create environment'];
  const menus = Object.values(await qdMenus()).flat().map((i) => i.label);
  const bars = [...document.querySelectorAll('button')].map((b) => b.textContent.trim());
  return [...menus, ...bars].filter((l) => old.includes(l));
})()"#;
    assert_eq!(ed.eval(jargon).await, json!([]));

    // a render that fails says so in a banner the person can close
    std::fs::write(
        ed.dir.path().join("deck.qmd"),
        format!("{P2}\n{{{{< include missing.qmd >}}}}\n"),
    )
    .unwrap();
    ed.wait("!document.querySelector('#banner').hidden && document.querySelector('#banner .msg').textContent !== ''")
        .await;
    ed.click_editor("#banner .close").await;
    ed.wait("document.querySelector('#banner').hidden").await;

    // VS Code's panel: ☰ in that same row
    let link = ed.cli(&["url"])["url"].as_str().unwrap().to_string();
    ed.page
        .goto(link.replace("/#t=", "/?host=vscode#t=").as_str())
        .await
        .unwrap();
    ed.wait_secs(READY, 60).await;
    ed.wait("document.querySelector('#menubar').classList.contains('folded')")
        .await;
    assert_eq!(ed.eval(&row("#menubar")).await, true);
}

/// T026 (spec 008 US7): bars with one height and a name on every control, the
/// common formats up front and the rest under "Aa", a source bar with the run
/// controls and the rest under ···, a fuller Insert, and box handles easy to hit.
#[tokio::test(flavor = "multi_thread")]
async fn tidier_bars_and_fuller_insert() {
    let Some(ed) = Ed::open(P2).await else {
        return;
    };
    let sheet = "document.querySelector('dialog.sheet[open]')";
    let shown = |bar: &str| {
        format!(
            "[...document.querySelectorAll('{bar} button, {bar} select')].filter((b) => b.offsetParent && !b.closest('.pop'))"
        )
    };

    // the slide bar: one height, a name on each control, a colour menu with None
    ed.go("b").await;
    ed.wait("document.querySelector('#slidebar').style.visibility !== 'hidden'")
        .await;
    let bar = shown("#slidebar");
    assert_eq!(
        ed.eval(&format!("new Set({bar}.map((b) => b.offsetHeight)).size"))
            .await,
        1,
        "one height"
    );
    assert_eq!(
        ed.eval(&format!("{bar}.filter((b) => !(b.title || b.getAttribute('aria-label') || b.textContent.trim())).length")).await,
        0,
        "every control named"
    );
    ed.click_editor("#slidebar [data-act=bg]").await;
    ed.wait(&format!(
        "{sheet}?.querySelector('[data-value=\"\"]') != null"
    ))
    .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!("!{sheet}")).await;

    // the formatting bar: the common few, then "Aa" for the rest
    ed.go("a").await;
    ed.click_in_slide("[data-qd=\"a/0\"]").await;
    ed.wait("window.qdSession() !== null && !document.querySelector('#fmtbar').hidden")
        .await;
    assert_eq!(
        ed.eval(&format!("{}.map((b) => b.dataset.fmt)", shown("#fmtbar")))
            .await,
        json!([
            "bold",
            "italic",
            "underline",
            "strike",
            "link",
            "ul",
            "color",
            "more"
        ])
    );
    ed.click_editor("#fmtbar [data-fmt=more]").await;
    ed.wait("['code', 'ol', 'outdent', 'indent', 'mark', 'smaller', 'larger', 'left', 'center', 'right'].every((f) => document.querySelector(`#fmtbar .pop [data-fmt=${f}]`)?.offsetParent)")
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.idle().await;

    // the source bar: Run all, Stop, Python's state in words, ··· for the rest
    ed.toggle_source().await;
    ed.wait("document.querySelector('#srcpane .kernel').textContent === 'Python · stopped'")
        .await;
    assert_eq!(
        ed.eval("[...document.querySelectorAll('#srcpane .runbar button')].filter((b) => b.offsetParent).map((b) => b.textContent.trim())").await,
        json!(["▶ Run all", "■ Stop", "···"])
    );
    ed.click_editor("#srcpane .runbar [data-act=more]").await;
    let labels = ed.menu_labels().await;
    for l in [
        "Restart",
        "Restart & run all",
        "Add Python package…",
        "Set up Python…",
    ] {
        // (Set up Python… is offered only when the deck has code cells)
        assert!(
            labels.iter().any(|x| x.starts_with(l)),
            "{l} missing from {labels:?}"
        );
    }
    ed.close_menu().await;
    ed.toggle_source().await;

    // Insert: each new kind of block is its markdown in the file
    for (label, md) in [
        ("Code cell", "```{python}\n1 + 1\n```"),
        ("Callout", "::: {.callout-note}\nA note.\n:::"),
        ("Columns", ":::: {.columns}\n"),
        ("Speaker notes", "::: notes\nSay this.\n:::"),
        ("Table", "| A | B |\n|---|---|\n| 1 | 2 |"),
        ("Quote", "> A quote."),
    ] {
        ed.settle().await;
        ed.go("b").await;
        ed.menu("Insert", label).await;
        ed.wait_file(|f| f.contains(md), label).await;
    }

    // a positioned box: handles drawn small, hit areas at least 20 px
    ed.settle().await;
    ed.cli(&["box", "add", "b", "--text", "Box"]);
    ed.settle().await;
    ed.go("b").await;
    ed.click_in_slide("#b .absolute").await;
    ed.wait(
        "document.querySelector('#preview').contentDocument.querySelector('.qd-handle') != null",
    )
    .await;
    assert_eq!(
        ed.eval("[...document.querySelector('#preview').contentDocument.querySelectorAll('.qd-handle')].every((h) => { const r = h.getBoundingClientRect(); return r.width >= 20 && r.height >= 20; })").await,
        true
    );
}
