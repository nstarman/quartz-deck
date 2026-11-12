//! Helpers shared by the integration tests that run `qd` against Quarto.
#![allow(dead_code)] // each test binary uses some of these

use std::path::Path;
use std::process::Command;

/// `qd` in `dir`, with its own cache and no inherited deck.
pub fn qd(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qd"));
    c.current_dir(dir)
        .env("QD_CACHE_DIR", dir.join(".cache"))
        .env("QD_CONFIG_DIR", dir.join(".config"))
        .env_remove("QD_DECK");
    c
}

/// `tool` (quarto, uv, pixi) is installed; tests that need it skip, with a message, when not.
pub fn have(tool: &str) -> bool {
    let ok = Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        eprintln!("{tool} not found: skipping");
    }
    ok
}

/// qd's temp files (`.qd-` in the name) left in `dir` or `dir/export`;
/// the offline export's `.qd-export` marker is meant to stay.
pub fn leftovers(dir: &Path) -> Vec<String> {
    [dir.to_path_buf(), dir.join("export")]
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".qd-") && n != ".qd-export")
        .collect()
}
