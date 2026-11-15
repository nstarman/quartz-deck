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

/// The quarto-revealjs skill ships its own copy of `paper`, so a deck set up
/// from the skill alone has it. `plugins/paper` is the source; the copy must
/// match it byte for byte.
#[test]
fn skill_copy_of_paper_matches_the_plugin() {
    let plugin = root().join("plugins/paper/_extensions/paper");
    let copy = root().join("skills/quarto-revealjs/assets/_extensions/paper");
    assert_eq!(files(&plugin), files(&copy), "the same files");
    for f in files(&plugin) {
        assert_eq!(
            std::fs::read(plugin.join(&f)).unwrap(),
            std::fs::read(copy.join(&f)).unwrap(),
            "{} differs: edit plugins/paper and copy it to the skill",
            f.display()
        );
    }
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
        let (html, _) = render_example(&name, |t| t, &[]);
        if dir.ends_with("paper") {
            assert!(
                html.contains("paper-ref") && html.contains("paper-corner"),
                "both forms of the paper link"
            );
        }
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
    let tags = regex::Regex::new(r"<[^>]+>").unwrap();
    re.captures_iter(html)
        .map(|c| {
            (
                c[1].to_string(),
                tags.replace_all(&c[2], "").trim().to_string(),
            )
        })
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
}

/// Spec 014 (T019): cite-links needs no network: its CSS and icon are
/// bundled. Needs Quarto and a browser; skips otherwise.
#[test]
fn cite_links_example_works_offline() {
    let t = copy_plugin("cite-links");
    let qd = |a: &[&str]| {
        common::qd(t.path())
            .env("QD_NO_DOWNLOAD", "1")
            .args(a)
            .output()
            .unwrap()
    };
    if !have("quarto") || !qd(&["browser"]).status.success() {
        eprintln!("no browser: skipping");
        return;
    }
    let out = qd(&["--deck", "example.qmd", "check", "--offline"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let problems = report["problems"].as_array().expect("a check report");
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
    let tags = regex::Regex::new(r"<[^>]+>").unwrap();
    tags.replace_all(html, "")
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
        "{}",
        &refs[..200.min(refs.len())]
    );
    // compact labels are the citations' own, suffixes included
    let cited = |key: &str| {
        let re = regex::Regex::new(&format!(
            r#"(?s)<span class="citation" data-cites="{key}">(.*?)</span>"#
        ))
        .unwrap();
        text(&re.captures(&html).unwrap_or_else(|| panic!("{key} cited"))[1])
    };
    for (key, href) in [
        ("evans2022first", "https://doi.org/10.5555/evans1"),
        ("evans2022second", "https://doi.org/10.5555/evans2"),
        ("bell2020", "https://doi.org/10.5555/bell"),
        ("survey2023", "https://arxiv.org/abs/0000.00001"),
    ] {
        let re =
            regex::Regex::new(&format!(r#"(?s)<div id="ref-{key}"[^>]*>(.*?)</div>"#)).unwrap();
        let entry = &re
            .captures(&refs)
            .unwrap_or_else(|| panic!("entry {key}: {refs}"))[1];
        assert!(
            entry.contains(&format!(r#"href="{href}""#)),
            "{key}: {entry}"
        );
        assert!(entry.contains(r#"class="cite-icon""#), "{key}: {entry}");
        assert_eq!(text(entry), cited(key), "{key}");
    }
    // however the deck's style tells them apart (talk.csl: 2022a/b; the default: co-authors)
    assert_ne!(cited("evans2022first"), cited("evans2022second"));
    // no DOI or URL: listed, not linked
    let nolink = regex::Regex::new(r#"(?s)<div id="ref-nolink2024"[^>]*>(.*?)</div>"#).unwrap();
    assert!(
        !nolink.captures(&refs).unwrap()[1].contains("href"),
        "{refs}"
    );

    // short, 2 columns
    let refs = refs_html(&with(r#"refs-style="short" refs-columns="2""#));
    assert!(
        refs.contains(r#"class="refs-slide refs-short refs-cols-2 refs-size-smaller"#),
        "{refs}"
    );
    assert!(
        text(&refs).contains("Bell &amp; Cho (2020), Example Journal"),
        "{}",
        text(&refs)
    );
    assert!(
        text(&refs).contains("Kim et al. (2018), Example Letters"),
        "{}",
        text(&refs)
    );
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
    assert!(
        text(&refs).to_lowercase().contains("two authors"),
        "{}",
        text(&refs)
    );
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
    let qd = |a: &[&str]| {
        common::qd(t.path())
            .env("QD_NO_DOWNLOAD", "1")
            .args(a)
            .output()
            .unwrap()
    };
    if !have("quarto") || !qd(&["browser"]).status.success() {
        eprintln!("no quarto or browser: skipping");
        return;
    }
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
    let out = qd(&["--deck", "example.qmd", "check", "--offline"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let problems = report["problems"].as_array().expect("a check report");
    assert!(
        !problems.iter().any(|p| p["code"] == "P_OVERFLOW"),
        "{problems:#?}"
    );
}
