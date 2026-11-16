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

fn stop(reason: impl Into<String>, at: usize) -> Stop {
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
        fn walk(stmts: &[Stmt], locked: bool, out: &mut Vec<Edge>) {
            for s in stmts {
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
                                        attrs: list_attrs(lists),
                                        locked,
                                    });
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

/// `qd dot show CELL` (contracts/cli.md).
pub fn show(d: &Deck, r: &str) -> Result<Value> {
    let (cell, body) = cell(d, r)?;
    let text = d.slice(body);
    let g = match parse(text) {
        Ok(g) => g,
        Err(s) => {
            let abs = body.start + s.at;
            let line = d.lines(Span {
                start: abs,
                end: abs,
            })[0];
            let col = abs - d.text[..abs].rfind('\n').map_or(0, |n| n + 1) + 1;
            return Ok(json!({"ok": true, "kind": "dot", "cell": cell,
                "editable": {"reason": s.reason, "at": {"line": line, "column": col}}}));
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
            json!({"name": n.name, "label": n.attr("label").unwrap_or_else(|| n.name.clone()),
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
    Ok(
        json!({"ok": true, "kind": "dot", "cell": cell, "directed": g.directed, "pinned": g.pinned(),
        "editable": true, "settings": attrs_json(&g.settings()), "nodes": nodes, "edges": edges}),
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

/// Set `key` in a statement's attribute lists: its value in place, else
/// added to the last list, else a new list after the statement.
fn set_in(lists: &[List], end: usize, key: &str, value: &str) -> (Span, String) {
    let v = dot_value(value);
    if let Some(a) = lists
        .iter()
        .rev()
        .flat_map(|l| l.attrs.iter().rev())
        .find(|a| a.key == key)
    {
        return (a.value_span, v);
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

/// Set a box's attribute (one edit, relative to `text`).
pub fn set_node_attr(
    text: &str,
    g: &Graph,
    name: &str,
    key: &str,
    value: &str,
) -> Result<(Span, String)> {
    let node = g
        .nodes()
        .into_iter()
        .find(|n| n.name == name)
        .ok_or_else(|| QdError::NotFound(format!("no box named '{name}'")))?;
    if node.locked {
        return Err(QdError::UnsafeEdit(format!(
            "'{name}' is inside a subgraph; edit it as text"
        )));
    }
    let stmt = g.stmts.iter().rev().find_map(|s| match &s.kind {
        Kind::Node { name: n, lists, .. } if n == name => Some((lists, s.span.end)),
        _ => None,
    });
    Ok(match stmt {
        Some((lists, end)) => set_in(lists, end, key, value),
        None => add_stmt(
            text,
            g,
            &format!("{} [{key}={}]", dot_value(name), dot_value(value)),
        ),
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
                return set_in(lists, s.span.end, key, value);
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
    let g = parse(text).map_err(|s| {
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

/// `qd dot move`: a box's pinned position, `at` or moved `by` (inches), snapped unless `exact`.
pub fn move_node(
    d: &Deck,
    r: &str,
    node: &str,
    at: Option<(f64, f64)>,
    by: Option<(f64, f64)>,
    exact: bool,
) -> Result<(crate::ops::Patch, (f64, f64))> {
    let (cell, body, text, g) = read(d, r)?;
    if !g.pinned() {
        return Err(QdError::InvalidArg(format!(
            "the diagram in {cell} isn't pinned, so its boxes can't be placed: run `qd dot pin {cell}` first"
        )));
    }
    let n = g
        .nodes()
        .into_iter()
        .find(|n| n.name == node)
        .ok_or_else(|| QdError::NotFound(format!("no box named '{node}' in {cell}")))?;
    let (x, y, _) = n.pos().expect("pinned diagrams pin every box");
    let (mut x, mut y) = match (at, by) {
        (Some(a), _) => a,
        (None, Some((dx, dy))) => (x + dx, y + dy),
        (None, None) => return Err(QdError::InvalidArg("give --at X,Y or --by DX,DY".into())),
    };
    if !exact {
        (x, y) = (snap(x), snap(y));
    }
    let edit = set_node_attr(text, &g, node, "pos", &pos_value(x, y))?;
    let p = crate::ops::patch(
        "dot-move",
        format!("{cell}:{node}"),
        vec![(
            Span {
                start: edit.0.start + body.start,
                end: edit.0.end + body.start,
            },
            edit.1,
        )],
        vec![],
    );
    Ok((p, (x, y)))
}

/// `qd dot pin`: `layout=neato`, `splines=ortho`, and every box at `positions` (inches).
pub fn pin(
    d: &Deck,
    r: &str,
    positions: &std::collections::HashMap<String, (f64, f64)>,
) -> Result<(crate::ops::Patch, usize)> {
    let (cell, body, text, g) = read(d, r)?;
    let names: Vec<String> = g
        .nodes()
        .into_iter()
        .filter(|n| !n.locked)
        .map(|n| n.name)
        .collect();
    if let Some(missing) = names.iter().find(|n| !positions.contains_key(*n)) {
        return Err(QdError::NotFound(format!(
            "no position for '{missing}' in {cell}"
        )));
    }
    // several edits to one body: apply them in turn, then replace the body once
    let mut t = text.to_string();
    type Edit<'a> = &'a dyn Fn(&str, &Graph) -> Result<(Span, String)>;
    let mut step = |f: Edit| -> Result<()> {
        let g = parse(&t).map_err(|s| QdError::UnsafeEdit(s.reason))?;
        let (span, new) = f(&t, &g)?;
        t.replace_range(span.start..span.end, &new);
        Ok(())
    };
    step(&|t, g| Ok(set_setting(t, g, "layout", "neato")))?;
    step(&|t, g| Ok(set_setting(t, g, "splines", "ortho")))?;
    for n in &names {
        let (x, y) = positions[n];
        step(&|t, g| set_node_attr(t, g, n, "pos", &pos_value(x, y)))?;
    }
    Ok((
        crate::ops::patch("dot-pin", cell, vec![(body, t)], vec![]),
        names.len(),
    ))
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
}
