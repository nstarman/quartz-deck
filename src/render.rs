//! Quarto rendering (research.md R5): locate Quarto, build the annotated
//! *preview copy* of a deck, render a deck text to reveal.js HTML.

use crate::deck::{BlockKind, Deck, Slide, SlideKind};
use crate::error::{QdError, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `$QD_QUARTO`, else `quarto` on `PATH`.
pub fn quarto() -> Result<PathBuf> {
    find_quarto(std::env::var_os("QD_QUARTO"), std::env::var_os("PATH"))
}

fn find_quarto(explicit: Option<OsString>, path: Option<OsString>) -> Result<PathBuf> {
    if let Some(q) = explicit {
        let q = PathBuf::from(q);
        return if q.is_file() {
            Ok(q)
        } else {
            Err(QdError::NoQuarto)
        };
    }
    path.iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join("quarto"))
        .find(|p| p.is_file())
        .ok_or(QdError::NoQuarto)
}

/// Id of a slide's `<section>` in the preview: its own id, or `qd-N` for
/// rule/implicit slides (which get a synthetic id in the preview copy).
pub fn preview_id(s: &Slide) -> String {
    match s.kind {
        SlideKind::Rule | SlideKind::Implicit if !s.id_explicit => format!("qd-{}", s.index),
        _ => s.id.clone(),
    }
}

/// Insert `data-qd="..."` into an attribute block `{...}` ending at `close` (index of `}`).
fn insert_attr(text: &str, close: usize, tag: &str) -> String {
    let sep = if text[..close].ends_with('{') {
        ""
    } else {
        " "
    };
    format!("{}{sep}data-qd=\"{tag}\"{}", &text[..close], &text[close..])
}

fn hide_empty_heading(id: &str, nl: &str) -> String {
    format!(
        "```{{=html}}{nl}<style>#{id} > h1:empty, #{id} > h2:empty, #{id} > h3:empty {{ display: none; }}</style>{nl}```"
    )
}

fn is_caption(t: &str) -> bool {
    t.starts_with(": ") || t.starts_with("Table:")
}

/// Deck text with every slide-level block marked `data-qd="<slide>/<i>"`, and
/// synthetic ids on rule/implicit slides. Never written back to the deck.
pub fn preview_copy(d: &Deck) -> String {
    let nl = d.nl;
    let mut out = d.slice(d.head).to_string();
    for s in &d.slides {
        let pid = preview_id(s);
        match s.kind {
            // A synthetic untitled heading gives the slide an anchor; Pandoc
            // emits an empty <h2> for it, which `---`/implicit slides don't
            // have, so hide it to keep the layout identical (layout-drift check).
            SlideKind::Rule if !s.id_explicit => {
                out += &format!(
                    "{} {{#{pid}}}{nl}{nl}{}{nl}",
                    "#".repeat(d.slide_level as usize),
                    hide_empty_heading(&pid, nl)
                );
            }
            SlideKind::Implicit if !s.id_explicit => {
                out += &format!(
                    "{} {{#{pid}}}{nl}{nl}{}{nl}{nl}",
                    "#".repeat(d.slide_level as usize),
                    hide_empty_heading(&pid, nl)
                );
            }
            // hidden slides are dropped by Quarto; show them in the editor, marked
            _ if s.attrs.as_ref().and_then(|a| a.get("visibility")) == Some("hidden") => {
                let head = &d.text[s.span.start..s.content_start()];
                out += &[
                    "visibility=\"hidden\"",
                    "visibility='hidden'",
                    "visibility=hidden",
                ]
                .iter()
                .fold(head.to_string(), |h, pat| {
                    h.replace(pat, "data-qd-hidden=\"true\"")
                });
            }
            _ => out += &d.text[s.span.start..s.content_start()],
        }
        if s.opaque {
            // qd does not model this slide's structure: render it verbatim
            out += &d.text[s.content_start()..s.span.end];
            continue;
        }
        let mut pos = s.content_start();
        for (i, b) in s.blocks.iter().enumerate() {
            out += &d.text[pos..b.span.start];
            pos = b.span.end;
            let text = d.slice(b.span);
            let tag = format!("{}/{i}", s.id);
            let next_is_caption = s
                .blocks
                .get(i + 1)
                .is_some_and(|n| is_caption(d.slice(n.span)));
            let skip = matches!(b.kind, BlockKind::Pause | BlockKind::Notes)
                || (b.kind == BlockKind::Html && text.starts_with("<!--"))
                || is_caption(text)
                || text.starts_with("[^")
                || (b.kind == BlockKind::Table && next_is_caption);
            if skip {
                out += text;
                continue;
            }
            let first_end = text.find(['\n', '\r']).unwrap_or(text.len());
            let first = &text[..first_end];
            out += &match b.kind {
                BlockKind::Div | BlockKind::Box if first.trim_start().starts_with(':') => {
                    match first.rfind('}') {
                        Some(close) if first.contains('{') => insert_attr(text, close, &tag),
                        _ => {
                            // `::: class` shorthand -> `::: {.class data-qd=...}`
                            let colons = first.len() - first.trim_start_matches(':').len();
                            let word = first[colons..].trim().trim_end_matches(':').trim();
                            format!(
                                "{} {{.{word} data-qd=\"{tag}\"}}{}",
                                &first[..colons],
                                &text[first_end..]
                            )
                        }
                    }
                }
                // a heading inside a div would start a slide in Quarto: mark the heading itself
                BlockKind::Heading => {
                    let line = first.trim_end();
                    let marked = match line.rfind('{') {
                        Some(o)
                            if line.ends_with('}')
                                && crate::attrs::Attrs::parse(&line[o..]).is_some() =>
                        {
                            insert_attr(line, line.len() - 1, &tag)
                        }
                        _ => format!("{line} {{data-qd=\"{tag}\"}}"),
                    };
                    format!("{marked}{}", &text[first_end..])
                }
                BlockKind::Image | BlockKind::Box
                    if b.attrs.is_some() && text.trim_end().ends_with('}') =>
                {
                    insert_attr(text, text.trim_end().len() - 1, &tag)
                }
                _ => format!("::: {{data-qd=\"{tag}\"}}{nl}{text}{nl}:::"),
            };
        }
        out += &d.text[pos..s.span.end];
    }
    out
}

/// Render `text` as if it were the deck at `deck_path` into `out_dir`. The
/// temporary copy `<stem>.qd-<tag>.qmd` sits next to the deck so relative paths
/// and a `_quarto.yml` project apply (Quarto leaves `.`/`_`-prefixed files out
/// of a project, so the name must not be hidden); it is removed afterwards.
/// `preview` turns off `embed-resources`: inlining (MathJax, plotly.js, …) on
/// every edit is slow, and the preview is served over HTTP anyway.
/// Returns the HTML path.
pub fn render(
    deck_path: &Path,
    text: &str,
    tag: &str,
    out_dir: &Path,
    preview: bool,
) -> Result<PathBuf> {
    let quarto = quarto()?;
    let dir = deck_path.parent().unwrap_or(Path::new("."));
    let stem = deck_path.file_stem().unwrap_or_default().to_string_lossy();
    let name = format!("{stem}.qd-{tag}");
    let copy = dir.join(format!("{name}.qmd"));
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(&copy, text)?;
    let out = Command::new(quarto)
        .arg("render")
        .arg(&copy)
        .args(["--to", "revealjs", "--output-dir"])
        .arg(out_dir)
        .args(if preview {
            &["-M", "embed-resources:false"][..]
        } else {
            &[]
        })
        .current_dir(dir)
        .output();
    let _ = std::fs::remove_file(&copy);
    let out = out?;
    if !out.status.success() {
        return Err(QdError::Render(clean_log(
            &String::from_utf8_lossy(&out.stderr),
            &name,
        )));
    }
    Ok(out_dir.join(format!("{name}.html")))
}

/// Final render of the deck file itself (US5): exactly `quarto render <deck>
/// [--to FORMAT]` run in the deck's directory, so a `_quarto.yml` project's
/// `output-dir`, `embed-resources` etc. apply. Returns the output path (from
/// Quarto's "Output created:" line) and a cleaned log.
pub fn render_deck(deck_path: &Path, to: Option<&str>) -> Result<(PathBuf, String)> {
    let quarto = quarto()?;
    if let Some(f) = to
        && (f.is_empty()
            || f.starts_with('-')
            || !f
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_+.".contains(c)))
    {
        return Err(QdError::InvalidArg(format!(
            "'{f}' is not a Quarto format name"
        )));
    }
    let dir = deck_path.parent().unwrap_or(Path::new("."));
    let mut cmd = Command::new(quarto);
    cmd.arg("render")
        .arg(deck_path.file_name().unwrap())
        .current_dir(dir);
    if let Some(f) = to {
        cmd.args(["--to", f]);
    }
    let out = cmd.output()?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        return Err(QdError::Render(clean_log(&log, "\u{0}")));
    }
    let created = log
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("Output created:"))
        .map(|p| dir.join(p.trim()))
        .ok_or_else(|| {
            QdError::Render(format!(
                "Quarto succeeded but reported no output file:\n{log}"
            ))
        })?;
    let created = std::fs::canonicalize(&created).unwrap_or(created);
    Ok((created, clean_log(&log, "\u{0}")))
}

/// Quarto's error output without ANSI colors, its JS stack trace, or our temp file name.
fn clean_log(log: &str, temp_name: &str) -> String {
    let mut plain = String::with_capacity(log.len());
    let mut chars = log.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next_if_eq(&'[').is_some() {
                while chars.next().is_some_and(|c| !c.is_ascii_alphabetic()) {}
            }
            continue;
        }
        plain.push(c);
    }
    let plain = plain.split("Stack trace:").next().unwrap_or("").trim();
    let lines: Vec<&str> = plain.lines().collect();
    let start = lines.len().saturating_sub(40);
    let deck = temp_name.split(".qd-").next().unwrap_or("").to_string();
    lines[start..].join("\n").replace(temp_name, &deck)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASIC: &str = include_str!("../tests/fixtures/basic.qmd");

    #[test]
    fn preview_copy_marks_blocks() {
        let d = Deck::parse(BASIC.to_string());
        let c = preview_copy(&d);
        assert!(
            c.contains("## Intro\n\n::: {data-qd=\"intro/0\"}\nHello world.\n:::\n"),
            "{c}"
        );
        assert!(
            c.contains("::: {data-qd=\"results/0\"}\n- one\n- two\n:::"),
            "{c}"
        );
        assert!(
            c.contains(
                "{#b-plot .absolute top=10 left=20 width=300 height=200 data-qd=\"results/1\"}"
            ),
            "{c}"
        );
        assert!(
            c.contains("## {#qd-3}\n\n```{=html}\n<style>#qd-3 > h1:empty"),
            "{c}"
        );
        assert!(
            c.contains("```\n\n::: {data-qd=\"@3/0\"}\nA rule slide.\n:::"),
            "{c}"
        );
        assert!(
            c.contains("::: {.notes}\nBye.\n:::"),
            "notes must stay untouched: {c}"
        );
        assert!(c.starts_with(&BASIC[..d.head.end]));
        let p = Deck::parse(c);
        assert_eq!(p.slides.len(), d.slides.len());
        assert_eq!(p.slides[2].id, "qd-3");
        assert!(p.slides.iter().all(|s| !s.opaque));
    }

    #[test]
    fn preview_copy_special_blocks() {
        let d = Deck::parse(
            "Pre.\n\n## T\n\n::: notes\nn\n:::\n\n::: aside\na\n:::\n\n. . .\n\n| a |\n|---|\n\n: Cap\n\n<!-- c -->\n\n![](x.png){.r}\n".into(),
        );
        let c = preview_copy(&d);
        assert!(
            c.starts_with("## {#qd-1}\n\n```{=html}\n<style>#qd-1"),
            "{c}"
        );
        assert!(
            c.contains("```\n\n::: {data-qd=\"@1/0\"}\nPre.\n:::"),
            "{c}"
        );
        assert!(c.contains("::: notes\nn\n:::"));
        assert!(c.contains("::: {.aside data-qd=\"t/1\"}\na\n:::"), "{c}");
        assert!(c.contains("\n. . .\n"));
        assert!(
            c.contains("| a |\n|---|\n\n: Cap"),
            "table+caption must stay adjacent: {c}"
        );
        assert!(c.contains("\n<!-- c -->\n"));
        assert!(c.contains("![](x.png){.r data-qd=\"t/6\"}"), "{c}");
    }

    #[test]
    fn preview_marks_headings_in_place_and_skips_opaque() {
        let d = Deck::parse("---\nslide-level: 3\n---\n\n### S\n\n#### Sub\n\n#### Sub2 {.c}\n\n## T\n\n```\nnever closed\n".into());
        let c = preview_copy(&d);
        assert!(c.contains("#### Sub {data-qd=\"s/0\"}\n"), "{c}");
        assert!(c.contains("#### Sub2 {.c data-qd=\"s/1\"}\n"), "{c}");
        assert!(
            c.ends_with("## T\n\n```\nnever closed\n"),
            "opaque slide must be verbatim: {c}"
        );
    }

    #[test]
    fn crlf_preview_copy() {
        let d = Deck::parse("## A\r\n\r\ntext\r\n".into());
        assert_eq!(
            preview_copy(&d),
            "## A\r\n\r\n::: {data-qd=\"a/0\"}\r\ntext\r\n:::\r\n"
        );
    }

    #[test]
    fn render_errors_are_cleaned() {
        let log = "\x1b[91mERROR: YAMLException: bad (3:1)\x1b[39m\n\n 1 | title: [x\n\nStack trace:\n    at foo (quarto.js:1:2)\nin talk.qd-preview.qmd";
        assert_eq!(
            clean_log(log, "talk.qd-preview"),
            "ERROR: YAMLException: bad (3:1)\n\n 1 | title: [x"
        );
        assert_eq!(
            clean_log("rendering talk.qd-check.qmd failed", "talk.qd-check"),
            "rendering talk.qmd failed"
        );
    }

    #[test]
    fn quarto_lookup() {
        assert_eq!(
            find_quarto(Some("/nonexistent/q".into()), None)
                .unwrap_err()
                .code(),
            "E_NO_QUARTO"
        );
        assert_eq!(
            find_quarto(None, Some("/nonexistent".into()))
                .unwrap_err()
                .code(),
            "E_NO_QUARTO"
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("quarto"), "").unwrap();
        assert_eq!(
            find_quarto(None, Some(dir.path().into())).unwrap(),
            dir.path().join("quarto")
        );
    }

    #[test]
    fn preview_shows_hidden_slides() {
        let d = Deck::parse(
            "## Draft {#draft visibility=\"hidden\"}\n\nx\n\n## B {visibility=hidden .c}\n".into(),
        );
        let c = preview_copy(&d);
        assert!(
            c.contains("## Draft {#draft data-qd-hidden=\"true\"}"),
            "{c}"
        );
        assert!(c.contains("## B {data-qd-hidden=\"true\" .c}"), "{c}");
        assert!(!c.contains("visibility"), "{c}");
    }

    /// A deck inside a Quarto project (the `quarto-revealjs` skill's folder
    /// layout: `_quarto.yml` with `output-dir`, `embed-resources`,
    /// `include-in-header`) renders, gets the project settings, and leaves the
    /// project's `_output/` untouched. Needs Quarto.
    #[test]
    fn renders_inside_a_quarto_project() {
        if quarto().is_err() {
            eprintln!("quarto not found: skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(
            p.join("_quarto.yml"),
            "project:\n  output-dir: _output\nformat:\n  revealjs:\n    embed-resources: true\n    include-in-header: _deck/header.html\n",
        )
        .unwrap();
        std::fs::create_dir(p.join("_deck")).unwrap();
        std::fs::write(
            p.join("_deck/header.html"),
            "<meta name=\"qd-test\" content=\"from-project\">\n",
        )
        .unwrap();
        let deck = p.join("talk.qmd");
        std::fs::write(&deck, BASIC).unwrap();
        let d = Deck::parse(BASIC.to_string());
        let out = p.join("cache");
        let html = render(&deck, &preview_copy(&d), "preview", &out, true).unwrap();
        let h = std::fs::read_to_string(&html).unwrap();
        assert!(
            h.contains("from-project"),
            "project include-in-header not applied"
        );
        assert!(h.contains("data-qd=\"intro/0\""));
        assert!(
            !p.join("_output").exists(),
            "must not write the project's output dir"
        );
        // the final render (check) keeps the project's embed-resources
        let html = render(&deck, &d.text, "check", &p.join("check"), false).unwrap();
        assert!(
            !html.with_file_name("talk.qd-check_files").exists(),
            "embed-resources should inline everything"
        );
        let left: Vec<_> = std::fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".qd-"))
            .collect();
        assert!(left.is_empty(), "temp copies left: {left:?}");
    }

    /// SC-006 / T041: a box placed by `qd box add` lands at the same px in a
    /// stock Quarto render. Needs Quarto.
    #[test]
    fn box_position_fidelity() {
        if quarto().is_err() {
            eprintln!("quarto not found: skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("deck.qmd");
        let d = Deck::parse(BASIC.to_string());
        let p = crate::ops::box_add(
            &d,
            "intro",
            crate::ops::BoxContent::Text("Box"),
            Some((200.0, 100.0)),
            Some((300.0, 50.0)),
        )
        .unwrap();
        let text = crate::ops::apply(&d.text, &p).unwrap();
        std::fs::write(&deck, &text).unwrap();
        let html = std::fs::read_to_string(
            render(&deck, &text, "check", &dir.path().join("out"), false).unwrap(),
        )
        .unwrap();
        let tag = html
            .split(&format!("id=\"{}\"", p.target))
            .nth(1)
            .expect("box rendered")
            .split('>')
            .next()
            .unwrap()
            .to_string();
        for want in ["left: 200px", "top: 100px", "width: 300px", "height: 50px"] {
            assert!(tag.contains(want), "{want} missing in {tag}");
        }
    }

    /// Rendered preview keeps slide count and carries the markers (needs Quarto).
    #[test]
    fn renders_with_quarto() {
        if quarto().is_err() {
            eprintln!("quarto not found: skipping");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("deck.qmd");
        std::fs::write(&deck, BASIC).unwrap();
        let d = Deck::parse(BASIC.to_string());
        let html = render(
            &deck,
            &preview_copy(&d),
            "preview",
            &dir.path().join("out"),
            true,
        )
        .unwrap();
        let h = std::fs::read_to_string(html).unwrap();
        let sections = h.matches("<section id=").count();
        assert_eq!(sections, d.slides.len() + 1, "slides + title slide");
        for s in &d.slides {
            assert!(
                h.contains(&format!("<section id=\"{}\"", preview_id(s))),
                "missing section {}",
                preview_id(s)
            );
            for i in 0..s.blocks.len() {
                if s.blocks[i].kind != BlockKind::Notes {
                    assert!(
                        h.contains(&format!("data-qd=\"{}/{i}\"", s.id)),
                        "missing marker {}/{i}",
                        s.id
                    );
                }
            }
        }
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            names
                .iter()
                .all(|n| !n.to_string_lossy().contains("qd-preview.qmd")),
            "copy left behind"
        );
    }
}
