//! `qd bib --zotero KEY` (spec 010 US5): write the References list from a
//! Zotero collection into the deck, between `<!-- bibliography:start … -->`
//! and `<!-- bibliography:end -->`. Port of the quarto-revealjs skill's
//! `bibliography.py`. Reads the running Zotero 7's local API on this machine
//! only; nothing leaves it.

use crate::deck::{Deck, Span};
use crate::error::{QdError, Result};
use crate::ops::Patch;
use crate::store::OtherFile;
use serde_json::{Value, json};
use serde_yaml_ng::Value as Yaml;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const START: &str = "<!-- bibliography:start";
const END: &str = "<!-- bibliography:end -->";

/// Zotero's local API (user 0 is the local user); `QD_ZOTERO_URL` replaces the host in tests.
fn api() -> String {
    std::env::var("QD_ZOTERO_URL").unwrap_or_else(|_| "http://localhost:23119".into())
}

/// The collection's top-level items, without attachments and notes (pages of 100).
pub fn fetch(key: &str) -> Result<Vec<Value>> {
    // it goes into a URL: Zotero keys are 8 letters and digits
    if key.len() != 8 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(QdError::InvalidArg(format!(
            "'{key}' is not a Zotero collection key (8 letters and digits, from the collection's URL)"
        )));
    }
    let mut items = Vec::new();
    for start in (0..).step_by(100) {
        let url = format!(
            "{}/api/users/0/collections/{key}/items/top?format=json&limit=100&start={start}",
            api()
        );
        let fail = |e: &dyn std::fmt::Display| QdError::Zotero(format!("{url}: {e}"));
        let text = ureq::get(&url)
            .call()
            .and_then(|mut r| r.body_mut().read_to_string())
            .map_err(|e| fail(&e))?;
        let page: Vec<Value> = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        let n = page.len();
        items.extend(
            page.into_iter()
                .map(|i| i["data"].clone())
                .filter(|d| !matches!(d["itemType"].as_str(), Some("attachment" | "note"))),
        );
        if n < 100 {
            break;
        }
    }
    Ok(items)
}

fn s<'a>(d: &'a Value, k: &str) -> &'a str {
    d[k].as_str().unwrap_or("")
}

/// "Name", "A & B" or "A et al.", and the year.
fn label(d: &Value) -> (String, String) {
    let names: Vec<&str> = d["creators"]
        .as_array()
        .into_iter()
        .flatten()
        // software lists programmers
        .filter(|c| {
            matches!(
                c["creatorType"].as_str().unwrap_or("author"),
                "author" | "programmer"
            )
        })
        .map(|c| {
            let last = c["lastName"].as_str().filter(|n| !n.is_empty());
            last.or(c["name"].as_str()).unwrap_or("")
        })
        .collect();
    let date = s(d, "date");
    let year = date
        .as_bytes()
        .windows(4)
        .position(|w| w.iter().all(u8::is_ascii_digit))
        .map_or("n.d.".to_string(), |i| date[i..i + 4].to_string());
    let who = match names[..] {
        [] => s(d, "title").chars().take(30).collect(),
        [a] => a.to_string(),
        [a, b] => format!("{a} & {b}"),
        [a, ..] => format!("{a} et al."),
    };
    (who, year)
}

/// The DOI, except arXiv's 10.48550 DOIs, where the item's URL is better.
fn link(d: &Value) -> String {
    let doi = s(d, "DOI");
    if !doi.is_empty() && !doi.to_lowercase().starts_with("10.48550") {
        return format!("https://doi.org/{doi}");
    }
    match s(d, "url") {
        "" if !doi.is_empty() => format!("https://doi.org/{doi}"),
        u => u.to_string(),
    }
}

/// Text for a shortcode argument: no tags, entities decoded, one line, no `"`.
fn clean(t: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in t.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    crate::inline::unescape(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'")
}

/// One `- {{< paper … >}}` line per item, sorted by label; equal labels get
/// 2021a, 2021b. Also the warnings (items with no link).
pub fn lines(items: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut items: Vec<(&Value, (String, String))> = items.iter().map(|d| (d, label(d))).collect();
    items.sort_by(|(a, la), (b, lb)| {
        (la.0.to_lowercase(), &la.1, s(a, "title")).cmp(&(
            lb.0.to_lowercase(),
            &lb.1,
            s(b, "title"),
        ))
    });
    let mut count: HashMap<&(String, String), usize> = HashMap::new();
    for (_, l) in &items {
        *count.entry(l).or_default() += 1;
    }
    let mut seen: HashMap<&(String, String), u8> = HashMap::new();
    let (mut out, mut warnings) = (Vec::new(), Vec::new());
    for (d, l) in &items {
        let (who, mut year) = l.clone();
        if count[l] > 1 {
            let n = seen.entry(l).or_default();
            year.push((b'a' + *n) as char);
            *n += 1;
        }
        let url = link(d);
        if url.is_empty() {
            warnings.push(format!(
                "{who} ({year}) has no DOI or URL; its entry links nowhere. Add one in Zotero."
            ));
        }
        out.push(format!(
            r#"- {{{{< paper url="{url}" who="{}" year="{year}" title="{}" >}}}}"#,
            clean(&who),
            clean(s(d, "title"))
        ));
    }
    (out, warnings)
}

/// Replace what is between the deck's one marker pair with `rows`.
pub fn patch(d: &Deck, rows: &[String]) -> Result<Patch> {
    let starts: Vec<usize> = d.text.match_indices(START).map(|(i, _)| i).collect();
    let ends: Vec<usize> = d.text.match_indices(END).map(|(i, _)| i).collect();
    let hint = format!("the deck needs one `{START} -->` … `{END}` pair on its References slide");
    match (starts.len(), ends.len()) {
        (0, _) | (_, 0) => return Err(QdError::NotFound(hint)),
        (1, 1) => {}
        _ => return Err(QdError::Ambiguous(hint)),
    }
    let line_end = d.text[starts[0]..].find('\n').map(|i| starts[0] + i + 1);
    let Some(from) = line_end.filter(|&f| f <= ends[0]) else {
        return Err(QdError::NotFound(hint));
    };
    let body: String = rows.iter().map(|r| format!("{r}{}", d.nl)).collect();
    Ok(Patch {
        op: "bib".into(),
        replacements: vec![(
            Span {
                start: from,
                end: ends[0],
            },
            body,
        )],
        assigned_ids: vec![],
        target: "bibliography".into(),
        converted_units: false,
    })
}

// ---------- spec 014: the deck's references, their files and formats (R5) ----------

/// A references file's format (`.json` is CSL-JSON, anything else BibTeX/BibLaTeX).
#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Format {
    Bib,
    Json,
}

impl Format {
    pub fn of(path: &Path) -> Format {
        match path.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("json") => Format::Json,
            _ => Format::Bib,
        }
    }

    fn ext(self) -> &'static str {
        match self {
            Format::Bib => "bib",
            Format::Json => "json",
        }
    }
}

const HEADER: &str =
    "% Written by qd bib. Comments and @string macros are not kept when qd rewrites this file.\n\n";

/// `quarto pandoc ARGS`, with `input` on stdin; its stdout.
fn pandoc(args: &[&str], input: &str) -> Result<String> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = std::process::Command::new(crate::render::quarto()?)
        .arg("pandoc")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(input.as_bytes())?;
    drop(stdin);
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(QdError::Parse(format!(
            "quarto pandoc {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn entries_of(json_text: &str, what: &Path) -> Result<Vec<Value>> {
    match serde_json::from_str::<Value>(json_text) {
        Ok(Value::Array(a)) if a.iter().all(Value::is_object) => Ok(a),
        _ => Err(QdError::Parse(format!(
            "{} is not a CSL-JSON list of references",
            what.display()
        ))),
    }
}

/// A `.bib` (read as BibLaTeX, which covers BibTeX) or CSL-JSON file's entries.
pub fn read_file(path: &Path) -> Result<Vec<Value>> {
    let text = fs::read_to_string(path)
        .map_err(|e| QdError::NotFound(format!("{}: {e}", path.display())))?;
    match Format::of(path) {
        Format::Json => entries_of(&text, path),
        Format::Bib => entries_of(&pandoc(&["-f", "biblatex", "-t", "csljson"], &text)?, path),
    }
}

/// The text of a references file holding `entries`.
pub fn file_text(entries: &[Value], f: Format) -> Result<String> {
    let json = serde_json::to_string_pretty(entries).expect("JSON values serialize");
    Ok(match f {
        Format::Json => json + "\n",
        Format::Bib => format!(
            "{HEADER}{}",
            pandoc(&["-f", "csljson", "-t", "biblatex"], &json)?
        ),
    })
}

/// A file named by `bibliography:`.
pub struct BibFile {
    /// As written in the front matter.
    pub written: String,
    pub path: PathBuf,
    /// Under the deck's folder: qd may rewrite it (FR-020, FR-021).
    pub inside: bool,
}

/// Where a deck's references live (data-model "Deck references").
pub struct Refs {
    pub files: Vec<BibFile>,
    pub embedded: Vec<Value>,
}

/// `p` resolved, also when the file doesn't exist (yet, or any more).
pub fn canonical(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| match (p.parent(), p.file_name()) {
        (Some(dir), Some(name)) => fs::canonicalize(dir).map_or(p.to_path_buf(), |d| d.join(name)),
        _ => p.to_path_buf(),
    })
}

pub fn refs(d: &Deck, dir: &Path) -> Refs {
    let written: Vec<String> = match d.meta.get("bibliography") {
        Some(Yaml::String(s)) => vec![s.clone()],
        Some(Yaml::Sequence(v)) => v
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    };
    let root = canonical(dir);
    let files = written
        .into_iter()
        .map(|w| {
            let path = canonical(&dir.join(&w));
            BibFile {
                inside: path.starts_with(&root),
                written: w,
                path,
            }
        })
        .collect();
    let embedded = d
        .meta
        .get("references")
        .and_then(|y| serde_json::to_value(y).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    Refs { files, embedded }
}

impl Refs {
    /// The one file qd may write: `None` when the deck names no file.
    fn writable(&self) -> Result<Option<&BibFile>> {
        let inside: Vec<&BibFile> = self.files.iter().filter(|f| f.inside).collect();
        match inside[..] {
            [f] => Ok(Some(f)),
            [] if self.files.is_empty() => Ok(None),
            [] => Err(QdError::ReadOnly(format!(
                "{} is outside the deck's folder",
                self.files[0].path.display()
            ))),
            _ => Err(QdError::Ambiguous(format!(
                "the deck names several bibliography files: {}",
                inside
                    .iter()
                    .map(|f| f.written.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    fn file_entries(&self) -> Result<Vec<Value>> {
        let mut all = Vec::new();
        for f in &self.files {
            all.extend(read_file(&f.path)?);
        }
        Ok(all)
    }
}

/// Where a new companion file goes: `_talk/` when the deck has one, else beside it.
fn default_place(dir: &Path, f: Format) -> String {
    let name = format!("references.{}", f.ext());
    if dir.join("_talk").is_dir() {
        format!("_talk/{name}")
    } else {
        name
    }
}

/// Keys cited on the slides (`@key`), outside code and math; Quarto's
/// cross-references (`@fig-…`) and e-mail addresses are not citations.
pub fn cited_keys(d: &Deck) -> Vec<(String, String)> {
    use crate::deck::BlockKind;
    let re = regex::Regex::new(
        r"(?:^|[^\w@.])@([A-Za-z0-9_](?:[A-Za-z0-9_]|[:.#$%&\-+?<>~/][A-Za-z0-9_])*)",
    )
    .expect("valid regex");
    const CROSSREF: [&str; 13] = [
        "fig-", "tbl-", "sec-", "eq-", "lst-", "thm-", "lem-", "cor-", "prp-", "cnj-", "def-",
        "exm-", "exr-",
    ];
    let mut out: Vec<(String, String)> = Vec::new();
    for slide in &d.slides {
        for b in &slide.blocks {
            if matches!(
                b.kind,
                BlockKind::Code | BlockKind::Cell | BlockKind::Math | BlockKind::Html
            ) {
                continue;
            }
            for c in re.captures_iter(d.slice(b.span)) {
                let key = c[1].to_string();
                if !CROSSREF.iter().any(|p| key.starts_with(p))
                    && !out.iter().any(|(k, s)| *k == key && *s == slide.id)
                {
                    out.push((key, slide.id.clone()));
                }
            }
        }
    }
    out
}

/// `qd bib`: where the references live, and cited keys with no entry.
pub fn list(d: &Deck, dir: &Path) -> Result<Value> {
    let r = refs(d, dir);
    let entries = [r.file_entries()?, r.embedded.clone()].concat();
    let ids: std::collections::HashSet<&str> = entries.iter().map(|e| s(e, "id")).collect();
    let mut missing: Vec<Value> = Vec::new();
    for (key, slide) in cited_keys(d) {
        if ids.contains(key.as_str()) {
            continue;
        }
        match missing.iter_mut().find(|m| m["key"] == key.as_str()) {
            Some(m) => m["slides"].as_array_mut().unwrap().push(json!(slide)),
            None => missing.push(json!({"key": key, "slides": [slide]})),
        }
    }
    let place = match (r.files.is_empty(), r.embedded.is_empty()) {
        (true, true) => "none",
        (false, true) => "companion",
        (true, false) => "embedded",
        (false, false) => "both",
    };
    let files: Vec<Value> = r
        .files
        .iter()
        .map(
            |f| json!({"path": f.written, "format": Format::of(&f.path).ext(), "inside": f.inside}),
        )
        .collect();
    Ok(
        json!({"ok": true, "kind": "bib", "where": place, "files": files,
        "embedded": r.embedded.len(), "entries": entries.len(), "missing": missing}),
    )
}

/// A `qd bib` write: the deck's new text, the other files, and what to report.
pub struct Edit {
    pub text: String,
    pub also: Vec<OtherFile>,
    pub report: Value,
}

/// Apply front-matter changes in turn (`None` removes a key that is there).
fn set_front(text: &str, changes: &[(&str, Option<Yaml>)]) -> Result<String> {
    let mut text = text.to_string();
    for (key, value) in changes {
        let d = Deck::parse(text.clone());
        let p = match value {
            Some(v) => crate::ops::meta_set_yaml(&d, key, v)?,
            None if d.meta.get(*key).is_some() => crate::ops::meta_unset(&d, key)?,
            None => continue,
        };
        text = crate::ops::apply(&text, &p)?;
    }
    Ok(text)
}

fn yaml(v: &impl serde::Serialize) -> Yaml {
    serde_yaml_ng::to_value(v).expect("JSON values are YAML")
}

fn file_change(path: &Path, after: String) -> OtherFile {
    OtherFile {
        path: path.to_path_buf(),
        before: fs::read_to_string(path).ok(),
        after: Some(after),
    }
}

fn merge_report(m: &Merge) -> Value {
    json!({"added": m.added, "changed": m.changed, "merged": m.merged, "gone": m.gone, "warnings": m.warnings})
}

/// `qd bib pull FILE`: merge a file's entries into the deck's references.
pub fn pull(d: &Deck, dir: &Path, incoming: Vec<Value>) -> Result<Edit> {
    let r = refs(d, dir);
    if let Some(f) = r.writable()? {
        let m = merge(read_file(&f.path)?, incoming);
        let after = file_text(&m.entries, Format::of(&f.path))?;
        let mut report = merge_report(&m);
        report["file"] = json!(f.written);
        report["created"] = json!(false);
        return Ok(Edit {
            text: d.text.clone(),
            also: vec![file_change(&f.path, after)],
            report,
        });
    }
    if !r.embedded.is_empty() {
        let m = merge(r.embedded, incoming);
        let mut report = merge_report(&m);
        report["embedded"] = json!(true);
        return Ok(Edit {
            text: set_front(&d.text, &[("references", Some(yaml(&m.entries)))])?,
            also: vec![],
            report,
        });
    }
    let written = default_place(dir, Format::Bib);
    let m = merge(vec![], incoming);
    let after = file_text(&m.entries, Format::Bib)?;
    let mut report = merge_report(&m);
    report["file"] = json!(written);
    report["created"] = json!(true);
    Ok(Edit {
        text: set_front(
            &d.text,
            &[("bibliography", Some(Yaml::String(written.clone())))],
        )?,
        also: vec![file_change(&dir.join(&written), after)],
        report,
    })
}

/// `qd bib embed`: every file's entries into `references:`; the files stay on disk.
pub fn embed(d: &Deck, dir: &Path) -> Result<Edit> {
    let r = refs(d, dir);
    if r.files.is_empty() {
        return Err(QdError::NotFound(
            "the deck names no bibliography file to embed".into(),
        ));
    }
    let m = merge(r.embedded.clone(), r.file_entries()?);
    let left: Vec<&str> = r.files.iter().map(|f| f.written.as_str()).collect();
    let mut report = json!({"entries": m.entries.len(), "warnings": m.warnings});
    report["left"] = if left.len() == 1 {
        json!(left[0])
    } else {
        json!(left)
    };
    Ok(Edit {
        text: set_front(
            &d.text,
            &[
                ("references", Some(yaml(&m.entries))),
                ("bibliography", None),
            ],
        )?,
        also: vec![],
        report,
    })
}

/// `qd bib extract`: `references:` into a companion file inside the deck's folder.
pub fn extract(d: &Deck, dir: &Path, to: Option<&Path>, format: Option<Format>) -> Result<Edit> {
    let r = refs(d, dir);
    if r.embedded.is_empty() {
        return Err(QdError::NotFound(
            "the deck has no embedded references".into(),
        ));
    }
    let written = match (to, r.writable()) {
        (Some(p), _) => p.to_string_lossy().into_owned(),
        (None, Ok(Some(f))) => f.written.clone(),
        _ => default_place(dir, format.unwrap_or(Format::Bib)),
    };
    let path = canonical(&dir.join(&written));
    if !path.starts_with(canonical(dir)) {
        return Err(QdError::ReadOnly(format!(
            "{} is outside the deck's folder",
            path.display()
        )));
    }
    let f = format.unwrap_or(Format::of(&path));
    let (written, path) = if Format::of(&path) == f {
        (written, path)
    } else {
        let w = Path::new(&written)
            .with_extension(f.ext())
            .to_string_lossy()
            .into_owned();
        (w.clone(), dir.join(w))
    };
    let existing = if path.exists() {
        read_file(&path)?
    } else {
        vec![]
    };
    let m = merge(existing, r.embedded);
    let after = file_text(&m.entries, f)?;
    Ok(Edit {
        text: set_front(
            &d.text,
            &[
                ("bibliography", Some(Yaml::String(written.clone()))),
                ("references", None),
            ],
        )?,
        also: vec![file_change(&path, after)],
        report: json!({"file": written, "entries": m.entries.len(), "warnings": m.warnings}),
    })
}

/// `qd bib convert`: the companion file in the other format, renamed.
pub fn convert(d: &Deck, dir: &Path, to: Format) -> Result<Edit> {
    let r = refs(d, dir);
    let f = r
        .writable()?
        .ok_or_else(|| QdError::NotFound("the deck names no bibliography file".into()))?;
    let written = Path::new(&f.written)
        .with_extension(to.ext())
        .to_string_lossy()
        .into_owned();
    if Format::of(&f.path) == to {
        return Ok(Edit {
            text: d.text.clone(),
            also: vec![],
            report: json!({"file": f.written}),
        });
    }
    let new_path = f.path.with_extension(to.ext());
    if new_path.exists() {
        return Err(QdError::Conflict(format!(
            "{} already exists",
            new_path.display()
        )));
    }
    let after = file_text(&read_file(&f.path)?, to)?;
    let bibliography = match d.meta.get("bibliography") {
        Some(Yaml::Sequence(v)) => Yaml::Sequence(
            v.iter()
                .map(|x| {
                    if x.as_str() == Some(&f.written) {
                        Yaml::String(written.clone())
                    } else {
                        x.clone()
                    }
                })
                .collect(),
        ),
        _ => Yaml::String(written.clone()),
    };
    Ok(Edit {
        text: set_front(&d.text, &[("bibliography", Some(bibliography))])?,
        also: vec![
            OtherFile {
                path: new_path,
                before: None,
                after: Some(after),
            },
            OtherFile {
                path: f.path.clone(),
                before: fs::read_to_string(&f.path).ok(),
                after: None,
            },
        ],
        report: json!({"file": written, "from": f.written}),
    })
}

// ---------- spec 014: references as CSL-JSON entries (research R6) ----------

/// A DOI as compared: lowercased, without a resolver prefix.
pub fn normalise_doi(doi: &str) -> String {
    let d = doi.trim().to_lowercase();
    let d = d
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let d = d
        .trim_start_matches("dx.doi.org/")
        .trim_start_matches("doi.org/");
    d.trim_start_matches("doi:").to_string()
}

/// A URL as compared: no scheme, `www.` or trailing `/`; an arXiv page is
/// `arxiv:ID` (no version, no `.pdf`).
pub fn normalise_url(url: &str) -> String {
    let u = url.trim().to_lowercase();
    let u = u.split_once("://").map_or(u.as_str(), |(_, r)| r);
    let u = u.trim_start_matches("www.").trim_end_matches('/');
    for p in ["arxiv.org/abs/", "arxiv.org/pdf/"] {
        if let Some(id) = u.strip_prefix(p) {
            let id = id.trim_end_matches(".pdf");
            let id = match id.rfind('v') {
                Some(i) if i + 1 < id.len() && id[i + 1..].bytes().all(|b| b.is_ascii_digit()) => {
                    &id[..i]
                }
                _ => id,
            };
            return format!("arxiv:{id}");
        }
    }
    u.to_string()
}

/// A title as compared: lowercase words, punctuation as spaces.
pub fn normalise_title(t: &str) -> String {
    t.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What identifies an entry, by kind: DOIs, then URLs (arXiv ids too), then titles.
fn identities(e: &Value, kind: usize) -> Vec<String> {
    let doi = s(e, "DOI");
    let url = s(e, "URL");
    let mut out = Vec::new();
    match kind {
        0 => {
            if !doi.is_empty() {
                out.push(normalise_doi(doi));
            }
            let u = normalise_url(url);
            if let Some(d) = u.strip_prefix("doi.org/").or(u.strip_prefix("dx.doi.org/")) {
                out.push(d.to_string());
            }
        }
        1 => {
            if !url.is_empty() {
                out.push(normalise_url(url));
            }
            if let Some(id) = normalise_doi(doi).strip_prefix("10.48550/arxiv.") {
                out.push(format!("arxiv:{id}"));
            }
        }
        _ => {
            let t = normalise_title(s(e, "title"));
            if !t.is_empty() {
                out.push(t);
            }
        }
    }
    out
}

/// The entry in `entries` that is the same paper as `e`: by DOI, else URL, else title.
pub fn find_match(entries: &[Value], e: &Value) -> Option<usize> {
    (0..3).find_map(|kind| {
        let mine = identities(e, kind);
        entries
            .iter()
            .position(|x| identities(x, kind).iter().any(|i| mine.contains(i)))
    })
}

/// Letters only, lowercased, common Latin accents folded. ponytail: a small
/// fold table, not Unicode normalisation; other letters are dropped.
fn ascii_letters(t: &str) -> String {
    const FOLD: [(&str, char); 14] = [
        ("àáâãäåāăą", 'a'),
        ("çćč", 'c'),
        ("ďđ", 'd'),
        ("èéêëēėęě", 'e'),
        ("ğ", 'g'),
        ("ìíîïī", 'i'),
        ("łľ", 'l'),
        ("ñńň", 'n'),
        ("òóôõöøō", 'o'),
        ("řŕ", 'r'),
        ("śšş", 's'),
        ("ùúûüūů", 'u'),
        ("ýÿ", 'y'),
        ("žźż", 'z'),
    ];
    t.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_ascii_lowercase() {
                return Some(c);
            }
            FOLD.iter()
                .find(|(from, _)| from.contains(c))
                .map(|&(_, to)| to)
        })
        .collect()
}

/// The year an entry was issued, or `nd`.
fn year(e: &Value) -> String {
    let i = &e["issued"];
    let first = &i["date-parts"][0][0];
    let y = first
        .as_u64()
        .map(|y| y.to_string())
        .or_else(|| first.as_str().map(str::to_string))
        .or_else(|| {
            i["raw"]
                .as_str()
                .or(i["literal"].as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let b = y.as_bytes();
    b.windows(4)
        .position(|w| w.iter().all(u8::is_ascii_digit))
        .map_or("nd".into(), |p| y[p..p + 4].to_string())
}

/// A key for an entry its source gave none: first author's surname + year,
/// with `a`, `b`… when `taken` has it (`bonaca2020`, `bonaca2020a`).
pub fn key_for(e: &Value, taken: &std::collections::HashSet<String>) -> String {
    let a = &e["author"][0];
    let name = a["family"].as_str().or(a["literal"].as_str());
    let mut stem = name.map(ascii_letters).unwrap_or_default();
    if stem.is_empty() {
        stem = s(e, "title")
            .split_whitespace()
            .next()
            .map(ascii_letters)
            .unwrap_or_default();
    }
    if stem.is_empty() {
        stem = "ref".into();
    }
    free_key(&format!("{stem}{}", year(e)), taken)
}

/// `base`, else `base` with `a`, `b`… — the first not in `taken`.
fn free_key(base: &str, taken: &std::collections::HashSet<String>) -> String {
    std::iter::once(String::new())
        .chain((b'a'..=b'z').map(|c| (c as char).to_string()))
        .map(|suffix| format!("{base}{suffix}"))
        .find(|k| !taken.contains(k))
        .unwrap_or_else(|| format!("{base}-{}", taken.len()))
}

/// What a pull did (data-model "Merge result").
#[derive(Default)]
pub struct Merge {
    pub entries: Vec<Value>,
    pub added: Vec<String>,
    pub changed: Vec<Value>,
    pub merged: Vec<Value>,
    pub gone: Vec<String>,
    pub warnings: Vec<String>,
}

/// Merge `incoming` into the deck's `existing` entries (spec FR-017–FR-019):
/// a match keeps its key and takes the source's fields; a new entry keeps
/// its source's key, else gets one; nothing is deleted.
pub fn merge(existing: Vec<Value>, incoming: Vec<Value>) -> Merge {
    let n_existing = existing.len();
    let mut m = Merge {
        entries: existing,
        ..Merge::default()
    };
    let mut seen = vec![false; n_existing];
    let mut taken: std::collections::HashSet<String> =
        m.entries.iter().map(|e| s(e, "id").to_string()).collect();
    for inc in incoming {
        let Some(fields) = inc.as_object() else {
            m.warnings
                .push(format!("skipped a reference that is not an object: {inc}"));
            continue;
        };
        if let Some(i) = find_match(&m.entries, &inc) {
            let entry = m.entries[i].as_object_mut().expect("entries are objects");
            let id = entry["id"].as_str().unwrap_or("").to_string();
            let mut changed = Vec::new();
            for (k, v) in fields.iter().filter(|(k, _)| *k != "id") {
                if entry.get(k) != Some(v) {
                    changed.push(k.clone());
                    entry.insert(k.clone(), v.clone());
                }
            }
            if i < n_existing {
                seen[i] = true;
                if !changed.is_empty() {
                    m.changed.push(json!({"id": id, "fields": changed}));
                }
            } else {
                let from = inc["id"].as_str().unwrap_or("").to_string();
                match m.merged.iter_mut().find(|x| x["id"] == id.as_str()) {
                    Some(x) => x["from"].as_array_mut().unwrap().push(json!(from)),
                    None => m.merged.push(json!({"id": id, "from": [from]})),
                }
            }
            continue;
        }
        let given = s(&inc, "id").to_string();
        let id = if given.is_empty() {
            key_for(&inc, &taken)
        } else if taken.contains(&given) {
            let k = free_key(&given, &taken);
            m.warnings.push(format!(
                "'{given}' is already the key of another paper; this one is '{k}'"
            ));
            k
        } else {
            given
        };
        let mut e = inc.clone();
        e["id"] = json!(id);
        taken.insert(id.clone());
        m.added.push(id);
        m.entries.push(e);
    }
    m.gone = m.entries[..n_existing]
        .iter()
        .zip(&seen)
        .filter(|(_, seen)| !**seen)
        .map(|(e, _)| s(e, "id").to_string())
        .collect();
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---------- spec 014: matching, keys, merge (research R6) ----------

    #[test]
    fn normalises_dois_urls_and_titles() {
        assert_eq!(normalise_doi("https://doi.org/10.1/AbC"), "10.1/abc");
        assert_eq!(normalise_doi("doi:10.1/x"), "10.1/x");
        assert_eq!(normalise_url("https://www.Example.org/a/"), "example.org/a");
        assert_eq!(
            normalise_url("http://arxiv.org/abs/2602.01806v2"),
            "arxiv:2602.01806"
        );
        assert_eq!(
            normalise_url("https://arxiv.org/pdf/2602.01806.pdf"),
            "arxiv:2602.01806"
        );
        assert_eq!(
            normalise_url("https://arxiv.org/abs/astro-ph/0508497"),
            "arxiv:astro-ph/0508497"
        );
        assert_eq!(
            normalise_title("The Shape of  Dark-Matter Halos!"),
            "the shape of dark matter halos"
        );
    }

    #[test]
    fn matches_doi_then_url_then_title() {
        let deck = vec![
            json!({"id": "a", "DOI": "10.1/a", "title": "A"}),
            json!({"id": "b", "URL": "https://arxiv.org/abs/2602.01806", "title": "B"}),
            json!({"id": "c", "title": "Hidden depths"}),
        ];
        let find = |v: Value| find_match(&deck, &v);
        assert_eq!(find(json!({"DOI": "https://doi.org/10.1/A"})), Some(0));
        // an arXiv DOI and an arXiv URL are the same paper
        assert_eq!(find(json!({"DOI": "10.48550/arXiv.2602.01806"})), Some(1));
        assert_eq!(
            find(json!({"URL": "http://arxiv.org/abs/2602.01806v1"})),
            Some(1)
        );
        assert_eq!(find(json!({"title": "Hidden Depths"})), Some(2));
        assert_eq!(find(json!({"title": "Other"})), None);
        // DOI wins over a title that matches another entry
        assert_eq!(find(json!({"DOI": "10.1/a", "title": "B"})), Some(0));
    }

    #[test]
    fn generated_keys() {
        let k =
            |v: Value, taken: &[&str]| key_for(&v, &taken.iter().map(|s| s.to_string()).collect());
        let issued = json!({"date-parts": [[2020]]});
        assert_eq!(
            k(
                json!({"author": [{"family": "Bonaca"}], "issued": issued}),
                &[]
            ),
            "bonaca2020"
        );
        assert_eq!(
            k(
                json!({"author": [{"family": "Bonaca"}], "issued": issued}),
                &["bonaca2020"]
            ),
            "bonaca2020a"
        );
        assert_eq!(
            k(
                json!({"author": [{"family": "Bonaca"}], "issued": issued}),
                &["bonaca2020", "bonaca2020a"]
            ),
            "bonaca2020b"
        );
        assert_eq!(
            k(
                json!({"author": [{"family": "Martínez-Delgado"}], "issued": {"date-parts": [[2023]]}}),
                &[]
            ),
            "martinezdelgado2023"
        );
        assert_eq!(
            k(
                json!({"author": [{"literal": "Astropy Collaboration"}], "issued": {"date-parts": [["2022"]]}}),
                &[]
            ),
            "astropycollaboration2022"
        );
        assert_eq!(k(json!({"author": [{"family": "Zed"}]}), &[]), "zednd");
        assert_eq!(k(json!({"title": "phasecurvefit"}), &[]), "phasecurvefitnd");
    }

    #[test]
    fn merge_keeps_keys_and_lists_changes() {
        let deck = vec![
            json!({"id": "bonaca2020", "DOI": "10.1/a", "title": "Spur", "note": "mine"}),
            json!({"id": "old2001", "title": "Gone"}),
        ];
        let incoming = vec![
            json!({"id": "Bonaca_Spur", "DOI": "10.1/A", "title": "Spur!", "volume": "3"}),
            json!({"author": [{"family": "Koposov"}], "issued": {"date-parts": [[2010]]}, "title": "GD-1"}),
            json!({"id": "dup", "author": [{"family": "Koposov"}], "title": "gd 1", "page": "9"}),
            json!({"id": "bonaca2020", "title": "Another paper"}),
        ];
        let m = merge(deck, incoming);
        let ids: Vec<&str> = m
            .entries
            .iter()
            .map(|e| e["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["bonaca2020", "old2001", "koposov2010", "bonaca2020a"]);
        assert_eq!(m.entries[0]["title"], "Spur!", "the source's fields win");
        assert_eq!(
            m.entries[0]["note"], "mine",
            "fields only the deck has stay"
        );
        assert_eq!(
            m.changed,
            vec![json!({"id": "bonaca2020", "fields": ["DOI", "title", "volume"]})]
        );
        assert_eq!(m.added, ["koposov2010", "bonaca2020a"]);
        assert_eq!(
            m.merged,
            vec![json!({"id": "koposov2010", "from": ["dup"]})]
        );
        assert_eq!(m.entries[2]["page"], "9");
        assert_eq!(m.gone, ["old2001"]);
        assert_eq!(
            m.warnings.len(),
            1,
            "the renamed key is warned about: {:?}",
            m.warnings
        );
    }

    fn item(last: &[&str], year: &str, title: &str, doi: &str, url: &str) -> Value {
        let creators: Vec<Value> = last
            .iter()
            .map(|n| json!({"creatorType": "author", "lastName": n}))
            .collect();
        json!({"itemType": "journalArticle", "creators": creators, "date": year,
               "title": title, "DOI": doi, "url": url})
    }

    #[test]
    fn labels_links_and_sorting() {
        let items = vec![
            item(&["Zed"], "2020-01-02", "Z", "10.1/z", ""),
            item(
                &["Bonaca", "Price-Whelan"],
                "2025",
                "Streams <i>in</i> &amp; \"Gaia\"",
                "10.1/b",
                "",
            ),
            item(
                &["Ab", "C", "D"],
                "March 2021",
                "Second",
                "10.48550/arXiv.1",
                "http://arxiv.org/abs/1",
            ),
            item(&["Ab", "X", "Y"], "2021", "First", "", ""),
            json!({"itemType": "computerProgram", "creators": [{"creatorType": "programmer", "name": "Astropy"}], "date": "", "title": "astropy", "url": "https://astropy.org"}),
        ];
        let (rows, warnings) = lines(&items);
        assert_eq!(
            rows,
            vec![
                r#"- {{< paper url="" who="Ab et al." year="2021a" title="First" >}}"#,
                r#"- {{< paper url="http://arxiv.org/abs/1" who="Ab et al." year="2021b" title="Second" >}}"#,
                r#"- {{< paper url="https://astropy.org" who="Astropy" year="n.d." title="astropy" >}}"#,
                r#"- {{< paper url="https://doi.org/10.1/b" who="Bonaca & Price-Whelan" year="2025" title="Streams in & 'Gaia'" >}}"#,
                r#"- {{< paper url="https://doi.org/10.1/z" who="Zed" year="2020" title="Z" >}}"#,
            ]
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("Ab et al. (2021a)"));
    }

    #[test]
    fn writes_between_the_markers_only() {
        let text = "## R {#refs}\r\n\r\n<!-- bibliography:start (by qd) -->\r\n- old\r\n<!-- bibliography:end -->\r\n\r\n## Next\r\n";
        let d = Deck::parse(text.to_string());
        let p = patch(&d, &["- a".into(), "- b".into()]).unwrap();
        let out = crate::ops::apply(text, &p).unwrap();
        assert_eq!(out, text.replace("- old\r\n", "- a\r\n- b\r\n"));
        let empty = crate::ops::apply(text, &patch(&d, &[]).unwrap()).unwrap();
        assert_eq!(empty, text.replace("- old\r\n", ""));
    }

    #[test]
    fn needs_exactly_one_marker_pair() {
        let one = "<!-- bibliography:start -->\n<!-- bibliography:end -->\n";
        let code = |t: &str| patch(&Deck::parse(t.to_string()), &[]).unwrap_err().code();
        assert_eq!(code("## R\n"), "E_NOT_FOUND");
        assert_eq!(code("<!-- bibliography:start -->\n"), "E_NOT_FOUND");
        assert_eq!(code(&one.repeat(2)), "E_AMBIGUOUS");
        assert_eq!(
            code("<!-- bibliography:end -->\n<!-- bibliography:start -->\n"),
            "E_NOT_FOUND"
        );
        assert!(patch(&Deck::parse(one.to_string()), &[]).is_ok());
    }

    #[test]
    fn rejects_keys_that_are_not_zotero_keys() {
        for k in ["", "ABC", "ABCD/EFG", "ABCDEFGHI"] {
            assert_eq!(fetch(k).unwrap_err().code(), "E_INVALID_ARG", "{k}");
        }
    }
}
