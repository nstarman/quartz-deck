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
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod common;
use common::{have_quarto, qd};

const SRC: &str = "document.querySelector('#srcedit textarea')";
/// Default wait: generous, since all browser tests (each with Quarto and Chrome) run in parallel.
const WAIT: u64 = 20;

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/corpus");

/// Chrome to use, or None (test skips) when Quarto or a browser is missing.
fn chrome() -> Option<PathBuf> {
    let quarto = have_quarto();
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

struct Ed {
    dir: tempfile::TempDir,
    page: Page,
    _browser: Browser,
    _profile: tempfile::TempDir,
}

impl Drop for Ed {
    fn drop(&mut self) {
        let _ = qd(self.dir.path()).arg("stop").output();
    }
}

impl Ed {
    async fn open(deck: &str) -> Option<Ed> {
        let chrome = chrome()?;
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
        };
        ed.wait_secs("(() => { const f = document.querySelector('#preview'); return !!(f && f.contentWindow?.Reveal?.isReady?.() && document.querySelector('#slides li') && window.qdSession); })()", 60).await;
        Some(ed)
    }

    fn file(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("deck.qmd")).unwrap()
    }

    fn cli(&self, args: &[&str]) {
        let out = qd(self.dir.path()).args(args).output().unwrap();
        assert!(
            out.status.success(),
            "qd {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    async fn eval(&self, js: &str) -> Value {
        let p = EvaluateParams::builder()
            .expression(js)
            .await_promise(true)
            .return_by_value(true)
            .build()
            .unwrap();
        let r = self
            .page
            .evaluate_expression(p)
            .await
            .unwrap_or_else(|e| panic!("eval failed: {e}\n{js}"));
        r.value().cloned().unwrap_or(Value::Null)
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

    /// Show the slide with this preview anchor and wait for it.
    async fn go(&self, anchor: &str) {
        self.eval(&format!(
            "document.querySelector('#preview').contentWindow.location.hash = '#/{anchor}'"
        ))
        .await;
        self.wait(&format!("document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide()?.id === '{anchor}'")).await;
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
        serde_json::json!([]),
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

    // Escape abandons an edit
    let before = ed.file();
    ed.idle().await;
    ed.click_in_slide("[data-qd=\"methods/1\"] li").await;
    ed.caret("[data-qd=\"methods/1\"]", None).await;
    ed.type_text(" nope").await;
    ed.key("Escape", "Escape", 27, 0).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(ed.session().await, Value::Null);
    assert_eq!(ed.file(), before);

    // undo restores the file byte-for-byte (shared history with the CLI)
    ed.click_editor("#undo").await;
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
    // the Source button shows the panel
    ed.click_editor("#toggle-source").await;
    assert_eq!(
        ed.eval("getComputedStyle(document.querySelector('#panel')).display")
            .await,
        "block"
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
