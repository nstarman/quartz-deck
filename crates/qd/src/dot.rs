//! Diagrams in `{dot}` cells (spec 015): a span-keeping reader of the DOT
//! language, so qd can edit a diagram with minimal patches (research R4).

use crate::deck::{Deck, Span};
use crate::error::{QdError, Result};
use serde_json::{Value, json};

/// Where and why the reader stopped: the cell stays editable as text only.
#[derive(Debug, Clone)]
pub struct Stop {
    pub reason: String,
    /// Byte offset in the text read.
    pub at: usize,
}

/// `key=value`, with where each sits in the text.
#[derive(Debug, Clone)]
pub struct Attr {
    pub key: String,
    /// As DOT means it: quotes removed, `\"` unescaped; an HTML label without its outer `<>`.
    pub value: String,
    pub key_span: Span,
    /// The value as written, quotes or `<>` included.
    pub value_span: Span,
    pub quoted: bool,
}

/// One `[ … ]`.
#[derive(Debug, Clone)]
pub struct List {
    pub span: Span,
    pub attrs: Vec<Attr>,
}

/// One end of an edge: a node (with its port), or a `{ … }` group.
#[derive(Debug, Clone)]
pub enum Operand {
    Node {
        name: String,
        port: Option<String>,
        span: Span,
    },
    Group(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// `key = value` at graph level.
    Setting(Attr),
    /// `graph|node|edge [ … ]`.
    Defaults {
        target: String,
        lists: Vec<List>,
    },
    Node {
        name: String,
        port: Option<String>,
        lists: Vec<List>,
    },
    Edge {
        ops: Vec<Operand>,
        lists: Vec<List>,
    },
    Subgraph {
        id: Option<String>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub struct Stmt {
    /// From its first token to its last, without the `;`.
    pub span: Span,
    /// Its `;`, when it has one.
    pub term: Option<Span>,
    pub kind: Kind,
}

/// One `{dot}` cell's graph.
#[derive(Debug, Clone)]
pub struct Graph {
    pub strict: bool,
    pub directed: bool,
    pub id: Option<String>,
    /// Just after the opening `{`.
    pub open: usize,
    /// The closing `}`.
    pub close: usize,
    /// The top-level statements, in order.
    pub stmts: Vec<Stmt>,
}

/// A box: its statements merged, in order of first appearance.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<Attr>,
    /// First seen inside a subgraph (or a `{…} -> x` edge): text-only.
    pub locked: bool,
}

/// The value of `key`, the last one written winning.
fn last(attrs: &[Attr], key: &str) -> Option<String> {
    attrs
        .iter()
        .rev()
        .find(|a| a.key == key)
        .map(|a| a.value.clone())
}

impl Node {
    pub fn attr(&self, key: &str) -> Option<String> {
        last(&self.attrs, key)
    }

    /// `pos` in inches and whether it is pinned (`!`).
    pub fn pos(&self) -> Option<(f64, f64, bool)> {
        parse_pos(&self.attr("pos")?)
    }
}

/// `"x,y"` or `"x,y!"`.
pub fn parse_pos(v: &str) -> Option<(f64, f64, bool)> {
    let v = v.trim();
    let (v, pinned) = v.strip_suffix('!').map_or((v, false), |v| (v, true));
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?, pinned))
}

/// An arrow: one link of an edge statement.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub from_port: Option<String>,
    pub to_port: Option<String>,
    /// 1-based among arrows with the same ends, in source order (`A->B#2`).
    pub index: usize,
    /// `(link, links)` when its statement is a chain of more than one link.
    pub chain: Option<(usize, usize)>,
    /// Its top-level statement (the subgraph's, inside one).
    pub stmt: usize,
    pub attrs: Vec<Attr>,
    pub locked: bool,
}

// ---------- tokens ----------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Id { text: String, quoted: bool },
    Sym(&'static str),
}

#[derive(Debug, Clone)]
struct T {
    tok: Tok,
    span: Span,
}

pub(crate) fn stop(reason: impl Into<String>, at: usize) -> Stop {
    Stop {
        reason: reason.into(),
        at,
    }
}

fn is_id_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || !c.is_ascii()
}

fn is_id_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || !c.is_ascii()
}

/// One-character symbols.
const SYMS: &str = "{}[];,=:";

fn tokens(text: &str) -> std::result::Result<Vec<T>, Stop> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let at_line_start = |i: usize| {
        text[..i]
            .rsplit('\n')
            .next()
            .is_none_or(|l| l.trim().is_empty())
    };
    while i < b.len() {
        let c = text[i..].chars().next().expect("in bounds");
        let start = i;
        match c {
            _ if c.is_whitespace() => i += c.len_utf8(),
            '/' if text[i..].starts_with("//") => {
                i += text[i..].find('\n').unwrap_or(text.len() - i)
            }
            '/' if text[i..].starts_with("/*") => {
                i += 2 + text[i + 2..]
                    .find("*/")
                    .map(|e| e + 2)
                    .ok_or_else(|| stop("a /* comment is never closed", i))?;
            }
            '#' if at_line_start(i) => i += text[i..].find('\n').unwrap_or(text.len() - i),
            '"' => {
                let mut v = String::new();
                let mut j = i + 1;
                loop {
                    let Some(ch) = text[j..].chars().next() else {
                        return Err(stop("a quoted string is never closed", i));
                    };
                    match ch {
                        '"' => break,
                        '\\' if text[j + 1..].starts_with('"') => {
                            v.push('"');
                            j += 2;
                            continue;
                        }
                        '\\' if text[j + 1..].starts_with('\n') => {
                            j += 2;
                            continue;
                        }
                        '\\' if text[j + 1..].starts_with("\r\n") => {
                            j += 3;
                            continue;
                        }
                        _ => v.push(ch),
                    }
                    j += ch.len_utf8();
                }
                i = j + 1;
                if text[i..].trim_start().starts_with('+') {
                    return Err(stop(
                        "joined strings (\"a\" + \"b\") can't be edited here",
                        i,
                    ));
                }
                out.push(T {
                    tok: Tok::Id {
                        text: v,
                        quoted: true,
                    },
                    span: Span { start, end: i },
                });
            }
            '<' => {
                let mut depth = 0usize;
                let mut j = i;
                for (k, ch) in text[i..].char_indices() {
                    match ch {
                        '<' => depth += 1,
                        '>' => {
                            depth -= 1;
                            if depth == 0 {
                                j = i + k + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth != 0 {
                    return Err(stop("an HTML label <…> is never closed", i));
                }
                out.push(T {
                    tok: Tok::Id {
                        text: text[i + 1..j - 1].to_string(),
                        quoted: true,
                    },
                    span: Span { start, end: j },
                });
                i = j;
            }
            '-' if text[i..].starts_with("->") || text[i..].starts_with("--") => {
                let op = if text[i + 1..].starts_with('>') {
                    "->"
                } else {
                    "--"
                };
                out.push(T {
                    tok: Tok::Sym(op),
                    span: Span { start, end: i + 2 },
                });
                i += 2;
            }
            _ if SYMS.contains(c) => {
                let k = SYMS.find(c).expect("a symbol");
                out.push(T {
                    tok: Tok::Sym(&SYMS[k..k + 1]),
                    span: Span { start, end: i + 1 },
                });
                i += 1;
            }
            '-' | '.' | '0'..='9' => {
                // a numeral: -?(.[0-9]+ | [0-9]+(.[0-9]*)?)
                let mut j = i + usize::from(c == '-');
                let digits = |j: usize| text[j..].chars().take_while(char::is_ascii_digit).count();
                j += digits(j);
                if text[j..].starts_with('.') {
                    j += 1 + digits(j + 1);
                }
                if !text[i..j].bytes().any(|b| b.is_ascii_digit()) {
                    return Err(stop(format!("unexpected '{c}'"), i));
                }
                out.push(T {
                    tok: Tok::Id {
                        text: text[i..j].to_string(),
                        quoted: false,
                    },
                    span: Span { start, end: j },
                });
                i = j;
            }
            _ if is_id_start(c) => {
                let len: usize = text[i..]
                    .chars()
                    .take_while(|&c| is_id_char(c))
                    .map(char::len_utf8)
                    .sum();
                out.push(T {
                    tok: Tok::Id {
                        text: text[i..i + len].to_string(),
                        quoted: false,
                    },
                    span: Span {
                        start,
                        end: i + len,
                    },
                });
                i += len;
            }
            _ => return Err(stop(format!("unexpected '{c}'"), i)),
        }
    }
    Ok(out)
}

// ---------- parser ----------

struct P<'a> {
    toks: &'a [T],
    i: usize,
    len: usize,
    /// `->` in a digraph, `--` in a graph.
    op: &'static str,
}

impl P<'_> {
    fn peek(&self) -> Option<&T> {
        self.toks.get(self.i)
    }

    fn peek_sym(&self, s: &str) -> bool {
        matches!(self.peek(), Some(T { tok: Tok::Sym(x), .. }) if *x == s)
    }

    fn at(&self) -> usize {
        self.peek().map_or(self.len, |t| t.span.start)
    }

    fn keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(T { tok: Tok::Id { text, quoted: false }, .. }) if text.eq_ignore_ascii_case(kw))
    }

    fn found(&self) -> String {
        match self.peek() {
            None => "the end".into(),
            Some(T {
                tok: Tok::Sym(s), ..
            }) => s.to_string(),
            Some(T {
                tok: Tok::Id { text, .. },
                ..
            }) => text.clone(),
        }
    }

    fn expect(&mut self, s: &str, what: &str) -> std::result::Result<Span, Stop> {
        if self.peek_sym(s) {
            self.i += 1;
            Ok(self.toks[self.i - 1].span)
        } else {
            Err(stop(
                format!("expected {what} but found {}", self.found()),
                self.at(),
            ))
        }
    }

    fn id(&mut self, what: &str) -> std::result::Result<(String, Span, bool), Stop> {
        match self.peek() {
            Some(T {
                tok: Tok::Id { text, quoted },
                span,
            }) => {
                let r = (text.clone(), *span, *quoted);
                self.i += 1;
                Ok(r)
            }
            _ => Err(stop(
                format!("expected {what} but found {}", self.found()),
                self.at(),
            )),
        }
    }

    fn lists(&mut self) -> std::result::Result<Vec<List>, Stop> {
        let mut out = Vec::new();
        while self.peek_sym("[") {
            let open = self.expect("[", "[")?;
            let mut attrs = Vec::new();
            while !self.peek_sym("]") {
                let (key, key_span, _) = self.id("an attribute name")?;
                self.expect("=", &format!("= after {key}"))?;
                let (value, value_span, quoted) = self.id(&format!("a value for {key}"))?;
                attrs.push(Attr {
                    key,
                    value,
                    key_span,
                    value_span,
                    quoted,
                });
                if self.peek_sym(",") || self.peek_sym(";") {
                    self.i += 1;
                }
            }
            let close = self.expect("]", "]")?;
            out.push(List {
                span: Span {
                    start: open.start,
                    end: close.end,
                },
                attrs,
            });
        }
        Ok(out)
    }

    fn port(&mut self) -> std::result::Result<Option<String>, Stop> {
        let mut parts = Vec::new();
        while self.peek_sym(":") {
            self.i += 1;
            parts.push(self.id("a port")?.0);
        }
        Ok((!parts.is_empty()).then(|| parts.join(":")))
    }

    /// `[subgraph [ID]] { … }`
    fn subgraph(&mut self) -> std::result::Result<(Option<String>, Vec<Stmt>, Span), Stop> {
        let start = self.at();
        let mut id = None;
        if self.keyword("subgraph") {
            self.i += 1;
            if !self.peek_sym("{") {
                id = Some(self.id("a subgraph name")?.0);
            }
        }
        self.expect("{", "{")?;
        let body = self.stmts()?;
        let close = self.expect("}", "the subgraph's closing }")?;
        Ok((
            id,
            body,
            Span {
                start,
                end: close.end,
            },
        ))
    }

    fn operand(&mut self) -> std::result::Result<(Operand, Span), Stop> {
        if self.keyword("subgraph") || self.peek_sym("{") {
            let (_, body, span) = self.subgraph()?;
            let mut names = Vec::new();
            collect_names(&body, &mut names);
            return Ok((Operand::Group(names), span));
        }
        let (name, span, _) = self.id(&format!("a node after {}", self.op))?;
        let port = self.port()?;
        let end = self.toks[self.i - 1].span.end;
        Ok((
            Operand::Node { name, port, span },
            Span {
                start: span.start,
                end,
            },
        ))
    }

    fn stmts(&mut self) -> std::result::Result<Vec<Stmt>, Stop> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => return Err(stop("the graph ends before its closing }", self.len)),
                Some(T {
                    tok: Tok::Sym("}"), ..
                }) => return Ok(out),
                Some(T {
                    tok: Tok::Sym(";"), ..
                }) => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let mut st = self.stmt()?;
            if self.peek_sym(";") {
                st.term = Some(self.toks[self.i].span);
                self.i += 1;
            }
            out.push(st);
        }
    }

    fn stmt(&mut self) -> std::result::Result<Stmt, Stop> {
        let start = self.at();
        let span_to_here = |p: &P| Span {
            start,
            end: p.toks[p.i - 1].span.end,
        };
        for target in ["graph", "node", "edge"] {
            if self.keyword(target)
                && matches!(
                    self.toks.get(self.i + 1),
                    Some(T {
                        tok: Tok::Sym("["),
                        ..
                    })
                )
            {
                self.i += 1;
                let lists = self.lists()?;
                return Ok(Stmt {
                    span: span_to_here(self),
                    term: None,
                    kind: Kind::Defaults {
                        target: target.into(),
                        lists,
                    },
                });
            }
        }
        let first = if self.keyword("subgraph") || self.peek_sym("{") {
            let (id, body, span) = self.subgraph()?;
            if !self.peek_sym("->") && !self.peek_sym("--") {
                return Ok(Stmt {
                    span,
                    term: None,
                    kind: Kind::Subgraph { id, body },
                });
            }
            let mut names = Vec::new();
            collect_names(&body, &mut names);
            Operand::Group(names)
        } else {
            let (name, span, _) = self.id("a statement")?;
            if self.peek_sym("=") {
                self.i += 1;
                let (value, value_span, vq) = self.id(&format!("a value for {name}"))?;
                let a = Attr {
                    key: name,
                    value,
                    key_span: span,
                    value_span,
                    quoted: vq,
                };
                return Ok(Stmt {
                    span: span_to_here(self),
                    term: None,
                    kind: Kind::Setting(a),
                });
            }
            let port = self.port()?;
            if !self.peek_sym("->") && !self.peek_sym("--") {
                let lists = self.lists()?;
                return Ok(Stmt {
                    span: span_to_here(self),
                    term: None,
                    kind: Kind::Node { name, port, lists },
                });
            }
            Operand::Node { name, port, span }
        };
        let mut ops = vec![first];
        while self.peek_sym("->") || self.peek_sym("--") {
            if !self.peek_sym(self.op) {
                let kind = if self.op == "->" {
                    "a digraph"
                } else {
                    "a graph"
                };
                return Err(stop(format!("{kind} uses {}", self.op), self.at()));
            }
            self.i += 1;
            ops.push(self.operand()?.0);
        }
        let lists = self.lists()?;
        Ok(Stmt {
            span: span_to_here(self),
            term: None,
            kind: Kind::Edge { ops, lists },
        })
    }
}

/// Node names declared in statements, in order (for `{p q} -> r`).
fn collect_names(stmts: &[Stmt], out: &mut Vec<String>) {
    let add = |n: &str, out: &mut Vec<String>| {
        if !out.iter().any(|x| x == n) {
            out.push(n.to_string());
        }
    };
    for s in stmts {
        match &s.kind {
            Kind::Node { name, .. } => add(name, out),
            Kind::Edge { ops, .. } => {
                for o in ops {
                    match o {
                        Operand::Node { name, .. } => add(name, out),
                        Operand::Group(g) => g.iter().for_each(|n| add(n, out)),
                    }
                }
            }
            Kind::Subgraph { body, .. } => collect_names(body, out),
            _ => {}
        }
    }
}

/// Read one graph: `[strict] (graph|digraph) [ID] { … }`, and nothing after it.
pub fn parse(text: &str) -> std::result::Result<Graph, Stop> {
    let toks = tokens(text)?;
    let mut p = P {
        toks: &toks,
        i: 0,
        len: text.len(),
        op: "->",
    };
    let strict = p.keyword("strict");
    if strict {
        p.i += 1;
    }
    p.op = if p.keyword("digraph") {
        "->"
    } else if p.keyword("graph") {
        "--"
    } else {
        return Err(stop(
            format!("expected graph or digraph but found {}", p.found()),
            p.at(),
        ));
    };
    p.i += 1;
    let id = if p.peek_sym("{") {
        None
    } else {
        Some(p.id("the graph's name")?.0)
    };
    let open = p.expect("{", "{")?.end;
    let stmts = p.stmts()?;
    let close = p.expect("}", "the graph's closing }")?.start;
    if p.peek().is_some() {
        return Err(stop(
            "a cell holds one graph; more follows its closing }",
            p.at(),
        ));
    }
    Ok(Graph {
        strict,
        directed: p.op == "->",
        id,
        open,
        close,
        stmts,
    })
}

fn list_attrs(lists: &[List]) -> Vec<Attr> {
    lists.iter().flat_map(|l| l.attrs.iter().cloned()).collect()
}

impl Graph {
    /// `->` or `--`.
    pub fn op(&self) -> &'static str {
        if self.directed { "->" } else { "--" }
    }

    /// A graph-level setting (`key=value` or `graph [key=value]`), the last one winning.
    pub fn setting(&self, key: &str) -> Option<String> {
        last(&self.settings(), key)
    }

    pub fn settings(&self) -> Vec<Attr> {
        let mut out = Vec::new();
        for s in &self.stmts {
            match &s.kind {
                Kind::Setting(a) => out.push(a.clone()),
                Kind::Defaults { target, lists } if target == "graph" => {
                    out.extend(list_attrs(lists))
                }
                _ => {}
            }
        }
        out
    }

    /// Every box, merged across its statements, in order of first appearance.
    pub fn nodes(&self) -> Vec<Node> {
        fn walk(stmts: &[Stmt], locked: bool, out: &mut Vec<Node>) {
            let touch = |name: &str, locked: bool, attrs: &[Attr], out: &mut Vec<Node>| match out
                .iter_mut()
                .find(|n| n.name == name)
            {
                Some(n) => n.attrs.extend(attrs.iter().cloned()),
                None => out.push(Node {
                    name: name.into(),
                    attrs: attrs.to_vec(),
                    locked,
                }),
            };
            for s in stmts {
                match &s.kind {
                    Kind::Node { name, lists, .. } => touch(name, locked, &list_attrs(lists), out),
                    Kind::Edge { ops, .. } => {
                        let edge_locked =
                            locked || ops.iter().any(|o| matches!(o, Operand::Group(_)));
                        for o in ops {
                            match o {
                                Operand::Node { name, .. } => touch(name, edge_locked, &[], out),
                                Operand::Group(g) => {
                                    g.iter().for_each(|n| touch(n, true, &[], out))
                                }
                            }
                        }
                    }
                    Kind::Subgraph { body, .. } => walk(body, true, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.stmts, false, &mut out);
        out
    }

    /// Every arrow, link by link, in source order.
    pub fn edges(&self) -> Vec<Edge> {
        fn ends(o: &Operand) -> Vec<(String, Option<String>)> {
            match o {
                Operand::Node { name, port, .. } => vec![(name.clone(), port.clone())],
                Operand::Group(g) => g.iter().map(|n| (n.clone(), None)).collect(),
            }
        }
        fn walk(stmts: &[Stmt], top: Option<usize>, locked: bool, out: &mut Vec<Edge>) {
            for (i, s) in stmts.iter().enumerate() {
                match &s.kind {
                    Kind::Edge { ops, lists } => {
                        let locked = locked || ops.iter().any(|o| matches!(o, Operand::Group(_)));
                        let links = ops.len() - 1;
                        for k in 0..links {
                            for (from, from_port) in ends(&ops[k]) {
                                for (to, to_port) in ends(&ops[k + 1]) {
                                    let index = 1 + out
                                        .iter()
                                        .filter(|e| e.from == from && e.to == to)
                                        .count();
                                    out.push(Edge {
                                        from: from.clone(),
                                        to,
                                        from_port: from_port.clone(),
                                        to_port,
                                        index,
                                        chain: (links > 1).then_some((k, links)),
                                        stmt: top.unwrap_or(i),
                                        attrs: list_attrs(lists),
                                        locked,
                                    });
                                }
                            }
                        }
                    }
                    Kind::Subgraph { body, .. } => walk(body, top.or(Some(i)), true, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.stmts, None, false, &mut out);
        out
    }

    /// Placeable by hand: `layout` neato or fdp, and every box pinned (`pos="x,y!"`).
    pub fn pinned(&self) -> bool {
        matches!(self.setting("layout").as_deref(), Some("neato" | "fdp"))
            && self.nodes().iter().all(|n| n.pos().is_some_and(|p| p.2))
    }
}

// ---------- the cell ----------

/// The keys of a box's look (data-model "Look").
pub const LOOK: [&str; 7] = [
    "shape",
    "style",
    "fillcolor",
    "color",
    "fontcolor",
    "fontname",
    "penwidth",
];

/// A cell's diagram. Quarto's `//| file: x.dot` keeps it in another file,
/// which qd doesn't edit: that is said, rather than a parse error.
fn parse_cell(text: &str) -> std::result::Result<Graph, Stop> {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let opt = line.trim().strip_prefix("//|").map(str::trim);
        if let Some(file) = opt.and_then(|o| o.strip_prefix("file:")) {
            return Err(Stop {
                reason: format!(
                    "this diagram is in {} (`//| file:`), which qd doesn't edit",
                    file.trim().trim_matches(['"', '\''])
                ),
                at,
            });
        }
        at += line.len();
    }
    parse(text)
}

/// A `{dot}` cell: its canonical ref and the span of its body (between the fences) in the deck.
pub fn cell(d: &Deck, r: &str) -> Result<(String, Span)> {
    let (_, b) = d.element(r)?;
    if b.kind != crate::deck::BlockKind::Cell || crate::kernel::cell_lang(d, b) != "dot" {
        return Err(QdError::NotFound(format!("{r} is not a {{dot}} cell")));
    }
    let text = d.slice(b.span);
    let start = b.span.start + text.find('\n').map_or(text.len(), |n| n + 1);
    let end = b.span.start + text.rfind('\n').map_or(text.len(), |n| n + 1);
    Ok((
        crate::focus::resolve(d, r)?.1,
        Span {
            start,
            end: end.max(start),
        },
    ))
}

fn attrs_json<'a>(attrs: impl IntoIterator<Item = &'a Attr>) -> Value {
    Value::Object(
        attrs
            .into_iter()
            .map(|a| (a.key.clone(), json!(a.value)))
            .collect(),
    )
}

/// Why text from `start` in the deck can't be edited, and where (`{reason, at: {line, column}}`).
pub(crate) fn stopped_at(d: &Deck, start: usize, s: Stop) -> Value {
    let abs = start + s.at;
    let line = d.lines(Span {
        start: abs,
        end: abs,
    })[0];
    let col = abs - d.text[..abs].rfind('\n').map_or(0, |n| n + 1) + 1;
    json!({"reason": s.reason, "at": {"line": line, "column": col}})
}

/// `qd dot show CELL` (contracts/cli.md).
pub fn show(d: &Deck, r: &str) -> Result<Value> {
    let (cell, body) = cell(d, r)?;
    let text = d.slice(body);
    let g = match parse_cell(text) {
        Ok(g) => g,
        Err(s) => {
            return Ok(json!({"ok": true, "kind": "dot", "cell": cell,
                "editable": stopped_at(d, body.start, s)}));
        }
    };
    let nodes: Vec<Value> = g
        .nodes()
        .iter()
        .map(|n| {
            let look = n.attrs.iter().filter(|a| LOOK.contains(&a.key.as_str()));
            let size = n
                .attrs
                .iter()
                .filter(|a| matches!(a.key.as_str(), "width" | "height" | "fixedsize"));
            let pos = n.pos();
            json!({"name": n.name, "label": label(n),
                "look": attrs_json(look), "url": n.attr("URL"), "image": n.attr("image"),
                "pos": pos.map(|(x, y, _)| json!([x, y])), "pinned": pos.is_some_and(|p| p.2),
                "size": attrs_json(size), "locked": n.locked})
        })
        .collect();
    let edges: Vec<Value> = g
        .edges()
        .iter()
        .map(|e| json!({"from": e.from, "to": e.to, "index": e.index, "attrs": attrs_json(&e.attrs), "locked": e.locked}))
        .collect();
    let starters: Vec<Value> = ["tool", "artifact"]
        .iter()
        .map(|n| {
            let look = starter(n)
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, json!(v)));
            json!({"name": n, "look": Value::Object(look.collect())})
        })
        .collect();
    Ok(
        json!({"ok": true, "kind": "dot", "cell": cell, "directed": g.directed, "pinned": g.pinned(),
        "editable": true, "text": text, "settings": attrs_json(&g.settings()), "nodes": nodes, "edges": edges,
        "starters": starters}),
    )
}

// ---------- editing (spec 015 US1, research R7) ----------

/// A value as DOT text: bare when it is an ID or a numeral, else quoted.
pub fn dot_value(v: &str) -> String {
    let id = v.chars().next().is_some_and(is_id_start) && v.chars().all(is_id_char);
    let keyword = ["node", "edge", "graph", "digraph", "subgraph", "strict"]
        .iter()
        .any(|k| v.eq_ignore_ascii_case(k));
    let d = v.strip_prefix('-').unwrap_or(v);
    let num = d.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        && d.matches('.').count() <= 1
        && d.bytes().any(|c| c.is_ascii_digit());
    if (id && !keyword) || num {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace('"', "\\\""))
    }
}

/// Snapped to the 0.25 in grid.
pub fn snap(v: f64) -> f64 {
    let s = (v * 4.0).round() / 4.0;
    if s == 0.0 { 0.0 } else { s }
}

fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// `"x,y!"`: a pinned position in inches.
pub fn pos_value(x: f64, y: f64) -> String {
    format!("{},{}!", num(x), num(y))
}

fn newline(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// A new statement before the graph's closing `}`: on its own line, indented
/// and terminated like its neighbours, or inline when the graph is one line.
fn add_stmt(text: &str, g: &Graph, stmt: &str) -> (Span, String) {
    let term = if g.stmts.iter().any(|s| s.term.is_some()) {
        ";"
    } else {
        ""
    };
    let line = text[..g.close].rfind('\n').map_or(0, |n| n + 1);
    if !text[line..g.close].trim().is_empty() || line <= g.open {
        return (
            Span {
                start: g.close,
                end: g.close,
            },
            format!("{stmt}{term} "),
        );
    }
    let indent = g.stmts.last().map_or("  ", |s| {
        let ls = text[..s.span.start].rfind('\n').map_or(0, |n| n + 1);
        let ws = &text[ls..s.span.start];
        if ws.trim().is_empty() { ws } else { "  " }
    });
    (
        Span {
            start: line,
            end: line,
        },
        format!("{indent}{stmt}{term}{}", newline(text)),
    )
}

/// Set `key` to `v` (as DOT text) in a statement's attribute lists: its value
/// in place, else added to the last list, else a new list after the statement.
fn set_in(lists: &[List], end: usize, key: &str, v: &str) -> (Span, String) {
    if let Some(a) = lists
        .iter()
        .rev()
        .flat_map(|l| l.attrs.iter().rev())
        .find(|a| a.key == key)
    {
        return (a.value_span, v.into());
    }
    match lists.last() {
        Some(l) => {
            let at = l.span.end - 1;
            let sep = if l.attrs.is_empty() { "" } else { ", " };
            (Span { start: at, end: at }, format!("{sep}{key}={v}"))
        }
        None => (Span { start: end, end }, format!(" [{key}={v}]")),
    }
}

/// A box qd may edit: it exists and isn't inside a subgraph.
fn node(g: &Graph, name: &str) -> Result<Node> {
    let n = g
        .nodes()
        .into_iter()
        .find(|n| n.name == name)
        .ok_or_else(|| QdError::NotFound(format!("no box named '{name}'")))?;
    if n.locked {
        return Err(QdError::UnsafeEdit(format!(
            "'{name}' is inside a subgraph; edit it as text"
        )));
    }
    Ok(n)
}

/// The text of a spacer label (data-model "Spacer label"), when `label` is one.
fn spacer_text(label: &str) -> Option<String> {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"^<TABLE BORDER="0" CELLPADDING="0" CELLSPACING="4"><TR><TD WIDTH="\d+" HEIGHT="\d+" FIXEDSIZE="TRUE"></TD><TD>(.*)</TD></TR></TABLE>$"#)
            .expect("valid regex")
    });
    RE.captures(label).map(|c| crate::inline::unescape(&c[1]))
}

/// A spacer label for `text` in a box `height` inches tall, as DOT text: an
/// empty square cell, where dot-image draws the picture, then the text.
fn spacer(text: &str, height: f64) -> String {
    let w = (height * 72.0 - 8.0).round();
    format!(
        "<<TABLE BORDER=\"0\" CELLPADDING=\"0\" CELLSPACING=\"4\"><TR><TD WIDTH=\"{w}\" HEIGHT=\"{w}\" FIXEDSIZE=\"TRUE\"></TD><TD>{}</TD></TR></TABLE>>",
        crate::render::esc(text)
    )
}

/// A box's label as the person reads it: a spacer label's text, else its `label`, else its name.
fn label(n: &Node) -> String {
    let l = n.attr("label");
    l.as_deref()
        .and_then(spacer_text)
        .or(l)
        .unwrap_or_else(|| n.name.clone())
}

/// A box with a picture keeps room for it beside its label, sized by its
/// height (default 0.5 in); without one, the label is plain again.
fn fit_label(doc: &mut Doc, name: &str) -> Result<()> {
    let g = doc.graph()?;
    let n = node(&g, name)?;
    let spaced = n.attr("label").as_deref().and_then(spacer_text).is_some();
    let edit = if n.attr("image").is_some_and(|i| !i.is_empty()) {
        let h = n.attr("height").and_then(|h| h.parse().ok()).unwrap_or(0.5);
        set_node_dot(&doc.0, &g, name, "label", &spacer(&label(&n), h))?
    } else if spaced {
        set_node_attr(&doc.0, &g, name, "label", &label(&n))?
    } else {
        return Ok(());
    };
    doc.apply(edit);
    Ok(())
}

/// `name [k=v, …]`, or just `name`.
fn node_stmt(name: &str, attrs: &[(String, String)]) -> String {
    let list: Vec<String> = attrs
        .iter()
        .map(|(k, v)| format!("{k}={}", dot_value(v)))
        .collect();
    if list.is_empty() {
        dot_value(name)
    } else {
        format!("{} [{}]", dot_value(name), list.join(", "))
    }
}

/// Set a box's attribute (one edit, relative to `text`).
pub fn set_node_attr(
    text: &str,
    g: &Graph,
    name: &str,
    key: &str,
    value: &str,
) -> Result<(Span, String)> {
    set_node_dot(text, g, name, key, &dot_value(value))
}

/// Set a box's attribute to `v`, written as DOT text (an HTML label's `<…>` included).
fn set_node_dot(text: &str, g: &Graph, name: &str, key: &str, v: &str) -> Result<(Span, String)> {
    node(g, name)?;
    let stmt = g.stmts.iter().rev().find_map(|s| match &s.kind {
        Kind::Node { name: n, lists, .. } if n == name => Some((lists, s.span.end)),
        _ => None,
    });
    Ok(match stmt {
        Some((lists, end)) => set_in(lists, end, key, v),
        None => add_stmt(text, g, &format!("{} [{key}={v}]", dot_value(name))),
    })
}

/// Set a graph setting: `key=value` or `graph [key=value]` in place, else a new `key=value` statement.
pub fn set_setting(text: &str, g: &Graph, key: &str, value: &str) -> (Span, String) {
    for s in g.stmts.iter().rev() {
        match &s.kind {
            Kind::Setting(a) if a.key == key => return (a.value_span, dot_value(value)),
            Kind::Defaults { target, lists }
                if target == "graph"
                    && lists.iter().any(|l| l.attrs.iter().any(|a| a.key == key)) =>
            {
                return set_in(lists, s.span.end, key, &dot_value(value));
            }
            _ => {}
        }
    }
    add_stmt(text, g, &format!("{key}={}", dot_value(value)))
}

/// The cell, its body's span, its text and its graph, or why it can't be edited.
fn read<'a>(d: &'a Deck, r: &str) -> Result<(String, Span, &'a str, Graph)> {
    let (cell, body) = cell(d, r)?;
    let text = d.slice(body);
    let g = parse_cell(text).map_err(|s| {
        let line = d.lines(Span {
            start: body.start + s.at,
            end: body.start + s.at,
        })[0];
        QdError::UnsafeEdit(format!(
            "{cell}: {} (line {line}); edit it as text",
            s.reason
        ))
    })?;
    Ok((cell, body, text, g))
}

/// A diagram's text through several edits: each step reads it afresh.
struct Doc(String);

impl Doc {
    fn graph(&self) -> Result<Graph> {
        parse(&self.0).map_err(|s| QdError::UnsafeEdit(s.reason))
    }

    fn apply(&mut self, (span, new): (Span, String)) {
        self.0.replace_range(span.start..span.end, &new);
    }

    /// The cell's body replaced by the edited text.
    fn patch(self, op: &str, target: String, body: Span) -> crate::ops::Patch {
        crate::ops::patch(op, target, vec![(body, self.0)], vec![])
    }
}

/// `qd dot move`: a box's pinned position, `at` or moved `by` (inches), snapped unless `exact`.
pub fn move_node(
    d: &Deck,
    r: &str,
    name: &str,
    at: Option<(f64, f64)>,
    by: Option<(f64, f64)>,
    exact: bool,
) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    if !g.pinned() {
        return Err(QdError::InvalidArg(format!(
            "the diagram in {cell} isn't pinned, so its boxes can't be placed: run `qd dot pin {cell}` first"
        )));
    }
    let (x, y, _) = node(&g, name)?
        .pos()
        .expect("pinned diagrams pin every box");
    let (mut x, mut y) = match (at, by) {
        (Some(a), _) => a,
        (None, Some((dx, dy))) => (x + dx, y + dy),
        (None, None) => return Err(QdError::InvalidArg("give --at X,Y or --by DX,DY".into())),
    };
    if !exact {
        (x, y) = (snap(x), snap(y));
    }
    let mut doc = Doc(text.into());
    doc.apply(set_node_attr(text, &g, name, "pos", &pos_value(x, y))?);
    Ok((
        doc.patch("dot-move", format!("{cell}:{name}"), body),
        json!({"pos": [x, y]}),
    ))
}

/// `qd dot pin`: `layout=neato`, `splines=ortho`, and every box at `positions` (inches).
pub fn pin(
    d: &Deck,
    r: &str,
    positions: &std::collections::HashMap<String, (f64, f64)>,
) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    // neato doesn't draw clusters, and boxes inside a subgraph can't be placed
    if g.stmts
        .iter()
        .any(|s| matches!(s.kind, Kind::Subgraph { .. }))
        || g.edges().iter().any(|e| e.locked)
    {
        return Err(QdError::UnsafeEdit(format!(
            "the diagram in {cell} has a subgraph, which a pinned layout can't keep: edit it as text"
        )));
    }
    let names: Vec<String> = g.nodes().into_iter().map(|n| n.name).collect();
    if let Some(missing) = names.iter().find(|n| !positions.contains_key(*n)) {
        return Err(QdError::NotFound(format!(
            "no position for '{missing}' in {cell}"
        )));
    }
    // right angles, unless the diagram chose its own splines or uses ports
    // (Graphviz's ortho routing ignores ports)
    let ortho = g.setting("splines").is_none()
        && !g
            .edges()
            .iter()
            .any(|e| e.from_port.is_some() || e.to_port.is_some());
    let mut doc = Doc(text.into());
    for (k, v) in [("layout", "neato"), ("splines", "ortho")]
        .into_iter()
        .filter(|(k, _)| *k != "splines" || ortho)
    {
        let g = doc.graph()?;
        doc.apply(set_setting(&doc.0, &g, k, v));
    }
    for n in &names {
        let (x, y) = positions[n];
        let g = doc.graph()?;
        doc.apply(set_node_attr(&doc.0, &g, n, "pos", &pos_value(x, y))?);
    }
    Ok((
        doc.patch("dot-pin", cell, body),
        json!({"pinned": names.len()}),
    ))
}

// ---------- building (spec 015 US2) ----------

/// A look: style attributes in `LOOK` order.
pub type Look = Vec<(String, String)>;

/// The two starter looks (data-model "Look").
pub fn starter(name: &str) -> Option<Look> {
    let l: &[(&str, &str)] = match name {
        "tool" => &[
            ("shape", "box"),
            ("style", "rounded,filled"),
            ("fillcolor", "#3d4a5c"),
            ("color", "#3d4a5c"),
            ("fontcolor", "white"),
        ],
        "artifact" => &[
            ("shape", "box"),
            ("style", "rounded,filled"),
            ("fillcolor", "white"),
            ("color", "#c8ccd2"),
            ("fontcolor", "#1f2933"),
        ],
        _ => return None,
    };
    Some(
        l.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

fn look_of(n: &Node) -> Look {
    LOOK.iter()
        .filter_map(|k| n.attr(k).map(|v| (k.to_string(), v)))
        .collect()
}

/// `--look NAME` or `--look-of NODE`.
fn chosen_look(g: &Graph, look: Option<&str>, look_of_node: Option<&str>) -> Result<Option<Look>> {
    match (look, look_of_node) {
        (Some(n), _) => starter(n).map(Some).ok_or_else(|| {
            QdError::InvalidArg(format!(
                "no look named '{n}': use tool or artifact, or --look-of NODE"
            ))
        }),
        (None, Some(n)) => Ok(Some(look_of(&node(g, n)?))),
        (None, None) => Ok(None),
    }
}

/// `n1`, `n2`, …: the first name not in use.
pub fn fresh(g: &Graph) -> String {
    let names: Vec<String> = g.nodes().into_iter().map(|n| n.name).collect();
    (1..)
        .map(|i| format!("n{i}"))
        .find(|n| !names.contains(n))
        .expect("a free name")
}

/// The cell `qd dot new` inserts: two placed boxes in the starter looks, joined.
pub fn starter_diagram() -> String {
    let with = |label: &str, pos: &str, look: &str| {
        let mut a = vec![("label".into(), label.into()), ("pos".into(), pos.into())];
        a.extend(starter(look).expect("a starter"));
        node_stmt(&a[0].1.to_lowercase(), &a)
    };
    format!(
        "```{{dot}}\ndigraph {{\n  layout=neato; splines=ortho;\n  {};\n  {};\n  data -> tool;\n}}\n```",
        with("Data", "0,0!", "artifact"),
        with("Tool", "2,0!", "tool"),
    )
}

/// `A->B#k`.
fn edge_name(g: &Graph, e: &Edge) -> String {
    format!("{}{}{}#{}", e.from, g.op(), e.to, e.index)
}

/// An arrow qd may edit, from `A->B`, `A--B` or `A->B#k`.
fn edge(g: &Graph, r: &str) -> Result<Edge> {
    let (ends, k) = match r.rsplit_once('#') {
        Some((e, k)) => (
            e,
            k.parse()
                .map_err(|_| QdError::InvalidArg(format!("'{r}': #k must be a number")))?,
        ),
        None => (r, 1),
    };
    let (a, b) = ends
        .split_once(g.op())
        .ok_or_else(|| QdError::InvalidArg(format!("'{r}' isn't an arrow: write A{}B", g.op())))?;
    let e = g
        .edges()
        .into_iter()
        .find(|e| e.from == a.trim() && e.to == b.trim() && e.index == k)
        .ok_or_else(|| QdError::NotFound(format!("no arrow {r}")))?;
    if e.locked {
        return Err(QdError::UnsafeEdit(format!(
            "{r} is inside a subgraph; edit it as text"
        )));
    }
    Ok(e)
}

/// A statement removed: its whole line when alone on it, else it and its spaces.
fn remove_stmt(text: &str, s: &Stmt) -> (Span, String) {
    let end = s.term.map_or(s.span.end, |t| t.end);
    let ls = text[..s.span.start].rfind('\n').map_or(0, |n| n + 1);
    let le = text[end..].find('\n').map_or(text.len(), |n| end + n + 1);
    let none = |t: &str| t.trim().is_empty();
    let span = if none(&text[ls..s.span.start]) && none(&text[end..le]) {
        Span { start: ls, end: le }
    } else if none(&text[end..le]) {
        let start = text[..s.span.start].trim_end_matches([' ', '\t']).len();
        Span { start, end }
    } else {
        let after = &text[end..];
        Span {
            start: s.span.start,
            end: end + after.len() - after.trim_start_matches([' ', '\t']).len(),
        }
    };
    (span, String::new())
}

/// Remove `key` from a statement's lists (the last one written), with its
/// separator, and the list itself when it empties.
fn unset_in(text: &str, lists: &[List], key: &str) -> Option<(Span, String)> {
    let (l, a) = lists
        .iter()
        .rev()
        .find_map(|l| l.attrs.iter().rev().find(|a| a.key == key).map(|a| (l, a)))?;
    let seps: &[char] = &[' ', '\t', '\r', '\n', ',', ';'];
    let span = if l.attrs.len() == 1 {
        Span {
            start: text[..l.span.start].trim_end_matches([' ', '\t']).len(),
            end: l.span.end,
        }
    } else {
        let close = l.span.end - 1;
        let rest = &text[a.value_span.end..close];
        let gap = rest.len() - rest.trim_start_matches(seps).len();
        if a.value_span.end + gap < close {
            Span {
                start: a.key_span.start,
                end: a.value_span.end + gap,
            }
        } else {
            Span {
                start: text[..a.key_span.start].trim_end_matches(seps).len(),
                end: a.value_span.end,
            }
        }
    };
    Some((span, String::new()))
}

/// An edge statement's operands as text ranges (each with its port), and its lists' text.
fn operands(text: &str, g: &Graph, s: &Stmt) -> (Vec<std::ops::Range<usize>>, String) {
    let Kind::Edge { ops, lists } = &s.kind else {
        unreachable!("an edge statement")
    };
    let starts: Vec<usize> = ops
        .iter()
        .map(|o| match o {
            Operand::Node { span, .. } => span.start,
            Operand::Group(_) => unreachable!("groups are locked"),
        })
        .collect();
    let last = lists.first().map_or(s.span.end, |l| l.span.start);
    let ranges = (0..starts.len())
        .map(|j| {
            let to = starts.get(j + 1).map_or(last, |&n| {
                starts[j] + text[starts[j]..n].rfind(g.op()).expect("an operator")
            });
            starts[j]..text[..to].trim_end().len()
        })
        .collect();
    let attrs = match (lists.first(), lists.last()) {
        (Some(f), Some(l)) => format!(" {}", &text[f.span.start..l.span.end]),
        _ => String::new(),
    };
    (ranges, attrs)
}

/// Edge statement `i` as `pieces` (runs of its operands), each keeping its
/// attributes; a lone operand stays as a node statement only when its box
/// would otherwise vanish (and isn't `dropped`).
fn split(
    text: &str,
    g: &Graph,
    i: usize,
    pieces: &[std::ops::Range<usize>],
    dropped: Option<&str>,
) -> (Span, String) {
    let s = &g.stmts[i];
    let (ranges, attrs) = operands(text, g, s);
    let Kind::Edge { ops, .. } = &s.kind else {
        unreachable!()
    };
    let name = |j: usize| match &ops[j] {
        Operand::Node { name, .. } => name.as_str(),
        Operand::Group(_) => unreachable!(),
    };
    let mut elsewhere = Vec::new();
    for (k, t) in g.stmts.iter().enumerate() {
        if k != i {
            collect_names(std::slice::from_ref(t), &mut elsewhere);
        }
    }
    let kept: Vec<String> = pieces
        .iter()
        .filter_map(|p| match p.len() {
            0 => None,
            1 => {
                let n = name(p.start);
                let other = pieces
                    .iter()
                    .any(|q| q != p && q.len() > 1 && q.clone().any(|j| name(j) == n));
                (Some(n) != dropped && !other && !elsewhere.iter().any(|e| e == n))
                    .then(|| text[ranges[p.start].clone()].to_string())
            }
            _ => Some(format!(
                "{}{attrs}",
                &text[ranges[p.start].start..ranges[p.end - 1].end]
            )),
        })
        .collect();
    if kept.is_empty() {
        remove_stmt(text, s)
    } else {
        (s.span, kept.join("; "))
    }
}

/// Link `k` of statement `i` cut out.
fn cut(text: &str, g: &Graph, e: &Edge) -> (Span, String) {
    let k = e.chain.map_or(0, |c| c.0);
    let Kind::Edge { ops, .. } = &g.stmts[e.stmt].kind else {
        unreachable!()
    };
    split(text, g, e.stmt, &[0..k + 1, k + 1..ops.len()], None)
}

/// An arrow as its own statement (out of its chain), returning that statement's index.
fn isolate(doc: &mut Doc, e: &Edge) -> Result<usize> {
    let Some((k, _)) = e.chain else {
        return Ok(e.stmt);
    };
    let g = doc.graph()?;
    let (r, attrs) = operands(&doc.0, &g, &g.stmts[e.stmt]);
    let alone = format!(
        "{} {} {}{attrs}",
        &doc.0[r[k].clone()],
        g.op(),
        &doc.0[r[k + 1].clone()]
    );
    doc.apply(cut(&doc.0, &g, e));
    let g = doc.graph()?;
    doc.apply(add_stmt(&doc.0, &g, &alone));
    Ok(doc.graph()?.stmts.len() - 1)
}

/// An arrow from `a` to `b` added at the end.
fn link(doc: &mut Doc, a: &str, b: &str) -> Result<()> {
    let g = doc.graph()?;
    node(&g, a)?;
    node(&g, b)?;
    let s = format!("{} {} {}", dot_value(a), g.op(), dot_value(b));
    doc.apply(add_stmt(&doc.0, &g, &s));
    Ok(())
}

/// `qd dot add`: a new box, optionally placed, styled and connected.
#[allow(clippy::too_many_arguments)]
pub fn add(
    d: &Deck,
    r: &str,
    label: Option<&str>,
    at: Option<(f64, f64)>,
    look: Option<&str>,
    look_of_node: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    // boxes that are there now: never the new box's own name
    for n in [from, to].into_iter().flatten() {
        node(&g, n)?;
    }
    let name = fresh(&g);
    let mut attrs: Look = label
        .map(|l| ("label".into(), l.into()))
        .into_iter()
        .collect();
    attrs.extend(chosen_look(&g, look, look_of_node)?.unwrap_or_default());
    let at = match at {
        Some(_) if !g.pinned() => {
            return Err(QdError::InvalidArg(format!(
                "the diagram in {cell} isn't pinned, so --at has no effect: run `qd dot pin {cell}` first"
            )));
        }
        Some((x, y)) => Some((snap(x), snap(y))),
        // a pinned diagram stays pinned: right of the rightmost box
        None if g.pinned() => g
            .nodes()
            .iter()
            .filter_map(|n| n.pos())
            .map(|(x, y, _)| (x + 2.0, y))
            .max_by(|a, b| a.0.total_cmp(&b.0)),
        None => None,
    };
    if let Some((x, y)) = at {
        attrs.push(("pos".into(), pos_value(x, y)));
    }
    let mut doc = Doc(text.into());
    doc.apply(add_stmt(text, &g, &node_stmt(&name, &attrs)));
    for (a, b) in [(from, Some(name.as_str())), (Some(name.as_str()), to)] {
        if let (Some(a), Some(b)) = (a, b) {
            link(&mut doc, a, b)?;
        }
    }
    Ok((
        doc.patch("dot-add", format!("{cell}:{name}"), body),
        json!({"node": name}),
    ))
}

/// `qd dot connect`: an arrow from `a` to `b`.
pub fn connect(d: &Deck, r: &str, a: &str, b: &str) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    let k = 1 + g
        .edges()
        .iter()
        .filter(|e| e.from == a && e.to == b)
        .count();
    let mut doc = Doc(text.into());
    link(&mut doc, a, b)?;
    let e = format!("{a}{}{b}#{k}", g.op());
    Ok((
        doc.patch("dot-connect", format!("{cell}:{e}"), body),
        json!({"edge": e}),
    ))
}

/// `qd dot set`: attributes of a box or an arrow (an empty value removes one),
/// and for a box, a look from `--look`/`--look-of`.
pub fn set(
    d: &Deck,
    r: &str,
    target: &str,
    pairs: &[String],
    look: Option<&str>,
    look_of_node: Option<&str>,
) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    let mut kv: Vec<(String, String)> = Vec::new();
    for p in pairs {
        let (k, v) = p
            .split_once('=')
            .ok_or_else(|| QdError::InvalidArg(format!("'{p}': write KEY=VALUE")))?;
        if !(k.chars().next().is_some_and(is_id_start) && k.chars().all(is_id_char)) {
            return Err(QdError::InvalidArg(format!(
                "'{k}' isn't an attribute name"
            )));
        }
        kv.push((k.into(), v.into()));
    }
    let mut doc = Doc(text.into());
    if target.contains(g.op()) {
        if look.is_some() || look_of_node.is_some() {
            return Err(QdError::InvalidArg(
                "looks are for boxes, not arrows".into(),
            ));
        }
        let i = isolate(&mut doc, &edge(&g, target)?)?;
        for (k, v) in &kv {
            let g = doc.graph()?;
            let s = &g.stmts[i];
            let Kind::Edge { lists, .. } = &s.kind else {
                unreachable!()
            };
            let e = if v.is_empty() {
                unset_in(&doc.0, lists, k)
            } else {
                Some(set_in(lists, s.span.end, k, &dot_value(v)))
            };
            e.into_iter().for_each(|e| doc.apply(e));
        }
    } else {
        node(&g, target)?;
        if let Some(l) = chosen_look(&g, look, look_of_node)? {
            for k in LOOK {
                let v = l.iter().find(|(lk, _)| lk == k).map_or("", |(_, v)| v);
                kv.push((k.into(), v.into()));
            }
        }
        for (k, v) in &kv {
            if !v.is_empty() {
                let g = doc.graph()?;
                doc.apply(set_node_attr(&doc.0, &g, target, k, v)?);
                continue;
            }
            // every statement of the box that writes it
            loop {
                let g = doc.graph()?;
                let e = g.stmts.iter().find_map(|s| match &s.kind {
                    Kind::Node { name, lists, .. } if name == target => unset_in(&doc.0, lists, k),
                    _ => None,
                });
                match e {
                    Some(e) => doc.apply(e),
                    None => break,
                }
            }
        }
    }
    if !target.contains(g.op()) {
        fit_label(&mut doc, target)?;
    }
    let p = doc.patch("dot-set", format!("{cell}:{target}"), body);
    if !kv.iter().any(|(k, v)| k == "image" && !v.is_empty()) {
        return Ok((p, json!({})));
    }
    // a picture needs the dot-image filter: named in the deck's `filters:` (research R5)
    let after = crate::ops::apply(&d.text, &p)?;
    let after = crate::plugins::with_filters(&after, &[crate::plugins::DOT_IMAGE])?;
    let whole = Span {
        start: 0,
        end: d.text.len(),
    };
    Ok((
        crate::ops::patch(&p.op, p.target, vec![(whole, after)], vec![]),
        json!({"extension": crate::plugins::DOT_IMAGE.name}),
    ))
}

/// `qd dot rename`: a box's label.
pub fn rename(d: &Deck, r: &str, name: &str, label: &str) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    let mut doc = Doc(text.into());
    doc.apply(set_node_attr(text, &g, name, "label", label)?);
    fit_label(&mut doc, name)?;
    Ok((
        doc.patch("dot-rename", format!("{cell}:{name}"), body),
        json!({}),
    ))
}

/// `qd dot rm`: a box and every arrow touching it, or one arrow.
pub fn rm(d: &Deck, r: &str, target: &str) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    let mut doc = Doc(text.into());
    let removed = if target.contains(g.op()) {
        let e = edge(&g, target)?;
        doc.apply(cut(text, &g, &e));
        json!({"nodes": [], "edges": [edge_name(&g, &e)]})
    } else {
        node(&g, target)?;
        let edges: Vec<String> = g
            .edges()
            .iter()
            .filter(|e| e.from == target || e.to == target)
            .map(|e| edge_name(&g, e))
            .collect();
        loop {
            let g = doc.graph()?;
            let hit = g.stmts.iter().enumerate().find_map(|(i, s)| match &s.kind {
                Kind::Node { name, .. } if name == target => Some(remove_stmt(&doc.0, s)),
                Kind::Edge { ops, .. } => ops
                    .iter()
                    .position(|o| matches!(o, Operand::Node { name, .. } if name == target))
                    .map(|k| split(&doc.0, &g, i, &[0..k, k + 1..ops.len()], Some(target))),
                _ => None,
            });
            match hit {
                Some(e) => doc.apply(e),
                None => break,
            }
        }
        if doc.graph()?.nodes().iter().any(|n| n.name == target) {
            return Err(QdError::UnsafeEdit(format!(
                "'{target}' is also used inside a subgraph; edit it as text"
            )));
        }
        json!({"nodes": [target], "edges": edges})
    };
    Ok((
        doc.patch("dot-rm", format!("{cell}:{target}"), body),
        json!({"removed": removed}),
    ))
}

/// `qd dot flip`: an arrow reversed (in place, or out of its chain).
pub fn flip(d: &Deck, r: &str, target: &str) -> Result<(crate::ops::Patch, Value)> {
    let (cell, body, text, g) = read(d, r)?;
    if !g.directed {
        return Err(QdError::InvalidArg(format!(
            "the diagram in {cell} is undirected (graph): its lines have no direction to flip"
        )));
    }
    let e = edge(&g, target)?;
    let mut doc = Doc(text.into());
    let i = isolate(&mut doc, &e)?;
    let g = doc.graph()?;
    let (r, _) = operands(&doc.0, &g, &g.stmts[i]);
    let t = &doc.0;
    let swapped = format!(
        "{}{}{}",
        &t[r[1].clone()],
        &t[r[0].end..r[1].start],
        &t[r[0].clone()]
    );
    doc.apply((
        Span {
            start: r[0].start,
            end: r[1].end,
        },
        swapped,
    ));
    let g = doc.graph()?;
    let flipped = g
        .edges()
        .into_iter()
        .find(|e| e.stmt == i)
        .expect("the flipped arrow");
    let name = edge_name(&g, &flipped);
    Ok((
        doc.patch("dot-flip", format!("{cell}:{target}"), body),
        json!({"edge": name}),
    ))
}

/// `qd dot new`: the starter diagram before or after a block, else at the end of slide `on`.
pub fn new(d: &Deck, on: Option<&str>, at: crate::ops::Pos) -> Result<(crate::ops::Patch, Value)> {
    use crate::ops::Pos;
    let slide = match (&at, on) {
        (Pos::After(r) | Pos::Before(r), _) => d.element(r)?.0.id.clone(),
        (Pos::End, Some(s)) => s.to_string(),
        (Pos::End, None) => {
            return Err(QdError::InvalidArg(
                "give --on SLIDE, --after REF or --before REF".into(),
            ));
        }
    };
    let mut p = crate::ops::block_add(d, &slide, at, &starter_diagram())?;
    p.op = "dot-new".into();
    let cell = p.target.clone();
    Ok((p, json!({"cell": cell})))
}

/// Box centres in inches from a rendered page: the `{dot}` cell tagged
/// `data-qd="CELL"` in the preview (points / 72, y up, offset to 0, 0).
pub fn positions_from_svg(
    html: &str,
    cell: &str,
) -> Option<std::collections::HashMap<String, (f64, f64)>> {
    let at = html.find(&format!("data-qd=\"{cell}\""))?;
    let start = at + html[at..].find("<svg")?;
    let svg = &html[start..start + html[start..].find("</svg>")?];
    let title =
        regex::Regex::new(r#"class="node[^"]*">\s*<title>(.*?)</title>"#).expect("valid regex");
    let points = regex::Regex::new(r#"points="([^"]+)"|cx="([-\d.]+)" cy="([-\d.]+)""#)
        .expect("valid regex");
    let marks: Vec<(usize, String)> = title
        .captures_iter(svg)
        .map(|c| {
            (
                c.get(0).expect("match").end(),
                crate::inline::unescape(&c[1].replace("&#45;", "-")),
            )
        })
        .collect();
    let mut out = std::collections::HashMap::new();
    for (k, (from, name)) in marks.iter().enumerate() {
        let to = marks.get(k + 1).map_or(svg.len(), |m| m.0);
        let c = points.captures(&svg[*from..to])?;
        let (x, y) = match (c.get(1), c.get(2), c.get(3)) {
            (Some(p), _, _) => {
                let pts: Vec<(f64, f64)> = p
                    .as_str()
                    .split_whitespace()
                    .filter_map(|xy| parse_pos(xy).map(|(x, y, _)| (x, y)))
                    .collect();
                let (xs, ys): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
                let mid = |v: &[f64]| {
                    (v.iter().cloned().fold(f64::MAX, f64::min)
                        + v.iter().cloned().fold(f64::MIN, f64::max))
                        / 2.0
                };
                (mid(&xs), mid(&ys))
            }
            (None, Some(x), Some(y)) => (x.as_str().parse().ok()?, y.as_str().parse().ok()?),
            _ => return None,
        };
        out.insert(name.clone(), (x / 72.0, -y / 72.0));
    }
    normalised(out)
}

/// Box centres in inches from Graphviz itself (`dot -Tjson`), when it is on PATH.
pub fn positions_from_graphviz(
    text: &str,
) -> Option<std::collections::HashMap<String, (f64, f64)>> {
    use std::io::Write;
    let mut child = std::process::Command::new("dot")
        .arg("-Tjson")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(text.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    let mut m = std::collections::HashMap::new();
    for o in v["objects"].as_array()? {
        let (Some(name), Some(pos)) = (o["name"].as_str(), o["pos"].as_str()) else {
            continue;
        };
        let (x, y, _) = parse_pos(pos)?;
        m.insert(name.to_string(), (x / 72.0, y / 72.0));
    }
    normalised(m)
}

/// The lowest, leftmost box at 0, 0, two decimals.
fn normalised(
    m: std::collections::HashMap<String, (f64, f64)>,
) -> Option<std::collections::HashMap<String, (f64, f64)>> {
    if m.is_empty() {
        return None;
    }
    let x0 = m.values().map(|p| p.0).fold(f64::MAX, f64::min);
    let y0 = m.values().map(|p| p.1).fold(f64::MAX, f64::min);
    let r = |v: f64| (v * 100.0).round() / 100.0;
    Some(
        m.into_iter()
            .map(|(k, (x, y))| (k, (r(x - x0), r(y - y0))))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dot")
    }

    fn corpus(name: &str) -> String {
        std::fs::read_to_string(dir().join(name)).unwrap()
    }

    fn names(g: &Graph) -> Vec<String> {
        g.nodes().into_iter().map(|n| n.name).collect()
    }

    fn edges(g: &Graph) -> Vec<String> {
        g.edges()
            .into_iter()
            .map(|e| format!("{}{}{}#{}", e.from, g.op(), e.to, e.index))
            .collect()
    }

    /// Every corpus file reads; statement spans are in order, inside the
    /// graph's braces, and each covers its own text.
    #[test]
    fn reads_the_corpus_with_spans() {
        let mut n = 0;
        for f in std::fs::read_dir(dir()).unwrap() {
            let f = f.unwrap().file_name().into_string().unwrap();
            if f.starts_with("bad-") {
                continue;
            }
            n += 1;
            let text = corpus(&f);
            let g = parse(&text).unwrap_or_else(|s| panic!("{f}: {s:?}"));
            assert!(g.open < g.close && text[g.close..].starts_with('}'), "{f}");
            let mut at = g.open;
            for s in &g.stmts {
                assert!(s.span.start >= at && s.span.end <= g.close, "{f}: {s:?}");
                assert!(!text[s.span.start..s.span.end].trim().is_empty(), "{f}");
                at = s.span.end;
            }
        }
        assert!(n >= 15);
    }

    #[test]
    fn settings_defaults_nodes_and_attributes() {
        let text = corpus("03-strict-settings.dot");
        let g = parse(&text).unwrap();
        assert!(g.directed && g.strict);
        assert_eq!(g.id.as_deref(), Some("My graph"));
        assert_eq!(g.setting("rankdir").as_deref(), Some("LR"));
        assert_eq!(g.setting("label").as_deref(), Some("A \"quoted\" title"));
        assert_eq!(names(&g), ["a", "b", "c"]);
        let e = &g.edges()[0];
        let attrs: Vec<(&str, &str)> = e
            .attrs
            .iter()
            .map(|a| (a.key.as_str(), a.value.as_str()))
            .collect();
        assert_eq!(
            attrs,
            [("label", "first"), ("weight", "2"), ("color", "red")],
            "both lists"
        );
        // an attribute's value span is the value as written, quotes included
        let label = &e.attrs[0];
        assert_eq!(
            &text[label.value_span.start..label.value_span.end],
            "\"first\""
        );
        assert!(label.quoted);
    }

    #[test]
    fn html_labels_numbers_and_unicode_ids() {
        let g = parse(&corpus("04-html-labels.dot")).unwrap();
        let t = &g.nodes()[0];
        assert!(t.attr("label").unwrap().starts_with("<TABLE"));
        assert!(t.attr("label").unwrap().ends_with("</TABLE>"));
        let g = parse(&corpus("10-numbers-ids.dot")).unwrap();
        assert_eq!(
            names(&g),
            ["1", "-2.5", ".5", "_x9", "with space", "node_é"]
        );
    }

    #[test]
    fn chains_ports_and_repeats() {
        let g = parse(&corpus("05-chains-ports.dot")).unwrap();
        assert_eq!(
            edges(&g),
            ["a->b#1", "b->c#1", "c->d#1", "a->d#1", "b->c#2", "a->b#2"]
        );
        let first = &g.edges()[0];
        assert_eq!(
            first.chain,
            Some((0, 3)),
            "first of three links in its statement"
        );
        assert_eq!(
            first.attrs[0].value, "blue",
            "the chain's list applies to each link"
        );
        assert_eq!(g.edges()[3].from_port.as_deref(), Some("e"));
        let g = parse(&corpus("02-undirected.dot")).unwrap();
        assert!(!g.directed);
        assert_eq!(edges(&g), ["a--b#1", "b--c#1", "b--d#1"]);
        let g = parse(&corpus("11-repeated-edges.dot")).unwrap();
        assert_eq!(edges(&g), ["a->b#1", "a->b#2", "b->a#1"]);
    }

    #[test]
    fn subgraphs_are_read_only() {
        let g = parse(&corpus("06-subgraph-cluster.dot")).unwrap();
        let locked: Vec<bool> = g.nodes().iter().map(|n| n.locked).collect();
        assert_eq!(names(&g), ["start", "mid", "x", "y", "end", "p", "q", "r"]);
        assert_eq!(locked, [false, false, true, true, true, true, true, true]);
        let e: Vec<(String, bool)> = g
            .edges()
            .iter()
            .map(|e| (format!("{}->{}", e.from, e.to), e.locked))
            .collect();
        assert!(
            e.contains(&("x->y".into(), true)) && e.contains(&("start->mid".into(), false)),
            "{e:?}"
        );
    }

    #[test]
    fn comments_and_line_endings() {
        let g = parse(&corpus("07-comments.dot")).unwrap();
        assert_eq!(names(&g), ["a", "b", "c"]);
        assert_eq!(
            g.nodes()[2].attr("label").as_deref(),
            Some("not // a comment")
        );
        let text = corpus("08-crlf.dot");
        assert!(text.contains("\r\n"));
        let g = parse(&text).unwrap();
        assert_eq!(g.nodes()[1].pos(), Some((1.0, 1.0, true)));
    }

    #[test]
    fn pinned_only_with_neato_and_every_pos_pinned() {
        let g = parse(&corpus("09-pinned.dot")).unwrap();
        assert!(g.pinned());
        assert_eq!(g.nodes()[3].pos(), Some((5.0, 1.5, true)));
        assert!(!parse(&corpus("05-chains-ports.dot")).unwrap().pinned());
        assert!(
            !parse("digraph { layout=dot; a [pos=\"0,0!\"] }")
                .unwrap()
                .pinned()
        );
        assert!(
            !parse("digraph { layout=neato; a [pos=\"0,0!\"]; b [pos=\"1,1\"] }")
                .unwrap()
                .pinned()
        );
        assert!(
            !parse("digraph { layout=neato; a [pos=\"0,0!\"]; a -> b }")
                .unwrap()
                .pinned(),
            "b has no pos"
        );
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        for (f, says) in [
            ("bad-unclosed.dot", "end"),
            ("bad-two-graphs.dot", "one graph"),
            ("bad-stray.dot", ";"),
        ] {
            let stop = parse(&corpus(f)).unwrap_err();
            assert!(stop.reason.contains(says), "{f}: {stop:?}");
            assert!(stop.at <= corpus(f).len());
        }
    }

    // ---------- patches (T007) ----------

    /// `text` with one edit applied (spans relative to `text`).
    fn applied(text: &str, (span, new): (Span, String)) -> String {
        format!("{}{new}{}", &text[..span.start], &text[span.end..])
    }

    const PINNED: &str = "digraph {\n  layout=neato; splines=ortho;\n  a [label=\"A\", pos=\"0,1!\"];\n  b [pos=\"2,1!\"]\n  c\n  a -> b; b -> c;\n}\n";

    #[test]
    fn sets_an_attribute_in_place_or_adds_it() {
        let g = parse(PINNED).unwrap();
        // an existing value, quoted: only the value changes
        let out = applied(
            PINNED,
            set_node_attr(PINNED, &g, "a", "pos", "1.5,1!").unwrap(),
        );
        assert_eq!(out, PINNED.replace("pos=\"0,1!\"", "pos=\"1.5,1!\""));
        // a new attribute in an existing list
        let out = applied(
            PINNED,
            set_node_attr(PINNED, &g, "b", "fillcolor", "white").unwrap(),
        );
        assert_eq!(
            out,
            PINNED.replace("b [pos=\"2,1!\"]", "b [pos=\"2,1!\", fillcolor=white]")
        );
        // a statement without a list gets one
        let out = applied(
            PINNED,
            set_node_attr(PINNED, &g, "c", "pos", "4,1!").unwrap(),
        );
        assert_eq!(out, PINNED.replace("  c\n", "  c [pos=\"4,1!\"]\n"));
        // bare values stay bare when they can
        let t = "digraph { a [shape=box] }";
        let g = parse(t).unwrap();
        assert_eq!(
            applied(t, set_node_attr(t, &g, "a", "shape", "ellipse").unwrap()),
            "digraph { a [shape=ellipse] }"
        );
    }

    #[test]
    fn a_node_only_in_edges_gets_its_own_statement() {
        let t = "digraph {\n  layout=neato;\n  a -> b;\n}\n";
        let g = parse(t).unwrap();
        let out = applied(t, set_node_attr(t, &g, "b", "pos", "1,0!").unwrap());
        assert_eq!(
            out,
            "digraph {\n  layout=neato;\n  a -> b;\n  b [pos=\"1,0!\"];\n}\n"
        );
        // one line, no statement terminators: before the closing brace
        let t = "digraph { a -> b }";
        let g = parse(t).unwrap();
        assert_eq!(
            applied(t, set_node_attr(t, &g, "a", "pos", "0,0!").unwrap()),
            "digraph { a -> b a [pos=\"0,0!\"] }"
        );
    }

    #[test]
    fn crlf_and_locked_nodes() {
        let t = "digraph {\r\n  layout=neato;\r\n  a -> b;\r\n}\r\n";
        let g = parse(t).unwrap();
        let out = applied(t, set_node_attr(t, &g, "a", "pos", "0,0!").unwrap());
        assert_eq!(
            out,
            "digraph {\r\n  layout=neato;\r\n  a -> b;\r\n  a [pos=\"0,0!\"];\r\n}\r\n"
        );
        let t = corpus("06-subgraph-cluster.dot");
        let g = parse(&t).unwrap();
        assert_eq!(
            set_node_attr(&t, &g, "x", "pos", "0,0!")
                .unwrap_err()
                .code(),
            "E_UNSAFE_EDIT"
        );
    }

    #[test]
    fn settings_replace_or_join() {
        let t = "digraph {\n  layout=dot\n  graph [splines=true];\n  a\n}\n";
        let g = parse(t).unwrap();
        let out = applied(t, set_setting(t, &g, "layout", "neato"));
        assert_eq!(out, t.replace("layout=dot", "layout=neato"));
        let out = applied(t, set_setting(t, &g, "splines", "ortho"));
        assert_eq!(out, t.replace("splines=true", "splines=ortho"));
        let t = "digraph {\n  a;\n}\n";
        let g = parse(t).unwrap();
        assert_eq!(
            applied(t, set_setting(t, &g, "layout", "neato")),
            "digraph {\n  a;\n  layout=neato;\n}\n"
        );
    }

    #[test]
    fn positions_snap_and_print_short() {
        assert_eq!(snap(1.13), 1.25);
        assert_eq!(snap(-0.12), 0.0);
        assert_eq!(pos_value(1.5, 2.0), "1.5,2!");
        assert_eq!(pos_value(0.333, -1.0), "0.33,-1!");
    }

    #[test]
    fn values_are_quoted_only_when_needed() {
        for (v, want) in [
            ("box", "box"),
            ("12.5", "12.5"),
            ("rounded,filled", "\"rounded,filled\""),
            ("a \"b\"", "\"a \\\"b\\\"\""),
            ("", "\"\""),
            ("node", "\"node\""),
        ] {
            assert_eq!(dot_value(v), want, "{v}");
        }
    }

    /// One edit applied to `text`.
    fn edited(text: &str, f: impl Fn(&str, &Graph) -> (Span, String)) -> String {
        let mut doc = Doc(text.into());
        let g = doc.graph().unwrap();
        doc.apply(f(text, &g));
        doc.0
    }

    #[test]
    fn add_stmt_follows_its_neighbours() {
        let t = "digraph {\n    a;\n    b;\n}\n";
        assert_eq!(
            edited(t, |t, g| add_stmt(t, g, "c")),
            "digraph {\n    a;\n    b;\n    c;\n}\n"
        );
        let t = "digraph {\n  a\n}";
        assert_eq!(
            edited(t, |t, g| add_stmt(t, g, "c")),
            "digraph {\n  a\n  c\n}"
        );
        let t = "digraph { a; }";
        assert_eq!(edited(t, |t, g| add_stmt(t, g, "c")), "digraph { a; c; }");
    }

    #[test]
    fn remove_stmt_alone_and_sharing_a_line() {
        let t = "digraph {\n  a;\n  b; c;\n}\n";
        let g = parse(t).unwrap();
        assert_eq!(
            edited(t, |t, _| remove_stmt(t, &g.stmts[0])),
            "digraph {\n  b; c;\n}\n"
        );
        assert_eq!(
            edited(t, |t, _| remove_stmt(t, &g.stmts[1])),
            "digraph {\n  a;\n  c;\n}\n"
        );
        assert_eq!(
            edited(t, |t, _| remove_stmt(t, &g.stmts[2])),
            "digraph {\n  a;\n  b;\n}\n"
        );
    }

    #[test]
    fn cut_splits_a_chain_keeping_its_attributes() {
        let t = "digraph {\n  a -> b:p -> c [color=red];\n}\n";
        let g = parse(t).unwrap();
        let e = g.edges().into_iter().find(|e| e.from == "b").unwrap();
        assert_eq!(
            edited(t, |t, g| cut(t, g, &e)),
            "digraph {\n  a -> b:p [color=red]; c;\n}\n",
            "c would vanish: kept as its own statement"
        );
        let t = "digraph {\n  c;\n  a -> b -> c [color=red];\n}\n";
        let g = parse(t).unwrap();
        let e = &g.edges()[0];
        assert_eq!(
            edited(t, |t, g| cut(t, g, e)),
            "digraph {\n  c;\n  a; b -> c [color=red];\n}\n"
        );
    }

    #[test]
    fn unset_takes_its_separator_and_empty_lists() {
        let t = "digraph {\n  a [x=1, y=2];\n  b [z=3];\n}\n";
        let g = parse(t).unwrap();
        let lists = |i: usize| match &g.stmts[i].kind {
            Kind::Node { lists, .. } => lists.clone(),
            _ => unreachable!(),
        };
        assert_eq!(
            edited(t, |t, _| unset_in(t, &lists(0), "x").unwrap()),
            "digraph {\n  a [y=2];\n  b [z=3];\n}\n"
        );
        assert_eq!(
            edited(t, |t, _| unset_in(t, &lists(0), "y").unwrap()),
            "digraph {\n  a [x=1];\n  b [z=3];\n}\n"
        );
        assert_eq!(
            edited(t, |t, _| unset_in(t, &lists(1), "z").unwrap()),
            "digraph {\n  a [x=1, y=2];\n  b;\n}\n"
        );
    }

    /// Spec 015 US3 (T019): a picture's spacer label wraps the label's text;
    /// a new label keeps the spacer; no picture, a plain label again.
    #[test]
    fn spacer_labels() {
        let mut doc = Doc("digraph {\n  t [label=\"A & B\", image=\"f.png\"];\n}\n".into());
        fit_label(&mut doc, "t").unwrap();
        assert_eq!(
            doc.0,
            "digraph {\n  t [label=<<TABLE BORDER=\"0\" CELLPADDING=\"0\" CELLSPACING=\"4\"><TR><TD WIDTH=\"28\" HEIGHT=\"28\" FIXEDSIZE=\"TRUE\"></TD><TD>A &amp; B</TD></TR></TABLE>>, image=\"f.png\"];\n}\n"
        );
        let n = || doc.graph().unwrap().nodes()[0].clone();
        assert_eq!(label(&n()), "A & B", "the text cell");
        // a new label keeps the spacer
        let g = doc.graph().unwrap();
        let e = set_node_attr(&doc.0, &g, "t", "label", "C").unwrap();
        doc.apply(e);
        fit_label(&mut doc, "t").unwrap();
        assert!(doc.0.contains("<TD>C</TD>"), "{}", doc.0);
        // taller box, bigger spacer
        let g = doc.graph().unwrap();
        let e = set_node_attr(&doc.0, &g, "t", "height", "1").unwrap();
        doc.apply(e);
        fit_label(&mut doc, "t").unwrap();
        assert!(doc.0.contains("WIDTH=\"64\" HEIGHT=\"64\""), "{}", doc.0);
        // no picture: plain
        let g = doc.graph().unwrap();
        let e = set_node_attr(&doc.0, &g, "t", "image", "").unwrap();
        doc.apply(e);
        fit_label(&mut doc, "t").unwrap();
        assert!(
            doc.0.contains("t [label=C, image=\"\", height=1]"),
            "{}",
            doc.0
        );
    }

    #[test]
    fn fresh_names_looks_and_the_starter() {
        let g = parse("digraph { n1; n3; x [shape=box]; y [shape=box]; z [shape=oval] }").unwrap();
        assert_eq!(fresh(&g), "n2");
        assert_eq!(
            starter("tool").unwrap()[2],
            ("fillcolor".into(), "#3d4a5c".into())
        );
        assert_eq!(
            starter("artifact").unwrap()[2],
            ("fillcolor".into(), "white".into())
        );
        assert!(starter("other").is_none());
        let cell = starter_diagram();
        assert!(cell.starts_with("```{dot}\n") && cell.ends_with("\n```"));
        let g = parse(&cell[9..cell.len() - 4]).unwrap();
        assert!(g.pinned());
        let looks: Vec<Look> = g.nodes().iter().map(look_of).collect();
        assert_eq!(
            looks,
            [starter("artifact").unwrap(), starter("tool").unwrap()]
        );
        assert_eq!(edges(&g), ["data->tool#1"]);
    }

    #[test]
    fn undirected_graphs_and_subgraphs() {
        let g = parse("graph { a -- b; subgraph s { c -- d } }").unwrap();
        assert_eq!(g.op(), "--");
        assert!(edge(&g, "a--b").is_ok());
        assert!(matches!(edge(&g, "c--d"), Err(QdError::UnsafeEdit(_))));
        assert!(matches!(node(&g, "c"), Err(QdError::UnsafeEdit(_))));
        assert!(matches!(edge(&g, "a->b"), Err(QdError::InvalidArg(_))));
    }
}
