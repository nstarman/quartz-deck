//! Arrows on the slide (spec 016): a slide paragraph holding one
//! quarto-arrows shortcode, `{{< arrow from="x,y" to="x,y" position="absolute" … >}}`,
//! in the slide's own pixels (research R1, R4). The reader keeps every
//! option's span, so an edit changes only what it changes.

use crate::deck::{BlockKind, Deck, Span};
pub use crate::dot::Stop;
use crate::dot::stop;
use crate::error::{QdError, Result};
use serde_json::{Value, json};

/// One option, `key=value`, with where its value sits in the text read
/// (quotes included) and the quote it was written with.
#[derive(Debug)]
pub struct Opt {
    pub key: String,
    pub value: String,
    /// Start of the key .. end of the value, quotes included.
    pub span: (usize, usize),
    pub quote: Option<char>,
}

/// An arrow qd can edit.
#[derive(Debug)]
pub struct Arrow {
    pub from: (f64, f64),
    pub to: (f64, f64),
    /// `control1`, `control2` when written.
    pub control: Vec<(f64, f64)>,
    pub opts: Vec<Opt>,
    /// Where `>}}` starts: new options go before it.
    pub close: usize,
}

/// `text` as an arrow: `None` when it isn't one at all, `Err` with the
/// reason when it is but can't be edited on the slide.
pub fn read(text: &str) -> Option<std::result::Result<Arrow, Stop>> {
    let after = text
        .trim_start()
        .strip_prefix("{{<")?
        .trim_start()
        .strip_prefix("arrow")?;
    if !after.starts_with(|c: char| c.is_whitespace() || c == '>') {
        return None; // `{{< arrows >}}`, another shortcode
    }
    Some(options(text, text.len() - after.len()).and_then(|(opts, close)| arrow(text, opts, close)))
}

/// The options from `at` to `>}}`, and where `>}}` starts.
fn options(text: &str, mut at: usize) -> std::result::Result<(Vec<Opt>, usize), Stop> {
    let b = text.as_bytes();
    let mut opts = vec![];
    loop {
        while at < b.len() && b[at].is_ascii_whitespace() {
            at += 1;
        }
        if b[at..].starts_with(b">}}") {
            return Ok((opts, at));
        }
        if at >= b.len() {
            return Err(stop("the shortcode isn't closed with >}}", at));
        }
        let key_at = at;
        while at < b.len() && (b[at].is_ascii_alphanumeric() || b[at] == b'-' || b[at] == b'_') {
            at += 1;
        }
        if at == key_at || b.get(at) != Some(&b'=') {
            return Err(stop(
                "a value without a name (qd edits only key=value options)",
                key_at,
            ));
        }
        let key = text[key_at..at].to_string();
        at += 1;
        let (value, quote) = match b.get(at) {
            Some(&q @ (b'"' | b'\'')) => {
                let end = text[at + 1..]
                    .find(q as char)
                    .map(|n| at + 1 + n)
                    .ok_or_else(|| stop(format!("{key}'s value has no closing quote"), at))?;
                let v = &text[at + 1..end];
                if v.contains(">}}") {
                    return Err(stop(
                        format!("{key}'s value holds >}}}}, which ends the shortcode"),
                        at,
                    ));
                }
                at = end + 1;
                if !(at == b.len() || b[at].is_ascii_whitespace() || b[at..].starts_with(b">}}")) {
                    return Err(stop(format!("{key}'s quotes don't pair up"), end));
                }
                (v.to_string(), Some(q as char))
            }
            _ => {
                let start = at;
                while at < b.len() && !b[at].is_ascii_whitespace() && !b[at..].starts_with(b">}}") {
                    if b[at] == b'"' || b[at] == b'\'' {
                        return Err(stop(format!("{key}'s value has a stray quote"), at));
                    }
                    at += 1;
                }
                (text[start..at].to_string(), None)
            }
        };
        opts.push(Opt {
            key,
            value,
            span: (key_at, at),
            quote,
        });
    }
}

/// `x,y`.
fn point(v: &str) -> Option<(f64, f64)> {
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn arrow(text: &str, opts: Vec<Opt>, close: usize) -> std::result::Result<Arrow, Stop> {
    if !text[close + 3..].trim().is_empty() {
        return Err(stop("the arrow isn't alone in its paragraph", close + 3));
    }
    let get = |k: &str| opts.iter().rev().find(|o| o.key == k);
    let at = |k: &str| get(k).map_or(close, |o| o.span.0);
    if get("position").map(|o| o.value.as_str()) != Some("absolute") {
        return Err(stop(
            "the arrow sits in the text's flow (it has no position=\"absolute\")",
            at("position"),
        ));
    }
    let need = |k: &str| {
        get(k)
            .and_then(|o| point(&o.value))
            .ok_or_else(|| stop(format!("{k} isn't a point x,y"), at(k)))
    };
    let control = ["control1", "control2"]
        .iter()
        .filter(|k| get(k).is_some())
        .map(|k| need(k))
        .collect::<std::result::Result<_, _>>()?;
    Ok(Arrow {
        from: need("from")?,
        to: need("to")?,
        control,
        opts,
        close,
    })
}

// ---------- editing (spec 016 US1, research R4) ----------

/// `x,y` in whole slide pixels.
pub fn pt(p: (f64, f64)) -> String {
    format!("{},{}", p.0.round() as i64, p.1.round() as i64)
}

/// `v` quoted for a shortcode, keeping `quote` (the one it was written with,
/// `None` for bare) where the value allows it.
fn quoted(v: &str, quote: Option<char>) -> Result<String> {
    if v.contains(">}}") {
        return Err(QdError::InvalidArg(format!(
            "{v:?}: a value can't hold >}}}}"
        )));
    }
    let bare = !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "#.,_-%".contains(c));
    Ok(match quote {
        None if bare => v.to_string(),
        Some('\'') if !v.contains('\'') => format!("'{v}'"),
        _ if !v.contains('"') => format!("\"{v}\""),
        _ if !v.contains('\'') => format!("'{v}'"),
        _ => {
            return Err(QdError::InvalidArg(format!(
                "{v:?} holds both kinds of quote, which a shortcode can't"
            )));
        }
    })
}

/// The change to `text` (an arrow's paragraph) that sets its option `o` to
/// `v` in place, keeping its quote, or (`None`) removes it and its space.
/// Offsets are in `text`.
fn change(text: &str, o: &Opt, v: Option<&str>) -> Result<(Span, String)> {
    Ok(match v {
        Some(v) => (
            Span {
                start: o.span.0,
                end: o.span.1,
            },
            format!("{}={}", o.key, quoted(v, o.quote)?),
        ),
        None => (
            Span {
                start: text[..o.span.0].trim_end().len(),
                end: o.span.1,
            },
            String::new(),
        ),
    })
}

/// A new arrow's line: start and end in slide pixels, then `opts` in order.
pub fn line(from: (f64, f64), to: (f64, f64), opts: &[(&str, &str)]) -> Result<String> {
    let mut s = format!(
        "{{{{< arrow from=\"{}\" to=\"{}\" position=\"absolute\"",
        pt(from),
        pt(to)
    );
    for (k, v) in opts {
        s.push_str(&format!(" {k}={}", quoted(v, Some('"'))?));
    }
    Ok(s + " >}}")
}

/// The editable arrow at `r`: its ref, paragraph span and text, and the arrow.
fn editable<'a>(d: &'a Deck, r: &str) -> Result<(String, Span, &'a str, Arrow)> {
    let (r, span, a) = block(d, r)?;
    let a = a.map_err(|s| QdError::UnsafeEdit(format!("{r}: {}; edit it as text", s.reason)))?;
    Ok((r, span, d.slice(span), a))
}

/// `qd arrow add SLIDE`: after the slide's content, before its notes.
pub fn add(
    d: &Deck,
    slide: &str,
    from: (f64, f64),
    to: (f64, f64),
    opts: &[(&str, &str)],
) -> Result<(crate::ops::Patch, Value)> {
    use crate::ops::{Pos, block_add};
    let s = d.slide(slide)?;
    // checked as `set` checks them; defaults aren't written
    let opts: Vec<_> = set_changes(opts.iter().copied())?
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .collect();
    let text = line(from, to, &opts)?;
    let notes = s
        .blocks
        .iter()
        .position(|b| b.kind == BlockKind::Notes)
        .map(|i| format!("{}/{i}", s.id));
    let mut p = block_add(
        d,
        slide,
        notes.as_deref().map_or(Pos::End, Pos::Before),
        &text,
    )?;
    p.op = "arrow add".into();
    let r = p.target.clone();
    Ok((p, json!({"ref": r})))
}

/// `qd arrow move REF`: the start, the end, both, or (`by`) the whole arrow,
/// control points included.
pub fn move_arrow(
    d: &Deck,
    r: &str,
    from: Option<(f64, f64)>,
    to: Option<(f64, f64)>,
    by: Option<(f64, f64)>,
) -> Result<(crate::ops::Patch, Value)> {
    let (r, span, text, a) = editable(d, r)?;
    if (from, to, by) == (None, None, None) {
        return Err(QdError::InvalidArg("give --from, --to or --by".into()));
    }
    let shift = |p: (f64, f64)| by.map_or(p, |b| (p.0 + b.0, p.1 + b.1));
    let from = from.or(by.map(|_| shift(a.from)));
    let to = to.or(by.map(|_| shift(a.to)));
    // only what moves is written: the other end keeps its bytes
    let mut changes: Vec<(&str, String)> = [("from", from), ("to", to)]
        .into_iter()
        .filter_map(|(k, p)| Some((k, pt(p?))))
        .collect();
    if by.is_some() {
        for (k, c) in ["control1", "control2"].into_iter().zip(&a.control) {
            changes.push((k, pt(shift(*c))));
        }
    }
    let (from, to) = (from.unwrap_or(a.from), to.unwrap_or(a.to));
    let changes: Vec<_> = changes
        .iter()
        .map(|(k, v)| (*k, Some(v.as_str())))
        .collect();
    let p = patch("arrow move", &r, span, text, &a, &changes)?;
    Ok((p, json!({"ref": r, "from": from, "to": to})))
}

/// The patch making `changes` (`None` removes) to the arrow `a`, whose
/// paragraph `text` is at `span`. New options go in together after the last
/// one, in the order given.
fn patch(
    op: &str,
    r: &str,
    span: Span,
    text: &str,
    a: &Arrow,
    changes: &[(&str, Option<&str>)],
) -> Result<crate::ops::Patch> {
    let mut reps = vec![];
    let mut added = String::new();
    for (k, v) in changes {
        match (a.opts.iter().rev().find(|o| o.key == *k), v) {
            (Some(o), v) => reps.push(change(text, o, *v)?),
            (None, Some(v)) => added.push_str(&format!(" {k}={}", quoted(v, Some('"'))?)),
            (None, None) => {}
        }
    }
    if !added.is_empty() {
        let at = a.opts.last().map_or(a.close, |o| o.span.1);
        reps.push((Span { start: at, end: at }, added));
    }
    let reps = reps
        .into_iter()
        .map(|(s, new)| {
            (
                Span {
                    start: span.start + s.start,
                    end: span.start + s.end,
                },
                new,
            )
        })
        .collect();
    Ok(crate::ops::patch(op, r.to_string(), reps, vec![]))
}

/// A value `qd arrow set` accepts.
enum Val {
    Text,
    Number,
    Bool,
    OneOf(&'static [&'static str]),
}

/// The options `set` changes, with quarto-arrows' default (not written).
/// Ends, curve and placement have their own commands.
const OPTIONS: &[(&str, Val, Option<&str>)] = &[
    ("color", Val::Text, Some("black")),
    ("width", Val::Number, Some("2")),
    ("dash", Val::Text, None),
    (
        "line",
        Val::OneOf(&["single", "dot", "double", "triple"]),
        Some("single"),
    ),
    (
        "head",
        Val::OneOf(&[
            "arrow", "stealth", "diamond", "circle", "dot", "square", "bar", "barbed", "none",
        ]),
        Some("arrow"),
    ),
    ("head-start", Val::Bool, Some("false")),
    ("head-end", Val::Bool, Some("true")),
    ("head-fill", Val::Bool, Some("true")),
    ("size", Val::Number, Some("10")),
    ("head-size", Val::Number, None),
    ("opacity", Val::Number, Some("1")),
    ("label", Val::Text, None),
    (
        "label-position",
        Val::OneOf(&["start", "middle", "end"]),
        Some("middle"),
    ),
    ("label-offset", Val::Number, Some("10")),
    ("fragment", Val::OneOf(&["true", "draw"]), None),
    ("fragment-index", Val::Number, None),
    ("aria-label", Val::Text, None),
    ("alt", Val::Text, None),
    ("title", Val::Text, None),
];

/// `KEY=VALUE` pairs as changes: checked, and a default value removes the option.
fn set_changes<'a>(
    pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Vec<(&'static str, Option<&'a str>)>> {
    pairs
        .into_iter()
        .map(|(k, v)| {
            let bad = |why: String| QdError::InvalidArg(format!("{k}={v}: {why}"));
            let (k, val, default) = OPTIONS
                .iter()
                .find(|o| o.0 == k)
                .ok_or_else(|| bad("not an option qd arrow set changes (ends: qd arrow move; curve: qd arrow bend)".into()))?;
            if v.is_empty() || Some(v) == *default {
                return Ok((*k, None));
            }
            let why = match val {
                Val::Number if v.parse::<f64>().is_err() => Some("a number".into()),
                Val::Bool if v != "true" && v != "false" => Some("true or false".into()),
                Val::OneOf(xs) if !xs.contains(&v) => Some(format!("one of {}", xs.join(", "))),
                _ => None,
            };
            match why {
                Some(w) => Err(bad(w)),
                None => Ok((*k, Some(v))),
            }
        })
        .collect()
}

/// `qd arrow set REF KEY=VALUE…`.
pub fn set(d: &Deck, r: &str, pairs: &[(String, String)]) -> Result<(crate::ops::Patch, Value)> {
    let (r, span, text, a) = editable(d, r)?;
    let pairs = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str()));
    let p = patch("arrow set", &r, span, text, &a, &set_changes(pairs)?)?;
    Ok((p, json!({"ref": r})))
}

/// The one control point, written as both `control1` and `control2`, that
/// makes the curve from `p0` to `p1` pass through `m` at its middle
/// (research R6); `None` when `m` is within 4 px of the straight line.
pub fn bend_through(p0: (f64, f64), p1: (f64, f64), m: (f64, f64)) -> Option<(f64, f64)> {
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let len = dx.hypot(dy);
    let off = if len == 0.0 {
        (m.0 - p0.0).hypot(m.1 - p0.1)
    } else {
        ((m.0 - p0.0) * dy - (m.1 - p0.1) * dx).abs() / len
    };
    (off >= 4.0).then(|| {
        (
            (8.0 * m.0 - p0.0 - p1.0) / 6.0,
            (8.0 * m.1 - p0.1 - p1.1) / 6.0,
        )
    })
}

/// `qd arrow bend REF --through X,Y | --straight`: a hand-written `curve` or
/// `bend` gives way to the control points.
pub fn bend(d: &Deck, r: &str, through: Option<(f64, f64)>) -> Result<(crate::ops::Patch, Value)> {
    let (r, span, text, a) = editable(d, r)?;
    let c = through.and_then(|m| bend_through(a.from, a.to, m)).map(pt);
    let changes = [
        ("control1", c.as_deref()),
        ("control2", c.as_deref()),
        ("curve", None),
        ("bend", None),
    ];
    let p = patch("arrow bend", &r, span, text, &a, &changes)?;
    Ok((p, json!({"ref": r, "straight": c.is_none()})))
}

/// `qd arrow rm REF`.
pub fn rm(d: &Deck, r: &str) -> Result<crate::ops::Patch> {
    let _ = block(d, r)?; // an arrow, editable or not
    let mut p = crate::ops::block_rm(d, r)?;
    p.op = "arrow rm".into();
    Ok(p)
}

/// The arrow at `r`: its canonical ref, its paragraph's span, and what was read.
fn block(d: &Deck, r: &str) -> Result<(String, Span, std::result::Result<Arrow, Stop>)> {
    let (_, b) = d.element(r)?;
    let text = d.slice(b.span);
    let a = (b.kind == BlockKind::Para)
        .then(|| read(text))
        .flatten()
        .ok_or_else(|| QdError::NotFound(format!("{r} is not an arrow")))?;
    Ok((crate::focus::resolve(d, r)?.1, b.span, a))
}

/// An arrow as `qd arrow show` prints it (contracts/cli.md).
fn show_json(d: &Deck, r: &str, span: Span, a: &std::result::Result<Arrow, Stop>) -> Value {
    match a {
        Ok(a) => json!({"ok": true, "ref": r, "from": a.from, "to": a.to,
            "control": (!a.control.is_empty()).then_some(&a.control),
            "options": a.opts.iter().map(|o| (o.key.clone(), json!(o.value))).collect::<serde_json::Map<_, _>>(),
            "editable": true}),
        Err(s) => {
            json!({"ok": true, "ref": r, "editable": crate::dot::stopped_at(d, span.start, s.clone())})
        }
    }
}

/// `qd arrow show REF`.
pub fn show(d: &Deck, r: &str) -> Result<Value> {
    let (r, span, a) = block(d, r)?;
    Ok(show_json(d, &r, span, &a))
}

/// `qd arrow list SLIDE`: the slide's arrows, in order.
pub fn list(d: &Deck, slide: &str) -> Result<Value> {
    let s = d.slide(slide)?;
    let arrows: Vec<Value> = (0..s.blocks.len())
        .filter_map(|i| block(d, &format!("{}/{i}", s.id)).ok())
        .map(|(r, span, a)| show_json(d, &r, span, &a))
        .collect();
    Ok(json!({"ok": true, "arrows": arrows}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &str) -> Arrow {
        read(t)
            .expect("an arrow")
            .unwrap_or_else(|s| panic!("{t}: {}", s.reason))
    }
    fn why(t: &str) -> String {
        read(t).expect("an arrow").expect_err(t).reason
    }

    #[test]
    fn non_ascii_value_is_read_whole() {
        let a = ok(r#"{{< arrow from="10,20" to="300,400" position="absolute" label=café >}}"#);
        assert!(a.opts.iter().any(|o| o.key == "label" && o.value == "café"));
    }

    #[test]
    fn reads_an_arrow() {
        let t =
            r#"{{< arrow from="100,200" to='300, 50' position="absolute" color=red width=3 >}}"#;
        let a = ok(t);
        assert_eq!((a.from, a.to), ((100.0, 200.0), (300.0, 50.0)));
        assert!(a.control.is_empty());
        let keys: Vec<_> = a
            .opts
            .iter()
            .map(|o| (o.key.as_str(), o.value.as_str(), o.quote))
            .collect();
        assert_eq!(
            keys,
            [
                ("from", "100,200", Some('"')),
                ("to", "300, 50", Some('\'')),
                ("position", "absolute", Some('"')),
                ("color", "red", None),
                ("width", "3", None)
            ]
        );
        // spans cover the key through the closing quote
        let c = &a.opts[3];
        assert_eq!(&t[c.span.0..c.span.1], "color=red");
        assert_eq!(&t[a.opts[1].span.0..a.opts[1].span.1], "to='300, 50'");
        assert_eq!(&t[a.close..], ">}}");
    }

    #[test]
    fn control_points_and_spacing() {
        let a = ok(
            "{{<arrow  from=\"0,0\"\tto=\"10,0\" position=\"absolute\" control1=\"3,4\" control2=\"7,4\"  >}}",
        );
        assert_eq!(a.control, [(3.0, 4.0), (7.0, 4.0)]);
    }

    #[test]
    fn read_only_reasons() {
        assert!(why(r#"{{< arrow from="0,0" to="10,0" >}}"#).contains("flow"));
        assert!(why(r#"{{< arrow from="0,0" to="10,0" position="fixed" >}}"#).contains("flow"));
        assert!(why(r#"{{< arrow from="0,0 to="10,0" position="absolute" >}}"#).contains("quote"));
        assert!(
            why(r#"{{< arrow from="0,0" to="10,0" position="absolute" label="a >}} b" >}}"#)
                .contains(">}}")
        );
        assert!(
            why(r#"{{< arrow from="0,0" to="1,1" position="absolute" >}} and text"#)
                .contains("alone")
        );
        assert!(why(r#"{{< arrow from="0,0" to="1,1" position="absolute" >}} {{< arrow from="0,0" to="1,1" position="absolute" >}}"#).contains("alone"));
        assert!(why(r#"{{< arrow from="x,0" to="1,1" position="absolute" >}}"#).contains("from"));
        assert!(why(r#"{{< arrow to="1,1" position="absolute" >}}"#).contains("from"));
        assert!(
            why(r#"{{< arrow from="0,0" to="1,1" position="absolute" curvy >}}"#).contains("name")
        );
    }

    /// `changes` made to `t`, as a command makes them.
    fn applied(t: &str, changes: &[(&str, Option<&str>)]) -> Result<String> {
        let p = patch(
            "t",
            "t",
            Span {
                start: 0,
                end: t.len(),
            },
            t,
            &ok(t),
            changes,
        )?;
        crate::ops::apply(t, &p)
    }
    fn patched(t: &str, key: &str, v: Option<&str>) -> String {
        applied(t, &[(key, v)]).unwrap()
    }

    #[test]
    fn patches_keep_the_rest() {
        let t = r#"{{< arrow from="100,200" to='300, 50' position="absolute" color=red >}}"#;
        // in place, each quote kept; a bare value stays bare while it can
        assert_eq!(
            patched(t, "to", Some("310,60")),
            t.replace("'300, 50'", "'310,60'")
        );
        assert_eq!(
            patched(t, "color", Some("blue")),
            t.replace("color=red", "color=blue")
        );
        assert_eq!(
            patched(t, "color", Some("light blue")),
            t.replace("color=red", r#"color="light blue""#)
        );
        // added after the last option; removed with its space
        assert_eq!(
            patched(t, "width", Some("4")),
            t.replace("color=red", r#"color=red width="4""#)
        );
        assert_eq!(patched(t, "color", None), t.replace(" color=red", ""));
        assert_eq!(
            patched(t, "dash", None),
            t,
            "removing an absent option changes nothing"
        );
        // a value with a double quote is written with single quotes
        assert_eq!(
            patched(t, "label", Some(r#"say "hi""#)),
            t.replace("color=red", r#"color=red label='say "hi"'"#)
        );
        assert!(applied(t, &[("label", Some(r#"it's "x""#))]).is_err());
        assert!(applied(t, &[("label", Some("a >}} b"))]).is_err());
    }

    #[test]
    fn new_arrow_line() {
        assert_eq!(
            line(
                (100.4, 199.6),
                (300.0, 50.0),
                &[("color", "red"), ("label", "the peak")]
            )
            .unwrap(),
            r#"{{< arrow from="100,200" to="300,50" position="absolute" color="red" label="the peak" >}}"#
        );
        let a = ok(&line((0.0, 0.0), (10.0, -5.0), &[]).unwrap());
        assert!(line((0.0, 0.0), (1.0, 1.0), &[("label", "a >}} b")]).is_err());
        assert_eq!((a.from, a.to), ((0.0, 0.0), (10.0, -5.0)));
    }

    #[test]
    fn bend_passes_through_the_point() {
        let c = bend_through((0.0, 0.0), (100.0, 0.0), (50.0, 50.0)).unwrap();
        // a cubic with both controls at c passes (p0 + 6c + p1) / 8 at its middle
        let mid = |a: f64, b: f64, c: f64| (a + 6.0 * c + b) / 8.0;
        assert!(
            (mid(0.0, 100.0, c.0) - 50.0).abs() < 1e-9 && (mid(0.0, 0.0, c.1) - 50.0).abs() < 1e-9,
            "{c:?}"
        );
        assert!(
            bend_through((0.0, 0.0), (100.0, 0.0), (50.0, 3.0)).is_none(),
            "near the line: straight"
        );
        assert!(bend_through((0.0, 0.0), (100.0, 100.0), (52.0, 48.0)).is_none());
    }

    fn set(t: &str, pairs: &[&str]) -> Result<String> {
        let pairs = pairs.iter().map(|p| p.split_once('=').unwrap_or((p, "")));
        applied(t, &set_changes(pairs)?)
    }

    #[test]
    fn set_style() {
        let t = r#"{{< arrow from="0,0" to="10,10" position="absolute" color=red >}}"#;
        // a default isn't written: color=black removes it, width=2 adds nothing
        assert_eq!(
            set(t, &["color=black", "width=2"]).unwrap(),
            t.replace(" color=red", "")
        );
        assert_eq!(
            set(t, &["width=4", "head=stealth", "dash=true"]).unwrap(),
            t.replace(
                "color=red",
                r#"color=red width="4" head="stealth" dash="true""#
            )
        );
        assert_eq!(set(t, &["color="]).unwrap(), t.replace(" color=red", ""));
        assert_eq!(
            set(t, &["head-start=true", "head-end=false"]).unwrap(),
            t.replace(
                "color=red",
                r#"color=red head-start="true" head-end="false""#
            )
        );
        for bad in [
            "head=spiral",
            "width=wide",
            "head-start=yes",
            "label-position=left",
            "nope=1",
            "from=1,2",
            "position=fixed",
            "noequals",
        ] {
            assert!(set(t, &[bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn not_arrows() {
        for t in [
            "Some text.",
            "{{< pagebreak >}}",
            "An {{< arrow from=\"0,0\" to=\"1,1\" >}} inline",
            "{{< arrows >}}",
        ] {
            assert!(read(t).is_none(), "{t}");
        }
    }
}
