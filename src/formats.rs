//! `qd export --to <format>` (spec 011): the deck in another format that
//! Quarto writes, with what didn't carry over reported slide by slide. A copy
//! is rendered next to the deck; the deck itself is never written (FR-003).

use crate::deck::{BlockKind, Deck, Slide};
use crate::error::{QdError, Result};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The filter that adapts slides for other formats (research R2).
const FILTER: &str = include_str!("formats.lua");

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Tex,
    Browser,
}

#[derive(serde::Serialize)]
pub struct Format {
    pub name: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub ext: &'static str,
    pub needs: Option<Tool>,
}

const fn f(
    name: &'static str,
    label: &'static str,
    group: &'static str,
    ext: &'static str,
    needs: Option<Tool>,
) -> Format {
    Format {
        name,
        label,
        group,
        ext,
        needs,
    }
}

/// The menu formats, in menu order. `revealjs` and `revealjs-pdf` are the
/// offline export of 002/010 (`qd export`, `qd export --pdf`).
pub const FORMATS: &[Format] = &[
    f(
        "revealjs",
        "Web page (works offline)",
        "slides",
        "html",
        None,
    ),
    f(
        "revealjs-pdf",
        "Slides PDF",
        "slides",
        "pdf",
        Some(Tool::Browser),
    ),
    f("pptx", "PowerPoint", "slides", "pptx", None),
    f("beamer", "Beamer PDF", "slides", "pdf", Some(Tool::Tex)),
    f("docx", "Word", "documents", "docx", None),
    f("html", "HTML page", "documents", "html", None),
    f("pdf", "PDF (LaTeX)", "documents", "pdf", Some(Tool::Tex)),
    f("typst", "PDF (Typst)", "documents", "pdf", None),
    f("gfm", "Markdown (GitHub)", "documents", "md", None),
    f(
        "commonmark",
        "Markdown (CommonMark)",
        "documents",
        "md",
        None,
    ),
    f("ipynb", "Jupyter notebook", "documents", "ipynb", None),
    f("odt", "OpenDocument", "documents", "odt", None),
    f("epub", "EPUB", "documents", "epub", None),
];

pub fn find(name: &str) -> Result<&'static Format> {
    FORMATS.iter().find(|f| f.name == name).ok_or_else(|| {
        QdError::InvalidArg(format!(
            "'{name}' is not an export format (see `qd export --list`)"
        ))
    })
}

/// A deck feature that a format may not keep (data-model "Feature").
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    Notes,
    Pause,
    Fragment,
    Columns,
    Placed,
    BackgroundImage,
    BackgroundMedia,
    Interactive,
    Theme,
}

impl Feature {
    fn noun(self) -> &'static str {
        match self {
            Self::Notes => "speaker notes",
            Self::Pause => "pauses",
            Self::Fragment => "fragments",
            Self::Columns => "columns",
            Self::Placed => "placed boxes",
            Self::BackgroundImage => "background images",
            Self::BackgroundMedia => "background videos and web pages",
            Self::Interactive => "interactive figures",
            Self::Theme => "the reveal.js theme",
        }
    }
}

/// What a format does with a feature: one cell of the spec's FR-012 table.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Keep {
    Kept,
    Flow,
    AllShown,
    Hidden,
    Lost,
}

impl Keep {
    fn says(self) -> &'static str {
        match self {
            Self::Kept => "are kept",
            Self::Flow => "are in the flow, without their position",
            Self::AllShown => "are shown all at once",
            Self::Hidden => "are in the file but not shown",
            Self::Lost => "are not in the file",
        }
    }
}

/// The FR-012 table, measured against Quarto 1.11.9 (research R1).
pub fn keeps(format: &str, f: Feature) -> Keep {
    use Feature::*;
    use Keep::*;
    match (format, f) {
        ("revealjs" | "revealjs-pdf", _) => Kept,
        (_, Placed) => Flow,
        ("html" | "ipynb", Interactive) => Kept,
        (_, BackgroundMedia | Interactive | Theme) => Lost,
        ("pptx", Pause | Fragment) => AllShown,
        ("beamer", Fragment) => AllShown,
        ("beamer", Notes) => Hidden,
        ("beamer", BackgroundImage) => Lost,
        ("pptx" | "beamer", _) => Kept,
        // documents (research R1): one section per slide, notes after the content
        (_, Notes) => Kept,
        (_, Pause | Fragment) => AllShown,
        ("html", Columns) => Kept,
        (_, Columns) => Flow,
        _ => Lost,
    }
}

/// Is this a `:::` fence or an inline span carrying class `c`?
fn has_class_marker(text: &str, c: &str) -> bool {
    let dot = format!(".{c}");
    text.lines().any(|l| {
        let l = l.trim_start();
        (l.starts_with(":::") || l.contains("]{")) && l.contains(&dot)
    })
}

fn slide_features(d: &Deck, s: &Slide) -> Vec<Feature> {
    use Feature::*;
    let text = d.slice(s.span);
    let attr = |k: &str| s.attrs.as_ref().and_then(|a| a.get(k)).is_some();
    let kind = |k: BlockKind| s.blocks.iter().any(|b| b.kind == k);
    [
        (Notes, kind(BlockKind::Notes)),
        (Pause, kind(BlockKind::Pause)),
        (Fragment, has_class_marker(text, "fragment")),
        (Columns, has_class_marker(text, "columns")),
        (Placed, kind(BlockKind::Box)),
        (BackgroundImage, attr("background-image")),
        (
            BackgroundMedia,
            attr("background-video") || attr("background-iframe"),
        ),
        (
            Interactive,
            s.blocks.iter().any(|b| crate::export::is_plotly_cell(d, b)),
        ),
    ]
    .into_iter()
    .filter_map(|(f, on)| on.then_some(f))
    .collect()
}

/// Every feature the deck uses that `format` does not keep (FR-013), with
/// the slides where it is used, in deck order.
pub fn losses(d: &Deck, format: &str) -> Vec<Value> {
    let mut by: Vec<(Feature, Vec<String>)> = vec![];
    for s in &d.slides {
        for f in slide_features(d, s) {
            match by.iter_mut().find(|(g, _)| *g == f) {
                Some((_, v)) => v.push(s.id.clone()),
                None => by.push((f, vec![s.id.clone()])),
            }
        }
    }
    if crate::export::opt(&d.meta, "theme").is_some() {
        by.push((Feature::Theme, vec![]));
    }
    by.into_iter()
        .filter_map(|(f, slides)| {
            let k = keeps(format, f);
            (k != Keep::Kept).then(|| {
                json!({"feature": f, "keep": k,
                       "what": format!("{} {}", f.noun(), k.says()), "slides": slides})
            })
        })
        .collect()
}

/// TeX on `path` (xelatex, lualatex or pdflatex) or in Quarto's TinyTeX under `home`.
pub fn tex_found(path: Option<OsString>, home: Option<PathBuf>) -> bool {
    let mut dirs: Vec<PathBuf> = path
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(h) = home {
        for t in ["Library/TinyTeX/bin", ".TinyTeX/bin"] {
            if let Ok(rd) = std::fs::read_dir(h.join(t)) {
                dirs.extend(rd.flatten().map(|e| e.path()));
            }
        }
    }
    dirs.iter().any(|d| {
        ["xelatex", "lualatex", "pdflatex"]
            .iter()
            .any(|x| d.join(x).is_file())
    })
}

fn tex_here() -> bool {
    tex_found(
        std::env::var_os("PATH"),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

/// `qd export --list` (FR-011).
pub fn list() -> Value {
    let tex = tex_here();
    let formats: Vec<Value> = FORMATS
        .iter()
        .map(|f| {
            let ready = f.needs != Some(Tool::Tex) || tex;
            let mut v = serde_json::to_value(f).unwrap_or_default();
            v["ready"] = json!(ready);
            if !ready {
                v["hint"] = json!(QdError::NoTex(String::new()).hint());
            }
            v
        })
        .collect();
    json!({"kind": "export-formats", "formats": formats})
}

/// The deck's own `echo` (top level, `execute:` or `format: revealjs:`), else
/// false: the slides hide code, so the export does too (research R1).
fn echo(d: &Deck) -> bool {
    crate::export::opt(&d.meta, "echo")
        .or_else(|| d.meta.get("execute")?.get("echo"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Removes the files it holds when dropped (the export's temp files).
struct Cleanup(Vec<PathBuf>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        for p in &self.0 {
            let _ = std::fs::remove_dir_all(p).or_else(|_| std::fs::remove_file(p));
        }
    }
}

/// `qd export --to <name>` for a Quarto format (not the offline export).
pub fn run(
    store: &crate::store::Store,
    name: &str,
    out: Option<PathBuf>,
    dry_run: bool,
) -> Result<Value> {
    let fmt = find(name)?;
    if fmt.needs == Some(Tool::Tex) && !tex_here() {
        return Err(QdError::NoTex(fmt.label.into()));
    }
    let d = store.read()?;
    let deck_dir = store.path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let stem = store
        .path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let out = out.unwrap_or_else(|| deck_dir.join("export").join(format!("{stem}.{}", fmt.ext)));
    let out = std::path::absolute(&out)?;
    let mut v = json!({"ok": true, "kind": "export-format", "format": fmt.name, "route": "quarto",
        "out": out, "files": [], "losses": losses(&d, fmt.name), "notes": [], "changed_during": false});
    if dry_run {
        v["dry_run"] = json!(true);
        return Ok(v);
    }

    let tag = format!("export-{}", fmt.name);
    let filter = deck_dir.join(format!(".{stem}.qd-{tag}.lua"));
    let meta = deck_dir.join(format!(".{stem}.qd-{tag}.yml"));
    let parent = out.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::fs::create_dir_all(&parent)?;
    let staging = parent.join(format!(".{stem}.qd-staging-{}", crate::export::nanos()));
    let _cleanup = Cleanup(vec![filter.clone(), meta.clone(), staging.clone()]);
    std::fs::write(&filter, FILTER)?;
    std::fs::write(
        &meta,
        format!(
            "echo: {}\nfilters:\n  - {}\n",
            echo(&d),
            json!(filter.to_string_lossy())
        ),
    )?;
    let meta_s = meta.to_string_lossy();
    let made = crate::render::render_to(
        &store.path,
        &d.text,
        &tag,
        &staging,
        fmt.name,
        fmt.ext,
        &["--metadata-file", &meta_s],
        true,
    )?;
    // Markdown and the html page keep figures and libraries in `<name>_files/`
    // beside them (FR-009): it moves with the file, under the file's own name
    let name = made
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let made_files = staging.join(format!("{name}_files"));
    // (a PDF's own `_files/` holds intermediate figures only: it stays behind)
    if made_files.is_dir() && matches!(fmt.ext, "md" | "html") {
        let out_stem = out.file_stem().unwrap_or_default().to_string_lossy();
        let files = parent.join(format!("{out_stem}_files"));
        let text = std::fs::read_to_string(&made)?;
        std::fs::write(
            &made,
            text.replace(&format!("{name}_files/"), &format!("{out_stem}_files/")),
        )?;
        // merged, not replaced: the html page and Markdown of one deck share it
        // (`libs/` and `figure-html/` beside `figure-commonmark/`)
        std::fs::create_dir_all(&files)?;
        for e in std::fs::read_dir(&made_files)?.flatten() {
            let to = files.join(e.file_name());
            let _ = std::fs::remove_dir_all(&to).or_else(|_| std::fs::remove_file(&to));
            std::fs::rename(e.path(), to)?;
        }
        v["files"] = json!([files]);
    }
    std::fs::rename(&made, &out).map_err(|e| {
        QdError::Io(std::io::Error::other(format!(
            "could not write {} ({e}); close the file if another app has it open, and try again",
            out.display()
        )))
    })?;
    if store.read().is_ok_and(|now| now.version != d.version) {
        v["changed_during"] = json!(true);
        v["notes"] = json!([
            "the deck changed while exporting; this file is the version from when the export started"
        ]);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carry() -> Deck {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/export/carry.qmd"
        );
        Deck::parse(std::fs::read_to_string(p).unwrap())
    }

    fn summary(v: &[Value]) -> Vec<(String, String, Vec<String>)> {
        v.iter()
            .map(|l| {
                (
                    l["feature"].as_str().unwrap().into(),
                    l["keep"].as_str().unwrap().into(),
                    l["slides"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|s| s.as_str().unwrap().to_string())
                        .collect(),
                )
            })
            .collect()
    }

    fn s(f: &str, k: &str, slides: &[&str]) -> (String, String, Vec<String>) {
        (
            f.into(),
            k.into(),
            slides.iter().map(|x| x.to_string()).collect(),
        )
    }

    #[test]
    fn pptx_losses_on_the_fixture() {
        assert_eq!(
            summary(&losses(&carry(), "pptx")),
            vec![
                s("placed", "flow", &["placed"]),
                s("pause", "all-shown", &["frags"]),
                s("fragment", "all-shown", &["frags"]),
                s("interactive", "lost", &["plotly"]),
            ]
        );
    }

    #[test]
    fn beamer_losses_on_the_fixture() {
        assert_eq!(
            summary(&losses(&carry(), "beamer")),
            vec![
                s("notes", "hidden", &["cols"]),
                s("placed", "flow", &["placed"]),
                s("fragment", "all-shown", &["frags"]),
                s("background-image", "lost", &["bg"]),
                s("interactive", "lost", &["plotly"]),
            ]
        );
    }

    #[test]
    fn document_columns_of_the_table() {
        use Feature::*;
        use Keep::*;
        for d in [
            "docx",
            "html",
            "pdf",
            "typst",
            "gfm",
            "commonmark",
            "ipynb",
            "odt",
            "epub",
        ] {
            assert_eq!(keeps(d, Notes), Kept, "{d}: notes after the content");
            assert_eq!(keeps(d, Pause), AllShown, "{d}");
            assert_eq!(keeps(d, Placed), Flow, "{d}");
            assert_eq!(keeps(d, BackgroundImage), Lost, "{d}");
            assert_eq!(
                keeps(d, Columns),
                if d == "html" { Kept } else { Flow },
                "{d}"
            );
            let live = matches!(d, "html" | "ipynb");
            assert_eq!(keeps(d, Interactive), if live { Kept } else { Lost }, "{d}");
        }
    }

    #[test]
    fn only_used_features_are_reported() {
        let d = Deck::parse(
            "---\ntitle: T\nformat:\n  revealjs:\n    theme: dark\n---\n\n## A\n\nText\n".into(),
        );
        assert_eq!(
            summary(&losses(&d, "beamer")),
            vec![s("theme", "lost", &[])]
        );
        assert!(losses(&d, "revealjs").is_empty());
    }

    #[test]
    fn unknown_format_points_at_the_list() {
        let e = find("nosuch").err().unwrap();
        assert!(e.to_string().contains("qd export --list"), "{e}");
        assert_eq!(find("pptx").unwrap().ext, "pptx");
    }

    #[test]
    fn echo_follows_the_deck() {
        assert!(!echo(&carry()));
        let d = Deck::parse("---\nexecute:\n  echo: true\n---\n\n## A\n".into());
        assert!(echo(&d));
    }

    #[test]
    fn tex_on_path_or_in_tinytex() {
        let t = tempfile::tempdir().unwrap();
        let bin = t.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        assert!(!tex_found(Some(bin.clone().into()), Some(t.path().into())));
        std::fs::write(bin.join("lualatex"), "").unwrap();
        assert!(tex_found(Some(bin.into()), None));
        let tiny = t.path().join("home/.TinyTeX/bin/x86_64-linux");
        std::fs::create_dir_all(&tiny).unwrap();
        std::fs::write(tiny.join("xelatex"), "").unwrap();
        assert!(tex_found(None, Some(t.path().join("home"))));
    }

    /// The filter on its own, through Quarto's Pandoc (skipped without Quarto).
    #[test]
    fn filter_adapts_slides() {
        let Ok(q) = crate::render::quarto() else {
            return;
        };
        let t = tempfile::tempdir().unwrap();
        let lua = t.path().join("f.lua");
        std::fs::write(&lua, FILTER).unwrap();
        let md =
            "- A\n\n. . .\n\n- B\n\n::: notes\nSAID\n:::\n\n::: {.absolute left=1}\nBOX\n:::\n";
        let run = |to: &str| {
            let mut c = std::process::Command::new(&q);
            c.args(["pandoc", "-f", "markdown", "-t", to, "-L"])
                .arg(&lua)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped());
            let mut ch = c.spawn().unwrap();
            std::io::Write::write_all(ch.stdin.as_mut().unwrap(), md.as_bytes()).unwrap();
            String::from_utf8(ch.wait_with_output().unwrap().stdout).unwrap()
        };
        let gfm = run("gfm");
        assert!(!gfm.contains(". . ."), "{gfm}");
        assert!(gfm.contains("> **Speaker notes**"), "{gfm}");
        assert!(gfm.contains("BOX") && !gfm.contains("absolute"), "{gfm}");
        let beamer = run("beamer");
        assert!(beamer.contains("\\pause"), "{beamer}");
        assert!(beamer.contains("\\note"), "{beamer}");
    }
}
