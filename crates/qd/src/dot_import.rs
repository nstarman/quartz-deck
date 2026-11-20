//! `qd dot import` (spec 017): a drawing, given as SVG, into one pinned
//! `{dot}` cell (spec 015), so a hand-drawn diagram becomes one the person
//! can edit on the slide. The drawing's rectangles, circles and ellipses with
//! their text become boxes; its lines between two boxes become arrows;
//! everything else is reported and left out (research R1–R6).

use crate::deck::Deck;
use crate::error::{QdError, Result};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::LazyLock;

/// An end within this many drawing units of a box's edge is at that box (R4).
const NEAR: f64 = 8.0;
/// Else, extended this far along its direction, the first box it meets.
const REACH: f64 = 24.0;

/// A 2×3 affine matrix `[a b c d e f]`: x' = a·x + c·y + e, y' = b·x + d·y + f.
type M = [f64; 6];
const ID: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
type P = (f64, f64);

/// `p` then `q`: the matrix that applies `q` (the child's) first, then `p`.
fn mul(p: &M, q: &M) -> M {
    [
        p[0] * q[0] + p[2] * q[1],
        p[1] * q[0] + p[3] * q[1],
        p[0] * q[2] + p[2] * q[3],
        p[1] * q[2] + p[3] * q[3],
        p[0] * q[4] + p[2] * q[5] + p[4],
        p[1] * q[4] + p[3] * q[5] + p[5],
    ]
}

fn at(m: &M, (x, y): P) -> P {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// The numbers in `s` (separated by spaces and commas, signs and exponents allowed).
fn numbers(s: &str) -> Vec<f64> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?").expect("valid regex")
    });
    RE.find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}

/// An SVG `transform` list as one matrix (translate, scale, rotate, skew, matrix).
fn transform(s: &str) -> M {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\w+)\s*\(([^)]*)\)").expect("valid regex"));
    RE.captures_iter(s).fold(ID, |m, c| {
        let v = numbers(&c[2]);
        let g = |i: usize, d: f64| v.get(i).copied().unwrap_or(d);
        let t = match &c[1] {
            "translate" => [1.0, 0.0, 0.0, 1.0, g(0, 0.0), g(1, 0.0)],
            "scale" => [g(0, 1.0), 0.0, 0.0, g(1, g(0, 1.0)), 0.0, 0.0],
            "rotate" => {
                let (s, c) = g(0, 0.0).to_radians().sin_cos();
                let r = [c, s, -s, c, 0.0, 0.0];
                let (cx, cy) = (g(1, 0.0), g(2, 0.0));
                mul(
                    &mul(&[1.0, 0.0, 0.0, 1.0, cx, cy], &r),
                    &[1.0, 0.0, 0.0, 1.0, -cx, -cy],
                )
            }
            "skewX" => [1.0, 0.0, g(0, 0.0).to_radians().tan(), 1.0, 0.0, 0.0],
            "skewY" => [1.0, g(0, 0.0).to_radians().tan(), 0.0, 1.0, 0.0, 0.0],
            "matrix" if v.len() == 6 => [v[0], v[1], v[2], v[3], v[4], v[5]],
            _ => ID,
        };
        mul(&m, &t)
    })
}

/// A box's area in the drawing: left, top, right, bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect(f64, f64, f64, f64);

impl Rect {
    /// The axis-aligned box around these points.
    fn around(pts: &[P]) -> Rect {
        let (xs, ys): (Vec<f64>, Vec<f64>) = pts.iter().copied().unzip();
        let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
        let max = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
        Rect(min(&xs), min(&ys), max(&xs), max(&ys))
    }
    fn contains(&self, (x, y): P) -> bool {
        x >= self.0 && x <= self.2 && y >= self.1 && y <= self.3
    }
    /// How far `p` is from the box (0 inside).
    fn distance(&self, (x, y): P) -> f64 {
        let dx = (self.0 - x).max(x - self.2).max(0.0);
        let dy = (self.1 - y).max(y - self.3).max(0.0);
        dx.hypot(dy)
    }
    fn area(&self) -> f64 {
        (self.2 - self.0) * (self.3 - self.1)
    }
    fn centre(&self) -> P {
        ((self.0 + self.2) / 2.0, (self.1 + self.3) / 2.0)
    }
}

/// A transformed ellipse's box: the four extreme points of the transformed curve.
fn ellipse_box(m: &M, (cx, cy): P, rx: f64, ry: f64) -> Rect {
    let pts: Vec<P> = (0..64)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 64.0;
            at(m, (cx + rx * t.cos(), cy + ry * t.sin()))
        })
        .collect();
    Rect::around(&pts)
}

/// A line's two ends, each with the direction pointing out of the line there.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ends {
    start: P,
    out_start: P,
    end: P,
    out_end: P,
}

fn ends_of(pts: &[P]) -> Option<Ends> {
    let mut pts = pts.to_vec();
    pts.dedup();
    if pts.len() < 2 {
        return None;
    }
    let d = |a: P, b: P| (a.0 - b.0, a.1 - b.1);
    let n = pts.len();
    Some(Ends {
        start: pts[0],
        out_start: d(pts[0], pts[1]),
        end: pts[n - 1],
        out_end: d(pts[n - 1], pts[n - 2]),
    })
}

/// The points of a path's `d` that set its ends and their directions: each
/// segment's control points and end (arcs as straight to their end).
fn path_points(d: &str) -> Vec<P> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"([MmLlHhVvCcSsQqTtAaZz])([^MmLlHhVvCcSsQqTtAaZz]*)").expect("valid regex")
    });
    let mut pts: Vec<P> = Vec::new();
    let (mut cur, mut first) = ((0.0, 0.0), (0.0, 0.0));
    for c in RE.captures_iter(d) {
        let cmd = c[1].chars().next().expect("a command");
        let v = numbers(&c[2]);
        let rel = cmd.is_ascii_lowercase();
        let off = |p: P, cur: P| if rel { (p.0 + cur.0, p.1 + cur.1) } else { p };
        // how many numbers one step of the command takes
        let step = match cmd.to_ascii_uppercase() {
            'M' | 'L' | 'T' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            'A' => 7,
            _ => 0,
        };
        if step == 0 {
            cur = first;
            pts.push(cur);
            continue;
        }
        for (k, chunk) in v.chunks(step).filter(|ch| ch.len() == step).enumerate() {
            let base = cur;
            match cmd.to_ascii_uppercase() {
                'H' => cur = (if rel { base.0 + chunk[0] } else { chunk[0] }, base.1),
                'V' => cur = (base.0, if rel { base.1 + chunk[0] } else { chunk[0] }),
                'A' => cur = off((chunk[5], chunk[6]), base),
                _ => {
                    for xy in chunk.chunks(2) {
                        pts.push(off((xy[0], xy[1]), base));
                    }
                    cur = *pts.last().expect("a point");
                    pts.pop();
                }
            }
            if cmd.eq_ignore_ascii_case(&'M') && k == 0 {
                first = cur;
            }
            pts.push(cur);
        }
    }
    pts
}

/// A presentation property: its attribute, else its `style` declaration.
fn prop(n: roxmltree::Node, key: &str) -> Option<String> {
    let from_style = n.attribute("style").and_then(|s| {
        s.split(';').find_map(|decl| {
            let (k, v) = decl.split_once(':')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
    });
    from_style.or_else(|| n.attribute(key).map(str::to_string))
}

/// The properties a box or a text takes from its groups.
#[derive(Clone, Default)]
struct Inherited {
    m: M,
    fill: Option<String>,
    stroke: Option<String>,
    font_size: Option<f64>,
    href: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Rect { rounded: bool },
    Ellipse,
    Circle,
}

#[derive(Clone, Debug)]
struct Shape {
    kind: Kind,
    area: Rect,
    fill: Option<String>,
    stroke: Option<String>,
    href: Option<String>,
}

#[derive(Clone, Debug)]
struct Text {
    at: P,
    text: String,
    fill: Option<String>,
    size: Option<f64>,
    href: Option<String>,
}

#[derive(Clone, Debug)]
struct Line {
    ends: Ends,
    head_start: bool,
    head_end: bool,
}

#[derive(Clone, Debug)]
struct Picture {
    href: String,
    area: Rect,
}

/// What the drawing holds (data-model "Drawing").
#[derive(Default, Debug)]
struct Drawing {
    graphviz: bool,
    shapes: Vec<Shape>,
    texts: Vec<Text>,
    lines: Vec<Line>,
    pictures: Vec<Picture>,
    /// elements qd doesn't read: (tag, where)
    other: Vec<(String, P)>,
}

/// A length attribute in drawing units (a trailing `px` allowed).
fn len(n: roxmltree::Node, key: &str) -> f64 {
    n.attribute(key)
        .and_then(|v| v.trim().trim_end_matches("px").parse().ok())
        .unwrap_or(0.0)
}

/// A link's or a picture's target (`href`, or SVG 1.1's `xlink:href`).
fn href<'a>(n: roxmltree::Node<'a, '_>) -> Option<&'a str> {
    n.attribute("href")
        .or_else(|| n.attribute(("http://www.w3.org/1999/xlink", "href")))
}

fn marker(n: roxmltree::Node, key: &str) -> bool {
    prop(n, key).is_some_and(|v| v != "none")
}

/// How much a matrix scales lengths (for font sizes).
fn scale(m: &M) -> f64 {
    (m[0] * m[3] - m[1] * m[2]).abs().sqrt()
}

/// Read an SVG file's text into its shapes, texts, lines and pictures (R1, R2).
fn read(svg: &str) -> Result<Drawing> {
    let doc = roxmltree::Document::parse(svg)
        .map_err(|e| QdError::InvalidArg(format!("not SVG: {e}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(QdError::InvalidArg("not SVG: the root isn't <svg>".into()));
    }
    let mut d = Drawing {
        graphviz: root
            .descendants()
            .any(|n| n.attribute("class") == Some("graph")),
        ..Default::default()
    };
    walk(
        root,
        &Inherited {
            m: ID,
            ..Default::default()
        },
        &mut d,
    );
    Ok(d)
}

fn walk(n: roxmltree::Node, up: &Inherited, d: &mut Drawing) {
    for c in n.children().filter(|c| c.is_element()) {
        let tag = c.tag_name().name();
        // definitions and metadata: never drawn as they stand
        if matches!(
            tag,
            "defs"
                | "marker"
                | "clipPath"
                | "mask"
                | "pattern"
                | "symbol"
                | "title"
                | "desc"
                | "metadata"
                | "style"
                | "script"
        ) {
            continue;
        }
        let mut here = up.clone();
        if let Some(t) = c.attribute("transform") {
            here.m = mul(&up.m, &transform(t));
        }
        here.fill = prop(c, "fill").or(here.fill);
        here.stroke = prop(c, "stroke").or(here.stroke);
        if let Some(s) =
            prop(c, "font-size").and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
        {
            here.font_size = Some(s * scale(&here.m));
        }
        let m = &here.m;
        let none = |v: &Option<String>| v.clone().filter(|v| v != "none");
        match tag {
            "g" | "svg" | "switch" => walk(c, &here, d),
            "a" => {
                here.href = href(c).map(str::to_string);
                walk(c, &here, d);
            }
            "rect" => {
                let (x, y, w, h) = (len(c, "x"), len(c, "y"), len(c, "width"), len(c, "height"));
                let pts = [
                    at(m, (x, y)),
                    at(m, (x + w, y)),
                    at(m, (x, y + h)),
                    at(m, (x + w, y + h)),
                ];
                d.shapes.push(Shape {
                    kind: Kind::Rect {
                        rounded: len(c, "rx") > 0.0 || len(c, "ry") > 0.0,
                    },
                    area: Rect::around(&pts),
                    fill: none(&here.fill),
                    stroke: none(&here.stroke),
                    href: here.href.clone(),
                });
            }
            "circle" | "ellipse" => {
                let (rx, ry) = if tag == "circle" {
                    (len(c, "r"), len(c, "r"))
                } else {
                    (len(c, "rx"), len(c, "ry"))
                };
                d.shapes.push(Shape {
                    kind: if tag == "circle" {
                        Kind::Circle
                    } else {
                        Kind::Ellipse
                    },
                    area: ellipse_box(m, (len(c, "cx"), len(c, "cy")), rx, ry),
                    fill: none(&here.fill),
                    stroke: none(&here.stroke),
                    href: here.href.clone(),
                });
            }
            "text" => text(c, &here, d),
            "line" | "polyline" | "path" => {
                let pts: Vec<P> = match tag {
                    "line" => vec![(len(c, "x1"), len(c, "y1")), (len(c, "x2"), len(c, "y2"))],
                    "polyline" => numbers(c.attribute("points").unwrap_or(""))
                        .chunks(2)
                        .filter(|p| p.len() == 2)
                        .map(|p| (p[0], p[1]))
                        .collect(),
                    _ => path_points(c.attribute("d").unwrap_or("")),
                };
                let pts: Vec<P> = pts.into_iter().map(|p| at(m, p)).collect();
                let (head_start, head_end) = (marker(c, "marker-start"), marker(c, "marker-end"));
                // a filled outline without markers is a drawn shape, not a line (spec edge case)
                let outline =
                    tag == "path" && !head_start && !head_end && none(&here.fill).is_some();
                match ends_of(&pts) {
                    Some(ends) if !outline => d.lines.push(Line {
                        ends,
                        head_start,
                        head_end,
                    }),
                    _ => d
                        .other
                        .push((tag.to_string(), pts.first().copied().unwrap_or_default())),
                }
            }
            "image" => {
                let (x, y, w, h) = (len(c, "x"), len(c, "y"), len(c, "width"), len(c, "height"));
                match href(c) {
                    Some(h2) if !h2.starts_with("data:") => d.pictures.push(Picture {
                        href: h2.to_string(),
                        area: Rect::around(&[at(m, (x, y)), at(m, (x + w, y + h))]),
                    }),
                    _ => d.other.push(("image".into(), at(m, (x, y)))),
                }
            }
            _ => {
                // where it is: its first point, else its x, y
                let first = c
                    .attribute("points")
                    .map(numbers)
                    .filter(|v| v.len() >= 2)
                    .map_or((len(c, "x"), len(c, "y")), |v| (v[0], v[1]));
                d.other.push((tag.to_string(), at(m, first)));
            }
        }
    }
}

/// A `<text>`: one line, or one per `<tspan>` placed on its own `y`.
fn text(c: roxmltree::Node, here: &Inherited, d: &mut Drawing) {
    let (x, y) = (len(c, "x"), len(c, "y"));
    let mut main = String::new();
    for k in c.children() {
        if k.is_text() {
            main.push_str(k.text().unwrap_or(""));
        } else if k.tag_name().name() == "tspan" {
            let s: String = k
                .descendants()
                .filter(|t| t.is_text())
                .filter_map(|t| t.text())
                .collect();
            if k.attribute("y").is_some() {
                let p = (
                    if k.attribute("x").is_some() {
                        len(k, "x")
                    } else {
                        x
                    },
                    len(k, "y"),
                );
                push_text(d, here, at(&here.m, p), &s, prop(k, "fill"));
            } else {
                main.push_str(&s);
            }
        }
    }
    push_text(d, here, at(&here.m, (x, y)), &main, None);
}

fn push_text(d: &mut Drawing, here: &Inherited, at: P, s: &str, fill: Option<String>) {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() {
        return;
    }
    d.texts.push(Text {
        at,
        text: s,
        fill: fill.or_else(|| here.fill.clone()).filter(|v| v != "none"),
        size: here.font_size,
        href: here.href.clone(),
    });
}

/// The smallest of `shapes` holding `p`.
fn holder(shapes: &[Shape], p: P) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(_, s)| s.area.contains(p))
        .min_by(|a, b| a.1.area.area().total_cmp(&b.1.area.area()))
        .map(|(i, _)| i)
}

/// The box an end of a line is at (R4): the nearest within `NEAR`, else the
/// first one met within `REACH` along its outward direction.
fn box_at(shapes: &[Shape], p: P, out: P) -> Option<usize> {
    let near = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.area.distance(p)))
        .filter(|(_, dist)| *dist <= NEAR)
        .min_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| shapes[a.0].area.area().total_cmp(&shapes[b.0].area.area()))
        });
    if let Some((i, _)) = near {
        return Some(i);
    }
    let len = out.0.hypot(out.1);
    if len == 0.0 {
        return None;
    }
    let u = (out.0 / len, out.1 / len);
    (1..=REACH as usize).find_map(|t| holder(shapes, (p.0 + u.0 * t as f64, p.1 + u.1 * t as f64)))
}

/// A box name from its label: lowercase letters and digits (spec, the person's choice).
fn name_from(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn where_(p: P) -> String {
    format!("x={} y={}", p.0.round(), p.1.round())
}

fn inches(v: f64) -> f64 {
    (v / 72.0 * 100.0).round() / 100.0
}

/// The diagram made from a drawing: the cell's text and the report (data-model "Report").
struct Made {
    cell: String,
    boxes: Vec<Value>,
    arrows: Vec<Value>,
    left_out: Vec<Value>,
    pictures: Vec<String>,
}

fn make(d: &Drawing) -> Result<Made> {
    if d.shapes.is_empty() {
        return Err(QdError::InvalidArg(
            "no boxes in this drawing: qd reads rectangles, circles and ellipses as boxes".into(),
        ));
    }
    let mut left_out = Vec::new();
    // each text into the smallest box around it (R3)
    let mut texts: Vec<Vec<&Text>> = vec![Vec::new(); d.shapes.len()];
    for t in &d.texts {
        match holder(&d.shapes, t.at) {
            Some(i) => texts[i].push(t),
            None => left_out.push(json!({"what": format!("text \"{}\"", t.text), "at": where_(t.at), "reason": "outside every box"})),
        }
    }
    let mut pics: Vec<Option<&Picture>> = vec![None; d.shapes.len()];
    for p in &d.pictures {
        match holder(&d.shapes, p.area.centre()) {
            Some(i) if pics[i].is_none() => pics[i] = Some(p),
            _ => left_out.push(json!({"what": format!("picture {}", p.href), "at": where_(p.area.centre()), "reason": "not inside a box"})),
        }
    }
    // names: from labels, unique; n1, n2… for boxes without text
    let mut names: Vec<String> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for ts in texts.iter_mut() {
        ts.sort_by(|a, b| a.at.1.total_cmp(&b.at.1).then(a.at.0.total_cmp(&b.at.0)));
        let label = ts
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        // the label's letters and digits; `_2`… when taken; n1, n2… with no label
        let base = name_from(&label);
        let name = if !base.is_empty() && !names.contains(&base) {
            base
        } else {
            let (stem, sep, k0) = if base.is_empty() {
                ("n".to_string(), "", 1)
            } else {
                (base, "_", 2)
            };
            (k0..)
                .map(|k| format!("{stem}{sep}{k}"))
                .find(|n| !names.contains(n))
                .expect("a free name")
        };
        names.push(name);
        labels.push(label);
    }
    // centres in inches, y up, the lowest leftmost at 0,0 (as `dot pin`)
    let centres: HashMap<String, (f64, f64)> = d
        .shapes
        .iter()
        .zip(&names)
        .map(|(s, n)| {
            let (x, y) = s.area.centre();
            (n.clone(), (x / 72.0, -y / 72.0))
        })
        .collect();
    let at = crate::dot::normalised(centres).expect("boxes");
    let mut stmts = Vec::new();
    let mut boxes = Vec::new();
    for (i, s) in d.shapes.iter().enumerate() {
        let (w, h) = (s.area.2 - s.area.0, s.area.3 - s.area.1);
        let (x, y) = at[&names[i]];
        let first = texts[i].first();
        let label = match pics[i] {
            Some(_) => crate::dot::spacer(&labels[i].replace('\n', " "), inches(h)),
            None => labels[i].replace('\n', "\\n"),
        };
        let mut a: Vec<(&str, String)> = vec![
            ("label", label),
            ("pos", crate::dot::pos_value(x, y)),
            ("width", inches(w).to_string()),
            ("height", inches(h).to_string()),
            ("fixedsize", "true".into()),
            (
                "shape",
                match s.kind {
                    Kind::Rect { .. } => "box",
                    Kind::Ellipse => "ellipse",
                    Kind::Circle => "circle",
                }
                .into(),
            ),
        ];
        let rounded = matches!(s.kind, Kind::Rect { rounded: true });
        let style = [
            rounded.then_some("rounded"),
            s.fill.is_some().then_some("filled"),
        ];
        let style: Vec<&str> = style.into_iter().flatten().collect();
        if !style.is_empty() {
            a.push(("style", style.join(",")));
        }
        a.extend(s.fill.clone().map(|v| ("fillcolor", v)));
        a.extend(s.stroke.clone().map(|v| ("color", v)));
        a.extend(first.and_then(|t| t.fill.clone()).map(|v| ("fontcolor", v)));
        a.extend(
            first
                .and_then(|t| t.size)
                .map(|v| ("fontsize", ((v * 10.0).round() / 10.0).to_string())),
        );
        a.extend(
            s.href
                .clone()
                .or_else(|| first.and_then(|t| t.href.clone()))
                .map(|v| ("URL", v)),
        );
        a.extend(pics[i].map(|p| ("image", p.href.clone())));
        let a: Vec<(String, String)> = a.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        stmts.push(crate::dot::node_stmt(&names[i], &a));
        boxes.push(json!({"name": names[i], "label": labels[i]}));
    }
    // arrows: a line whose two ends are at two different boxes (R4)
    let mut arrows = Vec::new();
    for l in &d.lines {
        let e = l.ends;
        let (a, b) = (
            box_at(&d.shapes, e.start, e.out_start),
            box_at(&d.shapes, e.end, e.out_end),
        );
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) if a != b => (a, b),
            (Some(_), Some(_)) => {
                left_out.push(json!({"what": "line", "at": where_(e.start), "reason": "both ends are at the same box"}));
                continue;
            }
            _ => {
                let end = if a.is_none() { e.start } else { e.end };
                left_out.push(json!({"what": "line", "at": where_(e.start), "reason": format!("its end at {} is near no box", where_(end))}));
                continue;
            }
        };
        // the arrow points where the drawn head does
        let (from, to, extra) = match (l.head_start, l.head_end) {
            (true, false) => (b, a, None),
            (true, true) => (a, b, Some("dir=both")),
            (false, false) => (a, b, Some("arrowhead=none")),
            (false, true) => (a, b, None),
        };
        let (f, t) = (
            crate::dot::dot_value(&names[from]),
            crate::dot::dot_value(&names[to]),
        );
        stmts.push(match extra {
            Some(x) => format!("{f} -> {t} [{x}]"),
            None => format!("{f} -> {t}"),
        });
        arrows.push(json!({"from": names[from], "to": names[to]}));
    }
    for (tag, p) in &d.other {
        left_out.push(json!({"what": tag, "at": where_(*p), "reason": "not a box, line, text or picture qd reads"}));
    }
    // shown on the slide at the drawing's own size (R6): Graphviz draws the boxes'
    // extent with a 4 pt margin, and Quarto sizes the figure at 96 px an inch, so
    // a figure that size shows one drawing unit as one slide pixel
    let corners: Vec<P> = d
        .shapes
        .iter()
        .flat_map(|s| [(s.area.0, s.area.1), (s.area.2, s.area.3)])
        .collect();
    let all = Rect::around(&corners);
    let fig = |v: f64| ((v + 8.0) / 96.0 * 100.0).round() / 100.0;
    let cell = format!(
        "```{{dot}}\n//| fig-width: {}\n//| fig-height: {}\ndigraph {{\n  layout=neato; splines=ortho;\n{}}}\n```",
        fig(all.2 - all.0),
        fig(all.3 - all.1),
        stmts
            .iter()
            .map(|s| format!("  {s};\n"))
            .collect::<String>()
    );
    Ok(Made {
        cell,
        boxes,
        arrows,
        left_out,
        pictures: d.pictures.iter().map(|p| p.href.clone()).collect(),
    })
}

/// `qd dot import FILE`: the drawing in `svg` (read from `file`) as a new cell
/// at `at` (or in place of the element `replace`), one edit (R8).
pub fn import(
    d: &Deck,
    file: &str,
    svg: &str,
    on: Option<&str>,
    at: crate::ops::Pos,
    replace: Option<&str>,
    deck_dir: &std::path::Path,
) -> Result<(crate::ops::Patch, Value)> {
    use crate::ops::Pos;
    let drawing = read(svg)?;
    let made = make(&drawing)?;
    let mut p = match (replace, &at, on) {
        (Some(r), Pos::End, None) => crate::ops::edit(d, r, &made.cell, None)?,
        (Some(_), _, _) => {
            return Err(QdError::InvalidArg(
                "give one place: --on, --after, --before or --replace".into(),
            ));
        }
        (None, _, _) => crate::dot::add_cell(d, on, at, &made.cell)?,
    };
    p.op = "dot-import".into();
    let notes: Vec<String> = made
        .pictures
        .iter()
        .filter(|h| !h.contains("://") && !deck_dir.join(h).exists())
        .map(|h| format!("picture {h} not found beside the deck"))
        .collect();
    let mut report = json!({
        "cell": p.target, "kind": if drawing.graphviz { "graphviz" } else { "hand" }, "file": file,
        "boxes": made.boxes, "arrows": made.arrows, "left_out": made.left_out, "notes": notes,
    });
    if made.pictures.is_empty() {
        return Ok((p, report));
    }
    // a picture needs the dot-image filter, folded into the same edit (as `dot set image=`)
    report["extension"] = json!(crate::plugins::DOT_IMAGE.name);
    Ok((crate::dot::with_dot_image(d, p)?, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAND: &str = include_str!("../tests/fixtures/dot-import/hand.svg");
    fn close(a: P, b: P) -> bool {
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
    }

    #[test]
    fn transforms_compose() {
        let m = transform("translate(380,300) scale(2)");
        assert!(close(at(&m, (110.0, 40.0)), (600.0, 380.0)));
        let r = transform("rotate(90)");
        assert!(close(at(&r, (1.0, 0.0)), (0.0, 1.0)));
        let around = transform("rotate(180, 10, 10)");
        assert!(close(at(&around, (0.0, 0.0)), (20.0, 20.0)));
        let mx = transform("matrix(1 0 0 1 5 -5)");
        assert!(close(at(&mx, (1.0, 1.0)), (6.0, -4.0)));
        // nested: the parent's after the child's
        let nested = mul(&transform("translate(10,0)"), &transform("scale(3)"));
        assert!(close(at(&nested, (1.0, 1.0)), (13.0, 3.0)));
    }

    #[test]
    fn line_ends_and_directions() {
        let e = ends_of(&path_points("M 490 134 L 490 220 L 470 220 L 470 296")).unwrap();
        assert_eq!((e.start, e.end), ((490.0, 134.0), (470.0, 296.0)));
        assert_eq!(e.out_start, (0.0, -86.0), "out of the start: up");
        assert_eq!(e.out_end, (0.0, 76.0), "out of the end: down");
        // relative commands, H and V
        let r = ends_of(&path_points("m10 10 h20 v30")).unwrap();
        assert_eq!(
            (r.start, r.end, r.out_end),
            ((10.0, 10.0), (30.0, 40.0), (0.0, 30.0))
        );
        // a curve's direction at its end comes from its last control point
        let c = ends_of(&path_points("M0 0 C 0 50 100 50 100 0")).unwrap();
        assert_eq!(c.out_end, (0.0, -50.0));
        assert!(ends_of(&path_points("M5 5")).is_none());
    }

    #[test]
    fn reads_the_drawing() {
        let d = read(HAND).unwrap();
        assert!(!d.graphviz);
        assert_eq!(d.shapes.len(), 4);
        assert_eq!(d.shapes[1].kind, Kind::Rect { rounded: true });
        assert_eq!(d.shapes[1].fill.as_deref(), Some("#3d4a5c"), "from style");
        assert_eq!(
            d.shapes[1].href.as_deref(),
            Some("https://example.org/tool")
        );
        assert_eq!(d.shapes[2].kind, Kind::Ellipse);
        assert_eq!(
            d.shapes[3].area,
            Rect(380.0, 300.0, 600.0, 380.0),
            "the group's transform"
        );
        assert_eq!(
            d.texts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
            ["Data", "Tool", "Paper", "draft", "Results", "a note"]
        );
        assert_eq!(d.texts[4].size, Some(24.0), "scaled with its group");
        assert_eq!(
            d.lines.len(),
            4,
            "the markers' own path in <defs> isn't a line"
        );
        assert!(
            d.lines[2].head_start && d.lines[2].head_end,
            "markers from style"
        );
        assert_eq!(d.pictures.len(), 1);
        assert_eq!(
            d.other.iter().map(|o| o.0.as_str()).collect::<Vec<_>>(),
            ["polygon"]
        );
        assert!(read("not xml").is_err());
        assert!(read("<html/>").is_err());
    }

    #[test]
    fn boxes_arrows_and_what_is_left_out() {
        let m = make(&read(HAND).unwrap()).unwrap();
        let names: Vec<&str> = m
            .boxes
            .iter()
            .map(|b| b["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["data", "tool", "paperdraft", "results"]);
        assert_eq!(m.boxes[2]["label"], "Paper\ndraft");
        let arrows: Vec<(String, String)> = m
            .arrows
            .iter()
            .map(|a| {
                (
                    a["from"].as_str().unwrap().into(),
                    a["to"].as_str().unwrap().into(),
                )
            })
            .collect();
        assert_eq!(
            arrows,
            [
                ("data".into(), "tool".into()),
                ("tool".into(), "results".into()),
                ("paperdraft".into(), "results".into())
            ]
        );
        let left: Vec<String> = m
            .left_out
            .iter()
            .map(|l| format!("{}: {}", l["what"], l["reason"]))
            .collect();
        assert_eq!(left.len(), 3, "{left:?}");
        assert!(left[0].contains("a note") && left[0].contains("outside every box"));
        assert!(left[1].contains("near no box"));
        assert!(left[2].contains("polygon"));
        for want in [
            "data [label=Data, pos=\"0,3.39!\", width=2.5, height=1, fixedsize=true, shape=box, style=filled, fillcolor=white, color=\"#c8ccd2\", fontcolor=\"#1f2933\", fontsize=20]",
            "shape=ellipse",
            "label=\"Paper\\ndraft\"",
            "URL=\"https://example.org/tool\"",
            "image=\"face.png\"",
            "label=<<TABLE",
            "style=\"rounded,filled\"",
            "paperdraft -> results [dir=both]",
            "data -> tool;",
            "//| fig-width: 8.42\n//| fig-height: 3.46\n",
        ] {
            assert!(m.cell.contains(want), "{want}\n{}", m.cell);
        }
        assert!(
            crate::dot::parse(
                m.cell
                    .lines()
                    .filter(|l| !l.starts_with("```") && !l.starts_with("//|"))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_str()
            )
            .is_ok()
        );
    }

    #[test]
    fn no_boxes_is_an_error() {
        let only_lines =
            r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="0" y1="0" x2="9" y2="9"/></svg>"#;
        assert!(make(&read(only_lines).unwrap()).is_err());
    }

    #[test]
    fn names_are_letters_and_digits() {
        assert_eq!(name_from("Tool kit 2"), "toolkit2");
        assert_eq!(name_from("—"), "");
    }
}
