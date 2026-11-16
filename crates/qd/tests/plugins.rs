//! `plugins/` (constitution VII): each plugin is a Quarto extension first,
//! with an example deck that renders with plain `quarto render` and no qd.

mod common;

use common::have;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `dir`, as paths relative to it, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push(p.strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    out
}

/// Each plugin's `example.qmd` renders with plain Quarto (no qd).
#[test]
fn plugin_examples_render_with_plain_quarto() {
    if !have("quarto") {
        return;
    }
    let plugins = root().join("plugins");
    let mut seen = 0;
    for e in std::fs::read_dir(&plugins).unwrap() {
        let dir = e.unwrap().path();
        if !dir.join("example.qmd").is_file() {
            continue;
        }
        seen += 1;
        let name = dir.file_name().unwrap().to_string_lossy();
        render_example(&name, |t| t, &[]);
    }
    assert!(seen > 0, "no plugin has an example.qmd");
}

/// `plugins/<name>/`, copied into a temp folder.
fn copy_plugin(name: &str) -> tempfile::TempDir {
    let dir = root().join("plugins").join(name);
    let t = tempfile::tempdir().unwrap();
    for f in files(&dir) {
        let to = t.path().join(&f);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(dir.join(&f), &to).unwrap();
    }
    t
}

/// Copy `plugins/<name>/` into a temp folder, change its example with `edit`,
/// render it with plain Quarto; the HTML and Quarto's stderr.
fn render_example(
    name: &str,
    edit: impl Fn(String) -> String,
    extra: &[(&str, &str)],
) -> (String, String) {
    let t = copy_plugin(name);
    for (f, text) in extra {
        std::fs::write(t.path().join(f), text).unwrap();
    }
    let qmd = t.path().join("example.qmd");
    std::fs::write(&qmd, edit(std::fs::read_to_string(&qmd).unwrap())).unwrap();
    let out = std::process::Command::new("quarto")
        .args(["render", "example.qmd", "--to", "revealjs"])
        .current_dir(t.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{name}: {stderr}");
    let html = std::fs::read_to_string(t.path().join("example.html")).unwrap();
    (html, stderr)
}

/// The `<a …>…</a>` elements with class `cite-link`, as (opening tag, text).
fn cite_links(html: &str) -> Vec<(String, String)> {
    let re = regex::Regex::new(r#"(?s)<a ([^>]*class="cite-link[^"]*"[^>]*)>(.*?)</a>"#).unwrap();
    re.captures_iter(html)
        .map(|c| (c[1].to_string(), text(&c[2])))
        .collect()
}

/// Spec 014 US1: citations link to the paper, with today's labels, an icon,
/// the title on hover, and the corner form.
#[test]
fn cite_links_example() {
    if !have("quarto") {
        return;
    }
    let (html, stderr) = render_example("cite-links", |t| t, &[]);
    let links = cite_links(&html);
    let find = |text: &str| {
        links
            .iter()
            .find(|(_, t)| t == text)
            .unwrap_or_else(|| panic!("no link reading '{text}' in {links:#?}"))
            .0
            .clone()
    };
    for (text, href, title) in [
        (
            "Arden (2019)",
            "https://example.org/arden",
            "Arden 2019: One author",
        ),
        (
            "Bell &amp; Cho (2020)",
            "https://doi.org/10.5555/bell",
            "Bell &amp; Cho 2020: Two authors",
        ),
        (
            "Dunn et al. (2021)",
            "https://doi.org/10.5555/dunn",
            "Dunn et al. 2021: Three authors",
        ),
        (
            "Evans et al. (2022a)",
            "https://doi.org/10.5555/evans1",
            "Evans et al. 2022a: First of two",
        ),
        (
            "Evans et al. (2022b)",
            "https://doi.org/10.5555/evans2",
            "Evans et al. 2022b: Second of two",
        ),
        // arXiv's own DOI yields to the entry's page
        (
            "Example Survey Collaboration (2023)",
            "https://arxiv.org/abs/0000.00001",
            "Example Survey Collaboration 2023: An arXiv preprint",
        ),
    ] {
        let tag = find(text);
        assert!(tag.contains(&format!(r#"href="{href}""#)), "{text}: {tag}");
        assert!(
            tag.contains(&format!(r#"title="{title}""#)),
            "{text}: {tag}"
        );
    }
    assert!(
        html.contains(r#"<span class="cite-icon"></span>"#),
        "the icon"
    );
    // two keys in one citation: a link each (the locator stays in its key's link)
    assert!(find("Bell &amp; Cho 2020").contains("10.5555/bell"));
    assert!(find("Dunn et al. 2021, p. 3").contains("10.5555/dunn"));
    // the corner form: the icon alone
    let corner: Vec<_> = links
        .iter()
        .filter(|(tag, _)| tag.contains("cite-corner"))
        .collect();
    assert_eq!(corner.len(), 1, "{links:#?}");
    assert_eq!(corner[0].1, "");
    // no DOI or URL: plain text, and a warning
    assert!(html.contains("Nolan (2024)") && !links.iter().any(|(_, t)| t.contains("Nolan")));
    assert!(
        stderr.contains("cite-links: no DOI or URL for 'nolink2024'"),
        "{stderr}"
    );

    // the deck's own choices win
    let (html, _) = render_example(
        "cite-links",
        |t| {
            t.replace(
                "filters: [cite-links]\n",
                "filters: [cite-links]\ncite-links:\n  icon: false\n  link: references\n",
            )
        },
        &[],
    );
    assert_eq!(
        html.matches(r#"class="cite-icon""#).count(),
        1,
        "no icon but the corner's, which is only an icon"
    );
    assert!(
        cite_links(&html)
            .iter()
            .any(|(tag, _)| tag.contains(r##"href="#/ref-bell2020""##))
    );
    let own = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>own</title><id>own</id><updated>2026-01-01T00:00:00+00:00</updated></info>
  <citation><layout><text value="OWN-STYLE"/></layout></citation>
  <bibliography><layout><text variable="title"/></layout></bibliography>
</style>
"#;
    let (html, _) = render_example(
        "cite-links",
        |t| {
            t.replace(
                "bibliography: references.bib\n",
                "bibliography: references.bib\ncsl: own.csl\n",
            )
        },
        &[("own.csl", own)],
    );
    assert!(html.contains("OWN-STYLE"), "the deck's csl: wins");

    // embedded references keep CSL's capitalised DOI/URL (a .bib gives doi/url)
    let (html, _) = render_example(
        "cite-links",
        |t| {
            t.replace(
                "bibliography: references.bib\n",
                "references:\n- id: bell2020\n  type: article-journal\n  author: [{family: Bell}, {family: Cho}]\n  issued: {date-parts: [[2020]]}\n  title: Two authors\n  DOI: 10.5555/bell\n",
            )
        },
        &[],
    );
    assert!(
        cite_links(&html)
            .iter()
            .any(|(tag, _)| tag.contains(r#"href="https://doi.org/10.5555/bell""#)),
        "an embedded entry links to its DOI"
    );
}

/// `qd check --offline`'s problems for `example.qmd` in `dir`, or None
/// without Quarto or a browser (downloads are off).
fn offline_problems(dir: &Path) -> Option<Vec<serde_json::Value>> {
    let qd = |a: &[&str]| {
        common::qd(dir)
            .env("QD_NO_DOWNLOAD", "1")
            .args(a)
            .output()
            .unwrap()
    };
    if !have("quarto") || !qd(&["browser"]).status.success() {
        eprintln!("no quarto or browser: skipping");
        return None;
    }
    let out = qd(&["--deck", "example.qmd", "check", "--offline"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    Some(
        report["problems"]
            .as_array()
            .expect("a check report")
            .clone(),
    )
}

/// Spec 014 (T019): cite-links needs no network: its CSS and icon are
/// bundled. Needs Quarto and a browser; skips otherwise.
#[test]
fn cite_links_example_works_offline() {
    let t = copy_plugin("cite-links");
    let Some(problems) = offline_problems(t.path()) else {
        return;
    };
    // every reveal.js deck loads MathJax from its CDN until `qd export`
    // bundles it; anything else would come from the extension
    let others: Vec<_> = problems
        .iter()
        .filter(|p| !p["url"].as_str().unwrap_or("").contains("/mathjax@"))
        .collect();
    assert!(others.is_empty(), "{others:#?}");
}

/// The HTML of the `#refs` div (to the end of its slide).
fn refs_html(html: &str) -> String {
    let start = html.find(r#"<div id="refs""#).expect("a #refs div");
    let end = html[start..]
        .find("</section>")
        .map_or(html.len(), |e| start + e);
    html[start..end].to_string()
}

/// Text without tags, spaces collapsed.
fn text(html: &str) -> String {
    static TAGS: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());
    TAGS.replace_all(html, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Spec 014 US4 (T027): the References slide in the style, columns and size
/// chosen on it, every entry linked to its paper.
#[test]
fn refs_slide_example() {
    if !have("quarto") {
        return;
    }
    let with = |attrs: &str| {
        render_example(
            "refs-slide",
            |t| {
                t.replace(
                    r#"{#refs refs-style="compact" refs-columns="3" refs-size="smaller"}"#,
                    &format!("{{#refs {attrs}}}"),
                )
            },
            &[],
        )
        .0
    };

    // the defaults: compact, 3 columns, smaller
    let html = with("");
    let refs = refs_html(&html);
    assert!(
        refs.starts_with(
            r#"<div id="refs" class="refs-slide refs-compact refs-cols-3 refs-size-smaller"#
        ),
        "{refs}"
    );
    // compact labels are the citations' own, suffixes included
    let cited = |key: &str| {
        let re = regex::Regex::new(&format!(
            r#"(?s)<span class="citation" data-cites="{key}">(.*?)</span>"#
        ))
        .unwrap();
        text(&re.captures(&html).unwrap_or_else(|| panic!("{key} cited"))[1])
    };
    let entry = |key: &str| {
        let re =
            regex::Regex::new(&format!(r#"(?s)<div id="ref-{key}"[^>]*>(.*?)</div>"#)).unwrap();
        re.captures(&refs)
            .unwrap_or_else(|| panic!("entry {key}: {refs}"))[1]
            .to_string()
    };
    for (key, href) in [
        ("evans2022first", "https://doi.org/10.5555/evans1"),
        ("evans2022second", "https://doi.org/10.5555/evans2"),
        ("bell2020", "https://doi.org/10.5555/bell"),
        ("survey2023", "https://arxiv.org/abs/0000.00001"),
    ] {
        let entry = entry(key);
        assert!(
            entry.contains(&format!(r#"href="{href}""#)),
            "{key}: {entry}"
        );
        assert!(entry.contains(r#"class="cite-icon""#), "{key}: {entry}");
        assert_eq!(text(&entry), cited(key), "{key}");
    }
    // however the deck's style tells them apart (talk.csl: 2022a/b; the default: co-authors)
    assert_ne!(cited("evans2022first"), cited("evans2022second"));
    // no DOI or URL: listed, not linked
    assert!(!entry("nolink2024").contains("href"), "{refs}");

    // short, 2 columns
    let refs = refs_html(&with(r#"refs-style="short" refs-columns="2""#));
    assert!(
        refs.contains(r#"class="refs-slide refs-short refs-cols-2 refs-size-smaller"#),
        "{refs}"
    );
    let t = text(&refs);
    assert!(t.contains("Bell &amp; Cho (2020), Example Journal"), "{t}");
    assert!(t.contains("Kim et al. (2018), Example Letters"), "{t}");
    assert!(refs.contains("<em>Example Journal</em>"));
    assert!(refs.contains(r#"href="https://doi.org/10.5555/kim""#));

    // full, 1 column, smallest: citeproc's entries, linked
    let refs = refs_html(&with(
        r#"refs-style="full" refs-columns="1" refs-size="smallest""#,
    ));
    assert!(
        refs.contains(r#"class="refs-slide refs-full refs-cols-1 refs-size-smallest"#),
        "{refs}"
    );
    // the deck's style decides the case (the default, Chicago, title-cases)
    let t = text(&refs);
    assert!(t.to_lowercase().contains("two authors"), "{t}");
    assert!(
        refs.contains(r#"href="https://doi.org/10.5555/bell""#),
        "{refs}"
    );
}

/// Spec 014 (T029): Quarto makes the slide holding `#refs` `smaller
/// scrollable`, so a long list scrolls, and `qd check --offline` must not
/// call it an overflow (it did before 014). Needs Quarto and a browser.
#[test]
fn long_refs_slide_scrolls_without_an_overflow_warning() {
    let t = copy_plugin("refs-slide");
    // 30 made-up entries, all listed, in the longest form on one column
    let bib: String = (0..30)
        .map(|n| format!("@book{{long{n},\n  author = {{Long, Lee}},\n  title = {{Entry number {n} with a title long enough to wrap}},\n  year = {{{}}},\n  doi = {{10.5555/long{n}}}\n}}\n", 1990 + n))
        .collect();
    std::fs::write(t.path().join("references.bib"), bib).unwrap();
    std::fs::write(
        t.path().join("example.qmd"),
        "---\nformat: revealjs\nbibliography: references.bib\nnocite: '@*'\nfilters: [refs-slide]\n---\n\n## References\n\n::: {#refs refs-style=\"full\" refs-columns=\"1\" refs-size=\"normal\"}\n:::\n",
    )
    .unwrap();
    let Some(problems) = offline_problems(t.path()) else {
        return;
    };
    assert!(
        !problems.iter().any(|p| p["code"] == "P_OVERFLOW"),
        "{problems:#?}"
    );
}

/// The SVG of node `name` in rendered `html`: from its `<title>` to its `</g>`.
fn node_svg<'a>(html: &'a str, name: &str) -> &'a str {
    let at = html
        .find(&format!("<title>{name}</title>"))
        .unwrap_or_else(|| panic!("no node {name}"));
    &html[at..at + html[at..].find("</g>").unwrap()]
}

fn num(s: &str, attr: &str) -> f64 {
    let re = regex::Regex::new(&format!(r#" {attr}="(-?[\d.]+)""#)).unwrap();
    re.captures(s).unwrap_or_else(|| panic!("no {attr} in {s}"))[1]
        .parse()
        .unwrap()
}

/// Spec 015 US3 (T021): the picture is inside its box, left of the label and
/// clear of it; the other box has none. A missing file gives no picture and
/// a warning naming it; an `include` gives the one-time warning.
#[test]
fn dot_image_example() {
    if !have("quarto") {
        return;
    }
    let (html, _) = render_example("dot-image", |t| t, &[]);
    let tool = node_svg(&html, "tool");
    let img = tool
        .find("<image class=\"dot-image\"")
        .expect("a picture in tool");
    let text = tool.find("<text").unwrap();
    assert!(
        img < text,
        "drawn before the label, so the text stays on top"
    );
    let img = &tool[img..];
    let (x, w, y, h) = (
        num(img, "x"),
        num(img, "width"),
        num(img, "y"),
        num(img, "height"),
    );
    assert!(img.contains("href=\"face.png\""), "{img}");
    let tx = num(&tool[text..], "x");
    assert!(x + w <= tx, "left of the label: {x} + {w} <= {tx}");
    let (lo, hi) = (-44.0, 0.0); // the box's top and bottom, from its path
    assert!(
        y >= lo && y + h <= hi && x >= 166.0,
        "inside the box: {img}"
    );
    assert!(!node_svg(&html, "data").contains("<image"));

    let (html, err) = render_example("dot-image", |t| t.replace("face.png", "nope.png"), &[]);
    assert!(!html.contains("class=\"dot-image\""));
    assert!(
        err.contains("dot-image: no file nope.png for the box tool"),
        "{err}"
    );

    let (_, err) = render_example(
        "dot-image",
        |t| format!("{t}\n{{{{< include more.qmd >}}}}\n"),
        &[("more.qmd", "## More\n")],
    );
    assert!(err.contains("are not scanned for pictures"), "{err}");
}

/// Spec 015 US3 (T023): the example needs no network beyond reveal.js's
/// MathJax, and its one-file export carries the picture inside it.
#[test]
fn dot_image_offline_and_one_file() {
    let t = copy_plugin("dot-image");
    let Some(problems) = offline_problems(t.path()) else {
        return;
    };
    let others: Vec<_> = problems
        .iter()
        .filter(|p| !p["url"].as_str().unwrap_or("").contains("/mathjax@"))
        .collect();
    assert!(others.is_empty(), "{others:#?}");
    let out = common::qd(t.path())
        .args(["--deck", "example.qmd", "export"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(t.path().join("export/example.html")).unwrap();
    assert!(
        html.contains("<image class=\"dot-image\" href=\"data:image/png;base64,"),
        "the picture is inside the file"
    );
}
