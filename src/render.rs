//! Quarto rendering (research.md R5): locate Quarto, build the annotated
//! *preview copy* of a deck, render a deck text to reveal.js HTML.

use crate::deck::{BlockKind, Deck, Slide, SlideKind};
use crate::error::{QdError, Result};
use serde_json::Value;
use std::ffi::{OsStr, OsString};
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
    which("quarto", path).ok_or(QdError::NoQuarto)
}

/// `name` in the folders of `path` (a `$PATH` value).
pub(crate) fn which(name: &str, path: Option<OsString>) -> Option<PathBuf> {
    path.iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// A finished process's stdout then stderr, as text.
pub(crate) fn log(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `quarto <args>` for a deck whose text is `text`, run in `dir`: inside the
/// deck's environment when it has code cells and an environment exists (R3),
/// using the environment's kernel unless the deck names one; else Quarto
/// directly (code-free decks never need an environment; `""` = the run
/// never executes code). The flag says which.
fn quarto_cmd(dir: &Path, text: &str, args: &[&OsStr]) -> Result<(Command, bool)> {
    let quarto = quarto()?;
    let d = Deck::parse(text.to_string());
    let cells = d.blocks().any(|b| b.kind == BlockKind::Cell);
    let (mut cmd, wrapped) = match crate::env::find(dir).filter(|_| cells) {
        Some(env) => (env.wrap(&quarto)?, true),
        None => (Command::new(quarto), false),
    };
    cmd.args(args).current_dir(dir);
    if wrapped && d.meta.get("jupyter").is_none() {
        cmd.args(["-M", &format!("jupyter:{}", crate::env::KERNEL)]);
    }
    Ok((cmd, wrapped))
}

/// One Quarto run at a time: runs in the same folder share its `.quarto/`
/// session files, and one deletes what another is writing (the live preview's
/// render and File ▸ Render or Export at once failed with "writefile …
/// project-cache"). ponytail: one lock for this process; a `qd render` from
/// another process can still overlap, add a lock file in `.quarto/` if that bites.
fn run_quarto(cmd: &mut Command) -> std::io::Result<std::process::Output> {
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    cmd.output()
}

/// A failed Quarto run: `E_LOCK_STALE` if the environment refused a stale lock.
fn failed(log: &str, wrapped: bool, name: &str) -> QdError {
    if wrapped && crate::env::is_stale(log) {
        QdError::LockStale(log.to_string())
    } else {
        QdError::Render(clean_log(log, name))
    }
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
pub fn preview_copy(d: &Deck, outputs: &Value) -> String {
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
                _ if b.kind == BlockKind::Cell => {
                    let shown = output_html(&outputs[&b.id], nl);
                    // .qd-cell: the editor's placeholder for a cell that shows
                    // nothing yet (revealjs hides code by default; spec 013)
                    let lang = crate::kernel::cell_lang(d, b);
                    format!(
                        "::: {{.qd-cell data-qd=\"{tag}\" data-qd-lang=\"{lang}\"}}{nl}{text}{nl}{shown}:::"
                    )
                }
                _ => format!("::: {{data-qd=\"{tag}\"}}{nl}{text}{nl}:::"),
            };
        }
        out += &d.text[pos..s.span.end];
    }
    out
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A cell's stored outputs (`outputs::Outputs` record) as a raw HTML block for
/// the preview, which never executes code (R7).
fn output_html(rec: &Value, nl: &str) -> String {
    if rec["items"].as_array().is_none() {
        return String::new();
    }
    format!("````{{=html}}{nl}{}{nl}````{nl}", output_div(rec))
}

/// The `<div class="qd-output">` of [`output_html`], stale ones dimmed: also
/// what the editor swaps into the slide when a run ends (spec 013 R6).
pub fn output_div(rec: &Value) -> String {
    let items = rec["items"].as_array().map_or(&[][..], |v| v);
    let pre = |class: &str, t: &str| format!("<pre class=\"{class}\">{}</pre>", esc(t));
    let body: String = items
        .iter()
        .map(|o| match o["kind"].as_str() {
            Some("html") => o["html"].as_str().unwrap_or_default().to_string(),
            Some("image") => format!(
                "<img src=\"/outputs/{}\" alt=\"cell output\">",
                esc(o["file"].as_str().unwrap_or_default())
            ),
            Some("error") => {
                let tb: Vec<&str> = o["traceback"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                pre("qd-out-error", &tb.join("\n"))
            }
            _ => pre("qd-out-text", o["text"].as_str().unwrap_or_default()),
        })
        .collect();
    let stale = rec["stale"] == true;
    format!(
        "<div class=\"cell-output qd-output{}\" data-counter=\"{}\"{}>{body}</div>",
        if stale { " qd-stale" } else { "" },
        rec["counter"],
        if stale {
            " title=\"stale: the cell changed since this ran\" style=\"opacity:.45\""
        } else {
            ""
        },
    )
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
    // stored outputs are in the preview copy; the preview never runs code (R7)
    let extra: &[&str] = if preview {
        &["-M", "embed-resources:false", "--no-execute"]
    } else {
        &[]
    };
    render_to(
        deck_path, text, tag, out_dir, "revealjs", "html", extra, !preview,
    )
}

/// [`render`] to any Quarto format: `<stem>.qd-<tag>.<ext>` in `out_dir`,
/// with `extra` arguments; `execute` false never runs code.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_to(
    deck_path: &Path,
    text: &str,
    tag: &str,
    out_dir: &Path,
    to: &str,
    ext: &str,
    extra: &[&str],
    execute: bool,
) -> Result<PathBuf> {
    let dir = deck_path.parent().unwrap_or(Path::new("."));
    let stem = deck_path.file_stem().unwrap_or_default().to_string_lossy();
    let name = format!("{stem}.qd-{tag}");
    let copy = dir.join(format!("{name}.qmd"));
    let mut args: Vec<&OsStr> = vec![
        "render".as_ref(),
        copy.as_os_str(),
        "--to".as_ref(),
        to.as_ref(),
        "--output-dir".as_ref(),
        out_dir.as_os_str(),
    ];
    args.extend(extra.iter().map(OsStr::new));
    let (mut quarto, wrapped) = quarto_cmd(dir, if execute { text } else { "" }, &args)?;
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(&copy, text)?;
    let out = run_quarto(&mut quarto);
    let _ = std::fs::remove_file(&copy);
    let out = out?;
    if !out.status.success() {
        return Err(failed(
            &String::from_utf8_lossy(&out.stderr),
            wrapped,
            &name,
        ));
    }
    Ok(out_dir.join(format!("{name}.{ext}")))
}

/// Final render of the deck file itself (US5): exactly `quarto render <deck>
/// [--to FORMAT]` run in the deck's directory, so a `_quarto.yml` project's
/// `output-dir`, `embed-resources` etc. apply. Returns the output path (from
/// Quarto's "Output created:" line) and a cleaned log.
pub fn render_deck(deck_path: &Path, to: Option<&str>) -> Result<(PathBuf, String)> {
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
    let mut args = vec!["render".as_ref(), deck_path.file_name().unwrap()];
    if let Some(f) = to {
        args.extend(["--to", f].map(OsStr::new));
    }
    let text = std::fs::read_to_string(deck_path)?;
    let (mut cmd, wrapped) = quarto_cmd(dir, &text, &args)?;
    let out = run_quarto(&mut cmd)?;
    let log = log(&out);
    if !out.status.success() {
        return Err(failed(&log, wrapped, "\u{0}"));
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
    use serde_json::json;

    const BASIC: &str = include_str!("../tests/fixtures/basic.qmd");

    #[test]
    fn output_div_is_the_block_the_preview_shows() {
        let rec = json!({"counter": 3, "items": [{"kind": "text", "text": "a<b"}]});
        let div = output_div(&rec);
        assert_eq!(
            div,
            "<div class=\"cell-output qd-output\" data-counter=\"3\"><pre class=\"qd-out-text\">a&lt;b</pre></div>"
        );
        assert_eq!(
            output_html(&rec, "\n"),
            format!("````{{=html}}\n{div}\n````\n")
        );
        assert!(output_div(&json!({"stale": true, "items": []})).contains("qd-stale"));
        let d = Deck::parse("## A\n\n```{python}\n1\n```\n".into());
        let c = preview_copy(&d, &json!({}));
        assert!(
            c.contains("::: {.qd-cell data-qd=\"a/0\" data-qd-lang=\"python\"}\n```{python}"),
            "{c}"
        );
    }

    #[test]
    fn preview_copy_marks_blocks() {
        let d = Deck::parse(BASIC.to_string());
        let c = preview_copy(&d, &Value::Null);
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
        let c = preview_copy(&d, &Value::Null);
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
        let c = preview_copy(&d, &Value::Null);
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
            preview_copy(&d, &Value::Null),
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
        let c = preview_copy(&d, &Value::Null);
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
        let html = render(
            &deck,
            &preview_copy(&d, &Value::Null),
            "preview",
            &out,
            true,
        )
        .unwrap();
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
            &preview_copy(&d, &Value::Null),
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
