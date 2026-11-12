//! Make a rendered deck one file (spec 010): ports of the quarto-revealjs
//! skill's `inline_apps.py` (apps, `<object>` and SVG `<image>` files,
//! repeated `data:` URIs, `--strip-notes`) and `size_report.py`.
//!
//! Apps go in gzipped, as base64 in a `<script type="text/plain">`; a small
//! script unzips each (DecompressionStream) into a `blob:` URL and points the
//! slide's background iframe or `<iframe data-app|data-src|src>` at it. A
//! `data:` URL would be simpler, but Chrome will not load one over 2 MB in an
//! iframe and a `data:` page has an opaque origin, which leaves many apps blank.

use base64::Engine;
use regex::{Captures, Regex};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).unwrap());
    };
}

re!(BG, r#"data-background-iframe="([^"]+)""#);
// data-app: an app pandoc must not inline (embed-resources turns src/data-src into data: URLs)
re!(
    IFRAME,
    r#"<iframe\b[^>]*?\b(?:data-src|src|data-app)="([^"]+)""#
);
re!(OBJ, r#"(<object\b[^>]*?\bdata=")([^"]+)(")"#);
// <image href> inside an inline <svg>: pandoc only inlines <img>
re!(SVGIMG, r#"(<image\b[^>]*?\b(?:xlink:)?href=")([^"]+)(")"#);
re!(
    NOTES,
    r#"(?s)<aside\b[^>]*\bclass="(?:[^"]*\s)?notes(?:\s[^"]*)?"[^>]*>.*?</aside>"#
);
re!(SCRIPT, r"(?is)<script\b.*?</script\b[^>]*>");
// a data: URI in an attribute (the attribute follows whitespace); size and kind are checked in code
re!(
    DATAURI,
    r#"(\s)(data-src|src|poster|data|xlink:href|href)="(data:[^"]+)""#
);
re!(SCHEME, r"^[A-Za-z][A-Za-z0-9+.-]*:");

/// Shortest `data:` URI worth de-duplicating (as `inline_apps.py`).
const DEDUPE_MIN: usize = 4000;

/// Is `p` a path relative to the deck (no scheme, not `//host`, not `#anchor`)?
pub fn local(p: &str) -> bool {
    !(p.is_empty() || p.starts_with("//") || p.starts_with('#') || SCHEME.is_match(p))
}

// ponytail: the five entities pandoc writes in attributes, not the full HTML table
pub(crate) fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The file a relative URL points at: without `?query`/`#hash`, `%`-decoded.
pub(crate) fn file_for(root: &Path, path: &str) -> PathBuf {
    let p = path.split(['?', '#']).next().unwrap_or("");
    root.join(&*percent_encoding::percent_decode_str(p).decode_utf8_lossy())
}

fn mime(f: &Path) -> String {
    mime_guess::from_path(f).first_or_octet_stream().to_string()
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Insert `snippet` before the script that calls `Reveal.initialize` (the
/// last mention: inlined libraries may name it too), else before `</body>`.
fn before_reveal(text: &str, snippet: &str) -> String {
    let k = text
        .rfind("Reveal.initialize(")
        .and_then(|k| text[..k].rfind("<script"))
        .or_else(|| text.rfind("</body>"))
        .unwrap_or(text.len());
    format!("{}{snippet}{}", &text[..k], &text[k..])
}

const RESTORE: &str = r#"<script>(() => {
  for (const a of ["data-src", "src", "poster", "data", "href"]) {
    const d = "data-dup-" + a;
    document.querySelectorAll("[" + d + "]").forEach((e) => {
      e.setAttribute(a, document.getElementById(e.getAttribute(d)).textContent);
      e.removeAttribute(d);
    });
  }
})();</script>
"#;

/// Keep each repeated large `data:` URI once, in a `<script type="text/plain">`;
/// a script puts it back into its attributes before reveal.js starts.
fn dedupe(text: &str) -> String {
    let worth = |u: &str| {
        u.len() >= DEDUPE_MIN
            && !u
                .split([';', ','])
                .next()
                .is_some_and(|t| t.ends_with("/javascript"))
    };
    let mut order: Vec<&str> = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for c in DATAURI.captures_iter(text) {
        let u = c.get(3).unwrap().as_str();
        if worth(u) {
            let n = counts.entry(u).or_default();
            if *n == 0 {
                order.push(u);
            }
            *n += 1;
        }
    }
    let ids: HashMap<&str, String> = order
        .iter()
        .filter(|u| counts[*u] > 1)
        .enumerate()
        .map(|(i, u)| (*u, format!("dup-{i}")))
        .collect();
    if ids.is_empty() {
        return text.to_string();
    }
    // xlink:href comes back as href, which SVG 2 reads the same
    let out = DATAURI.replace_all(text, |c: &Captures| match ids.get(&c[3]) {
        Some(id) => format!(
            "{}data-dup-{}=\"{id}\"",
            &c[1],
            c[2].trim_start_matches("xlink:")
        ),
        None => c[0].to_string(),
    });
    let holders: String = order
        .iter()
        .filter_map(|u| ids.get(u).map(|id| (u, id)))
        .map(|(u, id)| format!("<script type=\"text/plain\" id=\"{id}\">{u}</script>\n"))
        .collect();
    before_reveal(&out, &(holders + RESTORE))
}

// `window.qdApps` settles once every app has its blob: URL (qd's check waits for it)
const LOADER: &str = r#"
  const done = [];
  for (const [id, [path, type]] of Object.entries(apps)) {
    const gz = Uint8Array.from(atob(document.getElementById(id).textContent), (c) => c.charCodeAt(0));
    done.push(new Response(new Blob([gz]).stream().pipeThrough(new DecompressionStream("gzip"))).blob().then((b) => {
      const url = URL.createObjectURL(new Blob([b], { type }));
      document.querySelectorAll("[data-background-iframe]").forEach((e) => {
        if (e.getAttribute("data-background-iframe") === path) e.setAttribute("data-background-iframe", url);
      });
      document.querySelectorAll("iframe").forEach((f) => {
        if (![f.getAttribute("data-app"), f.getAttribute("data-src"), f.getAttribute("src")].includes(path)) return;
        f.setAttribute("data-src", url);
        // reveal.js got there first (the deck opened on this slide): load it now
        if (f.getAttribute("src") === path || (!f.getAttribute("src") && f.closest("section.present"))) f.setAttribute("src", url);
      });
    }));
  }
  window.qdApps = Promise.all(done);
})();</script>
"#;

/// The one-file deck and the referenced files that do not exist.
pub struct Inlined {
    pub html: String,
    pub missing: Vec<String>,
}

/// Inline a rendered deck's apps, `<object>` files and SVG `<image>` files
/// (paths relative to `root`, the deck's folder), store repeated `data:` URIs
/// once, and optionally remove speaker notes. Missing files are left as paths.
pub fn inline(text: &str, root: &Path, strip_notes: bool) -> Inlined {
    let mut missing = Vec::new();
    let mut text = if strip_notes {
        NOTES.replace_all(text, "").into_owned()
    } else {
        text.to_string()
    };
    for r in [&*OBJ, &*SVGIMG] {
        text = r
            .replace_all(&text, |c: &Captures| {
                let path = unescape(&c[2]);
                if !local(&path) {
                    return c[0].to_string();
                }
                let f = file_for(root, &path);
                match std::fs::read(&f) {
                    Ok(bytes) => {
                        format!("{}data:{};base64,{}{}", &c[1], mime(&f), b64(&bytes), &c[3])
                    }
                    Err(_) => {
                        missing.push(path);
                        c[0].to_string()
                    }
                }
            })
            .into_owned();
    }
    let text = dedupe(&text);

    // inlined libraries hold `<iframe src="${…}">`: look only outside scripts
    let markup = SCRIPT.replace_all(&text, "");
    let mut apps: Vec<String> = Vec::new();
    for c in BG
        .captures_iter(&markup)
        .chain(IFRAME.captures_iter(&markup))
    {
        let p = unescape(&c[1]);
        if local(&p) && !apps.contains(&p) {
            apps.push(p);
        }
    }
    let mut data = String::new();
    let mut paths = serde_json::Map::new();
    for (i, path) in apps.into_iter().enumerate() {
        let f = file_for(root, &path);
        let Ok(bytes) = std::fs::read(&f) else {
            missing.push(path);
            continue;
        };
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        gz.write_all(&bytes).unwrap();
        let gz = gz.finish().unwrap();
        let id = format!("inlined-app-{i}");
        data += &format!(
            "<script type=\"text/plain\" id=\"{id}\">{}</script>\n",
            b64(&gz)
        );
        paths.insert(id, serde_json::json!([path, mime(&f)]));
    }
    let html = if paths.is_empty() {
        text
    } else {
        let loader = format!(
            "{data}<script>(() => {{\n  const apps = {};\n{LOADER}",
            serde_json::Value::Object(paths)
        );
        before_reveal(&text, &loader) // before reveal.js creates the background iframes
    };
    Inlined { html, missing }
}

/// One of the largest parts of a one-file deck.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Part {
    /// Characters in the file (base64 included).
    pub bytes: usize,
    /// MIME type of a `data:` URI, `app <path> (gzip)`, or `script`.
    pub kind: String,
    /// The text just before it (alt text, an id), or a script's first characters.
    pub context: String,
}

re!(DATA64, r"data:([\w/+.-]+);base64,[A-Za-z0-9+/=]+");
re!(APPS, r"const apps = (\{.*?\});");
re!(SCRIPT_END, r"(?i)</script\b[^>]*>");
re!(SCRIPT_OPEN, r"(?is)<script\b([^>]*)>");
re!(APP_ID, r#"id="(inlined-app-\d+)""#);

/// Smallest part listed by [`largest`].
const PART_MIN: usize = 20_000;

fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The `n` largest embedded parts of a one-file deck: `data:` URIs and
/// scripts over 20 000 characters, inlined apps named by path.
pub fn largest(text: &str, n: usize) -> Vec<Part> {
    let mut items = Vec::new();
    for c in DATA64.captures_iter(text) {
        let m = c.get(0).unwrap();
        if m.len() < PART_MIN {
            continue;
        }
        let mut from = m.start().saturating_sub(160);
        while !text.is_char_boundary(from) {
            from -= 1;
        }
        let ctx: Vec<char> = squeeze(&text[from..m.start()]).chars().collect();
        items.push(Part {
            bytes: m.len(),
            kind: c[1].to_string(),
            context: ctx[ctx.len().saturating_sub(80)..].iter().collect(),
        });
    }
    let apps: HashMap<String, String> = APPS
        .captures(text)
        .and_then(|c| serde_json::from_str::<HashMap<String, (String, String)>>(&c[1]).ok())
        .map(|m| m.into_iter().map(|(k, (p, _))| (k, p)).collect())
        .unwrap_or_default();
    for chunk in SCRIPT_END.split(text) {
        let Some(c) = SCRIPT_OPEN.captures(chunk) else {
            continue;
        };
        let body = &chunk[c.get(0).unwrap().end()..];
        // a holder of one data: URI is listed above
        if body.len() <= PART_MIN || body.starts_with("data:") {
            continue;
        }
        let (kind, context) = match APP_ID.captures(&c[1]) {
            Some(a) => (
                format!(
                    "app {} (gzip)",
                    apps.get(&a[1]).unwrap_or(&a[1].to_string())
                ),
                String::new(),
            ),
            None => (
                "script".into(),
                body.chars().take(60).collect::<String>().replace('\n', " "),
            ),
        };
        items.push(Part {
            bytes: body.len(),
            kind,
            context,
        });
    }
    items.sort_by_key(|p| std::cmp::Reverse(p.bytes));
    items.truncate(n);
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    const DECK: &str = "<html><body><div class=\"reveal\"><div class=\"slides\">SLIDES</div></div>\n<script>Reveal.initialize({});</script>\n</body></html>";

    fn deck(slides: &str) -> String {
        DECK.replace("SLIDES", slides)
    }

    fn folder(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for (name, bytes) in files {
            let p = d.path().join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        }
        d
    }

    /// The gzipped, base64 app with this id, unpacked.
    fn app(html: &str, id: &str) -> Vec<u8> {
        let b64 = html.split_once(&format!("id=\"{id}\">")).unwrap().1;
        let gz = base64::engine::general_purpose::STANDARD
            .decode(b64.split_once("</script>").unwrap().0)
            .unwrap();
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&gz[..])
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    #[test]
    fn local_paths() {
        for p in ["apps/x.html", "../a b.png", "x.svg?v=1#y"] {
            assert!(local(p), "{p}");
        }
        for p in [
            "https://x/y",
            "data:image/png;base64,AA",
            "//cdn/x",
            "#s",
            "blob:x",
            "mailto:a",
            "",
        ] {
            assert!(!local(p), "{p}");
        }
    }

    #[test]
    fn apps_go_in_gzipped_and_load_from_blobs() {
        let d = folder(&[("apps/a.html", b"<p>A</p>"), ("apps/b%.html", b"<p>B</p>")]);
        let html = deck(concat!(
            r#"<section data-background-iframe="apps/a.html"></section>"#,
            r#"<section><iframe data-app="apps/b%25.html?x=1"></iframe></section>"#,
            r#"<section><iframe src="apps/a.html"></iframe><iframe src="https://example.org/"></iframe></section>"#,
            "<script>const t = `<iframe src=\"apps/in-a-library.html\">`;</script>",
        ));
        let out = inline(&html, d.path(), false);
        assert!(out.missing.is_empty(), "{:?}", out.missing);
        assert_eq!(
            app(&out.html, "inlined-app-0"),
            b"<p>A</p>",
            "bg iframe and src share one copy"
        );
        assert_eq!(app(&out.html, "inlined-app-1"), b"<p>B</p>");
        assert!(
            !out.html.contains("inlined-app-2"),
            "scripts and URLs are not apps"
        );
        assert!(
            out.html
                .contains(r#""inlined-app-1":["apps/b%25.html?x=1","text/html"]"#),
            "{}",
            out.html
        );
        let (loader, reveal) = (
            out.html.find("DecompressionStream").unwrap(),
            out.html.rfind("Reveal.initialize(").unwrap(),
        );
        assert!(loader < reveal, "loader runs before reveal.js starts");
        // the slides themselves keep their paths: the loader swaps in the blob: URLs
        assert!(out.html.contains(r#"data-background-iframe="apps/a.html""#));
    }

    #[test]
    fn objects_and_svg_images_become_data_uris() {
        let d = folder(&[("f/s.svg", b"<svg/>"), ("f/p.png", b"PNG")]);
        let html = deck(concat!(
            r#"<object data="f/s.svg" type="image/svg+xml"></object>"#,
            r#"<svg><image xlink:href="f/p.png"/><image href="f/p.png"/></svg>"#,
            r#"<object data="https://x/y.svg"></object>"#,
        ));
        let out = inline(&html, d.path(), false);
        assert!(out.missing.is_empty());
        assert!(out.html.contains(&format!(
            "<object data=\"data:image/svg+xml;base64,{}\"",
            b64(b"<svg/>")
        )));
        assert_eq!(
            out.html
                .matches(&format!("href=\"data:image/png;base64,{}\"", b64(b"PNG")))
                .count(),
            2
        );
        assert!(out.html.contains(r#"data="https://x/y.svg""#));
    }

    #[test]
    fn missing_files_are_listed_and_left_as_paths() {
        let d = folder(&[]);
        let html = deck(
            r#"<object data="gone.svg"></object><iframe data-app="apps/gone.html"></iframe><section data-background-iframe="a&amp;b.html"></section>"#,
        );
        let out = inline(&html, d.path(), false);
        assert_eq!(out.missing, vec!["gone.svg", "a&b.html", "apps/gone.html"]);
        assert!(
            out.html.contains(r#"data="gone.svg""#) && !out.html.contains("DecompressionStream")
        );
    }

    #[test]
    fn repeated_data_uris_are_stored_once() {
        let img = format!("data:image/png;base64,{}", "A".repeat(5000));
        let js = format!("data:application/javascript;base64,{}", "B".repeat(5000));
        let small = "data:image/gif;base64,R0lG";
        let html = deck(&format!(
            r#"<img src="{img}"><img src="{img}"><svg><image xlink:href="{img}"/></svg><script src="{js}"></script><script src="{js}"></script><img src="{small}"><img src="{small}">"#
        ));
        let out = dedupe(&html);
        assert_eq!(out.matches(&img).count(), 1, "stored once");
        assert_eq!(out.matches(r#"data-dup-src="dup-0""#).count(), 2);
        assert_eq!(
            out.matches(r#"data-dup-href="dup-0""#).count(),
            1,
            "xlink:href restored as href"
        );
        assert_eq!(out.matches(&js).count(), 2, "scripts are never deferred");
        assert_eq!(out.matches(small).count(), 2, "small URIs stay");
        assert!(
            out.find("<script type=\"text/plain\" id=\"dup-0\">")
                .unwrap()
                < out.rfind("Reveal.initialize(").unwrap()
        );
        assert_eq!(dedupe(&deck("<p>x</p>")), deck("<p>x</p>"));
    }

    #[test]
    fn strips_speaker_notes() {
        let html = deck(
            r#"<section><p>shown</p><aside class="notes">secret</aside><aside class="x notes y">two</aside><aside class="notesy">kept</aside></section>"#,
        );
        let out = inline(&html, Path::new("."), true);
        assert!(!out.html.contains("secret") && !out.html.contains("two"));
        assert!(out.html.contains("shown") && out.html.contains("kept"));
        assert!(inline(&html, Path::new("."), false).html.contains("secret"));
    }

    #[test]
    fn largest_parts_name_apps_by_path() {
        let d = folder(&[]);
        // noise: its gzip stays over the 20 000 characters listed
        let mut x = 1u32;
        let noise: Vec<u8> = (0..60_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        std::fs::create_dir(d.path().join("apps")).unwrap();
        std::fs::write(d.path().join("apps/big.html"), &noise).unwrap();
        let img = format!("data:image/jpeg;base64,{}", "C".repeat(30_000));
        let html = deck(&format!(
            r#"<iframe data-app="apps/big.html"></iframe><img alt="a   photo" src="{img}">"#
        ));
        let out = inline(&html, d.path(), false).html;
        let parts = largest(&out, 8);
        assert_eq!(parts[0].kind, "app apps/big.html (gzip)", "{parts:?}");
        assert_eq!(parts[1].kind, "image/jpeg");
        assert!(
            parts[1].context.ends_with(r#"<img alt="a photo" src=""#),
            "{:?}",
            parts[1].context
        );
        assert_eq!(parts.len(), 2);
        assert_eq!(largest(&out, 1).len(), 1);
    }
}
