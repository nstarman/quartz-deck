//! Lossless segmenter: a `.qmd` reveal.js deck → slides → top-level blocks, each
//! with exact byte spans (research.md R1–R3). Head + slide spans tile the file,
//! so an unedited deck round-trips byte-for-byte by construction.

use crate::attrs::Attrs;
use crate::error::{QdError, Result};
use serde::Serialize;
use serde_yaml_ng::Value as Yaml;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SlideKind {
    /// Heading at `slide-level`.
    Heading,
    /// Heading above `slide-level` (e.g. `#` with slide-level 2): section/title slide.
    Section,
    /// Started by a horizontal rule (`---`).
    Rule,
    /// Content before the first heading/rule.
    Implicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockKind {
    Para,
    Heading,
    List,
    Code,
    Cell,
    Div,
    Image,
    Table,
    Quote,
    Html,
    Math,
    Pause,
    Notes,
    Box,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: String,
    pub id_explicit: bool,
    pub kind: BlockKind,
    pub attrs: Option<Attrs>,
    /// First line start .. end of last line (line terminator excluded).
    pub span: Span,
    /// A `.columns` or `.column` div's own blocks (spec 024); empty for any other block.
    pub children: Vec<Block>,
}

#[derive(Clone, Debug)]
pub struct Slide {
    pub id: String,
    pub id_explicit: bool,
    pub index: usize,
    pub kind: SlideKind,
    pub level: u8,
    pub title: Option<String>,
    pub attrs: Option<Attrs>,
    /// From the slide start up to the next slide start (or EOF).
    pub span: Span,
    /// Heading/rule line(s) incl. their line terminators; `None` for implicit slides.
    pub head: Option<Span>,
    pub setext: bool,
    pub blocks: Vec<Block>,
    /// Segmentation was unsure here (e.g. unterminated fence): structural edits are refused.
    pub opaque: bool,
    /// A group head: a section slide at the deck's `group_level` (spec 018).
    pub is_head: bool,
    /// The position in `Deck::slides` of the group head above, if any.
    pub parent: Option<usize>,
}

impl Slide {
    /// Byte offset where the slide's content (after the heading) begins.
    pub fn content_start(&self) -> usize {
        self.head.map_or(self.span.start, |h| h.end)
    }
}

#[derive(Clone, Debug)]
pub struct Deck {
    pub text: String,
    pub version: String,
    /// Line ending style of the file.
    pub nl: &'static str,
    /// Everything before the first slide (front matter + blank lines).
    pub head: Span,
    pub front_matter: Option<Span>,
    pub meta: Yaml,
    pub slide_level: u8,
    pub width: u32,
    pub height: u32,
    pub slides: Vec<Slide>,
    /// Byte offsets of the text's newlines: a position's line without counting from the start.
    newlines: Vec<usize>,
}

pub fn version_of(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

// ---------- line classification ----------

struct Line<'a> {
    start: usize,
    /// End of content, terminator excluded.
    end: usize,
    /// End including terminator.
    full_end: usize,
    text: &'a str,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut pos = 0;
    for raw in text.split_inclusive('\n') {
        let content = raw.strip_suffix('\n').unwrap_or(raw);
        let content = content.strip_suffix('\r').unwrap_or(content);
        out.push(Line {
            start: pos,
            end: pos + content.len(),
            full_end: pos + raw.len(),
            text: content,
        });
        pos += raw.len();
    }
    out
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start_matches(' ').len()
}

fn blank(l: &str) -> bool {
    l.trim().is_empty()
}

fn atx_level(l: &str) -> Option<u8> {
    if indent(l) > 3 {
        return None;
    }
    let t = l.trim_start();
    let n = t.chars().take_while(|&c| c == '#').count();
    let rest = &t[n..];
    ((1..=6).contains(&n) && (rest.is_empty() || rest.starts_with([' ', '\t']))).then_some(n as u8)
}

fn is_rule(l: &str) -> bool {
    if indent(l) > 3 {
        return false;
    }
    let chars: Vec<char> = l.chars().filter(|c| !c.is_whitespace()).collect();
    chars.len() >= 3 && matches!(chars[0], '-' | '*' | '_') && chars.iter().all(|&c| c == chars[0])
}

fn setext_level(l: &str) -> Option<u8> {
    if indent(l) > 3 {
        return None;
    }
    let t = l.trim();
    if !t.is_empty() && t.chars().all(|c| c == '=') {
        Some(1)
    } else if !t.is_empty() && t.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

/// Opening code fence at any indent: (fence char, length, info string).
pub(crate) fn code_open(l: &str) -> Option<(char, usize, &str)> {
    let t = l.trim_start();
    let ch = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = t.chars().take_while(|&c| c == ch).count();
    let info = &t[n..];
    (n >= 3 && !(ch == '`' && info.contains('`'))).then_some((ch, n, info))
}

fn code_close(l: &str, ch: char, n: usize) -> bool {
    let t = l.trim();
    t.len() >= n && t.chars().all(|c| c == ch)
}

/// Fenced div opener `::: {attrs}` / `::: class`; returns the text after the colons.
fn div_open(l: &str) -> Option<&str> {
    if indent(l) > 3 {
        return None;
    }
    let t = l.trim_start();
    let n = t.chars().take_while(|&c| c == ':').count();
    let rest = t[n..].trim().trim_end_matches(':').trim_end();
    (n >= 3 && !rest.is_empty()).then_some(rest)
}

fn div_close(l: &str) -> bool {
    let t = l.trim();
    indent(l) <= 3 && t.len() >= 3 && t.chars().all(|c| c == ':')
}

pub(crate) fn is_list_item(l: &str) -> bool {
    let t = l.trim_start();
    if t == "-"
        || t == "*"
        || t == "+"
        || t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
    {
        return true;
    }
    if t.starts_with("#. ") || t.starts_with("(@") {
        return true;
    }
    let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
    d > 0 && t[d..].starts_with([')', '.']) && t[d + 1..].starts_with([' ', '\t'])
}

fn setext_text_ok(l: &str) -> bool {
    let t = l.trim_start();
    indent(l) <= 3
        && !is_list_item(l)
        && !t.starts_with(['|', '>', '<', '+', ':', '!'])
        && !t.starts_with("{{<")
}

// ---------- top-level chunks ----------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Chunk {
    Heading { level: u8, setext: bool },
    Rule,
    Code { cell: bool },
    Div,
    Math,
    Comment,
    Html,
    Text,
}

struct Chunked {
    kind: Chunk,
    first: usize,
    last: usize, // inclusive line index
    opaque: bool,
}

fn skip_code(ls: &[Line], i: usize, ch: char, n: usize) -> (usize, bool) {
    for (j, l) in ls.iter().enumerate().skip(i + 1) {
        if code_close(l.text, ch, n) {
            return (j, false);
        }
    }
    (ls.len() - 1, true)
}

fn scan(ls: &[Line], from: usize, slide_level: u8) -> Vec<Chunked> {
    let mut out = Vec::new();
    let mut i = from;
    let n = ls.len();
    while i < n {
        let l = ls[i].text;
        if blank(l) {
            i += 1;
            continue;
        }
        let (kind, last, opaque) = if let Some(level) = atx_level(l) {
            (
                Chunk::Heading {
                    level,
                    setext: false,
                },
                i,
                false,
            )
        } else if is_rule(l) {
            (Chunk::Rule, i, false)
        } else if let Some((ch, cnt, info)) = code_open(l).filter(|_| indent(l) <= 3) {
            let (j, op) = skip_code(ls, i, ch, cnt);
            (
                Chunk::Code {
                    cell: info.trim_start().starts_with('{'),
                },
                j,
                op,
            )
        } else if div_open(l).is_some() {
            let (j, op) = scan_div(ls, i, slide_level);
            (Chunk::Div, j, op)
        } else if l.trim_start().starts_with("$$") {
            let t = l.trim();
            if t.len() > 2 && t.ends_with("$$") {
                (Chunk::Math, i, false)
            } else {
                match (i + 1..n).find(|&j| ls[j].text.trim_end().ends_with("$$")) {
                    Some(j) => (Chunk::Math, j, false),
                    None => (Chunk::Math, n - 1, true),
                }
            }
        } else if l.trim_start().starts_with("<!--") {
            match (i..n).find(|&j| ls[j].text.contains("-->")) {
                Some(j) => (Chunk::Comment, j, false),
                None => (Chunk::Comment, n - 1, true),
            }
        } else if i + 1 < n && setext_text_ok(l) && setext_level(ls[i + 1].text).is_some() {
            let level = setext_level(ls[i + 1].text).unwrap();
            (
                Chunk::Heading {
                    level,
                    setext: true,
                },
                i + 1,
                false,
            )
        } else {
            let html = l.trim_start().starts_with('<');
            let (j, op) = scan_text(ls, i);
            (if html { Chunk::Html } else { Chunk::Text }, j, op)
        };
        out.push(Chunked {
            kind,
            first: i,
            last,
            opaque,
        });
        i = last + 1;
    }
    out
}

/// Ends at the matching closer. Opaque if unterminated, or if it contains a
/// slide-level heading (Quarto splits a slide there, which qd does not model).
fn scan_div(ls: &[Line], i: usize, slide_level: u8) -> (usize, bool) {
    let mut splits = false;
    let mut depth = 1;
    let mut j = i + 1;
    while j < ls.len() {
        let l = ls[j].text;
        if let Some((ch, cnt, _)) = code_open(l) {
            let (k, op) = skip_code(ls, j, ch, cnt);
            if op {
                return (k, true);
            }
            j = k + 1;
            continue;
        }
        if atx_level(l).is_some_and(|lv| lv <= slide_level) {
            splits = true;
        }
        if div_open(l).is_some() {
            depth += 1;
        } else if div_close(l) {
            depth -= 1;
            if depth == 0 {
                return (j, splits);
            }
        }
        j += 1;
    }
    (ls.len() - 1, true)
}

/// Paragraph-like block until a blank line; lists continue across blank lines
/// while the next non-blank line is an item or indented continuation.
fn scan_text(ls: &[Line], i: usize) -> (usize, bool) {
    let list = is_list_item(ls[i].text);
    let mut j = i;
    loop {
        while j + 1 < ls.len() && !blank(ls[j + 1].text) {
            j += 1;
            if list && let Some((ch, cnt, _)) = code_open(ls[j].text) {
                let (k, op) = skip_code(ls, j, ch, cnt);
                if op {
                    return (k, true);
                }
                j = k;
            }
        }
        if !list {
            return (j, false);
        }
        let mut k = j + 1;
        while k < ls.len() && blank(ls[k].text) {
            k += 1;
        }
        if k < ls.len()
            && (is_list_item(ls[k].text) || indent(ls[k].text) >= 2)
            && atx_level(ls[k].text).is_none()
        {
            j = k;
            if let Some((ch, cnt, _)) = code_open(ls[j].text) {
                let (e, op) = skip_code(ls, j, ch, cnt);
                if op {
                    return (e, true);
                }
                j = e;
            }
            continue;
        }
        return (j, false);
    }
}

/// The positional ref of block `b` of slide `s`, `<slide>/<i>[/<j>…]`, as
/// the preview tags it.
pub fn positional(s: &Slide, b: &Block) -> String {
    fn find(blocks: &[Block], b: &Block, path: &mut String) -> bool {
        blocks.iter().enumerate().any(|(i, x)| {
            let len = path.len();
            path.push_str(&format!("/{i}"));
            let hit = std::ptr::eq(x, b) || find(&x.children, b, path);
            if !hit {
                path.truncate(len);
            }
            hit
        })
    }
    let mut path = s.id.clone();
    find(&s.blocks, b, &mut path);
    path
}

/// Blocks and every block inside them (spec 024), in source order.
pub fn walk(blocks: &[Block]) -> Box<dyn Iterator<Item = &Block> + '_> {
    Box::new(
        blocks
            .iter()
            .flat_map(|b| std::iter::once(b).chain(walk(&b.children))),
    )
}

/// The blocks of a slide (or of a columns or column div, spec 024) from its
/// chunks; a block's id is `{prefix}/{i}` unless it has an explicit one.
fn blocks_of<'c>(
    ls: &[Line],
    text: &str,
    chunks: impl IntoIterator<Item = &'c Chunked>,
    prefix: &str,
    slide_level: u8,
    used: &mut HashSet<String>,
) -> Vec<Block> {
    let mut out = Vec::new();
    for (i, c) in chunks.into_iter().enumerate() {
        let span = Span {
            start: ls[c.first].start,
            end: ls[c.last].end,
        };
        let (kind, attrs) = classify(c.kind, &text[span.start..span.end], used);
        let (id, id_explicit) = match attrs.as_ref().and_then(|a| a.id()) {
            Some(x) => (x.to_string(), true),
            None => (format!("{prefix}/{i}"), false),
        };
        let children = if kind == BlockKind::Div && !c.opaque && opens_up(ls[c.first].text) {
            // the lines between the fences, scanned as a slide's are
            let inner = scan(&ls[..c.last], c.first + 1, slide_level);
            blocks_of(ls, text, &inner, &id, slide_level, used)
        } else {
            Vec::new()
        };
        out.push(Block {
            id,
            id_explicit,
            kind,
            attrs,
            span,
            children,
        });
    }
    out
}

/// A `.columns` or `.column` div, whose contents are elements of their own.
pub(crate) fn opens_up(fence: &str) -> bool {
    div_attrs(fence).is_some_and(|a| a.has_class("columns") || a.has_class("column"))
}

/// A div fence's attributes: `{...}`, or `{.class}` for the shorthand `::: class`.
fn div_attrs(fence: &str) -> Option<Attrs> {
    let rest = div_open(fence).unwrap_or("");
    if rest.starts_with('{') {
        Attrs::parse(rest)
    } else {
        Attrs::parse(&format!(
            "{{.{}}}",
            rest.split_whitespace().next().unwrap_or("")
        ))
    }
}

// ---------- headings & ids ----------

/// Split heading text into (title, attrs). `text` excludes the `#` marker.
fn split_heading(text: &str, atx: bool) -> (Option<String>, Option<Attrs>) {
    let mut t = text.trim();
    let mut attrs = None;
    if t.ends_with('}')
        && let Some(pos) = t.rfind('{')
        && let Some(a) = Attrs::parse(&t[pos..])
    {
        attrs = Some(a);
        t = t[..pos].trim_end();
    }
    if atx && attrs.is_none() {
        let stripped = t.trim_end_matches('#');
        if stripped.len() < t.len() && (stripped.is_empty() || stripped.ends_with([' ', '\t'])) {
            t = stripped.trim_end();
        }
    }
    ((!t.is_empty()).then(|| t.to_string()), attrs)
}

/// Pandoc's auto-identifier algorithm (approximate: links reduced to their text).
pub fn auto_identifier(title: &str) -> String {
    let mut plain = String::new();
    let mut chars = title.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ']' && chars.peek() == Some(&'(') {
            for c in chars.by_ref() {
                if c == ')' {
                    break;
                }
            }
            continue;
        }
        plain.push(c);
    }
    let kept: String = plain
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '_' | '-' | '.'))
        .flat_map(char::to_lowercase)
        .collect();
    let mut id = kept.split_whitespace().collect::<Vec<_>>().join("-");
    let first_letter = id
        .char_indices()
        .find(|(_, c)| c.is_alphabetic())
        .map(|(i, _)| i);
    id = first_letter.map_or(String::new(), |i| id[i..].to_string());
    if id.is_empty() { "section".into() } else { id }
}

fn yaml_u64(meta: &Yaml, key: &str) -> Option<u64> {
    // under `format: revealjs:` wins, as in Quarto
    (|| meta.get("format")?.get("revealjs")?.get(key)?.as_u64())()
        .or_else(|| meta.get(key).and_then(Yaml::as_u64))
}

// ---------- deck ----------

impl Deck {
    /// 1-based line range `[first, last]` of a span (spec 004: source ↔ slide sync).
    pub fn lines(&self, span: Span) -> [usize; 2] {
        // newlines before `pos`
        let line = |pos: usize| self.newlines.partition_point(|&n| n < pos) + 1;
        [line(span.start), line(span.end.max(span.start + 1) - 1)]
    }

    /// Every block of every slide, in order.
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.slides.iter().flat_map(|s| &s.blocks)
    }

    pub fn parse(text: String) -> Deck {
        let ls = lines(&text);
        let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };

        let mut front_matter = None;
        let mut meta = Yaml::Null;
        let mut body_line = 0;
        // a byte-order mark (some Windows editors write one) comes before the opening ---
        if ls
            .first()
            .is_some_and(|l| l.text.trim_start_matches('\u{feff}').trim_end() == "---")
            && let Some(close) =
                (1..ls.len()).find(|&j| matches!(ls[j].text.trim_end(), "---" | "..."))
        {
            front_matter = Some(Span {
                start: 0,
                end: ls[close].full_end,
            });
            let inner = &text[ls[0].full_end..ls[close].start];
            meta = serde_yaml_ng::from_str(inner).unwrap_or(Yaml::Null);
            body_line = close + 1;
        }
        let slide_level = yaml_u64(&meta, "slide-level").map_or(2, |v| v.clamp(1, 6) as u8);
        let width = yaml_u64(&meta, "width").map_or(1050, |v| v as u32);
        let height = yaml_u64(&meta, "height").map_or(700, |v| v as u32);

        let chunks = scan(&ls, body_line, slide_level);
        let starts_slide = |c: &Chunked| {
            matches!(c.kind, Chunk::Rule)
                || matches!(c.kind, Chunk::Heading { level, .. } if level <= slide_level)
        };

        // group chunks into slides: (start chunk index or None for implicit, member chunk indices)
        let mut groups: Vec<(Option<usize>, Vec<usize>)> = Vec::new();
        for (ci, c) in chunks.iter().enumerate() {
            if starts_slide(c) {
                groups.push((Some(ci), Vec::new()));
            } else {
                match groups.last_mut() {
                    Some(g) => g.1.push(ci),
                    None => groups.push((None, vec![ci])),
                }
            }
        }

        let head_end = groups
            .first()
            .map(|(s, m)| ls[chunks[s.unwrap_or_else(|| m[0])].first].start)
            .unwrap_or(text.len());

        let mut used: HashSet<String> = HashSet::new();
        // explicit ids reserve names first
        for c in &chunks {
            if let Chunk::Heading { setext, .. } = c.kind {
                let raw = ls[c.first].text;
                let body = if setext {
                    raw
                } else {
                    raw.trim_start().trim_start_matches('#')
                };
                if let (_, Some(a)) = split_heading(body, !setext)
                    && let Some(id) = a.id()
                {
                    used.insert(id.to_string());
                }
            }
        }

        let mut slides = Vec::with_capacity(groups.len());
        for (gi, (start, members)) in groups.iter().enumerate() {
            let first_chunk = start.unwrap_or_else(|| members[0]);
            let span_start = ls[chunks[first_chunk].first].start;
            let span_end = groups
                .get(gi + 1)
                .map(|(s, m)| ls[chunks[s.unwrap_or_else(|| m[0])].first].start)
                .unwrap_or(text.len());
            let index = gi + 1;
            let (kind, level, title, attrs, head, setext) = match start.map(|s| &chunks[s]) {
                None => (SlideKind::Implicit, slide_level, None, None, None, false),
                Some(c) => {
                    let head = Some(Span {
                        start: ls[c.first].start,
                        end: ls[c.last].full_end,
                    });
                    match c.kind {
                        Chunk::Rule => (SlideKind::Rule, slide_level, None, None, head, false),
                        Chunk::Heading { level, setext } => {
                            let raw = ls[c.first].text;
                            let body = if setext {
                                raw
                            } else {
                                raw.trim_start().trim_start_matches('#')
                            };
                            let (title, attrs) = split_heading(body, !setext);
                            let kind = if level < slide_level {
                                SlideKind::Section
                            } else {
                                SlideKind::Heading
                            };
                            (kind, level, title, attrs, head, setext)
                        }
                        _ => unreachable!(),
                    }
                }
            };
            let (id, id_explicit) = match attrs.as_ref().and_then(|a| a.id()) {
                Some(id) => (id.to_string(), true),
                None => match &title {
                    Some(t) => (unique(&mut used, auto_identifier(t)), false),
                    None if start
                        .is_some_and(|s| matches!(chunks[s].kind, Chunk::Heading { .. })) =>
                    {
                        (unique(&mut used, "section".into()), false)
                    }
                    None => (format!("@{index}"), false),
                },
            };
            let opaque = start.is_some_and(|s| chunks[s].opaque)
                || members.iter().any(|&ci| chunks[ci].opaque);
            let blocks = blocks_of(
                &ls,
                &text,
                members.iter().map(|&ci| &chunks[ci]),
                &id,
                slide_level,
                &mut used,
            );
            slides.push(Slide {
                id,
                id_explicit,
                index,
                kind,
                level,
                title,
                attrs,
                span: Span {
                    start: span_start,
                    end: span_end,
                },
                head,
                setext,
                blocks,
                opaque,
                is_head: false,
                parent: None,
            });
        }
        group(&mut slides, slide_level);

        Deck {
            version: version_of(text.as_bytes()),
            nl,
            head: Span {
                start: 0,
                end: head_end,
            },
            front_matter,
            meta,
            slide_level,
            width,
            height,
            slides,
            newlines: text.match_indices('\n').map(|(i, _)| i).collect(),
            text,
        }
    }

    pub fn slice(&self, s: Span) -> &str {
        &self.text[s.start..s.end]
    }

    /// The heading level of group heads (spec 018, research R1): the smallest
    /// section level present, else the level above `slide-level`; `None` at
    /// `slide-level: 1`, where every `#` is a plain slide.
    pub fn group_level(&self) -> Option<u8> {
        group_level(&self.slides, self.slide_level)
    }

    /// How the presentation moves through slides: Quarto's `navigation-mode`
    /// (`linear`, `vertical`, `grid`), the one under `format: revealjs:` over a
    /// top-level one; `linear`, Quarto's default, when the deck has none.
    pub fn navigation_mode(&self) -> &str {
        let key = "navigation-mode";
        (self
            .meta
            .get("format")
            .and_then(|f| f.get("revealjs")?.get(key)))
        .or_else(|| self.meta.get(key))
        .and_then(Yaml::as_str)
        .unwrap_or("linear")
    }

    pub fn has_title_slide(&self) -> bool {
        self.meta.get("title").is_some()
    }

    /// Resolve a slide ref: explicit id, auto-id, `@N`, `first`, `last` (a leading `#` is ignored).
    pub fn slide(&self, r: &str) -> Result<&Slide> {
        let r = r.strip_prefix('#').unwrap_or(r);
        let found: Vec<&Slide> = match r {
            "first" => self.slides.first().into_iter().collect(),
            "last" => self.slides.last().into_iter().collect(),
            _ if r.starts_with('@') => {
                let n: usize = r[1..]
                    .parse()
                    .map_err(|_| QdError::InvalidArg(format!("bad position ref '{r}'")))?;
                self.slides.get(n.wrapping_sub(1)).into_iter().collect()
            }
            _ => self.slides.iter().filter(|s| s.id == r).collect(),
        };
        match found.as_slice() {
            [s] => Ok(s),
            [] => Err(QdError::NotFound(format!("slide '{r}'"))),
            _ => Err(QdError::Ambiguous(format!(
                "{} slides have id '{r}'",
                found.len()
            ))),
        }
    }

    /// Resolve an element ref: `<slide>/<i>`, `<slide>/<i>/<j>…` inside
    /// columns (spec 024), or `#id` / `id` of a block at any depth.
    pub fn element(&self, r: &str) -> Result<(&Slide, &Block)> {
        if let Some((s, path)) = r.split_once('/') {
            let slide = self.slide(s)?;
            let (mut blocks, mut found) = (&slide.blocks, None);
            for i in path.split('/') {
                let i: usize = i
                    .parse()
                    .map_err(|_| QdError::InvalidArg(format!("bad element index in '{r}'")))?;
                let b = blocks
                    .get(i)
                    .ok_or_else(|| QdError::NotFound(format!("element '{r}'")))?;
                (blocks, found) = (&b.children, Some(b));
            }
            return Ok((slide, found.expect("a path has one index at least")));
        }
        let id = r.strip_prefix('#').unwrap_or(r);
        let found: Vec<(&Slide, &Block)> = self
            .slides
            .iter()
            .flat_map(|s| {
                walk(&s.blocks)
                    .filter(|b| b.id_explicit && b.id == id)
                    .map(move |b| (s, b))
            })
            .collect();
        match found.as_slice() {
            [x] => Ok(*x),
            [] => Err(QdError::NotFound(format!("element '{r}'"))),
            _ => Err(QdError::Ambiguous(format!(
                "{} elements have id '{id}'",
                found.len()
            ))),
        }
    }

    /// Where element `r` sits (spec 025): its slide, the block holding it
    /// (none at the top level), the list it is in and its index there.
    pub fn siblings(&self, r: &str) -> Result<(&Slide, Option<&Block>, &[Block], usize)> {
        type Found<'a> = (Option<&'a Block>, &'a [Block], usize);
        fn find<'a>(list: &'a [Block], parent: Option<&'a Block>, b: &Block) -> Option<Found<'a>> {
            list.iter().enumerate().find_map(|(i, x)| {
                if std::ptr::eq(x, b) {
                    Some((parent, list, i))
                } else {
                    find(&x.children, Some(x), b)
                }
            })
        }
        let (s, b) = self.element(r)?;
        let (parent, list, i) = find(&s.blocks, None, b).expect("an element is in its slide");
        Ok((s, parent, list, i))
    }

    /// `element`, refused when it is inside columns, for an op that
    /// rearranges a slide's blocks (`verb` it, spec 024).
    pub fn top_element(&self, r: &str, verb: &str) -> Result<(&Slide, &Block)> {
        let (s, b) = self.element(r)?;
        if !s.blocks.iter().any(|x| std::ptr::eq(x, b)) {
            return Err(QdError::InvalidArg(format!(
                "'{r}' is inside columns: {verb} it in the source for now"
            )));
        }
        Ok((s, b))
    }

    /// Every explicit id in the deck (slides and blocks).
    pub fn explicit_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        for s in &self.slides {
            if s.id_explicit {
                ids.push(s.id.as_str());
            }
            ids.extend(
                walk(&s.blocks)
                    .filter(|b| b.id_explicit)
                    .map(|b| b.id.as_str()),
            );
        }
        ids
    }

    /// Human-readable segmentation summary (used by snapshot tests).
    pub fn outline(&self) -> String {
        let mut out = format!(
            "slide_level={} size={}x{} front_matter={} title_slide={} nl={:?}\n",
            self.slide_level,
            self.width,
            self.height,
            self.front_matter.is_some(),
            self.has_title_slide(),
            self.nl
        );
        for s in &self.slides {
            out += &format!(
                "@{} {}{} {:?} L{} title={:?}{}{}\n",
                s.index,
                s.id,
                if s.id_explicit { "" } else { " (auto)" },
                s.kind,
                s.level,
                s.title,
                s.attrs
                    .as_ref()
                    .map(|a| format!(" attrs={}", a.render()))
                    .unwrap_or_default(),
                if s.opaque { " OPAQUE" } else { "" }
            );
            for b in &s.blocks {
                let first = self.slice(b.span).lines().next().unwrap_or("");
                let first: String = first.chars().take(50).collect();
                out += &format!("    {} {:?} {:?}\n", b.id, b.kind, first);
            }
        }
        out
    }
}

fn unique(used: &mut HashSet<String>, base: String) -> String {
    let mut id = base.clone();
    let mut n = 1;
    while used.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    used.insert(id.clone());
    id
}

fn classify(chunk: Chunk, text: &str, used: &mut HashSet<String>) -> (BlockKind, Option<Attrs>) {
    let first = text.lines().next().unwrap_or("").trim();
    match chunk {
        // ```{=html} etc.: a raw block, not an executable cell
        Chunk::Code { cell: true }
            if first
                .trim_start_matches(['`', '~'])
                .trim_start()
                .starts_with("{=") =>
        {
            (BlockKind::Html, None)
        }
        Chunk::Code { cell: true } => (BlockKind::Cell, None),
        Chunk::Code { cell: false } => (BlockKind::Code, None),
        Chunk::Math => (BlockKind::Math, None),
        Chunk::Comment | Chunk::Html => (BlockKind::Html, None),
        Chunk::Heading { setext, .. } => {
            let body = if setext {
                first
            } else {
                first.trim_start_matches('#')
            };
            let (title, attrs) = split_heading(body, !setext);
            if attrs.as_ref().and_then(|a| a.id()).is_none()
                && let Some(t) = title
            {
                unique(used, auto_identifier(&t));
            }
            (BlockKind::Heading, attrs)
        }
        Chunk::Div => {
            let rest = div_open(first).unwrap_or("");
            let attrs = div_attrs(first);
            let kind = match &attrs {
                Some(a) if a.has_class("absolute") => BlockKind::Box,
                Some(a) if a.has_class("notes") => BlockKind::Notes,
                _ => BlockKind::Div,
            };
            // shorthand `::: notes` attrs are synthetic: don't expose a fake original text
            (kind, if rest.starts_with('{') { attrs } else { None })
        }
        Chunk::Rule => (BlockKind::Para, None),
        Chunk::Text => {
            if first == ". . ." {
                (BlockKind::Pause, None)
            } else if first.starts_with('|') || first.starts_with("+-") {
                (BlockKind::Table, None)
            } else if first.starts_with('>') {
                (BlockKind::Quote, None)
            } else if is_list_item(first) {
                (BlockKind::List, None)
            } else if text.trim().starts_with("![") && !text.trim().contains('\n') {
                let t = text.trim();
                let attrs = t.rfind("){").and_then(|p| Attrs::parse(&t[p + 1..]));
                let kind = if attrs.as_ref().is_some_and(|a| a.has_class("absolute")) {
                    BlockKind::Box
                } else {
                    BlockKind::Image
                };
                (kind, attrs)
            } else {
                (BlockKind::Para, None)
            }
        }
    }
}

fn group_level(slides: &[Slide], slide_level: u8) -> Option<u8> {
    slides
        .iter()
        .filter(|s| s.kind == SlideKind::Section)
        .map(|s| s.level)
        .min()
        .or((slide_level > 1).then(|| slide_level - 1))
}

/// Mark the heads and give every slide after one its head, as Quarto stacks them.
fn group(slides: &mut [Slide], slide_level: u8) {
    let Some(top) = group_level(slides, slide_level) else {
        return;
    };
    let mut head = None;
    for (i, s) in slides.iter_mut().enumerate() {
        s.is_head = s.kind == SlideKind::Section && s.level == top;
        if s.is_head {
            head = Some(i);
        } else {
            s.parent = head;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(s: &str) -> Deck {
        Deck::parse(s.to_string())
    }

    fn ids(d: &Deck) -> Vec<&str> {
        d.slides.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn front_matter_after_a_byte_order_mark() {
        let d = deck("\u{feff}---\ntitle: T\n---\n\n## A\n\na\n");
        assert!(d.front_matter.is_some());
        assert_eq!(ids(&d), vec!["a"]);
    }

    #[test]
    fn lines_of_text_ending_in_non_ascii() {
        let d = deck("## A\n\nCafé\n\n## B\n\n→");
        for s in &d.slides {
            d.lines(s.span);
        }
        let b = d.blocks().last().unwrap();
        assert_eq!(d.lines(b.span), [7, 7]);
    }

    #[test]
    fn headings_and_rules_start_slides() {
        let d = deck("---\ntitle: T\n---\n\n## A\n\na\n\n---\n\nrule\n\n## B {#bee}\n\nb\n");
        assert_eq!(ids(&d), vec!["a", "@2", "bee"]);
        assert_eq!(d.slides[1].kind, SlideKind::Rule);
        assert!(d.slides[2].id_explicit);
        assert!(d.has_title_slide());
    }

    /// Each slide's id, `H` for a head, and its head's id (spec 018, research R1).
    fn groups(d: &Deck) -> Vec<String> {
        d.slides
            .iter()
            .map(|s| match (s.is_head, s.parent) {
                (true, _) => format!("{}:H", s.id),
                (false, Some(p)) => format!("{}<{}", s.id, d.slides[p].id),
                (false, None) => s.id.clone(),
            })
            .collect()
    }

    #[test]
    fn groups_follow_top_level_sections() {
        let d = deck("## Intro\n\n# A\n\n## A1\n\n---\n\nruled\n\n# B\n\n# C\n\n## C1\n");
        assert_eq!(d.group_level(), Some(1));
        assert_eq!(
            groups(&d),
            ["intro", "a:H", "a1<a", "@4<a", "b:H", "c:H", "c1<c"]
        );
    }

    #[test]
    fn group_level_without_sections() {
        let d = deck("## A\n\n## B\n");
        assert_eq!(d.group_level(), Some(1));
        assert_eq!(groups(&d), ["a", "b"]);
        let d = deck("---\nslide-level: 1\n---\n\n# A\n\n# B\n");
        assert_eq!(d.group_level(), None);
        assert_eq!(groups(&d), ["a", "b"]);
    }

    #[test]
    fn groups_at_deeper_slide_levels() {
        // the stacks are the smallest section level present (checked with Quarto 1.11)
        let lv3 = "---\nslide-level: 3\n---\n\n";
        let d = deck(&format!("{lv3}# P\n\n## S\n\n### s1\n\n# Q\n\n### q1\n"));
        assert_eq!(d.group_level(), Some(1));
        assert_eq!(groups(&d), ["p:H", "s<p", "s1<p", "q:H", "q1<q"]);
        let d = deck(&format!("{lv3}## P\n\n### p1\n\n## Q\n"));
        assert_eq!(d.group_level(), Some(2));
        assert_eq!(groups(&d), ["p:H", "p1<p", "q:H"]);
    }

    #[test]
    fn implicit_first_slide_is_top_level() {
        let d = deck("---\nformat: revealjs\n---\n\nPreamble.\n\n# A\n\n## a\n");
        assert_eq!(groups(&d), ["@1", "a:H", "a-1<a"]);
    }

    #[test]
    fn level_one_is_section() {
        let d = deck("# Part\n\n## S\n\nx\n");
        assert_eq!(d.slides[0].kind, SlideKind::Section);
        assert_eq!(d.slides[1].kind, SlideKind::Heading);
    }

    #[test]
    fn hashes_inside_code_div_html_are_not_slides() {
        let d = deck(
            "## A\n\n```python\n## no\n```\n\n::: {.x}\n## no\n:::\n\n<!--\n\n## no\n-->\n\n## B\n",
        );
        assert_eq!(ids(&d), vec!["a", "b"]);
        assert_eq!(d.slides[0].blocks.len(), 3);
    }

    #[test]
    fn nested_divs_close_by_depth() {
        let d = deck(
            "## A\n\n:::: {.columns}\n::: {.column}\nx\n:::\n::: {.column}\ny\n:::\n::::\n\nafter\n",
        );
        let kinds: Vec<_> = d.slides[0].blocks.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, vec![BlockKind::Div, BlockKind::Para]);
    }

    #[test]
    fn unterminated_fence_is_opaque() {
        let d = deck("## A\n\nok\n\n## B\n\n```\ncode\n\n## C\n");
        assert_eq!(ids(&d), vec!["a", "b"]);
        assert!(!d.slides[0].opaque);
        assert!(d.slides[1].opaque);
    }

    #[test]
    fn slide_level_from_front_matter() {
        let d = deck("---\nslide-level: 3\n---\n\n## S\n\n### X\n\n#### sub\n\n### Y\n");
        assert_eq!(d.slide_level, 3);
        assert_eq!(ids(&d), vec!["s", "x", "y"]);
        assert_eq!(d.slides[1].blocks[0].kind, BlockKind::Heading);
        let d = deck(
            "---\nformat:\n  revealjs:\n    slide-level: 1\n    width: 1280\n---\n\n# A\n\n## b\n",
        );
        assert_eq!((d.slide_level, d.width, d.height), (1, 1280, 700));
        assert_eq!(d.slides.len(), 1);
    }

    #[test]
    fn auto_ids_follow_pandoc() {
        assert_eq!(auto_identifier("Café ☕ résumé"), "café-résumé");
        assert_eq!(auto_identifier("123 Numbers first"), "numbers-first");
        assert_eq!(auto_identifier("A [link](http://x.y) here!"), "a-link-here");
        assert_eq!(auto_identifier("!!!"), "section");
        let d = deck("## Dup\n\n## Dup\n\n## dup-1 {#dup-1}\n");
        assert_eq!(ids(&d), vec!["dup", "dup-2", "dup-1"]);
    }

    #[test]
    fn heading_attrs_and_closing_hashes() {
        let d =
            deck("## Title {#t .smaller bg=x}\n\n## Closing ##\n\n## {background-color=\"red\"}\n");
        assert_eq!(d.slides[0].title.as_deref(), Some("Title"));
        assert_eq!(d.slides[0].attrs.as_ref().unwrap().get("bg"), Some("x"));
        assert_eq!(d.slides[1].title.as_deref(), Some("Closing"));
        assert_eq!(d.slides[2].title, None);
        assert_eq!(d.slides[2].id, "section");
    }

    #[test]
    fn implicit_and_setext() {
        let d = deck("---\nformat: revealjs\n---\n\nPreamble.\n\nSetext\n------\n\nbody\n");
        assert_eq!(d.slides[0].kind, SlideKind::Implicit);
        assert_eq!(d.slides[1].title.as_deref(), Some("Setext"));
        assert!(d.slides[1].setext);
        assert_eq!(d.head.end, d.text.find("Preamble").unwrap());
    }

    #[test]
    fn loose_lists_stay_one_block() {
        let d = deck("## L\n\n- a\n\n- b\n\n  more b\n\n- c\n\nAfter.\n");
        let kinds: Vec<_> = d.slides[0].blocks.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, vec![BlockKind::List, BlockKind::Para]);
    }

    #[test]
    fn boxes_and_notes() {
        let d = deck(
            "## B\n\n![](x.png){#b-1 .absolute top=1}\n\n::: {#b-2 .absolute left=3}\nt\n:::\n\n::: notes\nn\n:::\n\n![](y.png)\n",
        );
        let kinds: Vec<_> = d.slides[0]
            .blocks
            .iter()
            .map(|b| (b.id.as_str(), b.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("b-1", BlockKind::Box),
                ("b-2", BlockKind::Box),
                ("b/2", BlockKind::Notes),
                ("b/3", BlockKind::Image)
            ]
        );
    }

    #[test]
    fn raw_blocks_are_html() {
        let d = deck("## A\n\n```{=html}\n<p>raw</p>\n```\n\n```{python}\nx\n```\n");
        let kinds: Vec<_> = d.slides[0].blocks.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, vec![BlockKind::Html, BlockKind::Cell]);
    }

    #[test]
    fn refs() {
        let d = deck("## A\n\n![](x.png){#img}\n\n## Dup {#d}\n\n## Dup2 {#d}\n");
        assert_eq!(d.slide("a").unwrap().index, 1);
        assert_eq!(d.slide("#a").unwrap().index, 1);
        assert_eq!(d.slide("@2").unwrap().index, 2);
        assert_eq!(d.slide("last").unwrap().index, 3);
        assert_eq!(d.slide("nope").unwrap_err().code(), "E_NOT_FOUND");
        assert_eq!(d.slide("@9").unwrap_err().code(), "E_NOT_FOUND");
        assert_eq!(d.slide("d").unwrap_err().code(), "E_AMBIGUOUS");
        assert_eq!(d.element("a/0").unwrap().1.id, "img");
        assert_eq!(d.element("#img").unwrap().0.id, "a");
        assert_eq!(d.element("a/5").unwrap_err().code(), "E_NOT_FOUND");
    }

    /// Spec 024: columns and their columns open up; other divs don't.
    #[test]
    fn columns_have_children() {
        let d = deck(concat!(
            "## R\n\n:::: {.columns}\n::: {.column width=\"40%\"}\n- one\n- two\n:::\n",
            "::: column\n![A plot](p.png){#fig}\n\n:::: columns\n::: column\nx\n:::\n::::\n:::\n::::\n\n",
            "::: {.fragment}\n- a\n:::\n\n::: {.callout-note}\nn\n:::\n",
        ));
        let ids = |bs: &[Block]| bs.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
        let r = &d.slides[0].blocks;
        assert_eq!(ids(r), ["r/0", "r/1", "r/2"]);
        assert_eq!(ids(&r[0].children), ["r/0/0", "r/0/1"]);
        assert_eq!(ids(&r[0].children[1].children), ["fig", "r/0/1/1"]);
        assert!(r[1].children.is_empty() && r[2].children.is_empty());
        let (_, list) = d.element("r/0/0/0").unwrap();
        assert_eq!(
            (list.kind, d.slice(list.span)),
            (BlockKind::List, "- one\n- two")
        );
        assert_eq!(d.element("r/0/1/1/0/0").unwrap().1.id, "r/0/1/1/0/0");
        assert_eq!(d.element("#fig").unwrap().1.kind, BlockKind::Image);
        assert_eq!(d.element("r/0/1/0").unwrap().1.id, "fig");
        assert_eq!(
            positional(&d.slides[0], d.element("#fig").unwrap().1),
            "r/0/1/0"
        );
        let (_, parent, list, i) = d.siblings("#fig").unwrap();
        assert_eq!(
            (parent.unwrap().id.as_str(), list.len(), i),
            ("r/0/1", 2, 0)
        );
        let (_, parent, list, i) = d.siblings("r/1").unwrap();
        assert_eq!((parent.is_none(), list.len(), i), (true, 3, 1));
        for bad in ["r/0/9/0", "r/1/0", "r/0/0/0/0"] {
            assert_eq!(d.element(bad).unwrap_err().code(), "E_NOT_FOUND", "{bad}");
        }
        assert!(d.explicit_ids().contains(&"fig"));
        let twice =
            deck("## A\n\n![](a.png){#x}\n\n:::: columns\n::: column\n![](b.png){#x}\n:::\n::::\n");
        assert_eq!(twice.element("#x").unwrap_err().code(), "E_AMBIGUOUS");
    }

    #[test]
    fn crlf_and_no_trailing_newline() {
        let d = deck("## A\r\n\r\ntext\r\n\r\n## B\r\n\r\nend");
        assert_eq!(d.nl, "\r\n");
        assert_eq!(d.slice(d.slides[0].blocks[0].span), "text");
        assert_eq!(d.slice(d.slides[1].blocks[0].span), "end");
        assert_eq!(d.slides[0].title.as_deref(), Some("A"));
    }

    #[test]
    fn empty_and_yaml_only() {
        assert!(deck("").slides.is_empty());
        let d = deck("---\ntitle: x\n---\n");
        assert!(d.slides.is_empty());
        assert_eq!(d.head.end, d.text.len());
    }
}
