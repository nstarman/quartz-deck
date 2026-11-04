//! Edit operations. Pure functions: `&Deck` + args → `Patch` (span
//! replacements). Each op rewrites only the slide(s) it targets (FR-003);
//! any targeted slide without an explicit id gets one written (research R3).

use crate::attrs::{Attrs, valid_name};
use crate::deck::{BlockKind, Deck, Slide, SlideKind, Span};
use crate::error::{QdError, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
pub struct Patch {
    pub op: String,
    #[serde(skip)]
    pub replacements: Vec<(Span, String)>,
    pub assigned_ids: Vec<String>,
    pub target: String,
    /// A box's non-px position/size was rewritten in px (research R4).
    pub converted_units: bool,
}

/// Apply non-overlapping replacements (right to left).
pub fn apply(text: &str, p: &Patch) -> Result<String> {
    let mut reps = p.replacements.clone();
    reps.sort_by_key(|(s, _)| (s.start, s.end));
    for w in reps.windows(2) {
        if w[0].0.end > w[1].0.start {
            return Err(QdError::UnsafeEdit("overlapping replacements".into()));
        }
    }
    let mut out = text.to_string();
    for (span, new) in reps.iter().rev() {
        out.replace_range(span.start..span.end, new);
    }
    Ok(out)
}

pub fn diff(before: &str, after: &str) -> String {
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(2)
        .header("before", "after")
        .to_string()
}

#[derive(Clone, Copy, Debug)]
pub enum Pos<'a> {
    End,
    After(&'a str),
    Before(&'a str),
}

#[derive(Clone, Debug, Default)]
pub struct SlideSpec {
    pub title: Option<String>,
    pub body: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AttrChange {
    pub set: Vec<(String, String)>,
    pub unset: Vec<String>,
    pub add_class: Vec<String>,
    pub remove_class: Vec<String>,
}

impl AttrChange {
    fn validate(&self) -> Result<()> {
        let names = self
            .set
            .iter()
            .map(|(k, _)| k)
            .chain(&self.unset)
            .chain(&self.add_class)
            .chain(&self.remove_class);
        for n in names {
            if !valid_name(n) {
                return Err(QdError::InvalidArg(format!(
                    "'{n}' is not a valid attribute/class name"
                )));
            }
        }
        if self.set.iter().any(|(k, _)| k == "id") {
            return Err(QdError::InvalidArg(
                "ids are managed by qd; set them by editing the heading".into(),
            ));
        }
        Ok(())
    }

    fn apply_to(&self, a: &mut Attrs) {
        for (k, v) in &self.set {
            a.set(k, v);
        }
        for k in &self.unset {
            a.unset(k);
        }
        for c in &self.add_class {
            a.add_class(c);
        }
        for c in &self.remove_class {
            a.remove_class(c);
        }
    }
}

// ---------- helpers ----------

struct Ctx<'a> {
    d: &'a Deck,
    taken: HashSet<String>,
    salt: u32,
}

impl<'a> Ctx<'a> {
    fn new(d: &'a Deck) -> Self {
        let mut taken: HashSet<String> = d.slides.iter().map(|s| s.id.clone()).collect();
        taken.extend(
            d.slides
                .iter()
                .flat_map(|s| s.blocks.iter().map(|b| b.id.clone())),
        );
        Ctx { d, taken, salt: 0 }
    }

    /// Deterministic fresh id `<prefix>-xxxx` derived from the deck version.
    fn fresh(&mut self, prefix: &str) -> String {
        loop {
            self.salt += 1;
            let h = Sha256::digest(format!("{}:{prefix}:{}", self.d.version, self.salt));
            let id = format!("{prefix}-{:02x}{:02x}", h[0], h[1]);
            if self.taken.insert(id.clone()) {
                return id;
            }
        }
    }

    /// Id for a new slide: slug of its title (unique), else `s-xxxx`.
    fn slide_id(&mut self, title: Option<&str>) -> String {
        match title.filter(|t| !t.trim().is_empty()) {
            Some(t) => {
                let base = crate::deck::auto_identifier(t);
                let mut id = base.clone();
                let mut n = 1;
                while self.taken.contains(&id) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                self.taken.insert(id.clone());
                id
            }
            None => self.fresh("s"),
        }
    }

    fn nl(&self) -> &'static str {
        self.d.nl
    }

    fn slide(&self, r: &str) -> Result<&'a Slide> {
        let s = self.d.slide(r)?;
        if s.opaque {
            return Err(QdError::UnsafeEdit(format!(
                "slide '{}' contains a construct qd cannot edit safely (unterminated fence, or a slide heading inside a div)",
                s.id
            )));
        }
        Ok(s)
    }

    /// Rebuild a slide's full text. `title`: `Some(new)` to change it.
    /// `attrs_fn` mutates heading attrs; `rest` replaces the content after the heading.
    fn rewrite(
        &mut self,
        s: &Slide,
        title: Option<Option<String>>,
        change: Option<&AttrChange>,
        rest: Option<String>,
        assigned: &mut Vec<String>,
    ) -> (String, String) {
        let d = self.d;
        let nl = self.nl();
        let mut attrs = s.attrs.clone().unwrap_or_default();
        let id = if s.id_explicit {
            s.id.clone()
        } else {
            let id = match (&s.kind, &s.title) {
                (SlideKind::Heading | SlideKind::Section, Some(_)) => s.id.clone(),
                _ => self.fresh("s"),
            };
            attrs.set_id(&id);
            assigned.push(id.clone());
            id
        };
        if let Some(c) = change {
            c.apply_to(&mut attrs);
        }
        let new_title = title.clone().unwrap_or_else(|| s.title.clone());
        let rest = rest.unwrap_or_else(|| d.text[s.content_start()..s.span.end].to_string());
        let unchanged_head =
            s.id_explicit && title.is_none() && attrs == s.attrs.clone().unwrap_or_default();
        if unchanged_head && let Some(h) = s.head {
            return (format!("{}{rest}", d.slice(h)), id);
        }
        let level = match s.kind {
            SlideKind::Heading | SlideKind::Section => s.level,
            _ => d.slide_level,
        };
        let mut head = "#".repeat(level as usize);
        if let Some(t) = &new_title {
            head.push(' ');
            head.push_str(t);
        }
        if !attrs.is_empty() {
            head.push(' ');
            head.push_str(&attrs.render());
        }
        head.push_str(nl);
        if s.kind == SlideKind::Implicit {
            head.push_str(nl);
        }
        (head + &rest, id)
    }

    /// Insertion point and separator prefix for a new slide at `pos`.
    fn insertion(&self, pos: Pos) -> Result<usize> {
        let d = self.d;
        Ok(match pos {
            Pos::End => {
                if d.slides.last().is_some_and(|s| s.opaque) {
                    return Err(QdError::UnsafeEdit("the last slide has an unterminated construct; inserting at the end would be swallowed".into()));
                }
                d.text.len()
            }
            Pos::After(r) => self.slide(r)?.span.end,
            Pos::Before(r) => self.slide(r)?.span.start,
        })
    }

    /// Text to insert at `at` so that `slide_text` becomes its own slide.
    fn place(&self, at: usize, slide_text: &str) -> String {
        let nl = self.nl();
        let before = &self.d.text[..at];
        let core = slide_text.trim_end_matches(['\n', '\r']);
        let prefix = if before.is_empty() || before.ends_with(&format!("{nl}{nl}")) {
            ""
        } else if before.ends_with(nl) {
            nl
        } else {
            "\n\n"
        };
        let prefix = if prefix == "\n\n" {
            format!("{nl}{nl}")
        } else {
            prefix.to_string()
        };
        let suffix = if at == self.d.text.len() {
            nl.to_string()
        } else {
            format!("{nl}{nl}")
        };
        format!("{prefix}{core}{suffix}")
    }
}

fn single_line(s: &str, what: &str) -> Result<()> {
    if s.contains(['\n', '\r']) {
        return Err(QdError::InvalidArg(format!("{what} must be a single line")));
    }
    Ok(())
}

/// Normalize user markdown: trim surrounding blank lines, match the deck's line endings.
fn norm(md: &str, nl: &str) -> String {
    let md = md.replace("\r\n", "\n");
    let md = md.trim_matches('\n').trim_end();
    if nl == "\n" {
        md.to_string()
    } else {
        md.replace('\n', nl)
    }
}

fn patch(
    op: &str,
    target: String,
    replacements: Vec<(Span, String)>,
    assigned_ids: Vec<String>,
) -> Patch {
    Patch {
        op: op.into(),
        replacements,
        assigned_ids,
        target,
        converted_units: false,
    }
}

fn new_slide_text(ctx: &mut Ctx, spec: &SlideSpec, id: &str) -> Result<String> {
    let nl = ctx.nl();
    let mut attrs = Attrs::default();
    attrs.set_id(id);
    let change = AttrChange {
        set: spec.attrs.clone(),
        add_class: spec.classes.clone(),
        ..Default::default()
    };
    change.validate()?;
    change.apply_to(&mut attrs);
    let mut head = "#".repeat(ctx.d.slide_level as usize);
    if let Some(t) = spec.title.as_deref().filter(|t| !t.is_empty()) {
        single_line(t, "title")?;
        head = format!("{head} {t}");
    }
    let mut text = format!("{head} {}{nl}", attrs.render());
    if let Some(b) = spec
        .body
        .as_deref()
        .map(|b| norm(b, nl))
        .filter(|b| !b.is_empty())
    {
        text = format!("{text}{nl}{b}{nl}");
    }
    Ok(text)
}

// ---------- ops ----------

pub fn add(d: &Deck, spec: &SlideSpec, pos: Pos) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let at = ctx.insertion(pos)?;
    let id = ctx.slide_id(spec.title.as_deref());
    let text = new_slide_text(&mut ctx, spec, &id)?;
    let ins = ctx.place(at, &text);
    Ok(patch(
        "add",
        id.clone(),
        vec![(Span { start: at, end: at }, ins)],
        vec![id],
    ))
}

pub fn dup(d: &Deck, slide: &str, after: Option<&str>) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let at = ctx.insertion(Pos::After(after.unwrap_or(slide)))?;
    let mut assigned = Vec::new();
    let (src_text, _) = ctx.rewrite(s, None, None, None, &mut assigned);

    // copy: fresh slide id, fresh ids for blocks with explicit ids
    let new_id = ctx.slide_id(s.title.as_deref());
    let mut rest = String::new();
    let mut pos = s.content_start();
    for b in &s.blocks {
        rest.push_str(&d.text[pos..b.span.start]);
        let mut t = d.slice(b.span).to_string();
        if b.id_explicit {
            let nid = ctx.fresh(if b.kind == BlockKind::Box { "b" } else { "e" });
            t = t.replacen(&format!("#{}", b.id), &format!("#{nid}"), 1);
            assigned.push(nid);
        }
        rest.push_str(&t);
        pos = b.span.end;
    }
    rest.push_str(&d.text[pos..s.span.end]);
    let mut attrs = s.attrs.clone().unwrap_or_default();
    attrs.set_id(&new_id);
    let fake = Slide {
        attrs: Some(attrs),
        id_explicit: true,
        ..s.clone()
    };
    let change = AttrChange::default();
    let (copy, _) = ctx.rewrite(
        &fake,
        Some(s.title.clone()),
        Some(&change),
        Some(rest),
        &mut Vec::new(),
    );
    assigned.insert(0, new_id.clone());
    let mut reps = vec![(Span { start: at, end: at }, ctx.place(at, &copy))];
    if src_text != d.slice(s.span) {
        reps.push((s.span, src_text));
    }
    Ok(patch("dup", new_id, reps, assigned))
}

pub fn mv(d: &Deck, slide: &str, pos: Pos) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let target = match pos {
        Pos::After(r) | Pos::Before(r) => ctx.slide(r)?,
        Pos::End => return Err(QdError::InvalidArg("move needs --after or --before".into())),
    };
    if target.index == s.index {
        return Err(QdError::InvalidArg(
            "cannot move a slide relative to itself".into(),
        ));
    }
    let at = ctx.insertion(pos)?;
    let mut assigned = Vec::new();
    let (text, id) = ctx.rewrite(s, None, None, None, &mut assigned);
    let reps = if at == s.span.start || at == s.span.end {
        vec![(s.span, text)] // already there: only the id assignment
    } else {
        vec![
            (s.span, String::new()),
            (Span { start: at, end: at }, ctx.place(at, &text)),
        ]
    };
    Ok(patch("move", id, reps, assigned))
}

pub fn rm(d: &Deck, slide: &str) -> Result<Patch> {
    let ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    Ok(patch(
        "rm",
        s.id.clone(),
        vec![(s.span, String::new())],
        vec![],
    ))
}

/// `--expect-text`: refuse unless `current` still hashes to `expected`.
fn check_expect(what: &str, current: &str, expected: Option<&str>) -> Result<()> {
    let hash = crate::deck::version_of(current.as_bytes());
    match expected {
        Some(e) if e != hash => Err(QdError::StaleElement {
            what: what.into(),
            current_source: current.into(),
            current_hash: hash,
        }),
        _ => Ok(()),
    }
}

pub fn title(d: &Deck, slide: &str, text: &str, expect_text: Option<&str>) -> Result<Patch> {
    single_line(text, "title")?;
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    check_expect(
        &format!("the title of '{}'", s.id),
        s.title.as_deref().unwrap_or(""),
        expect_text,
    )?;
    let mut assigned = Vec::new();
    let t = text.trim();
    let (new, id) = ctx.rewrite(
        s,
        Some((!t.is_empty()).then(|| t.to_string())),
        None,
        None,
        &mut assigned,
    );
    Ok(patch("title", id, vec![(s.span, new)], assigned))
}

pub fn set(d: &Deck, slide: &str, change: &AttrChange) -> Result<Patch> {
    change.validate()?;
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let mut assigned = Vec::new();
    let (new, id) = ctx.rewrite(s, None, Some(change), None, &mut assigned);
    Ok(patch("set", id, vec![(s.span, new)], assigned))
}

/// Replace all non-box blocks of a slide with `md`; boxes are kept in order after it.
pub fn body(d: &Deck, slide: &str, md: &str) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let nl = ctx.nl();
    let region = &d.text[s.content_start()..s.span.end];
    let trailing = match s.blocks.last() {
        Some(b) => d.text[b.span.end..s.span.end].to_string(),
        None => region.to_string(),
    };
    let blank = format!("{nl}{nl}");
    let trailing = if s.span.end < d.text.len() && !trailing.contains(&blank) {
        // content must end in a blank line, or the next slide's heading would
        // run into it (Pandoc folds it into a list or paragraph)
        blank
    } else if trailing.is_empty() {
        nl.to_string()
    } else {
        trailing
    };
    let mut parts: Vec<String> = Vec::new();
    let md = norm(md, nl);
    if !md.is_empty() {
        parts.push(md);
    }
    parts.extend(
        s.blocks
            .iter()
            .filter(|b| b.kind == BlockKind::Box)
            .map(|b| d.slice(b.span).to_string()),
    );
    let rest = if parts.is_empty() {
        trailing
    } else {
        format!("{nl}{}{trailing}", parts.join(&format!("{nl}{nl}")))
    };
    let mut assigned = Vec::new();
    let (new, id) = ctx.rewrite(s, None, None, Some(rest), &mut assigned);
    Ok(patch("body", id, vec![(s.span, new)], assigned))
}

/// Replace one element's source. Only the element's span is written: the slide
/// heading is left as it is (no id pinning; editing a block can't change the
/// slide's id), so the file changes nowhere else (spec 003 FR-002).
pub fn edit(d: &Deck, element: &str, md: &str, expect_text: Option<&str>) -> Result<Patch> {
    let (_, b) = d.element(element)?;
    check_expect(&format!("element '{}'", b.id), d.slice(b.span), expect_text)?;
    let md = norm(md, d.nl);
    if md.is_empty() {
        return Err(QdError::InvalidArg(
            "replacement text is empty; to delete use `qd body` or edit the file".into(),
        ));
    }
    Ok(patch("edit", b.id.clone(), vec![(b.span, md)], Vec::new()))
}

// ---------- front matter ----------

/// Line range [first, end) of a top-level YAML key inside the front matter body.
fn key_extent(lines: &[&str], key: &str) -> Option<(usize, usize)> {
    let first = lines.iter().position(|l| {
        l.strip_prefix(key).is_some_and(|rest| {
            rest.starts_with(':') && (rest.len() == 1 || rest[1..].starts_with([' ', '\t']))
        })
    })?;
    let mut end = first + 1;
    while end < lines.len()
        && (lines[end].trim().is_empty() || lines[end].starts_with([' ', '\t', '-']))
    {
        end += 1;
    }
    while end > first + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    Some((first, end))
}

fn edit_front_matter(d: &Deck, key: &str, value: Option<&str>) -> Result<Patch> {
    if !valid_name(key) {
        return Err(QdError::InvalidArg(format!("'{key}' is not a valid key")));
    }
    let nl = d.nl;
    if let Some(v) = value {
        single_line(v, "value")?;
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&format!("{key}: {v}"))
            .map_err(|e| QdError::InvalidArg(format!("value is not valid YAML: {e}")))?;
    }
    let Some(fm) = d.front_matter else {
        let Some(v) = value else {
            return Err(QdError::NotFound(format!("front matter key '{key}'")));
        };
        let text = format!("---{nl}{key}: {v}{nl}---{nl}{nl}");
        return Ok(patch(
            "meta",
            key.into(),
            vec![(Span { start: 0, end: 0 }, text)],
            vec![],
        ));
    };
    let fm_text = d.slice(fm);
    let raw: Vec<&str> = fm_text.split_inclusive('\n').collect();
    let mut lines: Vec<String> = raw.iter().map(|l| l.to_string()).collect();
    let bare: Vec<&str> = raw
        .iter()
        .map(|l| l.trim_end_matches(['\n', '\r']))
        .collect();
    let inner = &bare[1..bare.len() - 1];
    let new_line = value.map(|v| format!("{key}: {v}{nl}"));
    match key_extent(inner, key) {
        Some((a, b)) => {
            lines.splice(a + 1..b + 1, new_line);
        }
        None => match new_line {
            Some(l) => lines.insert(lines.len() - 1, l),
            None => return Err(QdError::NotFound(format!("front matter key '{key}'"))),
        },
    }
    let new_fm: String = lines.concat();
    let inner_text = new_fm.split_inclusive('\n').skip(1).collect::<Vec<_>>();
    let inner_text = inner_text[..inner_text.len() - 1].concat();
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&inner_text).map_err(|e| {
        QdError::InvalidArg(format!("resulting front matter is not valid YAML: {e}"))
    })?;
    Ok(patch("meta", key.into(), vec![(fm, new_fm)], vec![]))
}

/// Set a top-level front matter key to a single-line YAML value.
pub fn meta_set(d: &Deck, key: &str, value: &str) -> Result<Patch> {
    edit_front_matter(d, key, Some(value))
}

pub fn meta_unset(d: &Deck, key: &str) -> Result<Patch> {
    edit_front_matter(d, key, None)
}

/// Text of a minimal reveal.js deck.
pub fn new_deck(title: Option<&str>) -> String {
    let t = title.unwrap_or("Untitled");
    let quoted = serde_json::to_string(t).unwrap();
    format!(
        "---\ntitle: {quoted}\nformat: revealjs\n---\n\n## First slide {{#first-slide}}\n\nHello.\n"
    )
}

// ---------- positioned boxes (US4, research R4) ----------

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Length {
    pub value: f64,
    pub unit: String,
}

impl Length {
    /// `100`, `100px`, `20%`, `1.5em`; a unitless number is px (Quarto).
    pub fn parse(s: &str) -> Option<Length> {
        let s = s.trim();
        let n = s
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .unwrap_or(s.len());
        let value: f64 = s[..n].parse().ok()?;
        let unit = if n == s.len() { "px" } else { &s[n..] };
        Some(Length {
            value,
            unit: unit.into(),
        })
    }

    fn px(&self) -> Option<f64> {
        (self.unit == "px").then_some(self.value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Geom {
    pub left: Option<Length>,
    pub top: Option<Length>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
}

pub fn geom(a: &Attrs) -> Geom {
    let g = |k| a.get(k).and_then(Length::parse);
    Geom {
        left: g("left"),
        top: g("top"),
        width: g("width"),
        height: g("height"),
        right: g("right"),
        bottom: g("bottom"),
    }
}

pub enum BoxContent<'a> {
    Text(&'a str),
    Image(&'a str),
}

fn px(v: f64) -> String {
    format!("{}", v.round() as i64)
}

fn check_size(w: f64, h: f64) -> Result<()> {
    if !(w >= 1.0 && h >= 1.0 && w.is_finite() && h.is_finite()) {
        return Err(QdError::InvalidArg(format!(
            "box size must be at least 1x1 px, got {w}x{h}"
        )));
    }
    Ok(())
}

/// Byte range of a box block's `{...}` attribute text (div: first line; image: trailing).
fn box_attrs_range(text: &str, image: bool) -> Option<(usize, usize)> {
    if image {
        let start = text.rfind("){")? + 1;
        Some((start, text.trim_end().len()))
    } else {
        let first = &text[..text.find(['\n', '\r']).unwrap_or(text.len())];
        Some((first.find('{')?, first.rfind('}')? + 1))
    }
}

/// Insert a positioned box at the end of a slide.
pub fn box_add(
    d: &Deck,
    slide: &str,
    content: BoxContent,
    at: Option<(f64, f64)>,
    size: Option<(f64, f64)>,
) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let nl = ctx.nl();
    let (w, h) = size.unwrap_or((300.0, 100.0));
    check_size(w, h)?;
    let (l, t) = at.unwrap_or(((d.width as f64 - w) / 2.0, (d.height as f64 - h) / 2.0));
    let id = ctx.fresh("b");
    let attrs = format!(
        "{{#{id} .absolute left={} top={} width={} height={}}}",
        px(l),
        px(t),
        px(w),
        px(h)
    );
    let block = match content {
        BoxContent::Text(md) => {
            let md = norm(md, nl);
            if md.is_empty() {
                return Err(QdError::InvalidArg("box text is empty".into()));
            }
            format!("::: {attrs}{nl}{md}{nl}:::")
        }
        BoxContent::Image(path) => {
            single_line(path, "image path")?;
            format!("![]({}){attrs}", path.trim())
        }
    };
    let (before, trailing) = match s.blocks.last() {
        Some(b) => (
            &d.text[s.content_start()..b.span.end],
            &d.text[b.span.end..s.span.end],
        ),
        None => ("", &d.text[s.content_start()..s.span.end]),
    };
    let trailing = if trailing.is_empty() { nl } else { trailing };
    let rest = format!("{before}{nl}{nl}{block}{trailing}");
    let rest = if s.blocks.is_empty() {
        format!("{nl}{block}{trailing}")
    } else {
        rest
    };
    let mut assigned = Vec::new();
    let (new, _) = ctx.rewrite(s, None, None, Some(rest), &mut assigned);
    assigned.push(id.clone());
    Ok(patch("box add", id, vec![(s.span, new)], assigned))
}

/// Edit a box's attributes; gives an id to boxes that lack one.
fn edit_box(
    d: &Deck,
    r: &str,
    op: &str,
    f: impl FnOnce(&mut Attrs) -> Result<bool>,
) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let (s, b) = d.element(r)?;
    if b.kind != BlockKind::Box {
        return Err(QdError::InvalidArg(format!(
            "'{r}' is a {:?} block, not a positioned (.absolute) box",
            b.kind
        )));
    }
    let s = ctx.slide(&format!("@{}", s.index))?;
    let text = d.slice(b.span);
    let image = text.trim_start().starts_with("![");
    let (a0, a1) = box_attrs_range(text, image)
        .ok_or_else(|| QdError::UnsafeEdit(format!("cannot locate attributes of '{r}'")))?;
    let mut attrs = Attrs::parse(&text[a0..a1])
        .ok_or_else(|| QdError::UnsafeEdit(format!("cannot parse attributes of '{r}'")))?;
    let mut assigned = Vec::new();
    let id = match attrs.id() {
        Some(id) => id.to_string(),
        None => {
            let id = ctx.fresh("b");
            attrs.set_id(&id);
            assigned.push(id.clone());
            id
        }
    };
    let converted = f(&mut attrs)?;
    let new_block = format!("{}{}{}", &text[..a0], attrs.render(), &text[a1..]);
    let rest = format!(
        "{}{new_block}{}",
        &d.text[s.content_start()..b.span.start],
        &d.text[b.span.end..s.span.end]
    );
    let (new, _) = ctx.rewrite(s, None, None, Some(rest), &mut assigned);
    let mut p = patch(op, id, vec![(s.span, new)], assigned);
    p.converted_units = converted;
    Ok(p)
}

/// Write px `left`/`top` (dropping `right`/`bottom`). Returns whether units changed.
fn set_position(a: &mut Attrs, l: f64, t: f64) -> bool {
    let g = geom(a);
    let converted = g.right.is_some()
        || g.bottom.is_some()
        || [&g.left, &g.top]
            .iter()
            .any(|v| v.as_ref().is_some_and(|v| v.unit != "px"));
    a.unset("right");
    a.unset("bottom");
    a.set("left", &px(l));
    a.set("top", &px(t));
    converted
}

/// Move a box to `at` (px), or by `by` (px; needs px `left`/`top`).
pub fn box_move(
    d: &Deck,
    r: &str,
    at: Option<(f64, f64)>,
    by: Option<(f64, f64)>,
) -> Result<Patch> {
    edit_box(d, r, "box move", |a| match (at, by) {
        (Some((l, t)), _) => Ok(set_position(a, l, t)),
        (None, Some((dx, dy))) => {
            let g = geom(a);
            let (Some(l), Some(t)) = (g.left.and_then(|v| v.px()), g.top.and_then(|v| v.px()))
            else {
                return Err(QdError::InvalidArg(
                    "--by needs the box to have px left/top; use --at".into(),
                ));
            };
            Ok(set_position(a, l + dx, t + dy))
        }
        (None, None) => Err(QdError::Usage("box move needs --at or --by".into())),
    })
}

/// Set a box's size (px), optionally moving it too (e.g. resizing from the top-left).
pub fn box_resize(d: &Deck, r: &str, size: (f64, f64), at: Option<(f64, f64)>) -> Result<Patch> {
    check_size(size.0, size.1)?;
    edit_box(d, r, "box resize", |a| {
        let g = geom(a);
        let mut converted = [&g.width, &g.height]
            .iter()
            .any(|v| v.as_ref().is_some_and(|v| v.unit != "px"));
        if let Some((l, t)) = at {
            converted |= set_position(a, l, t);
        }
        a.set("width", &px(size.0));
        a.set("height", &px(size.1));
        Ok(converted)
    })
}

pub fn box_rm(d: &Deck, r: &str) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let (s, b) = d.element(r)?;
    if b.kind != BlockKind::Box {
        return Err(QdError::InvalidArg(format!(
            "'{r}' is not a positioned box"
        )));
    }
    let s = ctx.slide(&format!("@{}", s.index))?;
    let i = s.blocks.iter().position(|x| x.span == b.span).unwrap();
    // drop the box and the blank lines that separate it from its neighbour
    let (cut0, cut1) = match (i.checked_sub(1).map(|j| &s.blocks[j]), s.blocks.get(i + 1)) {
        (_, Some(next)) => (b.span.start, next.span.start),
        (Some(prev), None) => (prev.span.end, b.span.end),
        (None, None) => (b.span.start, b.span.end),
    };
    let rest = format!(
        "{}{}",
        &d.text[s.content_start()..cut0],
        &d.text[cut1..s.span.end]
    );
    let mut assigned = Vec::new();
    let (new, _) = ctx.rewrite(s, None, None, Some(rest), &mut assigned);
    Ok(patch("box rm", b.id.clone(), vec![(s.span, new)], assigned))
}
