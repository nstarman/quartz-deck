//! T049 / SC-007: three qd writers and an external (non-qd) appender hammer
//! one deck at once. Afterwards the deck parses, every qd writer's last
//! successful edit is present, and no external line was lost.

use assert_cmd::cargo::cargo_bin;
use quartz_deck::deck::Deck;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn concurrent_writers_lose_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("deck.qmd");
    let mut text = String::from("---\ntitle: Stress\nformat: revealjs\n---\n\n");
    for i in 0..3 {
        text += &format!("## T{i} {{#t{i}}}\n\nBody {i}.\n\n");
    }
    text += "## Log {#log}\n\n";
    std::fs::write(&deck, &text).unwrap();
    let cache = dir.path().join(".cache");
    let stop = Arc::new(AtomicBool::new(false));
    let duration = Duration::from_secs(4);

    // qd writers: each retitles its own slide; conflicts are retried
    let mut writers = Vec::new();
    for i in 0..3 {
        let (deck, cache, stop) = (deck.clone(), cache.clone(), stop.clone());
        writers.push(std::thread::spawn(move || {
            let (mut last_ok, mut ok, mut conflicts) = (None, 0, 0);
            let mut n = 0;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                let title = format!("T{i} v{n}");
                let out = Command::new(cargo_bin("qd"))
                    .args([
                        "--deck",
                        deck.to_str().unwrap(),
                        "title",
                        &format!("t{i}"),
                        &title,
                    ])
                    .env("QD_CACHE_DIR", &cache)
                    .output()
                    .unwrap();
                if out.status.success() {
                    last_ok = Some(title);
                    ok += 1;
                } else {
                    let err = String::from_utf8_lossy(&out.stderr);
                    assert!(err.contains("E_CONFLICT"), "unexpected failure: {err}");
                    conflicts += 1;
                }
            }
            (last_ok, ok, conflicts)
        }));
    }

    // external writer: plain appends, like a tool that knows nothing about qd
    let appended = Arc::new(Mutex::new(Vec::new()));
    let ext = {
        let (deck, stop, appended) = (deck.clone(), stop.clone(), appended.clone());
        std::thread::spawn(move || {
            use std::io::Write;
            let mut n = 0;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                let line = format!("<!-- ext {n} -->\n");
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&deck)
                    .unwrap()
                    .write_all(line.as_bytes())
                    .unwrap();
                appended.lock().unwrap().push(line);
                std::thread::sleep(Duration::from_millis(25));
            }
        })
    };

    let t = Instant::now();
    while t.elapsed() < duration {
        std::thread::sleep(Duration::from_millis(50));
        // the file must parse with all slides at every moment we look
        let d = Deck::parse(std::fs::read_to_string(&deck).unwrap());
        assert_eq!(d.slides.len(), 4, "slides lost mid-run");
    }
    stop.store(true, Ordering::Relaxed);
    let results: Vec<_> = writers.into_iter().map(|w| w.join().unwrap()).collect();
    ext.join().unwrap();

    let final_text = std::fs::read_to_string(&deck).unwrap();
    let d = Deck::parse(final_text.clone());
    for (i, (last_ok, ok, conflicts)) in results.iter().enumerate() {
        eprintln!("writer {i}: {ok} ok, {conflicts} conflicts");
        assert!(*ok > 0, "writer {i} never succeeded");
        assert_eq!(
            d.slide(&format!("t{i}")).unwrap().title.as_deref(),
            last_ok.as_deref(),
            "writer {i}'s last edit lost"
        );
    }
    let appended = appended.lock().unwrap();
    eprintln!("external appends: {}", appended.len());
    let lost: Vec<_> = appended
        .iter()
        .filter(|l| !final_text.contains(l.as_str()))
        .collect();
    assert!(lost.is_empty(), "external appends lost: {lost:?}");
}
