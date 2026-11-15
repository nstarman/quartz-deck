//! `qd bib migrate` (spec 014 US5, research R11): a deck citing with the
//! old `{{< paper >}}` shortcode cites with Pandoc's own `@key` instead.
//! The before/after check is the safety net: any difference in what the
//! audience sees fails the conversion and puts the deck back, unless the
//! person accepts the report.

use crate::bib::{self, Format};
use crate::deck::{Deck, Span};
use crate::error::{QdError, Result};
use crate::store::{OtherFile, Store};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static SHORTCODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{<\s*paper\s+(.*?)\s*>\}\}").expect("valid regex"));
static ARG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\w+)="([^"]*)""#).expect("valid regex"));
static MAKE_REFS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"qd bib --zotero (\$\([A-Z_]+\)|[A-Za-z0-9]{8})").expect("valid regex")
});

/// One `{{< paper url= who= year= title= [corner="true"] >}}`.
#[derive(Debug, Clone)]
pub struct Shortcode {
    pub span: Span,
    pub slide: String,
    pub url: String,
    pub who: String,
    pub year: String,
    pub title: String,
    pub corner: bool,
    /// In the old References list (between the bibliography markers).
    pub listed: bool,
}

/// The lines from `<!-- bibliography:start` to `<!-- bibliography:end -->`, whole.
fn marker_block(text: &str) -> Option<Span> {
    let start = text.find("<!-- bibliography:start")?;
    let start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let end = start + text[start..].find("<!-- bibliography:end -->")?;
    let end = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
    Some(Span { start, end })
}

/// Every paper shortcode in the deck; an argument qd doesn't know stops it.
pub fn shortcodes(d: &Deck) -> Result<Vec<Shortcode>> {
    let list = marker_block(&d.text);
    let slide_at = |at: usize| {
        d.slides
            .iter()
            .find(|s| s.span.start <= at && at < s.span.end)
            .map_or("front-matter".to_string(), |s| s.id.clone())
    };
    SHORTCODE
        .captures_iter(&d.text)
        .map(|c| {
            let m = c.get(0).expect("whole match");
            let slide = slide_at(m.start());
            let mut sc = Shortcode {
                span: Span { start: m.start(), end: m.end() },
                listed: list.is_some_and(|l| l.start <= m.start() && m.end() <= l.end),
                url: String::new(),
                who: String::new(),
                year: String::new(),
                title: String::new(),
                corner: false,
                slide,
            };
            for a in ARG.captures_iter(&c[1]) {
                let v = a[2].to_string();
                match &a[1] {
                    "url" => sc.url = v,
                    "who" => sc.who = v,
                    "year" => sc.year = v,
                    "title" => sc.title = v,
                    "corner" => sc.corner = v == "true",
                    other => {
                        return Err(QdError::InvalidArg(format!(
                            "slide {}: the paper shortcode has an argument qd doesn't know: {other}",
                            sc.slide
                        )));
                    }
                }
            }
            if !ARG.replace_all(&c[1], "").trim().is_empty() {
                return Err(QdError::InvalidArg(format!(
                    "slide {}: a paper shortcode qd can't read: {}",
                    sc.slide,
                    m.as_str()
                )));
            }
            Ok(sc)
        })
        .collect()
}

/// What identifies the shortcode's paper: its link (a DOI when it is one) and title.
fn identity(sc: &Shortcode) -> Value {
    let mut e = json!({"title": sc.title});
    match sc.url.strip_prefix("https://doi.org/") {
        Some(doi) => e["DOI"] = json!(doi),
        None if !sc.url.is_empty() => e["URL"] = json!(sc.url),
        None => {}
    }
    e
}

/// A reference built from the shortcode itself (`who` as a literal author,
/// so it renders as before), keyed `firstword + year`.
pub fn entry_of(sc: &Shortcode, taken: &HashSet<String>) -> Value {
    let year: String = sc
        .year
        .chars()
        .filter(char::is_ascii_digit)
        .take(4)
        .collect();
    let mut e = identity(sc);
    e["type"] = json!("article-journal");
    if !sc.who.is_empty() {
        e["author"] = json!([{"literal": sc.who}]);
    }
    if !year.is_empty() {
        e["issued"] = json!({"date-parts": [[year.parse::<u32>().unwrap_or(0)]]});
    }
    let first = sc
        .who
        .split(|c: char| c == ',' || c == '&' || c.is_whitespace())
        .find(|w| !w.is_empty())
        .unwrap_or("");
    let key_source =
        json!({"author": [{"literal": first}], "issued": e["issued"], "title": sc.title});
    e["id"] = json!(bib::key_for(&key_source, taken));
    e
}

/// The deck's text with each shortcode cited by its key, and the marker list
/// replaced by a References div in today's look.
pub fn rewrite(d: &Deck, cites: &[(Shortcode, String)]) -> String {
    let nl = d.nl;
    let mut reps: Vec<(Span, String)> = cites
        .iter()
        .filter(|(sc, _)| !sc.listed)
        .map(|(sc, key)| {
            let cite = if sc.corner {
                format!("[[{}]]{{.corner}}", bib::cite(key))
            } else {
                bib::cite(key)
            };
            (sc.span, cite)
        })
        .collect();
    if let Some(l) = marker_block(&d.text) {
        reps.push((
            l,
            format!("::: {{#refs refs-style=\"compact\" refs-columns=\"3\"}}{nl}:::{nl}"),
        ));
    }
    reps.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
    let mut text = d.text.clone();
    for (s, new) in reps {
        text.replace_range(s.start..s.end, &new);
    }
    text
}

/// The Zotero key the Makefile's `refs` target passes to `qd bib --zotero`.
pub fn makefile_key(makefile: &str) -> Option<String> {
    let c = MAKE_REFS.captures(makefile)?;
    let v = c[1].to_string();
    match v.strip_prefix("$(").and_then(|n| n.strip_suffix(')')) {
        // a variable: its value, `NAME = KEY` (a comment may follow)
        Some(name) => makefile.lines().find_map(|l| {
            let (k, rest) = l.split_once('=')?;
            (k.trim() == name)
                .then(|| rest.split('#').next().unwrap_or("").trim().to_string())
                .filter(|k| k.len() == 8 && k.chars().all(|c| c.is_ascii_alphanumeric()))
        }),
        None => Some(v),
    }
}

/// The Makefile with `qd bib --zotero KEY` as `qd bib pull --from zotero KEY`.
pub fn makefile_rewrite(makefile: &str) -> String {
    MAKE_REFS
        .replace_all(makefile, "qd bib pull --from zotero $1")
        .into_owned()
}

/// A link the audience sees: where it goes and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    pub href: String,
    pub text: String,
}

/// Per slide (by section id, in order), the links with `class` in rendered HTML.
pub fn links(html: &str, class: &str) -> Vec<(String, Vec<Shown>)> {
    static SECTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"<section id="([^"]*)""#).expect("valid regex"));
    static A: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?s)<a ([^>]*)>(.*?)</a>"#).expect("valid regex"));
    static HREF: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"href="([^"]*)""#).expect("valid regex"));
    static CLASS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"class="([^"]*)""#).expect("valid regex"));
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid regex"));
    let starts: Vec<(usize, String)> = SECTION
        .captures_iter(html)
        .map(|c| (c.get(0).expect("match").start(), c[1].to_string()))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, (at, id))| {
            let end = starts.get(i + 1).map_or(html.len(), |n| n.0);
            let shown = A
                .captures_iter(&html[*at..end])
                .filter(|c| {
                    CLASS
                        .captures(&c[1])
                        .is_some_and(|k| k[1].split_whitespace().any(|w| w == class))
                })
                .map(|c| Shown {
                    href: HREF
                        .captures(&c[1])
                        .map_or(String::new(), |h| h[1].to_string()),
                    text: TAG
                        .replace_all(&c[2], "")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                })
                .collect();
            (id.clone(), shown)
        })
        .collect()
}

/// Every difference, slide by slide, between the links before and after.
pub fn compare(before: &[(String, Vec<Shown>)], after: &[(String, Vec<Shown>)]) -> Vec<Value> {
    let none = vec![];
    let mut out = Vec::new();
    for (slide, was) in before {
        let now = after
            .iter()
            .find(|(s, _)| s == slide)
            .map_or(&none, |(_, l)| l);
        for i in 0..was.len().max(now.len()) {
            let (b, a) = (was.get(i), now.get(i));
            let kind = match (b, a) {
                (Some(b), Some(a)) if b.text != a.text => "text",
                (Some(b), Some(a)) if b.href != a.href => "href",
                (Some(_), Some(_)) => continue,
                (Some(_), None) => "missing",
                _ => "extra",
            };
            let j = |s: Option<&Shown>| s.map(|s| json!({"href": s.href, "text": s.text}));
            out.push(json!({"slide": slide, "kind": kind, "before": j(b), "after": j(a)}));
        }
    }
    out
}

/// The options of `qd bib migrate`.
pub struct Options {
    /// A source plugin and its arguments; else the Makefile's Zotero key.
    pub from: Option<(String, Vec<String>)>,
    pub backup: Option<PathBuf>,
    pub accept: bool,
    pub dry_run: bool,
    pub expect: Option<String>,
}

fn in_git(dir: &Path) -> bool {
    crate::history::git(dir, &["rev-parse", "--is-inside-work-tree"])
        .is_ok_and(|o| o.trim() == "true")
}

/// `qd bib migrate`: convert, render before and after, compare, and keep the
/// result only with no difference or `--accept` (research R11).
pub fn run(store: &Store, o: &Options) -> Result<Value> {
    let d = store.read()?;
    if let Some(e) = o.expect.as_ref().filter(|e| **e != d.version) {
        return Err(QdError::Conflict(format!(
            "expected version {e}, file is at {}",
            d.version
        )));
    }
    let dir = store.path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let scs = shortcodes(&d)?;
    if scs.is_empty() {
        return Err(QdError::InvalidArg(
            "the deck has no paper shortcodes to convert".into(),
        ));
    }
    if o.backup.is_none() && !in_git(&dir) {
        return Err(QdError::InvalidArg(format!(
            "{} is not under git: pass --backup DIR so the deck can be put back",
            dir.display()
        )));
    }
    let makefile = dir.join("Makefile");
    let make_text = fs::read_to_string(&makefile).ok();
    let from = o.from.clone().or_else(|| {
        let key = makefile_key(make_text.as_deref()?)?;
        Some(("zotero".to_string(), vec![key]))
    });

    // the references: what the deck has, the source's, then the shortcodes' own
    let r = bib::refs(&d, &dir);
    let target = r.writable()?.map(|f| f.path.clone());
    let existing = [r.file_entries()?, r.embedded.clone()].concat();
    let source = match &from {
        Some((name, args)) => bib::from_source(name, args)?,
        None => vec![],
    };
    let m = bib::merge(existing, source);
    let mut entries = m.entries;
    let mut taken: HashSet<String> = entries
        .iter()
        .map(|e| e["id"].as_str().unwrap_or("").to_string())
        .collect();
    let (mut cites, mut unmatched) = (Vec::new(), Vec::new());
    for sc in scs {
        let key = match bib::find_match(&entries, &identity(&sc)) {
            Some(i) => entries[i]["id"].as_str().unwrap_or("").to_string(),
            None => {
                let e = entry_of(&sc, &taken);
                let key = e["id"].as_str().unwrap_or("").to_string();
                taken.insert(key.clone());
                entries.push(e);
                unmatched
                    .push(json!({"slide": sc.slide, "url": sc.url, "who": sc.who, "key": key}));
                key
            }
        };
        cites.push((sc, key));
    }

    // the new deck, references file, Makefile; `paper` goes
    let (path, written) = match &target {
        Some(p) => (p.clone(), None),
        None => {
            let w = bib::default_place(&dir, Format::Bib);
            (dir.join(&w), Some(w))
        }
    };
    let mut text = rewrite(&d, &cites);
    if let Some(w) = &written {
        text = bib::set_front(
            &text,
            &[(
                "bibliography",
                Some(serde_yaml_ng::Value::String(w.clone())),
            )],
        )?;
    }
    // the old list held every entry (the whole collection), cited or not;
    // without one, citeproc would append a list to the last slide
    let (key, value) = match marker_block(&d.text) {
        Some(_) => ("nocite", serde_yaml_ng::Value::String("@*".into())),
        None => ("suppress-bibliography", serde_yaml_ng::Value::Bool(true)),
    };
    text = bib::set_front(&text, &[(key, Some(value))])?;
    let text = crate::plugins::with_filters(&text)?;
    let mut also = vec![bib::file_change(
        &path,
        bib::file_text(&entries, Format::of(&path))?,
    )];
    if let Some(mt) = &make_text {
        let new = makefile_rewrite(mt);
        if new != *mt {
            also.push(bib::file_change(&makefile, new));
        }
    }
    let paper = dir.join("_extensions/paper");
    for f in fs::read_dir(&paper).into_iter().flatten().flatten() {
        if f.path().is_file() {
            also.push(OtherFile {
                before: fs::read_to_string(f.path()).ok(),
                after: None,
                path: f.path(),
            });
        }
    }
    let n_inline = cites.iter().filter(|(s, _)| !s.listed && !s.corner).count();
    let n_corner = cites.iter().filter(|(s, _)| s.corner).count();
    let mut report = json!({
        "ok": true, "op": "bib-migrate", "deck": store.path, "accepted": o.accept,
        "entries": {"total": entries.len(), "from_shortcodes": unmatched.len()},
        "unmatched": unmatched, "citations": {"inline": n_inline, "corner": n_corner},
        "warnings": m.warnings, "diff": crate::ops::diff(&d.text, &text),
    });
    if o.dry_run {
        report["dry_run"] = json!(true);
        report["version"] = json!(d.version);
        return Ok(report);
    }

    // a backup of every file the conversion touches, before any write
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_secs());
    let work = store.cache_dir().join("migrate").join(ts.to_string());
    let backup = o.backup.clone().unwrap_or_else(|| work.join("backup"));
    for f in std::iter::once(&store.path).chain(also.iter().map(|a| &a.path)) {
        if let (Ok(rel), true) = (f.strip_prefix(bib::canonical(&dir)), f.is_file()) {
            let to = backup.join(rel);
            fs::create_dir_all(to.parent().expect("inside the backup"))?;
            fs::copy(f, to)?;
        }
    }
    report["backup"] = json!(backup);

    // render before, convert, render after
    let before = crate::render::render(&store.path, &d.text, "migrate-before", &work, true)?;
    let before = fs::read_to_string(before)?;
    let absent: Vec<&str> = crate::plugins::EXTENSIONS
        .iter()
        .map(|e| e.name)
        .filter(|n| !dir.join("_extensions").join(n).exists())
        .collect();
    let version = store.write_many(&d, &text, also, "bib-migrate", "references")?;
    report["installed"] = crate::plugins::install(&dir)?["installed"].clone();
    let put_back = || -> Result<()> {
        store.undo()?;
        for n in &absent {
            let _ = fs::remove_dir_all(dir.join("_extensions").join(n));
        }
        Ok(())
    };
    let after = match crate::render::render(&store.path, &text, "migrate-after", &work, true) {
        Ok(p) => fs::read_to_string(p)?,
        Err(e) => {
            put_back()?;
            return Err(e);
        }
    };
    let differences = compare(&links(&before, "paper-ref"), &links(&after, "cite-link"));
    report["differences"] = json!(differences);
    let path = work.join("report.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&report).expect("JSON values serialize"),
    )?;
    report["report"] = json!(path);
    if !differences.is_empty() && !o.accept {
        put_back()?;
        return Err(QdError::MigrateDiff(format!(
            "{} difference(s) between the deck before and after; the deck is back as it was. Report: {}",
            differences.len(),
            path.display()
        )));
    }
    report["version"] = json!(version);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &str = "---\ntitle: T\n---\n\n## Results {#res}\n\n{{< paper url=\"https://doi.org/10.1/a\" who=\"Bonaca & Hogg\" year=\"2020\" title=\"Spur\" corner=\"true\" >}}\n\nSee {{< paper url=\"http://arxiv.org/abs/2602.01806\" who=\"Myers, Starkman & Necib\" year=\"2025\" title=\"PINNs\" >}} here.\n\n## References {#bibliography .bibliography}\n\n<!-- bibliography:start (written by qd bib) -->\n- {{< paper url=\"https://doi.org/10.1/a\" who=\"Bonaca & Hogg\" year=\"2020\" title=\"Spur\" >}}\n<!-- bibliography:end -->\n\n## End\n";

    #[test]
    fn reads_shortcodes_and_refuses_unknown_arguments() {
        let d = Deck::parse(DECK.to_string());
        let scs = shortcodes(&d).unwrap();
        assert_eq!(scs.len(), 3);
        assert!(scs[0].corner && !scs[0].listed && scs[0].slide == "res");
        assert_eq!(
            (scs[1].who.as_str(), scs[1].year.as_str()),
            ("Myers, Starkman & Necib", "2025")
        );
        assert!(scs[2].listed);
        let bad = DECK.replace("corner=\"true\"", "size=\"big\"");
        let e = shortcodes(&Deck::parse(bad)).unwrap_err();
        assert_eq!(e.code(), "E_INVALID_ARG");
        assert!(
            e.to_string().contains("res") && e.to_string().contains("size"),
            "{e}"
        );
    }

    #[test]
    fn builds_entries_from_shortcodes() {
        let d = Deck::parse(DECK.to_string());
        let scs = shortcodes(&d).unwrap();
        let taken = HashSet::from(["myers2025".to_string()]);
        let a = entry_of(&scs[0], &HashSet::new());
        assert_eq!(a["id"], "bonaca2020");
        assert_eq!(a["DOI"], "10.1/a");
        assert_eq!(a["author"], json!([{"literal": "Bonaca & Hogg"}]));
        let b = entry_of(&scs[1], &taken);
        assert_eq!(b["id"], "myers2025a");
        assert_eq!(b["URL"], "http://arxiv.org/abs/2602.01806");
        assert_eq!(b["issued"], json!({"date-parts": [[2025]]}));
    }

    #[test]
    fn rewrites_citations_and_the_list_only() {
        let d = Deck::parse(DECK.to_string());
        let scs = shortcodes(&d).unwrap();
        let keys = ["bonaca2020", "myers2026", "bonaca2020"];
        let cites: Vec<_> = scs
            .into_iter()
            .zip(keys)
            .map(|(s, k)| (s, k.to_string()))
            .collect();
        let out = rewrite(&d, &cites);
        let want = DECK
            .replace(
                "{{< paper url=\"https://doi.org/10.1/a\" who=\"Bonaca & Hogg\" year=\"2020\" title=\"Spur\" corner=\"true\" >}}",
                "[[@bonaca2020]]{.corner}",
            )
            .replace(
                "{{< paper url=\"http://arxiv.org/abs/2602.01806\" who=\"Myers, Starkman & Necib\" year=\"2025\" title=\"PINNs\" >}}",
                "@myers2026",
            )
            .replace(
                "<!-- bibliography:start (written by qd bib) -->\n- {{< paper url=\"https://doi.org/10.1/a\" who=\"Bonaca & Hogg\" year=\"2020\" title=\"Spur\" >}}\n<!-- bibliography:end -->\n",
                "::: {#refs refs-style=\"compact\" refs-columns=\"3\"}\n:::\n",
            );
        assert_eq!(out, want);
    }

    #[test]
    fn makefile_key_and_rewrite() {
        let mk = "ZOTERO = S376YTIE   # the key\nrefs:\n\tqd bib --zotero $(ZOTERO)\n";
        assert_eq!(makefile_key(mk).as_deref(), Some("S376YTIE"));
        assert_eq!(
            makefile_rewrite(mk),
            mk.replace("qd bib --zotero", "qd bib pull --from zotero")
        );
        assert_eq!(
            makefile_key("refs:\n\tqd bib --zotero ABCD1234\n").as_deref(),
            Some("ABCD1234")
        );
        assert_eq!(
            makefile_key("ZOTERO = XXXX\nrefs:\n\tqd bib --zotero $(ZOTERO)\n"),
            None
        );
        assert_eq!(makefile_key("all:\n\tqd export\n"), None);
    }

    #[test]
    fn compares_links_per_slide() {
        let html = |a: &str, b: &str| {
            format!(
                r#"<section id="one" class="slide"><p><a href="x" class="{a}"><span></span> Bonaca (2020)</a></p></section><section id="two"><a href="y" class="{a} {b}"><span></span></a></section>"#
            )
        };
        let before = links(&html("paper-ref", "paper-corner"), "paper-ref");
        assert_eq!(
            before[0].1,
            vec![Shown {
                href: "x".into(),
                text: "Bonaca (2020)".into()
            }]
        );
        let after = links(&html("cite-link", "cite-corner"), "cite-link");
        assert!(compare(&before, &after).is_empty());
        let moved = links(
            &html("cite-link", "cite-corner").replace("href=\"x\"", "href=\"z\""),
            "cite-link",
        );
        assert_eq!(compare(&before, &moved)[0]["kind"], "href");
        let renamed = links(
            &html("cite-link", "x").replace("Bonaca (2020)", "Bonaca et al. (2020)"),
            "cite-link",
        );
        assert_eq!(compare(&before, &renamed)[0]["kind"], "text");
        let gone = links(
            r#"<section id="one"></section><section id="two"></section>"#,
            "cite-link",
        );
        let kinds: Vec<_> = compare(&before, &gone)
            .iter()
            .map(|d| d["kind"].clone())
            .collect();
        assert_eq!(kinds, [json!("missing"), json!("missing")]);
    }
}
