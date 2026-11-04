//! US1 op tests (T015–T017): each op's diff is pinned by a snapshot, and every
//! op must leave all non-target slides byte-identical (FR-003, SC-003).

use quartz_deck::deck::Deck;
use quartz_deck::ops::{self, AttrChange, Patch, Pos, SlideSpec};

const BASIC: &str = include_str!("fixtures/basic.qmd");

fn deck() -> Deck {
    Deck::parse(BASIC.to_string())
}

/// Apply, check non-target slides untouched (order preserved), return new deck.
fn run(d: &Deck, p: Patch, touched_before: &[&str]) -> Deck {
    let after = Deck::parse(ops::apply(&d.text, &p).unwrap());
    let before_others: Vec<&str> = d
        .slides
        .iter()
        .filter(|s| !touched_before.contains(&s.id.as_str()))
        .map(|s| d.slice(s.span).trim_end())
        .collect();
    let mut targets: Vec<&str> = touched_before.to_vec();
    targets.extend(p.assigned_ids.iter().map(String::as_str));
    targets.push(&p.target);
    let after_others: Vec<&str> = after
        .slides
        .iter()
        .filter(|s| !targets.contains(&s.id.as_str()))
        .map(|s| after.slice(s.span).trim_end())
        .collect();
    assert_eq!(before_others, after_others, "non-target slides changed");
    after
}

fn snap(name: &str, d: &Deck, p: &Patch) {
    let after = ops::apply(&d.text, p).unwrap();
    insta::assert_snapshot!(name, ops::diff(&d.text, &after));
}

fn spec(title: Option<&str>, body: Option<&str>) -> SlideSpec {
    SlideSpec {
        title: title.map(Into::into),
        body: body.map(Into::into),
        ..Default::default()
    }
}

#[test]
fn add_positions() {
    let d = deck();
    let p = ops::add(&d, &spec(Some("New"), Some("Body.")), Pos::End).unwrap();
    snap("add_end", &d, &p);
    let a = run(&d, p.clone(), &[]);
    assert_eq!(a.slides.last().unwrap().id, p.target);
    assert_eq!(p.target, "new"); // id from title
    let untitled = ops::add(&d, &spec(None, Some("x")), Pos::End).unwrap();
    assert!(untitled.target.starts_with("s-") && untitled.target.len() == 6);
    let clash = ops::add(&d, &spec(Some("Results"), None), Pos::End).unwrap();
    assert_eq!(clash.target, "results-1");
    assert_eq!(p.assigned_ids, vec![p.target.clone()]);

    let p = ops::add(&d, &spec(Some("After Intro"), None), Pos::After("intro")).unwrap();
    snap("add_after_intro", &d, &p);
    let a = run(&d, p.clone(), &[]);
    assert_eq!(a.slides[1].id, p.target);

    let p = ops::add(
        &d,
        &spec(None, Some("Untitled first.")),
        Pos::Before("first"),
    )
    .unwrap();
    snap("add_before_first", &d, &p);
    let a = run(&d, p.clone(), &[]);
    assert_eq!(a.slides[0].id, p.target);
    assert_eq!(a.slides[0].title, None);

    let mut s = spec(Some("Styled"), None);
    s.attrs = vec![("background-color".into(), "aquamarine".into())];
    s.classes = vec!["center".into()];
    let p = ops::add(&d, &s, Pos::End).unwrap();
    let a = run(&d, p, &[]);
    let attrs = a.slides.last().unwrap().attrs.clone().unwrap();
    assert_eq!(attrs.get("background-color"), Some("aquamarine"));
    assert!(attrs.has_class("center"));
}

#[test]
fn add_to_files_without_trailing_newline_or_slides() {
    let d = Deck::parse("## A\n\nend".into());
    let p = ops::add(&d, &spec(Some("B"), Some("b")), Pos::End).unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(out.starts_with("## A\n\nend\n\n## B {#b}"), "{out}");
    assert!(out.ends_with("}\n\nb\n"), "{out:?}");

    let d = Deck::parse("---\ntitle: x\n---\n".into());
    let p = ops::add(&d, &spec(Some("First"), None), Pos::End).unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert_eq!(Deck::parse(out).slides.len(), 1);

    let d = Deck::parse("## A\r\n\r\ntext\r\n".into());
    let p = ops::add(&d, &spec(Some("B"), Some("x\ny")), Pos::End).unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(
        !out.replace("\r\n", "").contains('\n'),
        "mixed line endings: {out:?}"
    );
}

#[test]
fn dup_gets_fresh_ids() {
    let d = deck();
    let p = ops::dup(&d, "results", None).unwrap();
    snap("dup_results", &d, &p);
    let a = run(&d, p.clone(), &[]);
    assert_eq!(a.slides[2].id, p.target);
    let ids = a.explicit_ids();
    let mut uniq = ids.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(ids.len(), uniq.len(), "duplicate ids after dup: {ids:?}");
    assert!(
        a.slides[2]
            .blocks
            .iter()
            .any(|b| b.id.starts_with("b-") && b.id != "b-plot")
    );
}

#[test]
fn move_slides() {
    let d = deck();
    let p = ops::mv(&d, "conclusion", Pos::Before("intro")).unwrap();
    snap("move_conclusion_first", &d, &p);
    let a = run(&d, p, &["conclusion"]);
    assert_eq!(a.slides[0].id, "conclusion");
    assert_eq!(a.slides.len(), d.slides.len());

    let p = ops::mv(&d, "intro", Pos::After("last")).unwrap();
    let a = run(&d, p, &["intro"]);
    assert_eq!(a.slides.last().unwrap().id, "intro");
    assert!(a.text.ends_with("Hello world.\n"), "{:?}", a.text);

    // moving next to itself is a no-op; onto itself is an error
    let p = ops::mv(&d, "intro", Pos::Before("results")).unwrap();
    assert_eq!(
        ops::apply(&d.text, &p)
            .unwrap()
            .replace("## Intro {#intro}", "## Intro"),
        d.text
    );
    assert_eq!(
        ops::mv(&d, "intro", Pos::After("intro"))
            .unwrap_err()
            .code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::mv(&d, "intro", Pos::End).unwrap_err().code(),
        "E_INVALID_ARG"
    );
}

#[test]
fn remove_slide() {
    let d = deck();
    let p = ops::rm(&d, "@3").unwrap();
    snap("rm_rule_slide", &d, &p);
    let a = run(&d, p, &["@3"]);
    assert_eq!(a.slides.len(), 3);
}

#[test]
fn retitle_assigns_explicit_id() {
    let d = deck();
    let p = ops::title(&d, "intro", "Introduction", None).unwrap();
    snap("title_intro", &d, &p);
    assert_eq!(p.assigned_ids, vec!["intro"]);
    let a = run(&d, p, &["intro"]);
    assert_eq!(
        a.slide("intro").unwrap().title.as_deref(),
        Some("Introduction")
    );

    let p = ops::title(&d, "results", "", None).unwrap();
    let a = run(&d, p, &["results"]);
    assert!(a.text.contains("\n## {#results .smaller}\n"));
    assert!(a.slide("results").unwrap().title.is_none());
}

#[test]
fn rule_slide_becomes_heading_when_targeted() {
    let d = deck();
    let p = ops::title(&d, "@3", "Ruled", None).unwrap();
    snap("title_rule_slide", &d, &p);
    let a = run(&d, p.clone(), &["@3"]);
    let s = a.slide(&p.target).unwrap();
    assert_eq!(s.title.as_deref(), Some("Ruled"));
    assert!(s.id_explicit && s.id.starts_with("s-"));
    assert_eq!(p.assigned_ids, vec![p.target.clone()]);

    let p = ops::set(
        &d,
        "@3",
        &AttrChange {
            set: vec![("background-color".into(), "red".into())],
            ..Default::default()
        },
    )
    .unwrap();
    let a = run(&d, p.clone(), &["@3"]);
    assert!(
        a.text.contains(&format!(
            "## {{#{} background-color=red}}\n\nA rule slide.",
            p.target
        )),
        "{}",
        a.text
    );
}

#[test]
fn implicit_and_setext_slides() {
    let d =
        Deck::parse("---\nformat: revealjs\n---\n\nPreamble.\n\nSetext\n------\n\nbody\n".into());
    let p = ops::title(&d, "@1", "Now titled", None).unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(
        out.contains(&format!("\n## Now titled {{#{}}}\n\nPreamble.\n", p.target)),
        "{out}"
    );
    let p = ops::set(
        &d,
        "setext",
        &AttrChange {
            add_class: vec!["center".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(
        out.contains("## Setext {#setext .center}\n\nbody\n"),
        "{out}"
    );
}

#[test]
fn body_replaces_content_but_keeps_boxes() {
    let d = deck();
    let p = ops::body(&d, "results", "New results.\n\n- three").unwrap();
    snap("body_results", &d, &p);
    let a = run(&d, p, &["results"]);
    let s = a.slide("results").unwrap();
    assert_eq!(a.slice(s.blocks[0].span), "New results.");
    assert!(s.blocks.iter().any(|b| b.id == "b-plot"));

    let p = ops::body(&d, "intro", "").unwrap();
    let a = run(&d, p, &["intro"]);
    assert!(a.slide("intro").unwrap().blocks.is_empty());
}

#[test]
fn set_attrs_and_classes() {
    let d = deck();
    let change = AttrChange {
        set: vec![("background-color".into(), "#283b95".into())],
        unset: vec![],
        add_class: vec!["center".into()],
        remove_class: vec!["smaller".into()],
    };
    let p = ops::set(&d, "results", &change).unwrap();
    snap("set_results", &d, &p);
    let a = run(&d, p, &["results"]);
    let at = a.slide("results").unwrap().attrs.clone().unwrap();
    assert_eq!(at.get("background-color"), Some("#283b95"));
    assert_eq!(at.classes(), vec!["center"]);

    // idempotent: unsetting a missing key / removing a missing class changes nothing
    let unset = AttrChange {
        unset: vec!["missing".into()],
        remove_class: vec!["nope".into()],
        ..Default::default()
    };
    let p = ops::set(&d, "results", &unset).unwrap();
    assert_eq!(ops::apply(&d.text, &p).unwrap(), d.text);
}

#[test]
fn edit_element() {
    let d = deck();
    let p = ops::edit(&d, "results/0", "- three\n- four", None).unwrap();
    snap("edit_results_list", &d, &p);
    let a = run(&d, p, &["results"]);
    assert_eq!(
        a.slice(a.slide("results").unwrap().blocks[0].span),
        "- three\n- four"
    );

    let p = ops::edit(
        &d,
        "#b-plot",
        "![](other.png){#b-plot .absolute top=0 left=0 width=10 height=10}",
        None,
    )
    .unwrap();
    let a = run(&d, p, &["results"]);
    assert!(a.text.contains("other.png"));
    assert_eq!(
        ops::edit(&d, "results/9", "x", None).unwrap_err().code(),
        "E_NOT_FOUND"
    );
}

#[test]
fn meta_set_unset() {
    let d = deck();
    let p = ops::meta_set(&d, "theme", "dark").unwrap();
    snap("meta_set_theme", &d, &p);
    let a = run(&d, p, &[]);
    assert_eq!(a.meta.get("theme").and_then(|v| v.as_str()), Some("dark"));

    let p = ops::meta_set(&d, "title", "\"New title\"").unwrap();
    let a = run(&d, p, &[]);
    assert_eq!(
        a.meta.get("title").and_then(|v| v.as_str()),
        Some("New title")
    );

    let p = ops::meta_unset(&d, "title").unwrap();
    let a = run(&d, p, &[]);
    assert!(a.meta.get("title").is_none());
    assert!(a.meta.get("format").is_some());

    let nested =
        Deck::parse("---\nformat:\n  revealjs:\n    theme: dark\ntitle: x\n---\n\n## A\n".into());
    let p = ops::meta_set(&nested, "format", "revealjs").unwrap();
    let out = ops::apply(&nested.text, &p).unwrap();
    assert_eq!(out, "---\nformat: revealjs\ntitle: x\n---\n\n## A\n");

    let bare = Deck::parse("## A\n\nx\n".into());
    let p = ops::meta_set(&bare, "title", "T").unwrap();
    assert_eq!(
        ops::apply(&bare.text, &p).unwrap(),
        "---\ntitle: T\n---\n\n## A\n\nx\n"
    );

    assert_eq!(
        ops::meta_set(&d, "bad key", "x").unwrap_err().code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::meta_set(&d, "k", "[unclosed").unwrap_err().code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::meta_set(&d, "k", "a\nb").unwrap_err().code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::meta_unset(&d, "nope").unwrap_err().code(),
        "E_NOT_FOUND"
    );
}

#[test]
fn refusals_leave_deck_untouched() {
    let broken = Deck::parse(include_str!("corpus/21-unterminated.qmd").into());
    for err in [
        ops::title(&broken, "broken-slide", "x", None).unwrap_err(),
        ops::body(&broken, "broken-slide", "x").unwrap_err(),
        ops::rm(&broken, "broken-slide").unwrap_err(),
        ops::add(&broken, &spec(Some("x"), None), Pos::End).unwrap_err(),
        ops::mv(&broken, "good-slide", Pos::After("broken-slide")).unwrap_err(),
    ] {
        assert_eq!(err.code(), "E_UNSAFE_EDIT");
    }
    // the good slide stays editable
    ops::title(&broken, "good-slide", "Fine", None).unwrap();

    let d = deck();
    let bad = AttrChange {
        set: vec![("bad key".into(), "v".into())],
        ..Default::default()
    };
    assert_eq!(
        ops::set(&d, "intro", &bad).unwrap_err().code(),
        "E_INVALID_ARG"
    );
    let bad = AttrChange {
        add_class: vec!["a b".into()],
        ..Default::default()
    };
    assert_eq!(
        ops::set(&d, "intro", &bad).unwrap_err().code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::title(&d, "nope", "x", None).unwrap_err().code(),
        "E_NOT_FOUND"
    );
    assert_eq!(
        ops::title(&d, "intro", "two\nlines", None)
            .unwrap_err()
            .code(),
        "E_INVALID_ARG"
    );
}

#[test]
fn ids_are_deterministic() {
    let d = deck();
    let a = ops::add(&d, &spec(Some("x"), None), Pos::End).unwrap();
    let b = ops::add(&d, &spec(Some("x"), None), Pos::End).unwrap();
    assert_eq!(a.target, b.target);
}

#[test]
fn new_deck_is_valid() {
    let text = ops::new_deck(Some("Demo"));
    let d = Deck::parse(text);
    assert!(d.has_title_slide());
    assert_eq!(d.slides.len(), 1);
    assert!(d.slides[0].id_explicit);
}

// ---------- US4: positioned boxes (T040) ----------

use quartz_deck::ops::BoxContent;

#[test]
fn box_add_text_and_image() {
    let d = deck();
    let p = ops::box_add(
        &d,
        "intro",
        BoxContent::Text("A note"),
        Some((100.0, 50.0)),
        Some((300.0, 80.0)),
    )
    .unwrap();
    snap("box_add_text", &d, &p);
    assert!(p.target.starts_with("b-") && p.target.len() == 6);
    let a = run(&d, p.clone(), &["intro"]);
    let (_, b) = a.element(&format!("#{}", p.target)).unwrap();
    assert_eq!(b.kind, quartz_deck::deck::BlockKind::Box);
    assert!(a.text.contains(&format!(
        "::: {{#{} .absolute left=100 top=50 width=300 height=80}}\nA note\n:::",
        p.target
    )));

    let p = ops::box_add(
        &d,
        "@3",
        BoxContent::Image("fig.png"),
        Some((0.4, 9.6)),
        Some((10.0, 10.0)),
    )
    .unwrap();
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(
        out.contains(&format!(
            "![](fig.png){{#{} .absolute left=0 top=10 width=10 height=10}}",
            p.target
        )),
        "{out}"
    );

    // defaults: centred on the 1050x700 slide, 300x100
    let p = ops::box_add(&d, "conclusion", BoxContent::Text("x"), None, None).unwrap();
    assert!(
        ops::apply(&d.text, &p)
            .unwrap()
            .contains("left=375 top=300 width=300 height=100}")
    );
    // multi-line text, and the notes block stays where it is
    let p = ops::box_add(
        &d,
        "conclusion",
        BoxContent::Text("line 1\n\nline 2"),
        Some((1.0, 2.0)),
        None,
    )
    .unwrap();
    let a = run(&d, p, &["conclusion"]);
    let kinds: Vec<_> = a
        .slide("conclusion")
        .unwrap()
        .blocks
        .iter()
        .map(|b| b.kind)
        .collect();
    use quartz_deck::deck::BlockKind::*;
    assert_eq!(kinds, vec![Para, Notes, Box]);
}

#[test]
fn box_move_resize_rm() {
    let d = deck();
    let p = ops::box_move(&d, "#b-plot", Some((200.0, 100.0)), None).unwrap();
    snap("box_move_at", &d, &p);
    let a = run(&d, p.clone(), &["results"]);
    assert!(
        a.text
            .contains("{#b-plot .absolute top=100 left=200 width=300 height=200}"),
        "{}",
        a.text
    );
    assert_eq!(p.target, "b-plot");

    let p = ops::box_move(&d, "#b-plot", None, Some((-5.0, 7.0))).unwrap();
    assert!(
        ops::apply(&d.text, &p)
            .unwrap()
            .contains("{#b-plot .absolute top=17 left=15 width=300 height=200}")
    );

    let p = ops::box_resize(&d, "results/1", (640.5, 480.0), None).unwrap();
    assert!(
        ops::apply(&d.text, &p)
            .unwrap()
            .contains("{#b-plot .absolute top=10 left=20 width=641 height=480}")
    );
    let p = ops::box_resize(&d, "#b-plot", (100.0, 100.0), Some((1.0, 2.0))).unwrap();
    assert!(
        ops::apply(&d.text, &p)
            .unwrap()
            .contains("{#b-plot .absolute top=2 left=1 width=100 height=100}")
    );

    let p = ops::box_rm(&d, "#b-plot").unwrap();
    snap("box_rm", &d, &p);
    let a = run(&d, p, &["results"]);
    assert!(
        a.slide("results")
            .unwrap()
            .blocks
            .iter()
            .all(|b| b.id != "b-plot")
    );
    assert!(a.text.contains("- two\n\n---\n"), "{}", a.text);
}

#[test]
fn box_units_and_refusals() {
    let d = Deck::parse(include_str!("corpus/11-absolute.qmd").into());
    // right/bottom + % width: --at writes px left/top, drops right/bottom, reports conversion
    let p = ops::box_move(&d, "boxes/2", Some((10.0, 20.0)), None).unwrap();
    assert!(p.converted_units);
    let out = ops::apply(&d.text, &p).unwrap();
    assert!(
        out.contains(&format!(
            "::: {{#{} .absolute width=20% left=10 top=20}}",
            p.target
        )),
        "{out}"
    );
    assert_eq!(
        p.assigned_ids,
        vec![p.target.clone()],
        "unnamed box gets an id"
    );
    // --by needs px left/top
    assert_eq!(
        ops::box_move(&d, "boxes/2", None, Some((1.0, 1.0)))
            .unwrap_err()
            .code(),
        "E_INVALID_ARG"
    );
    // quoted px values are fine
    let p = ops::box_move(&d, "#b-note", None, Some((10.0, 10.0))).unwrap();
    assert!(!p.converted_units);
    assert!(
        ops::apply(&d.text, &p)
            .unwrap()
            .contains("{#b-note .absolute top=60 left=610 width=400 height=120}")
    );
    // not a box / bad sizes
    let basic = deck();
    assert_eq!(
        ops::box_move(&basic, "results/0", Some((0.0, 0.0)), None)
            .unwrap_err()
            .code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::box_resize(&basic, "#b-plot", (0.0, 10.0), None)
            .unwrap_err()
            .code(),
        "E_INVALID_ARG"
    );
    assert_eq!(
        ops::box_rm(&basic, "intro/0").unwrap_err().code(),
        "E_INVALID_ARG"
    );
}

#[test]
fn geom_parsing() {
    use quartz_deck::attrs::Attrs;
    use quartz_deck::ops::{Length, geom};
    let g =
        geom(&Attrs::parse(r#"{.absolute top="50" left=600px width=20% height=1.5em}"#).unwrap());
    assert_eq!(
        g.top,
        Some(Length {
            value: 50.0,
            unit: "px".into()
        })
    );
    assert_eq!(
        g.left,
        Some(Length {
            value: 600.0,
            unit: "px".into()
        })
    );
    assert_eq!(
        g.width,
        Some(Length {
            value: 20.0,
            unit: "%".into()
        })
    );
    assert_eq!(
        g.height,
        Some(Length {
            value: 1.5,
            unit: "em".into()
        })
    );
    assert_eq!(g.right, None);
}
