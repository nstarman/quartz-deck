//! Which engine runs a deck's cells (spec 013 US3, research R2–R4): a Jupyter
//! kernel (Python as in 004, or any other kernel), or R for knitr decks.

use crate::deck::Deck;
use crate::error::{QdError, Result};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Jupyter,
    Knitr,
    /// Quarto's native Julia engine (`engine: julia`, US5).
    Julia,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Engine {
    pub kind: Kind,
    /// Jupyter: the kernel the deck names; `None` for qd's own (a Python deck)
    /// or the first one installed for `language`.
    pub kernel: Option<String>,
    /// The deck's first executable language (`python`, `r`, `julia`, …).
    pub language: String,
    /// Figure width and height (inches) and dpi, as a render makes them.
    pub fig: [f64; 3],
}

impl Engine {
    pub fn to_json(&self) -> Value {
        json!({"kind": self.kind, "kernel": self.kernel, "label": self.label()})
    }

    /// What the editor's badge says: the language, capitalised (`Python`, `R`).
    pub fn label(&self) -> String {
        let mut c = self.language.chars();
        c.next()
            .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
    }

    /// Whether this engine runs `lang` cells (as Quarto: knitr runs R, a
    /// Jupyter kernel its own language).
    pub fn runs(&self, lang: &str) -> bool {
        match self.kind {
            Kind::Knitr => lang == "r",
            Kind::Julia => lang == "julia",
            Kind::Jupyter => lang == self.language,
        }
    }
}

/// Quarto's revealjs figure size (inches) and dpi, for decks qd doesn't ask about.
pub const FIG: [f64; 3] = [10.0, 5.0, 96.0];

/// The languages of the deck's executable cells, in order of first use.
fn languages(d: &Deck) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for b in d
        .blocks()
        .filter(|b| b.kind == crate::deck::BlockKind::Cell)
    {
        let l = crate::kernel::cell_lang(d, b).to_string();
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// The kernel the front matter names: `jupyter: NAME` or `jupyter: {kernelspec: {name}}`.
fn kernel_named(d: &Deck) -> Option<String> {
    let j = d.meta.get("jupyter")?;
    j.as_str()
        .or_else(|| j.get("kernelspec")?.get("name")?.as_str())
        .map(String::from)
}

/// A deck of Python cells with no `engine:`/`knitr:` key runs as in 004, in a
/// Jupyter kernel, without asking Quarto; anything else is Quarto's call.
fn local(d: &Deck) -> Option<Engine> {
    let python = languages(d).iter().all(|l| l == "python");
    let keyed = ["engine", "knitr"].iter().any(|k| d.meta.get(k).is_some());
    (python && !keyed).then(|| Engine {
        kind: Kind::Jupyter,
        kernel: kernel_named(d),
        language: "python".into(),
        fig: FIG,
    })
}

/// The engine from `quarto inspect`'s answer (research R2).
fn from_inspect(v: &Value, d: &Deck) -> Result<Engine> {
    let language = languages(d)
        .into_iter()
        .next()
        .unwrap_or_else(|| "python".into());
    let kind = match v["engines"][0].as_str() {
        Some("knitr") => Kind::Knitr,
        Some("jupyter") => Kind::Jupyter,
        Some("julia") => Kind::Julia,
        Some("markdown") => {
            return Err(QdError::Engine(format!(
                "{language} cells here: they run in the page, not in a session"
            )));
        }
        other => {
            return Err(QdError::Engine(format!(
                "{language} cells: Quarto's {} engine isn't supported here",
                other.unwrap_or("?")
            )));
        }
    };
    let x = &v["formats"]["revealjs"]["execute"];
    let num = |k: &str, d: f64| x[k].as_f64().unwrap_or(d);
    Ok(Engine {
        kernel: (kind == Kind::Jupyter).then(|| kernel_named(d)).flatten(),
        kind,
        language,
        fig: [
            num("fig-width", FIG[0]),
            num("fig-height", FIG[1]),
            num("fig-dpi", FIG[2]),
        ],
    })
}

/// What Quarto's answer depends on: the front matter and the cells' languages.
fn cache_key(d: &Deck) -> String {
    let fm = d.front_matter.map_or("", |s| d.slice(s));
    crate::deck::version_of(format!("{fm}\n{}", languages(d).join(",")).as_bytes())
}

/// The engine for `d` (the deck at `deck_path`): [`local`], else `quarto
/// inspect`, asked once per front matter and set of languages (cached).
pub fn detect(deck_path: &Path, d: &Deck) -> Result<Engine> {
    if let Some(e) = local(d) {
        return Ok(e);
    }
    let cached = crate::store::cache_root()
        .join("engine")
        .join(format!("{}.json", cache_key(d)));
    if let Some(v) = std::fs::read(&cached)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
    {
        return from_inspect(&v, d);
    }
    let dir = deck_path.parent().unwrap_or(Path::new("."));
    let out = std::process::Command::new(crate::render::quarto()?)
        .arg("inspect")
        .arg(deck_path)
        .current_dir(dir)
        .output()?;
    if !out.status.success() {
        // Quarto's own words (e.g. the kernel isn't installed, with the ones that are)
        let log = String::from_utf8_lossy(&out.stderr);
        let line = log
            .lines()
            .find_map(|l| l.split_once("ERROR: ").map(|(_, m)| m))
            .unwrap_or(log.trim());
        let lang = languages(d).into_iter().next().unwrap_or_default();
        return Err(QdError::Engine(format!("{lang} cells: {line}")));
    }
    let v: Value = serde_json::from_slice(&out.stdout).map_err(std::io::Error::other)?;
    let keep = json!({"engines": v["engines"], "formats": {"revealjs": {"execute": v["formats"]["revealjs"]["execute"]}}});
    std::fs::create_dir_all(cached.parent().unwrap())?;
    std::fs::write(&cached, keep.to_string())?;
    from_inspect(&keep, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(t: &str) -> Deck {
        Deck::parse(t.into())
    }

    #[test]
    fn python_decks_need_no_quarto() {
        let d = deck("## A\n\n```{python}\n1\n```\n");
        assert_eq!(
            local(&d),
            Some(Engine {
                kind: Kind::Jupyter,
                kernel: None,
                language: "python".into(),
                fig: FIG
            })
        );
        let named = deck("---\njupyter: qd-test\n---\n\n## A\n\n```{python}\n1\n```\n");
        assert_eq!(local(&named).unwrap().kernel.as_deref(), Some("qd-test"));
        let spec = deck(
            "---\njupyter:\n  kernelspec:\n    name: other\n---\n\n## A\n\n```{python}\n1\n```\n",
        );
        assert_eq!(local(&spec).unwrap().kernel.as_deref(), Some("other"));
        // anything else is Quarto's call
        assert_eq!(local(&deck("## A\n\n```{r}\n1\n```\n")), None);
        assert_eq!(
            local(&deck(
                "---\nengine: knitr\n---\n\n## A\n\n```{python}\n1\n```\n"
            )),
            None
        );
    }

    #[test]
    fn reads_quarto_inspect() {
        let d = deck("## A\n\n```{r}\nplot(1)\n```\n");
        let inspect = json!({"engines": ["knitr"], "formats": {"revealjs": {"execute":
            {"fig-width": 10, "fig-height": 5, "fig-dpi": 96}}}});
        assert_eq!(
            from_inspect(&inspect, &d).unwrap(),
            Engine {
                kind: Kind::Knitr,
                kernel: None,
                language: "r".into(),
                fig: [10.0, 5.0, 96.0]
            }
        );
        let jl = deck("---\njupyter: julia-1.10\n---\n\n## A\n\n```{julia}\n1\n```\n");
        let e = from_inspect(&json!({"engines": ["jupyter"]}), &jl).unwrap();
        assert_eq!(
            (e.kind, e.kernel.as_deref(), e.fig),
            (Kind::Jupyter, Some("julia-1.10"), FIG)
        );
        // Quarto's native Julia engine (US5)
        let jn = deck("---\nengine: julia\n---\n\n## A\n\n```{julia}\n1\n```\n");
        let e = from_inspect(&json!({"engines": ["julia"]}), &jn).unwrap();
        assert_eq!(
            (e.kind, e.language.as_str(), e.label()),
            (Kind::Julia, "julia", "Julia".into())
        );
        assert!(e.runs("julia") && !e.runs("python"));
        // OJS-only (Quarto says `markdown`): nothing runs in a session
        let err = from_inspect(
            &json!({"engines": ["markdown"]}),
            &deck("## A\n\n```{ojs}\n1\n```\n"),
        );
        assert_eq!(err.unwrap_err().code(), "E_ENGINE");
    }

    #[test]
    fn asks_again_only_when_the_answer_could_change() {
        let a = deck("---\ntitle: A\n---\n\n## A\n\n```{r}\n1\n```\n");
        let body = deck("---\ntitle: A\n---\n\n## A\n\n```{r}\n2\n```\n");
        let title = deck("---\ntitle: B\n---\n\n## A\n\n```{r}\n1\n```\n");
        let lang = deck("---\ntitle: A\n---\n\n## A\n\n```{r}\n1\n```\n\n```{bash}\nls\n```\n");
        assert_eq!(cache_key(&a), cache_key(&body));
        assert_ne!(cache_key(&a), cache_key(&title));
        assert_ne!(cache_key(&a), cache_key(&lang));
    }
}
