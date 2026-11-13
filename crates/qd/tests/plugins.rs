//! `plugins/` (constitution VII): each plugin is a Quarto extension first,
//! with an example deck that renders with plain `quarto render` and no qd.

mod common;

use common::have;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `dir`, as paths relative to it, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push(p.strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    out
}

/// The quarto-revealjs skill ships its own copy of `paper`, so a deck set up
/// from the skill alone has it. `plugins/paper` is the source; the copy must
/// match it byte for byte.
#[test]
fn skill_copy_of_paper_matches_the_plugin() {
    let plugin = root().join("plugins/paper/_extensions/paper");
    let copy = root().join("skills/quarto-revealjs/assets/_extensions/paper");
    assert_eq!(files(&plugin), files(&copy), "the same files");
    for f in files(&plugin) {
        assert_eq!(
            std::fs::read(plugin.join(&f)).unwrap(),
            std::fs::read(copy.join(&f)).unwrap(),
            "{} differs: edit plugins/paper and copy it to the skill",
            f.display()
        );
    }
}

/// Each plugin's `example.qmd` renders with plain Quarto (no qd).
#[test]
fn plugin_examples_render_with_plain_quarto() {
    if !have("quarto") {
        return;
    }
    let plugins = root().join("plugins");
    let mut seen = 0;
    for e in std::fs::read_dir(&plugins).unwrap() {
        let dir = e.unwrap().path();
        if !dir.join("example.qmd").is_file() {
            continue;
        }
        seen += 1;
        let t = tempfile::tempdir().unwrap();
        for f in files(&dir) {
            let to = t.path().join(&f);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(dir.join(&f), &to).unwrap();
        }
        let out = std::process::Command::new("quarto")
            .args(["render", "example.qmd", "--to", "revealjs"])
            .current_dir(t.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}: {}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let html = std::fs::read_to_string(t.path().join("example.html")).unwrap();
        if dir.ends_with("paper") {
            assert!(
                html.contains("paper-ref") && html.contains("paper-corner"),
                "both forms of the paper link"
            );
        }
    }
    assert!(seen > 0, "no plugin has an example.qmd");
}
