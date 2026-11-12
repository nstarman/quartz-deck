//! `qd export` (spec 002 US2; research R1, R2, R7, R8): render an export copy
//! of the deck with the offline setup applied, ship the files that cannot be
//! inlined, check it offline, optionally print a PDF, then swap it into the
//! export folder. The deck and project files are never written.

use crate::deck::Deck;
use crate::error::{QdError, Result};
use serde_yaml_ng::{Mapping, Value as Yaml};
use std::path::{Path, PathBuf};

/// The offline setup to apply to an export copy.
pub struct Setup<'a> {
    /// Absolute path of qd's header (loads MathJax/plotly.js, print CSS).
    pub header: &'a Path,
    /// Insert the Plotly renderer cell (deck has Python Plotly figures).
    pub plotly: bool,
}

/// The quarto-revealjs skill's Plotly renderer (`_deck/setup.qmd`): figures
/// become bare divs with no web MathJax or plotly.js of their own.
const PLOTLY_CELL: &str = r#"```{python}
#| include: false
# added by qd export: Plotly figures without CDN scripts (plotly.js is loaded once)
import plotly.io as pio
from plotly.io.base_renderers import MimetypeRenderer

class _QdDeckRenderer(MimetypeRenderer):
    def to_mimebundle(self, fig_dict):
        html = pio.to_html(fig_dict, full_html=False, include_plotlyjs=False,
                           include_mathjax=False, config={"responsive": True})
        return {"text/html": html}

pio.renderers["qd-deck"] = _QdDeckRenderer()
pio.renderers.default = "qd-deck"
```
"#;

/// Does the deck have Python cells that import Plotly?
pub fn uses_plotly(d: &Deck) -> bool {
    d.blocks().any(|b| {
        let t = d.slice(b.span);
        b.kind == crate::deck::BlockKind::Cell
            && t.lines().next().is_some_and(|l| l.contains("{python"))
            && t.lines().any(|l| {
                let l = l.trim_start();
                l.starts_with("import plotly") || l.starts_with("from plotly")
            })
    })
}

/// A Quarto option set on the deck: top level or under `format: revealjs:`.
fn opt<'a>(meta: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    meta.get(key)
        .or_else(|| meta.get("format")?.get("revealjs")?.get(key))
        .filter(|v| !v.is_null())
}

/// The nearest `_quarto.yml` (Quarto project file) from the deck folder up, parsed.
fn project_config(deck_dir: &Path) -> Option<(PathBuf, Yaml)> {
    let mut dir = Some(deck_dir);
    while let Some(d) = dir {
        for name in ["_quarto.yml", "_quarto.yaml"] {
            let p = d.join(name);
            if let Ok(text) = std::fs::read_to_string(&p) {
                return Some((p, serde_yaml_ng::from_str(&text).unwrap_or(Yaml::Null)));
            }
        }
        dir = d.parent();
    }
    None
}

/// Deck front matter, then the project config.
fn sources(d: &Deck, deck_dir: &Path) -> Vec<(String, Yaml)> {
    let mut v = vec![("deck front matter".to_string(), d.meta.clone())];
    if let Some((p, y)) = project_config(deck_dir) {
        v.push((p.display().to_string(), y));
    }
    v
}

/// Feature that cannot be a single offline file, with where it was found.
pub fn offline_incompatible(d: &Deck, deck_dir: &Path) -> Option<String> {
    for (src, y) in sources(d, deck_dir) {
        for key in ["chalkboard", "multiplex"] {
            if opt(&y, key).is_some_and(|v| v.as_bool() != Some(false)) {
                return Some(format!("{key} ({src})"));
            }
        }
    }
    None
}

/// Is the deck/project already set up per the quarto-revealjs skill (math plugin at about:blank)?
pub fn already_set_up(d: &Deck, deck_dir: &Path) -> bool {
    sources(d, deck_dir).iter().any(|(_, y)| {
        opt(y, "html-math-method")
            .and_then(|m| m.get("url"))
            .and_then(Yaml::as_str)
            == Some("about:blank")
    })
}

fn ykey(k: &str) -> Yaml {
    Yaml::String(k.into())
}

/// Text of the export copy: the deck with the offline setup merged into its front matter.
pub fn export_copy(d: &Deck, deck_dir: &Path, setup: &Setup) -> Result<String> {
    if d.front_matter.is_some() && !d.meta.is_mapping() && !d.meta.is_null() {
        return Err(QdError::InvalidArg(
            "the front matter is not a YAML mapping".into(),
        ));
    }
    let mut meta = d.meta.as_mapping().cloned().unwrap_or_default();
    let mut format = match meta.get("format") {
        Some(Yaml::Mapping(m)) => m.clone(),
        Some(Yaml::String(s)) => Mapping::from_iter([(ykey(s), Yaml::Null)]),
        _ => Mapping::new(),
    };
    let mut rj = match format.get("revealjs") {
        Some(Yaml::Mapping(m)) => m.clone(),
        _ => Mapping::new(),
    };
    rj.insert(ykey("embed-resources"), Yaml::Bool(true));
    if !already_set_up(d, deck_dir) {
        let math = Mapping::from_iter([
            (ykey("method"), ykey("mathjax")),
            (ykey("url"), ykey("about:blank")),
        ]);
        rj.insert(ykey("html-math-method"), Yaml::Mapping(math));
        let mut inc = match rj.get("include-in-header") {
            Some(Yaml::Sequence(s)) => s.clone(),
            Some(v @ (Yaml::String(_) | Yaml::Mapping(_))) => vec![v.clone()],
            _ => vec![],
        };
        inc.push(ykey(&setup.header.to_string_lossy()));
        rj.insert(ykey("include-in-header"), Yaml::Sequence(inc));
    }
    format.insert(ykey("revealjs"), Yaml::Mapping(rj));
    meta.insert(ykey("format"), Yaml::Mapping(format));
    let yaml = serde_yaml_ng::to_string(&Yaml::Mapping(meta))
        .map_err(|e| QdError::InvalidArg(e.to_string()))?;
    let body = &d.text[d.front_matter.map_or(0, |f| f.end)..];
    let plotly = if setup.plotly {
        format!("\n{PLOTLY_CELL}")
    } else {
        String::new()
    };
    Ok(format!(
        "---\n{yaml}---\n{plotly}\n{}",
        body.trim_start_matches(['\r', '\n'])
    ))
}

/// Files the deck references that neither `embed-resources` nor
/// [`crate::inline`] inlines (background video, parallax image), relative to the deck.
pub fn collect_files(d: &Deck) -> Vec<String> {
    let mut out: Vec<String> = d
        .slides
        .iter()
        .filter_map(|s| s.attrs.as_ref())
        .filter_map(|a| a.get("background-video")) // background iframes (apps): crate::inline
        .map(String::from)
        .collect();
    out.extend(
        opt(&d.meta, "parallax-background-image")
            .and_then(Yaml::as_str)
            .map(String::from),
    );
    out.retain(|p| crate::inline::local(p));
    out.sort();
    out.dedup();
    out
}

// ---------- orchestration (T017) ----------

pub struct Options {
    pub out: Option<PathBuf>,
    pub pdf: bool,
    pub notes: bool,
    pub per_slide: bool,
    pub strip_notes: bool,
    pub allow_download: bool,
}

/// qd's offline header (from the quarto-revealjs skill's `_deck/header.html`):
/// MathJax config + local MathJax (+ plotly.js), print stills, fragments in
/// equations, keyboard back from embedded apps.
fn header_html(mathjax: &Path, plotly: Option<&Path>) -> String {
    let src = |p: &Path| p.to_string_lossy().replace('"', "%22");
    let plotly = plotly
        .map(|p| format!("<script src=\"{}\"></script>\n", src(p)))
        .unwrap_or_default();
    format!(
        r#"<!-- added by qd export: offline libraries (local copies, inlined by embed-resources) -->
<script>
  window.MathJax = {{ tex: {{ packages: {{ "[+]": ["html"] }} }}, svg: {{ fontCache: "global" }} }};
</script>
<script src="{mj}"></script>
{plotly}<style>
  .print-only {{ display: none; }}
  html.print-pdf .print-only {{ display: block; }}
  html.print-pdf .print-hide {{ display: none !important; }}
</style>
<script>
  window.addEventListener("load", () => window.MathJax && MathJax.startup && MathJax.startup.promise.then(() => {{
    document.querySelectorAll("mjx-assistive-mml .fragment").forEach((e) => e.classList.remove("fragment"));
    document.querySelectorAll("section").forEach((s) => {{
      if (s.querySelector("mjx-container .fragment"))
        s.querySelectorAll(".fragment").forEach((e) => e.removeAttribute("data-fragment-index"));
    }});
    Reveal.isReady() ? Reveal.sync() : Reveal.on("ready", () => Reveal.sync());
  }}));
  window.addEventListener("blur", () => setTimeout(() => {{
    if (document.activeElement && document.activeElement.tagName === "IFRAME") {{ document.activeElement.blur(); window.focus(); }}
  }}));
</script>
"#,
        mj = src(mathjax)
    )
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

const MARKER: &str = ".qd-export";

/// Export the deck. Returns the result JSON (`kind: "export"`); `ok` is false
/// when the offline check failed (previous export untouched, staging kept).
pub fn run(store: &crate::store::Store, opts: &Options) -> Result<serde_json::Value> {
    use serde_json::json;
    let d = store.read()?;
    let deck_dir = store.path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let stem = store
        .path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if let Some(what) = offline_incompatible(&d, &deck_dir) {
        return Err(QdError::OfflineIncompatible(what));
    }
    let out = match &opts.out {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => std::env::current_dir()?.join(p),
        None => deck_dir.join("export"),
    };
    let out_name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export".into());
    let parent = out.parent().unwrap_or(Path::new(".")).to_path_buf();
    if out.exists() && !out.join(MARKER).exists() && std::fs::read_dir(&out)?.next().is_some() {
        return Err(QdError::InvalidArg(format!(
            "{} exists and was not created by qd export; refusing to replace it (pass --out DIR)",
            out.display()
        )));
    }

    // assets and header
    let root = store.root();
    let mathjax = crate::assets::ensure(&root, &crate::assets::MATHJAX, opts.allow_download)?;
    let plotly = if uses_plotly(&d) {
        Some(crate::assets::plotly_from_python().ok_or_else(|| {
            QdError::InvalidArg("the deck has Python Plotly cells but Python plotly is not importable (pip install plotly)".into())
        })?)
    } else {
        None
    };
    let header = root.join("lib").join(if plotly.is_some() {
        "qd-offline-header-plotly.html"
    } else {
        "qd-offline-header.html"
    });
    std::fs::create_dir_all(header.parent().unwrap())?;
    std::fs::write(
        &header,
        header_html(&mathjax.path, plotly.as_ref().map(|a| a.path.as_path())),
    )?;
    let copy = export_copy(
        &d,
        &deck_dir,
        &Setup {
            header: &header,
            plotly: plotly.is_some(),
        },
    )?;
    let chrome = crate::browser::find(&root, opts.allow_download)?;

    // render into a staging folder next to the destination (same filesystem: atomic swap)
    std::fs::create_dir_all(&parent)?;
    for e in std::fs::read_dir(&parent)?.flatten() {
        if e.file_name()
            .to_string_lossy()
            .starts_with(&format!(".{out_name}.qd-staging-"))
        {
            let _ = std::fs::remove_dir_all(e.path()); // leftovers of earlier failed exports
        }
    }
    let staging = parent.join(format!(".{out_name}.qd-staging-{}", nanos()));
    let render_dir = staging.join(".render");
    let rendered = crate::render::render(&store.path, &copy, "export", &render_dir, false)?;
    let html = staging.join(format!("{stem}.html"));
    let one = crate::inline::inline(
        &std::fs::read_to_string(&rendered)?,
        &deck_dir,
        opts.strip_notes,
    );
    std::fs::write(&html, &one.html)?;
    // the deck is one file now: Quarto's copies of its resources (never pruned) are not needed
    std::fs::remove_dir_all(&render_dir)?;
    let files = collect_files(&d);
    for f in &files {
        let src = deck_dir.join(f);
        if src.is_file() {
            let dst = staging.join(f);
            std::fs::create_dir_all(dst.parent().unwrap())?;
            std::fs::copy(&src, &dst)?;
        }
    }
    std::fs::write(
        staging.join(MARKER),
        json!({"deck": store.path, "by": "qd export"}).to_string(),
    )?;

    let mut report = crate::offline::check(&html, &chrome)?;
    for p in one.missing {
        if !report.problems.iter().any(|q| q.url.as_ref() == Some(&p)) {
            report.problems.push(crate::offline::Problem::error(
                "P_MISSING_FILE",
                None,
                format!("'{p}' (an app, <object> or SVG image) is missing"),
                Some(p),
            ));
        }
    }
    report.ok = !report.problems.iter().any(|p| p.severity == "error");
    let mut assets = vec![mathjax];
    assets.extend(plotly);
    let final_html = out.join(format!("{stem}.html"));
    if !report.ok {
        return Ok(
            json!({"kind": "export", "ok": false, "out": out, "html": final_html, "files": files,
            "check": report, "assets": assets, "staging": staging}),
        );
    }
    let pdf = if opts.pdf {
        let p = crate::offline::print_pdf(
            &html,
            &staging.join(crate::offline::pdf_name(&stem, opts.notes)),
            opts.notes,
            opts.per_slide,
            &chrome,
        )?;
        Some(p)
    } else {
        None
    };

    // swap: the old export is removed only once the new one is in place
    let old = parent.join(format!(".{out_name}.qd-old-{}", nanos()));
    if out.exists() {
        std::fs::rename(&out, &old)?;
    }
    if let Err(e) = std::fs::rename(&staging, &out) {
        if old.exists() {
            let _ = std::fs::rename(&old, &out);
        }
        return Err(e.into());
    }
    let _ = std::fs::remove_dir_all(&old);
    report.html = final_html.clone();
    let mut v = json!({"kind": "export", "ok": true, "out": out, "html": final_html, "files": files,
        "check": report, "assets": assets});
    if let Some(mut p) = pdf {
        p.path = out.join(crate::offline::pdf_name(&stem, opts.notes));
        v["pdf"] = serde_json::to_value(p).unwrap();
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(s: &str) -> Deck {
        Deck::parse(s.to_string())
    }

    fn setup(plotly: bool) -> Setup<'static> {
        Setup {
            header: Path::new("/cache/lib/qd-offline-header.html"),
            plotly,
        }
    }

    fn fm(text: &str) -> Yaml {
        let d = Deck::parse(text.to_string());
        d.meta
    }

    fn revealjs(text: &str) -> Yaml {
        fm(text)["format"]["revealjs"].clone()
    }

    #[test]
    fn merges_offline_setup_into_front_matter() {
        let dir = tempfile::tempdir().unwrap();
        let d = deck(
            "---\ntitle: T\nformat:\n  revealjs:\n    theme: dark\n    include-in-header: mine.html\nauthor: A\n---\n\n## S\n\nx\n",
        );
        let out = export_copy(&d, dir.path(), &setup(false)).unwrap();
        let r = revealjs(&out);
        assert_eq!(r["embed-resources"], Yaml::Bool(true));
        assert_eq!(r["html-math-method"]["method"], Yaml::from("mathjax"));
        assert_eq!(r["html-math-method"]["url"], Yaml::from("about:blank"));
        assert_eq!(r["theme"], Yaml::from("dark"), "other options kept");
        let inc: Vec<&str> = r["include-in-header"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(inc, vec!["mine.html", "/cache/lib/qd-offline-header.html"]);
        assert_eq!(fm(&out)["author"], Yaml::from("A"));
        assert!(out.ends_with("## S\n\nx\n"), "body unchanged: {out}");
    }

    #[test]
    fn format_forms() {
        let dir = tempfile::tempdir().unwrap();
        for text in [
            "---\nformat: revealjs\n---\n\n## S\n",
            "## S\n\nno front matter\n",
            "---\ntitle: x\n---\n\n## S\n",
            "---\nformat:\n  revealjs: default\n---\n\n## S\n",
        ] {
            let out = export_copy(&deck(text), dir.path(), &setup(false)).unwrap();
            let r = revealjs(&out);
            assert_eq!(r["embed-resources"], Yaml::Bool(true), "{text:?} -> {out}");
            assert_eq!(
                r["include-in-header"].as_sequence().unwrap().len(),
                1,
                "{out}"
            );
        }
    }

    #[test]
    fn respects_an_existing_skill_setup() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("_quarto.yml"), "format:\n  revealjs:\n    html-math-method:\n      method: mathjax\n      url: \"about:blank\"\n    include-in-header: _deck/header.html\n").unwrap();
        let d = deck("---\ntitle: T\n---\n\n## S\n");
        assert!(already_set_up(&d, dir.path()));
        let out = export_copy(&d, dir.path(), &setup(false)).unwrap();
        let r = revealjs(&out);
        assert!(
            r.get("include-in-header").is_none(),
            "must not load MathJax twice: {out}"
        );
        assert_eq!(r["embed-resources"], Yaml::Bool(true));

        let d =
            deck("---\nhtml-math-method:\n  method: mathjax\n  url: about:blank\n---\n\n## S\n");
        assert!(already_set_up(&d, tempfile::tempdir().unwrap().path()));
        assert!(!already_set_up(
            &deck("## S\n"),
            tempfile::tempdir().unwrap().path()
        ));
    }

    #[test]
    fn plotly_renderer_cell() {
        let dir = tempfile::tempdir().unwrap();
        let text = "---\ntitle: P\njupyter: python3\n---\n\n## F\n\n```{python}\nimport plotly.express as px\npx.line(y=[1,2])\n```\n";
        let d = deck(text);
        assert!(uses_plotly(&d));
        assert!(
            !uses_plotly(&deck("## F\n\n```{python}\nprint('plotly')\n```\n")),
            "mentioning plotly is not importing it"
        );
        assert!(
            !uses_plotly(&deck("## F\n\n```python\nimport plotly\n```\n")),
            "plain code blocks do not run"
        );
        let out = export_copy(&d, dir.path(), &setup(true)).unwrap();
        let fm_end = out.find("\n---\n").unwrap() + 5;
        let after = &out[fm_end..];
        assert!(
            after.trim_start().starts_with("```{python}"),
            "renderer cell right after front matter: {out}"
        );
        assert!(
            after.contains("include_plotlyjs=False") && after.contains("include_mathjax=False"),
            "{out}"
        );
        assert!(after.contains("#| include: false"));
        let no = export_copy(&d, dir.path(), &setup(false)).unwrap();
        assert!(!no.contains("include_plotlyjs"));
    }

    #[test]
    fn detects_offline_incompatible_features() {
        let dir = tempfile::tempdir().unwrap();
        let d = deck("---\nformat:\n  revealjs:\n    chalkboard: true\n---\n\n## S\n");
        assert!(
            offline_incompatible(&d, dir.path())
                .unwrap()
                .contains("chalkboard")
        );
        let d = deck("---\nmultiplex: true\n---\n\n## S\n");
        assert!(
            offline_incompatible(&d, dir.path())
                .unwrap()
                .contains("multiplex")
        );
        std::fs::write(
            dir.path().join("_quarto.yml"),
            "format:\n  revealjs:\n    chalkboard:\n      theme: whiteboard\n",
        )
        .unwrap();
        let found = offline_incompatible(&deck("## S\n"), dir.path()).unwrap();
        assert!(
            found.contains("chalkboard") && found.contains("_quarto.yml"),
            "{found}"
        );
        assert!(
            offline_incompatible(
                &deck("---\nformat:\n  revealjs:\n    chalkboard: false\n---\n"),
                tempfile::tempdir().unwrap().path()
            )
            .is_none()
        );
    }

    #[test]
    fn collects_files_that_are_not_inlined() {
        let d = deck(
            "---\nformat:\n  revealjs:\n    parallax-background-image: img/sky.png\n---\n\n## V {background-video=\"media/clip.mp4\"}\n\n## {background-iframe=\"apps/app.html\" background-interactive=\"true\"}\n\n## R {background-video=\"https://example.com/x.mp4\"}\n\n## I {background-image=\"img/inlined.png\"}\n",
        );
        let mut f = collect_files(&d);
        f.sort();
        assert_eq!(f, vec!["img/sky.png", "media/clip.mp4"]);
    }
}
