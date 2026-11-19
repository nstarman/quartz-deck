//! Spec 018 US2: indent and outdent change only heading marks (research R2),
//! by the rules of research R3, and `move` can land a slide indented (R4).

use quartz_deck::deck::Deck;
use quartz_deck::error::QdError;
use quartz_deck::ops::{self, Depth, Pos};

/// The file after indenting (`In`) or outdenting `slide`, and the slides whose level changed.
fn re(src: &str, slide: &str, depth: Depth) -> (String, Vec<String>) {
    let d = Deck::parse(src.into());
    let (p, changed) = ops::relevel(&d, slide, depth).unwrap();
    (ops::apply(&d.text, &p).unwrap(), changed)
}

fn refused(src: &str, slide: &str, depth: Depth) -> QdError {
    ops::relevel(&Deck::parse(src.into()), slide, depth).unwrap_err()
}

/// Each slide's id, `H` for a head, and its head's id.
fn groups(text: &str) -> Vec<String> {
    let d = Deck::parse(text.into());
    d.slides
        .iter()
        .map(|s| match (s.is_head, s.parent) {
            (true, _) => format!("{}:H", s.id),
            (false, Some(p)) => format!("{}<{}", s.id, d.slides[p].id),
            (false, None) => s.id.clone(),
        })
        .collect()
}

// ---------- R2: only the marks change ----------

#[test]
fn atx_marks_change_and_nothing_else() {
    let src = "## A  {#first .big data-x=\"1\"}\n\nx\n\n## B\n\ny\n";
    let (t, changed) = re(src, "b", Depth::In);
    assert_eq!(t, "# A  {#first .big data-x=\"1\"}\n\nx\n\n## B\n\ny\n");
    assert_eq!(changed, ["first"]);
}

#[test]
fn setext_swaps_its_underline() {
    let (t, _) = re("A\n--\n\nx\n\nB\n-----\n", "b", Depth::In);
    assert_eq!(t, "A\n==\n\nx\n\nB\n-----\n");
}

#[test]
fn setext_below_level_two_becomes_atx() {
    let lv3 = "---\nslide-level: 3\n---\n\n";
    let src = format!("{lv3}# P\n\n## S\n\nT\n====\n\n### t1\n");
    let (t, changed) = re(&src, "t", Depth::In);
    assert_eq!(t, format!("{lv3}# P\n\n## S\n\n### T\n\n### t1\n"));
    assert_eq!(changed, ["t"]);
}

#[test]
fn a_rule_slide_becomes_an_empty_head() {
    let (t, changed) = re("# A\n\n## a\n\n---\n\nruled\n", "@3", Depth::Out);
    assert_eq!(t, "# A\n\n## a\n\n#\n\nruled\n");
    assert_eq!(changed.len(), 1);
}

#[test]
fn auto_ids_stay() {
    let (t, _) = re("## Alpha\n\n## Beta\n", "beta", Depth::In);
    assert_eq!(t, "# Alpha\n\n## Beta\n");
    assert_eq!(groups(&t), ["alpha:H", "beta<alpha"]);
}

#[test]
fn an_untitled_first_slide_cannot_become_a_head() {
    let e = refused(
        "---\nformat: revealjs\n---\n\nPreamble.\n\n## B\n",
        "b",
        Depth::In,
    );
    assert!(
        matches!(e, QdError::InvalidArg(ref m) if m.contains("title")),
        "{e}"
    );
}

// ---------- R3: the rules ----------

#[test]
fn indenting_a_head_joins_the_group_above() {
    let (t, changed) = re("# A\n\n## a1\n\n# B\n\n## b1\n", "b", Depth::In);
    assert_eq!(t, "# A\n\n## a1\n\n## B\n\n## b1\n");
    assert_eq!(changed, ["b"]);
    assert_eq!(groups(&t), ["a:H", "a1<a", "b<a", "b1<a"]);
}

#[test]
fn indenting_under_a_top_level_slide_makes_it_a_head() {
    let (t, changed) = re("## A\n\n## B\n\n## C\n", "b", Depth::In);
    assert_eq!(t, "# A\n\n## B\n\n## C\n");
    assert_eq!(changed, ["a"]);
    assert_eq!(groups(&t), ["a:H", "b<a", "c<a"]);
}

#[test]
fn indenting_a_head_under_a_top_level_slide() {
    let (t, changed) = re("## I\n\n# A\n\n## a1\n", "a", Depth::In);
    assert_eq!(t, "# I\n\n## A\n\n## a1\n");
    assert_eq!(changed, ["i", "a"]);
    assert_eq!(groups(&t), ["i:H", "a<i", "a1<i"]);
}

#[test]
fn outdenting_mid_group_takes_the_rest() {
    let (t, changed) = re("# A\n\n## a1\n\n## a2\n\n## a3\n", "a2", Depth::Out);
    assert_eq!(t, "# A\n\n## a1\n\n# a2\n\n## a3\n");
    assert_eq!(changed, ["a2"]);
    assert_eq!(groups(&t), ["a:H", "a1<a", "a2:H", "a3<a2"]);
}

#[test]
fn deeper_slide_levels_move_between_the_two_levels() {
    let lv3 = "---\nslide-level: 3\n---\n\n";
    let src = format!("{lv3}# P\n\n## S\n\n### s1\n");
    assert_eq!(
        re(&src, "s", Depth::Out).0,
        format!("{lv3}# P\n\n# S\n\n### s1\n")
    );
    assert_eq!(
        re(&src, "s1", Depth::Out).0,
        format!("{lv3}# P\n\n## S\n\n# s1\n")
    );
}

#[test]
fn refusals_say_why() {
    let reason = |src: &str, slide: &str, depth| match refused(src, slide, depth) {
        QdError::InvalidArg(m) => m,
        e => panic!("{e}"),
    };
    let flat = "## A\n\n## B\n";
    let grouped = "# A\n\n## a1\n";
    assert!(reason(flat, "a", Depth::In).contains("nothing above"));
    assert!(reason(grouped, "a1", Depth::In).contains("already"));
    assert!(reason(grouped, "a", Depth::Out).contains("already"));
    assert!(reason(flat, "b", Depth::Out).contains("not in a group"));
    let lv1 = "---\nslide-level: 1\n---\n\n# A\n\n# B\n";
    assert!(reason(lv1, "b", Depth::In).contains("slide-level"));
    // a slide qd can't segment safely is never restructured
    let opaque = "## A\n\nok\n\n## B\n\n```\nunterminated\n";
    assert!(matches!(
        refused(opaque, "b", Depth::In),
        QdError::UnsafeEdit(_)
    ));
}

// ---------- R4: a move that lands indented, as one patch ----------

#[test]
fn move_lands_indented_or_outdented() {
    let d = Deck::parse("## A\n\n## B\n\n## C\n\n## D\n".into());
    let (p, changed) = ops::mv_depth(&d, &["d"], Pos::After("a"), Depth::In).unwrap();
    let t = ops::apply(&d.text, &p).unwrap();
    assert_eq!(groups(&t), ["a:H", "d<a", "b<a", "c<a"]);
    assert_eq!(changed, ["a"]);

    let d = Deck::parse("# A\n\n## a1\n\n## a2\n\n## B\n".into());
    let (p, changed) = ops::mv_depth(&d, &["a1"], Pos::After("a2"), Depth::Out).unwrap();
    let t = ops::apply(&d.text, &p).unwrap();
    assert_eq!(groups(&t), ["a:H", "a2<a", "a1:H", "b<a1"]);
    assert_eq!(changed, ["a1"]);
}
