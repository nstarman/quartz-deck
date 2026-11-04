//! Helpers shared by the integration tests that run `qd` against Quarto.

use std::path::Path;
use std::process::Command;

/// `qd` in `dir`, with its own cache and no inherited deck.
pub fn qd(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qd"));
    c.current_dir(dir)
        .env("QD_CACHE_DIR", dir.join(".cache"))
        .env_remove("QD_DECK");
    c
}

/// Quarto is installed (tests that need it skip, with a message, when not).
pub fn have_quarto() -> bool {
    let ok = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        eprintln!("quarto not found: skipping");
    }
    ok
}
