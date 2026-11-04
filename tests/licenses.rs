//! FR-041: every dependency must carry a license compatible with private,
//! proprietary use. Fails on unknown or copyleft-only licenses.

use std::process::Command;

const ALLOWED: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "MPL-2.0",
    "0BSD",
    "Unlicense",
    "BSL-1.0",
    "Apache-2.0 WITH LLVM-exception",
];

/// `a OR b` passes if any alternative passes; `a AND b` needs all parts.
fn allowed(expr: &str) -> bool {
    let expr = expr.replace(['(', ')'], " ").replace('/', " OR ");
    expr.split(" OR ").any(|alt| {
        alt.split(" AND ")
            .all(|part| ALLOWED.contains(&part.trim()))
    })
}

#[test]
fn spdx_expressions() {
    assert!(allowed("MIT OR Apache-2.0"));
    assert!(allowed("(MIT OR Apache-2.0) AND Unicode-3.0"));
    assert!(allowed("MIT/Apache-2.0"));
    assert!(!allowed("GPL-3.0"));
    assert!(!allowed("MIT AND GPL-3.0"));
}

#[test]
fn dependency_licenses_are_allowed() {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let bad: Vec<String> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["name"] != "quartz-deck")
        .filter(|p| !p["license"].as_str().is_some_and(allowed))
        .map(|p| format!("{} {} ({})", p["name"], p["version"], p["license"]))
        .collect();
    assert!(bad.is_empty(), "disallowed licenses:\n{}", bad.join("\n"));
}
