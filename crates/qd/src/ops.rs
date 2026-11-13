//! Edit operations. Pure functions: `&Deck` + args → `Patch` (span
//! replacements). Each op rewrites only the slide(s) it targets (FR-003);
//! any targeted slide without an explicit id gets one written (research R3).

use crate::attrs::{Attrs, valid_name};
use crate::deck::{Block, BlockKind, Deck, Slide, SlideKind, Span};
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

/// The change from `old` to `new` as line edits for VS Code: each
/// `{start, end, text}` replaces lines `[start, end)` of `old` (0-based)
/// with `text`; non-overlapping, in order. Lines, not offsets: VS Code
/// counts UTF-16 units, qd bytes, and lines mean the same to both.
pub fn line_edits(old: &str, new: &str) -> Vec<serde_json::Value> {
    let diff = similar::TextDiff::from_lines(old, new);
    let lines: Vec<&str> = diff.iter_new_slices().collect();
    diff.ops()
        .iter()
        .filter(|op| op.tag() != similar::DiffTag::Equal)
        .map(|op| {
            let (o, n) = (op.old_range(), op.new_range());
            serde_json::json!({"start": o.start, "end": o.end, "text": lines[n].concat()})
        })
        .collect()
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

    fn place(&self, at: usize, slide_text: &str) -> String {
        place(self.d, at, slide_text)
    }
}

/// Text to insert at `at` in `d` so that `slide_text` becomes its own slide.
pub(crate) fn place(d: &Deck, at: usize, slide_text: &str) -> String {
    let nl = d.nl;
    let before = &d.text[..at];
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
    let suffix = if at == d.text.len() {
        nl.to_string()
    } else {
        format!("{nl}{nl}")
    };
    format!("{prefix}{core}{suffix}")
}

fn single_line(s: &str, what: &str) -> Result<()> {
    if s.contains(['\n', '\r']) {
        return Err(QdError::InvalidArg(format!("{what} must be a single line")));
    }
    Ok(())
}

/// Normalize user markdown: trim surrounding blank lines, match the deck's line endings.
/// `md` with the deck's line endings.
fn with_nl(md: &str, nl: &str) -> String {
    let md = md.replace("\r\n", "\n");
    if nl == "\n" { md } else { md.replace('\n', nl) }
}

/// `md` trimmed of surrounding blank lines, with the deck's line endings.
fn norm(md: &str, nl: &str) -> String {
    with_nl(md.replace("\r\n", "\n").trim_matches('\n').trim_end(), nl)
}

pub(crate) fn patch(
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
    add_in(&mut Ctx::new(d), spec, pos)
}

/// `add` with `ctx`'s ids already taken (a layout's boxes).
fn add_in(ctx: &mut Ctx, spec: &SlideSpec, pos: Pos) -> Result<Patch> {
    let at = ctx.insertion(pos)?;
    let id = ctx.slide_id(spec.title.as_deref());
    let text = new_slide_text(ctx, spec, &id)?;
    let ins = ctx.place(at, &text);
    Ok(patch(
        "add",
        id.clone(),
        vec![(Span { start: at, end: at }, ins)],
        vec![id],
    ))
}

/// A new slide from a layout (spec 009 US1): its heading and boxes, each box
/// with a fresh id and scaled to the deck's slide size, placeholders empty
/// (their purpose comments are the layout's, not the deck's). Returns the patch
/// and each placeholder's place among the slide's blocks, id, kind and purpose.
pub fn add_layout(
    d: &Deck,
    l: &crate::layouts::Layout,
    title: Option<&str>,
    pos: Pos,
) -> Result<(Patch, Vec<serde_json::Value>)> {
    let src = &l.deck;
    let s = &src.slides[0];
    let mut ctx = Ctx::new(d);
    let nl = ctx.nl();
    let ph = l.placeholders();
    let (mut found, mut ids, mut parts) = (Vec::new(), Vec::new(), Vec::new());
    for b in &s.blocks {
        let marked = b
            .attrs
            .as_ref()
            .filter(|a| b.kind == BlockKind::Box && a.has_class("qd-ph"));
        let Some(a) = marked else {
            parts.push(match b.kind {
                BlockKind::Notes => format!("::: notes{nl}:::"),
                _ => src.slice(b.span).to_string(),
            });
            continue;
        };
        let (mut a, p) = (a.clone(), &ph[found.len()]);
        let id = ctx.fresh("b");
        a.set_id(&id);
        let r = crate::layouts::scale(p.rect, d.width, d.height);
        for (k, v) in ["left", "top", "width", "height"].into_iter().zip(r) {
            a.set(k, &px(v));
        }
        parts.push(format!("::: {}{nl}:::", a.render()));
        found.push(serde_json::json!({"index": parts.len() - 1, "id": id, "kind": p.kind, "purpose": p.purpose}));
        ids.push(id);
    }
    let head = s.attrs.clone().unwrap_or_default();
    let spec = SlideSpec {
        title: title.map(str::to_string),
        body: Some(parts.join(&format!("{nl}{nl}"))),
        attrs: head
            .kv()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        classes: head.classes().into_iter().map(str::to_string).collect(),
    };
    let mut p = add_in(&mut ctx, &spec, pos)?;
    p.assigned_ids.extend(ids);
    Ok((p, found))
}

/// A placeholder box filled (spec 009): its first line without the markers,
/// around `md` (or `md` itself, when it is the whole block with its fences).
fn fill_placeholder(old: &str, md: &str, nl: &str) -> Option<String> {
    let first = old.lines().next()?;
    let (pre, rest) = first.split_once('{')?;
    let mut a = Attrs::parse(&format!("{{{rest}"))?;
    if !a.has_class("qd-ph") {
        return None;
    }
    a.remove_class("qd-ph");
    a.unset("ph");
    let head = format!("{pre}{}", a.render());
    Some(match md.split_once(nl) {
        Some((line, body)) if line.trim_start().starts_with(":::") && line.contains("qd-ph") => {
            format!("{head}{nl}{body}")
        }
        _ if md.trim_start().starts_with(":::") => md.to_string(),
        _ => format!("{head}{nl}{md}{nl}{}", pre.trim_end()),
    })
}

pub fn dup(d: &Deck, slide: &str, after: Option<&str>) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let at = ctx.insertion(Pos::After(after.unwrap_or(slide)))?;
    let mut assigned = Vec::new();
    let (src_text, _) = ctx.rewrite(s, None, None, None, &mut assigned);

    // copy: fresh slide id, fresh ids for blocks with explicit ids
    let new_id = ctx.slide_id(s.title.as_deref());
    let rest = copy_body(&mut ctx, d, s, false, &mut assigned);
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

/// Insert slides given as markdown (a copy from this or another deck). An
/// explicit id already in the deck, or repeated in the paste, gets a fresh one.
pub fn paste(d: &Deck, md: &str, pos: Pos) -> Result<Patch> {
    let src = Deck::parse(md.replace("\r\n", "\n"));
    if src.slides.is_empty() || src.slides[0].kind == SlideKind::Implicit {
        return Err(QdError::Parse(
            "the pasted text does not start with a slide heading or ---".into(),
        ));
    }
    if let Some(s) = src.slides.iter().find(|s| s.opaque) {
        return Err(QdError::UnsafeEdit(format!(
            "pasted slide '{}' has an unterminated construct",
            s.id
        )));
    }
    let mut ctx = Ctx::new(d);
    let at = ctx.insertion(pos)?;
    let mut assigned = Vec::new();
    let mut text = String::new();
    let mut first = None;
    for s in &src.slides {
        let mut head = src.text[s.span.start..s.content_start()].to_string();
        let mut id = s.id.clone();
        if s.id_explicit && !ctx.taken.insert(id.clone()) {
            id = ctx.slide_id(s.title.as_deref());
            head = head.replacen(&format!("#{}", s.id), &format!("#{id}"), 1);
            assigned.push(id.clone());
        }
        first.get_or_insert(id);
        text.push_str(&head);
        text.push_str(&copy_body(&mut ctx, &src, s, true, &mut assigned));
    }
    let text = with_nl(&text, d.nl);
    Ok(patch(
        "paste",
        first.unwrap(),
        vec![(Span { start: at, end: at }, ctx.place(at, &text))],
        assigned,
    ))
}

/// Move one or more slides, kept in deck order, next to another (spec 008 US9:
/// several at once, one patch).
pub fn mv(d: &Deck, slides: &[impl AsRef<str>], pos: Pos) -> Result<Patch> {
    let mut ctx = Ctx::new(d);
    let moved = several(&ctx, slides)?;
    let target = match pos {
        Pos::After(r) | Pos::Before(r) => ctx.slide(r)?,
        Pos::End => return Err(QdError::InvalidArg("move needs --after or --before".into())),
    };
    if moved.iter().any(|s| s.index == target.index) {
        return Err(QdError::InvalidArg(
            "cannot move a slide relative to itself".into(),
        ));
    }
    let at = ctx.insertion(pos)?;
    let mut assigned = Vec::new();
    let texts: Vec<_> = moved
        .iter()
        .map(|s| ctx.rewrite(s, None, None, None, &mut assigned))
        .collect();
    let id = texts[0].1.clone();
    if let [s] = moved[..]
        && (at == s.span.start || at == s.span.end)
    {
        // already there: only the id assignment
        return Ok(patch(
            "move",
            id,
            vec![(s.span, texts[0].0.clone())],
            assigned,
        ));
    }
    let gap = format!("{0}{0}", ctx.nl());
    let joined = texts
        .iter()
        .map(|(t, _)| t.trim_end_matches(['\n', '\r']))
        .collect::<Vec<_>>()
        .join(&gap);
    let mut reps: Vec<_> = moved.iter().map(|s| (s.span, String::new())).collect();
    reps.push((Span { start: at, end: at }, ctx.place(at, &joined)));
    Ok(patch("move", id, reps, assigned))
}

/// Remove one or more slides, as one patch.
pub fn rm(d: &Deck, slides: &[impl AsRef<str>]) -> Result<Patch> {
    let ctx = Ctx::new(d);
    let gone = several(&ctx, slides)?;
    let reps = gone.iter().map(|s| (s.span, String::new())).collect();
    Ok(patch("rm", gone[0].id.clone(), reps, vec![]))
}

/// The slides `refs` name, each once, in deck order.
fn several<'d>(ctx: &Ctx<'d>, refs: &[impl AsRef<str>]) -> Result<Vec<&'d Slide>> {
    let mut v = refs
        .iter()
        .map(|r| ctx.slide(r.as_ref()))
        .collect::<Result<Vec<_>>>()?;
    v.sort_by_key(|s| s.index);
    v.dedup_by_key(|s| s.index);
    if v.is_empty() {
        return Err(QdError::InvalidArg("name at least one slide".into()));
    }
    Ok(v)
}

/// Source edits (`edit`, `body`) may not change the slide structure: refuse a
/// patch that adds or removes a slide (a slide-level heading or `---`) or
/// leaves slide `index` with an unclosed ``` / `:::` fence (spec 003 FR-008).
fn keep_structure(d: &Deck, p: Patch, index: usize) -> Result<Patch> {
    let after = Deck::parse(apply(&d.text, &p)?);
    if after.slides.len() != d.slides.len() {
        return Err(QdError::InvalidArg(
            "the text would start or remove a slide (a slide-level heading or `---`); add slides with `qd add`".into(),
        ));
    }
    // `index` is 1-based position, and both decks have the same slide count
    if after.slides[index - 1].opaque && !d.slides[index - 1].opaque {
        return Err(QdError::InvalidArg(
            "the text leaves a fence open (``` or :::); close it before saving".into(),
        ));
    }
    Ok(p)
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
    keep_structure(d, patch("body", id, vec![(s.span, new)], assigned), s.index)
}

/// Replace one element's source. Only the element's span is written: the slide
/// heading is left as it is (no id pinning; editing a block can't change the
/// slide's id), so the file changes nowhere else (spec 003 FR-002).
pub fn edit(d: &Deck, element: &str, md: &str, expect_text: Option<&str>) -> Result<Patch> {
    let (slide, b) = d.element(element)?;
    check_expect(&format!("element '{}'", b.id), d.slice(b.span), expect_text)?;
    let md = norm(md, d.nl);
    if md.is_empty() {
        return Err(QdError::InvalidArg(
            "replacement text is empty; to delete use `qd body` or edit the file".into(),
        ));
    }
    let md = fill_placeholder(d.slice(b.span), &md, d.nl).unwrap_or(md);
    keep_structure(
        d,
        patch("edit", b.id.clone(), vec![(b.span, md)], Vec::new()),
        slide.index,
    )
}

// ---------- single elements: qd block (spec 003 US4) ----------

/// Rewrite the run of blocks of `s` (first block to last) after `f` has
/// rearranged their texts and the separators between them. Bytes outside that
/// run — the heading, blank lines around it — are untouched.
fn rearrange(
    d: &Deck,
    s: &Slide,
    f: impl FnOnce(&mut Vec<String>, &mut Vec<String>),
) -> (Span, String) {
    let span = Span {
        start: s.blocks[0].span.start,
        end: s.blocks[s.blocks.len() - 1].span.end,
    };
    let mut texts: Vec<String> = s
        .blocks
        .iter()
        .map(|b| d.slice(b.span).to_string())
        .collect();
    let mut seps: Vec<String> = s
        .blocks
        .windows(2)
        .map(|w| d.text[w[0].span.end..w[1].span.start].to_string())
        .collect();
    f(&mut texts, &mut seps);
    let mut out = texts[0].clone(); // never empty: removing a slide's last block goes via `body`
    for (sep, t) in seps.iter().zip(&texts[1..]) {
        out += sep;
        out += t;
    }
    (span, out)
}

/// Element `r`, which must be on slide `s`; returns its index there.
fn on_slide(d: &Deck, s: &Slide, r: &str) -> Result<usize> {
    let (es, b) = d.element(r)?;
    if es.index != s.index {
        return Err(QdError::InvalidArg(format!(
            "'{r}' is on another slide; move elements between slides with cut and paste (`block rm` + `block add`)"
        )));
    }
    Ok(s.blocks.iter().position(|x| x.span == b.span).unwrap())
}

pub fn block_add(d: &Deck, slide: &str, at: Pos, md: &str) -> Result<Patch> {
    let ctx = Ctx::new(d);
    let s = ctx.slide(slide)?;
    let md = norm(md, d.nl);
    if md.is_empty() {
        return Err(QdError::InvalidArg("nothing to add".into()));
    }
    if s.blocks.is_empty() {
        // first content of an empty slide: `body` handles the blank lines
        let mut p = body(d, slide, &md)?;
        p.op = "block add".into();
        p.target = format!("{}/0", s.id);
        return Ok(p);
    }
    let i = match at {
        Pos::End => s.blocks.len(),
        Pos::After(r) => on_slide(d, s, r)? + 1,
        Pos::Before(r) => on_slide(d, s, r)?,
    };
    let blank = format!("{0}{0}", d.nl);
    let (span, new) = rearrange(d, s, |texts, seps| {
        texts.insert(i, md);
        seps.insert(i.min(seps.len()), blank);
    });
    keep_structure(
        d,
        patch(
            "block add",
            format!("{}/{i}", s.id),
            vec![(span, new)],
            Vec::new(),
        ),
        s.index,
    )
}

pub fn block_move(d: &Deck, element: &str, to: Pos) -> Result<Patch> {
    let (s, _) = d.element(element)?;
    let from = on_slide(d, s, element)?;
    let mut i = match to {
        Pos::Before(r) => on_slide(d, s, r)?,
        Pos::After(r) => on_slide(d, s, r)? + 1,
        Pos::End => s.blocks.len(),
    };
    if i > from {
        i -= 1; // the moving block no longer sits before its destination
    }
    let (span, new) = rearrange(d, s, |texts, _| {
        let t = texts.remove(from);
        texts.insert(i, t);
    });
    Ok(patch(
        "block move",
        format!("{}/{i}", s.id),
        vec![(span, new)],
        Vec::new(),
    ))
}

pub fn block_rm(d: &Deck, element: &str) -> Result<Patch> {
    let (s, b) = d.element(element)?;
    if s.blocks.len() == 1 {
        // the slide's only content: leave it empty, cleanly separated
        let mut p = if b.kind == BlockKind::Box {
            box_rm(d, element)?
        } else {
            body(d, &s.id, "")?
        };
        p.op = "block rm".into();
        p.target = b.id.clone();
        return Ok(p);
    }
    let i = on_slide(d, s, element)?;
    let (span, new) = rearrange(d, s, |texts, seps| {
        texts.remove(i);
        seps.remove(i.min(seps.len() - 1));
    });
    Ok(patch(
        "block rm",
        b.id.clone(),
        vec![(span, new)],
        Vec::new(),
    ))
}

/// A copy of slide `s`'s content (from deck `src`), explicit block ids renewed;
/// with `keep_free`, an id not yet in the deck is kept.
fn copy_body(
    ctx: &mut Ctx,
    src: &Deck,
    s: &Slide,
    keep_free: bool,
    assigned: &mut Vec<String>,
) -> String {
    let mut out = String::new();
    let mut pos = s.content_start();
    for b in &s.blocks {
        out.push_str(&src.text[pos..b.span.start]);
        let t = src.slice(b.span);
        if keep_free && b.id_explicit && ctx.taken.insert(b.id.clone()) {
            out.push_str(t);
        } else {
            out.push_str(&renew_id(ctx, b, t, assigned));
        }
        pos = b.span.end;
    }
    out.push_str(&src.text[pos..s.span.end]);
    out
}

/// A copy's text: an explicit id is replaced by a fresh one.
fn renew_id(ctx: &mut Ctx, b: &Block, text: &str, assigned: &mut Vec<String>) -> String {
    if !b.id_explicit {
        return text.to_string();
    }
    let nid = ctx.fresh(if b.kind == BlockKind::Box { "b" } else { "e" });
    let t = text.replacen(&format!("#{}", b.id), &format!("#{nid}"), 1);
    assigned.push(nid);
    t
}

pub fn block_dup(d: &Deck, element: &str) -> Result<Patch> {
    let (s, b) = d.element(element)?;
    let mut assigned = Vec::new();
    let copy = renew_id(&mut Ctx::new(d), b, d.slice(b.span), &mut assigned);
    let mut p = block_add(d, &s.id, Pos::After(element), &copy)?;
    p.op = "block dup".into();
    p.assigned_ids = assigned;
    Ok(p)
}

/// Add/remove classes on one element (spec 003 US3/US6: "Appears on click",
/// appearance). The class goes into the element's own attribute block (div
/// fence, heading, image/box), keeping untouched attributes byte-for-byte; an
/// element without one is wrapped in `::: {.class}`, and that wrapper goes
/// again when its last class is removed.
pub fn block_class(d: &Deck, element: &str, add: &[String], remove: &[String]) -> Result<Patch> {
    let (s, b) = d.element(element)?;
    let text = d.slice(b.span);
    let nl = d.nl;
    let lines: Vec<&str> = text.split(nl).collect();
    let fenced = lines[0].trim_start().starts_with(":::")
        && lines.len() > 1
        && lines[lines.len() - 1].trim().chars().all(|c| c == ':');
    // (line index, byte range of `{...}` in that line) of the element's attributes
    let found = if fenced {
        box_attrs_range(text, false).map(|(o, c)| (0, o, c))
    } else {
        let i = if b.kind == BlockKind::Heading {
            0
        } else {
            lines.len() - 1
        };
        let l = lines[i].trim_end();
        match (l.rfind('{'), l.ends_with('}')) {
            (Some(o), true) if Attrs::parse(&l[o..]).is_some() => Some((i, o, l.len())),
            _ => None,
        }
    };
    let change = |a: &mut Attrs| -> Result<()> {
        for c in remove {
            if !a.remove_class(c) {
                return Err(QdError::InvalidArg(format!(
                    "'{element}' has no class '{c}'"
                )));
            }
        }
        for c in add {
            a.add_class(c);
        }
        Ok(())
    };
    let new = match found {
        Some((i, o, c)) => {
            let mut a = Attrs::parse(&lines[i][o..c]).expect("checked above");
            change(&mut a)?;
            if a.items.is_empty() && fenced && lines.len() > 2 {
                // our own `::: {.x}` wrapper, now empty: unwrap it
                lines[1..lines.len() - 1].join(nl)
            } else {
                // splice the new `{...}` (or nothing, with the space before it) into the text
                let off: usize = lines[..i].iter().map(|l| l.len() + nl.len()).sum();
                let (left, right) = (&text[..off + o], &text[off + c..]);
                if a.items.is_empty() {
                    format!("{}{right}", left.trim_end())
                } else {
                    format!("{left}{}{right}", a.render())
                }
            }
        }
        None => {
            let mut a = Attrs::default();
            change(&mut a)?;
            if a.items.is_empty() {
                return Err(QdError::InvalidArg("nothing to change".into()));
            }
            format!("::: {}{nl}{text}{nl}:::", a.render())
        }
    };
    keep_structure(
        d,
        patch("block class", b.id.clone(), vec![(b.span, new)], Vec::new()),
        s.index,
    )
}

/// Replace the unique occurrence of `old` with `new` (spec 004: source-pane
/// saves, and a literal search-and-replace for agents). Not found means the
/// text changed elsewhere (`E_CONFLICT`); found twice needs more context
/// (`E_AMBIGUOUS`). Line endings follow the deck (a browser textarea sends `\n`).
pub fn replace(d: &Deck, old: &str, new: &str) -> Result<Patch> {
    let (old, new) = (with_nl(old, d.nl), with_nl(new, d.nl));
    if old.is_empty() {
        return Err(QdError::InvalidArg("the text to replace is empty".into()));
    }
    let mut hits = d.text.match_indices(&old).map(|(i, _)| i);
    let at = hits.next().ok_or_else(|| {
        QdError::Conflict(
            "the text to replace is not in the deck (it changed meanwhile?); re-read and retry"
                .into(),
        )
    })?;
    if hits.next().is_some() {
        return Err(QdError::Ambiguous(
            "the text to replace occurs more than once; include more surrounding text".into(),
        ));
    }
    let target = d
        .slides
        .iter()
        .find(|s| s.span.start <= at && at < s.span.end)
        .map_or("front-matter".to_string(), |s| s.id.clone());
    let span = Span {
        start: at,
        end: at + old.len(),
    };
    Ok(patch("replace", target, vec![(span, new)], Vec::new()))
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
    if let Some(v) = value {
        single_line(v, "value")?;
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&format!("{key}: {v}"))
            .map_err(|e| QdError::InvalidArg(format!("value is not valid YAML: {e}")))?;
    }
    replace_key(d, key, value.map(|v| format!("{key}: {v}{}", d.nl)))
}

/// Set a top-level key to any YAML value, written as a block (spec 014:
/// `references:`); the rest of the front matter is kept byte for byte.
pub fn meta_set_yaml(d: &Deck, key: &str, value: &serde_yaml_ng::Value) -> Result<Patch> {
    let mut m = serde_yaml_ng::Mapping::new();
    m.insert(key.into(), value.clone());
    let text = serde_yaml_ng::to_string(&m)
        .map_err(|e| QdError::InvalidArg(format!("cannot write YAML: {e}")))?;
    replace_key(d, key, Some(text.replace('\n', d.nl)))
}

/// Replace a top-level key's lines with `new` (whole lines, `key:` included),
/// add them at the end, or remove the key for `None`.
fn replace_key(d: &Deck, key: &str, new_line: Option<String>) -> Result<Patch> {
    let nl = d.nl;
    let Some(fm) = d.front_matter else {
        let Some(l) = new_line else {
            return Err(QdError::NotFound(format!("front matter key '{key}'")));
        };
        let text = format!("---{nl}{l}---{nl}{nl}");
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

    pub fn px(&self) -> Option<f64> {
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
    let i = on_slide(d, s, r)?;
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

const GEOM: [&str; 6] = ["left", "top", "width", "height", "right", "bottom"];

/// Take an element out of the flow (R4): an image gets `.absolute` and its
/// position in its own attributes; any other block is wrapped, bytes unchanged.
pub fn block_place(
    d: &Deck,
    r: &str,
    at: (f64, f64),
    width: Option<f64>,
    height: Option<f64>,
) -> Result<Patch> {
    let (_, b) = d.element(r)?;
    let refuse = |why: &str| Err(QdError::InvalidArg(format!("'{r}' can't be placed: {why}")));
    match b.kind {
        BlockKind::Box => return refuse("it is already placed freely"),
        BlockKind::Notes => return refuse("speaker notes aren't on the slide"),
        BlockKind::Div => {
            return refuse("it is a div (columns, callout…); place the elements inside it instead");
        }
        BlockKind::Pause => return refuse("a pause has nothing to show"),
        _ => {}
    }
    check_size(width.unwrap_or(1.0), height.unwrap_or(1.0))?;
    let mut ctx = Ctx::new(d);
    let id = ctx.fresh("b");
    let text = d.slice(b.span);
    let set = |a: &mut Attrs| {
        a.add_class("absolute");
        a.set("left", &px(at.0));
        a.set("top", &px(at.1));
        for (k, v) in [("width", width), ("height", height)] {
            if let Some(v) = v {
                a.set(k, &px(v));
            }
        }
    };
    // an image that already has a size keeps it inside a wrapper, so unplace can restore it
    let new = if b.kind == BlockKind::Image
        && b.attrs
            .as_ref()
            .is_none_or(|a| GEOM.iter().all(|k| a.get(k).is_none()))
    {
        let mut a = b.attrs.clone().unwrap_or_default();
        if a.id().is_none() {
            a.set_id(&id);
        }
        set(&mut a);
        let t = text.trim_end();
        let head = match b.attrs {
            Some(_) => box_attrs_range(text, true).map_or(t, |(a0, _)| &text[..a0]),
            None => t,
        };
        format!("{head}{}{}", a.render(), &text[t.len()..])
    } else {
        let mut a = Attrs::default();
        a.set_id(&id);
        set(&mut a);
        let nl = ctx.nl();
        format!("::: {}{nl}{text}{nl}:::", a.render())
    };
    let target = match b.attrs.as_ref().and_then(|a| a.id()) {
        Some(own) if b.kind == BlockKind::Image => own.to_string(),
        _ => id.clone(),
    };
    let assigned = if target == id { vec![id] } else { vec![] };
    Ok(patch("block place", target, vec![(b.span, new)], assigned))
}

/// Put a placed element back in the flow: exactly undoes `block_place`.
pub fn block_unplace(d: &Deck, r: &str) -> Result<Patch> {
    let (s, b) = d.element(r)?;
    let i = on_slide(d, s, r)?;
    if b.kind != BlockKind::Box {
        return Err(QdError::InvalidArg(format!("'{r}' is not placed freely")));
    }
    let text = d.slice(b.span);
    let image = text.trim_start().starts_with("![");
    let (a0, a1) = box_attrs_range(text, image)
        .ok_or_else(|| QdError::UnsafeEdit(format!("cannot locate attributes of '{r}'")))?;
    let mut a = Attrs::parse(&text[a0..a1])
        .ok_or_else(|| QdError::UnsafeEdit(format!("cannot parse attributes of '{r}'")))?;
    a.remove_class("absolute");
    for k in GEOM {
        a.unset(k);
    }
    if a.id().is_some_and(|id| id.starts_with("b-")) {
        a.items.retain(|i| !matches!(i, crate::attrs::Item::Id(_)));
    }
    let new = if image {
        let rest = if a.is_empty() {
            String::new()
        } else {
            a.render()
        };
        format!("{}{rest}{}", &text[..a0], &text[a1..])
    } else {
        if !a.is_empty() {
            return Err(QdError::InvalidArg(format!(
                "'{r}' has attributes besides its place ({}); edit them in the source",
                a.render()
            )));
        }
        let lines: Vec<&str> = text.split(d.nl).collect();
        lines[1..lines.len() - 1].join(d.nl)
    };
    Ok(patch(
        "block unplace",
        format!("{}/{i}", s.id),
        vec![(b.span, new)],
        Vec::new(),
    ))
}

/// Stack a box (R6): positioned elements stack in source order, later on top.
/// `front`/`back` move it after the slide's last box or before its first;
/// `forward`/`backward` past the next or previous box. Flow elements keep their order.
pub fn box_order(d: &Deck, r: &str, to: &str) -> Result<Patch> {
    let (s, b) = d.element(r)?;
    if b.kind != BlockKind::Box {
        return Err(QdError::InvalidArg(format!(
            "'{r}' is not placed freely; only placed elements stack"
        )));
    }
    let boxes: Vec<usize> = (0..s.blocks.len())
        .filter(|&j| s.blocks[j].kind == BlockKind::Box)
        .collect();
    let k = boxes
        .iter()
        .position(|&j| s.blocks[j].span == b.span)
        .unwrap();
    let refuse = |m: &str| Err(QdError::InvalidArg(format!("'{r}' is already {m}")));
    let (after, j) = match to {
        "front" | "forward" if k + 1 == boxes.len() => return refuse("in front"),
        "back" | "backward" if k == 0 => return refuse("at the back"),
        "front" => (true, boxes[boxes.len() - 1]),
        "forward" => (true, boxes[k + 1]),
        "back" => (false, boxes[0]),
        "backward" => (false, boxes[k - 1]),
        _ => unreachable!("box order {to}"),
    };
    let next = format!("{}/{j}", s.id);
    let pos = if after {
        Pos::After(&next)
    } else {
        Pos::Before(&next)
    };
    let mut p = block_move(d, r, pos)?;
    p.op = format!("box {to}");
    Ok(p)
}
