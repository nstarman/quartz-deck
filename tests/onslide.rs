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
        };
        ed.wait_secs(READY, 60).await;
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
            assert!(
                start.elapsed() < Duration::from_secs(secs),
                "timed out waiting for: {cond}"
            );
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
            "(async () => { const v = (await fetch('/api/deck').then((r) => r.json())).version; const st = document.querySelector('#status').textContent; const f = document.querySelector('#preview').contentWindow; return window.qdSession() === null && st === 'rendered ' + v.slice(0, 7) && !!f.Reveal?.isReady?.(); })()",
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

    async fn open_source(&self) {
        self.click_editor("#toggle-source").await;
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
    ed.idle().await;
    ed.menu("Edit", "Undo").await;
    ed.wait_file(|f| f == before, "one undo step").await;

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
    ed.click_editor("#fmtbar .swatch").await;
    assert_eq!(
        ed.session().await["draft"],
        "- *one*\n- **[two]{style=\"color:#c0392b\"}**"
    );
    // remove all three again: the draft is byte-identical to the source
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
        "Edit source",
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
        "Edit source"
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
    ed.wait("!document.querySelector('#slidebar').hidden").await;
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
    ed.click_editor("#toggle-source").await;
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
    let sel =
        "document.querySelector('#preview').contentDocument.querySelector('.qd-sel')?.dataset.qd";
    ed.click_blank().await;
    for (shift, want) in [(0, "f/0"), (0, "f/1"), (8, "f/0")] {
        ed.key("Tab", "Tab", 9, shift).await; // 8 = Shift+Tab
        ed.wait(&format!("{sel} === '{want}'")).await;
    }
    ed.wait("document.querySelector('#notice').textContent.startsWith('Selected')")
        .await;
    ed.key("Enter", "Enter", 13, 0).await;
    ed.wait("window.qdSession()?.ref === 'f/0'").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!("window.qdSession() === null && {sel} === 'f/0'"))
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
    let sel =
        "document.querySelector('#preview').contentDocument.querySelector('.qd-sel')?.dataset.qd";
    ed.wait("window.qdSession() !== null").await;
    esc().await; // ends the edit the click started (the element stays selected)
    ed.wait(&format!("window.qdSession() === null && {sel} === 'a/0'"))
        .await;
    esc().await; // clears the selection
    ed.wait(&format!("!{sel}")).await;
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
        ed.eval("document.querySelectorAll('#menubar[role=menubar] > [role=menuitem][aria-haspopup=menu]').length").await,
        7
    );

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
    ed.eval("window.confirm = (m) => ((window.__asked = m), true)")
        .await;
    ed.key(",", "Comma", 188, 2).await;
    ed.wait("document.querySelector('#settings').open").await;
    ed.click_js("(r => [r.left + r.width / 2, r.top + r.height / 2])([...document.querySelectorAll('#settings button')].find((b) => b.textContent.startsWith('Reset')).getBoundingClientRect())").await;
    ed.wait(&format!(
        "{theme} === undefined && !{main}.contains('source-right')"
    ))
    .await;
    assert!(
        ed.eval("window.__asked")
            .await
            .as_str()
            .unwrap()
            .contains("default")
    );
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
    let sel = "(document.querySelector('#preview').contentDocument.querySelector('.qd-sel')?.dataset.qd ?? null)";

    // Escape: edit → selected element (the edit saved) → nothing selected
    let before = ed.file();
    ed.type_into("a", "[data-qd=\"a/0\"]", " More.").await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait_file(|f| f.contains("First para. More."), "kept by Escape")
        .await;
    ed.wait(&format!("window.qdSession() === null && {sel} === 'a/0'"))
        .await;
    ed.key("Escape", "Escape", 27, 0).await;
    ed.wait(&format!("{sel} === null")).await;
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
    ed.click_editor("#slides li[data-id=b] button").await;
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
    ed.click_editor("#toggle-source").await;

    // slides on the clipboard: copy in the slide list, paste with fresh ids.
    // The system clipboard is stubbed (headless Chrome won't share one).
    ed.settle().await;
    ed.eval("window.__clip = null; navigator.clipboard.writeText = (t) => { __clip = t; return Promise.resolve(); }; navigator.clipboard.readText = () => Promise.resolve(__clip)").await;
    ed.click_editor("#slides li[data-id=b] button").await;
    ed.key("c", "KeyC", 67, 2).await;
    ed.wait("window.__clip?.startsWith('## B {#b}')").await;
    ed.key("v", "KeyV", 86, 2).await;
    ed.wait_file(|f| f.matches("Other slide.").count() == 2, "pasted")
        .await;
    let ids = ed.cli(&["slides"])["slides"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 4, "{ids:?}");
    assert!(
        ids[2] != "b" && ids[1] == "b",
        "the copy has a fresh id: {ids:?}"
    );
    ed.undo_to(&before).await;

    // text from another application: a paragraph, never raw HTML
    ed.settle().await;
    ed.eval("window.__clip = '1 < 2 & <b>x</b>'").await;
    ed.click_editor("#slides li[data-id=b] button").await;
    ed.key("v", "KeyV", 86, 2).await;
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
        for target in ["#menubar [role=menuitem]", "#slides li button"] {
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
