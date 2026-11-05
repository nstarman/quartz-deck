//! A live Jupyter session for a deck (spec 004 R4): the embedded Python bridge
//! (`kernel_bridge.py`) runs inside the deck's environment and starts a kernel
//! over IPC sockets; qd talks to it in JSON lines. One request at a time;
//! `interrupt` may be sent while a cell runs.

use crate::deck::{Block, BlockKind, Deck, code_open, version_of};
use crate::env::Env;
use crate::error::{QdError, Result};
use crate::outputs::Outputs;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BRIDGE: &str = include_str!("kernel_bridge.py");

/// Cold start measured at ~8 s (environment + kernel); allow for a first sync.
const START: Duration = Duration::from_secs(180);

/// The code of cell `b` to execute, or None for `#| eval: false`;
/// `E_NOT_CODE` for anything but an executable cell, `E_ENGINE` unless Python.
pub fn cell_code(d: &Deck, b: &Block) -> Result<Option<String>> {
    if b.kind != BlockKind::Cell {
        return Err(QdError::NotCode(b.id.clone()));
    }
    let text = d.slice(b.span);
    let mut lines = text.lines();
    let info = lines.next().and_then(code_open).map_or("", |(_, _, i)| i);
    let lang = info.trim_start_matches([' ', '{']);
    let lang = &lang[..lang.find([' ', ',', '}']).unwrap_or(lang.len())];
    if lang != "python" {
        return Err(QdError::Engine(lang.to_string()));
    }
    let body: Vec<&str> = lines.collect();
    let body = &body[..body.len().saturating_sub(1)]; // the closing fence
    let off = body.iter().any(|l| {
        let o = l
            .trim_start()
            .strip_prefix("#|")
            .map(|o| o.replace(' ', ""));
        o.as_deref() == Some("eval:false")
    });
    Ok((!off).then(|| body.join("\n")))
}

/// A cell to run: (ref, fingerprint, code).
pub type Planned = (String, String, String);

/// The cells to run, in order: the picked `cells` (in the order given), or
/// every cell (`all`), or from the top through the last picked one (`fresh`,
/// a new session has no state). Run all / fresh skip what can't run here;
/// picked cells must be runnable code.
pub fn plan(d: &Deck, cells: &[String], all: bool, fresh: bool) -> Result<Vec<Planned>> {
    let mut picked = vec![];
    for r in cells {
        let (_, b) = d.element(r)?;
        cell_code(d, b)?;
        picked.push(b.id.clone());
    }
    let last = d.blocks().position(|b| picked.last() == Some(&b.id));
    let mut plan = vec![];
    for (i, b) in d.blocks().enumerate() {
        let wanted = all || picked.contains(&b.id) || (fresh && last.is_some_and(|l| i <= l));
        if b.kind != BlockKind::Cell || !wanted {
            continue;
        }
        if let Ok(Some(code)) = cell_code(d, b) {
            plan.push((b.id.clone(), version_of(d.slice(b.span).as_bytes()), code));
        }
    }
    if !all && !fresh {
        plan.sort_by_key(|(id, ..)| picked.iter().position(|p| p == id));
    }
    Ok(plan)
}

/// Run `plan` in `k`, storing each output; stops at the first cell that
/// fails or is stopped (FR-010). `each(cell, None)` before a cell runs and
/// `each(cell, Some(record))` after. Returns `done`, `error` or `stopped`.
pub fn run_plan(
    k: &Session,
    out: &Outputs,
    plan: Vec<Planned>,
    timeout: Option<Duration>,
    mut each: impl FnMut(&str, Option<&Value>),
) -> Result<&'static str> {
    for (cell, fp, code) in plan {
        each(&cell, None);
        let rec = out.save(&cell, &fp, &k.execute(&code, timeout)?)?;
        each(&cell, Some(&rec));
        match rec["status"].as_str() {
            Some("ok") => {}
            Some("stopped") => return Ok("stopped"),
            _ => return Ok("error"),
        }
    }
    Ok("done")
}

pub struct Session {
    child: Child,
    stdin: Mutex<Option<ChildStdin>>,
    /// Replies in request order; holding the lock = the session is busy.
    replies: Mutex<Receiver<Value>>,
    stderr: Arc<Mutex<String>>,
}

impl Session {
    /// Start the bridge in `env` (cwd `dir`) and wait until its kernel is ready.
    pub fn start(env: &Env, dir: &Path) -> Result<Session> {
        let mut child = env
            .wrap(Path::new("python"))?
            .args(["-I", "-c", BRIDGE])
            .env("QD_KERNEL", crate::env::KERNEL)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (tx, rx) = channel();
        let out = BufReader::new(child.stdout.take().unwrap());
        std::thread::spawn(move || {
            for line in out.lines().map_while(|l| l.ok()) {
                if let Ok(v) = serde_json::from_str(&line)
                    && tx.send(v).is_err()
                {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let (mut err, log) = (child.stderr.take().unwrap(), stderr.clone());
        std::thread::spawn(move || {
            let mut buf = [0; 4096];
            while let Ok(n @ 1..) = err.read(&mut buf) {
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
        let s = Session {
            stdin: Mutex::new(child.stdin.take()),
            child,
            replies: Mutex::new(rx),
            stderr,
        };
        s.reply(&s.replies.lock().unwrap(), START, "start")?;
        Ok(s)
    }

    fn send(&self, req: Value) -> Result<()> {
        let mut stdin = self.stdin.lock().unwrap();
        let w = stdin.as_mut().ok_or_else(|| self.died())?;
        writeln!(w, "{req}")
            .and_then(|_| w.flush())
            .map_err(|_| self.died())
    }

    fn reply(&self, rx: &Receiver<Value>, wait: Duration, what: &str) -> Result<Value> {
        rx.recv_timeout(wait).map_err(|e| match e {
            RecvTimeoutError::Timeout => QdError::Env(format!(
                "the kernel did not answer ({what}) within {}s\n{}",
                wait.as_secs(),
                self.stderr.lock().unwrap()
            )),
            RecvTimeoutError::Disconnected => self.died(),
        })
    }

    /// The bridge exited: its output says why (e.g. a stale lock → `E_LOCK_STALE`).
    fn died(&self) -> QdError {
        // the stderr reader may still be draining the last lines
        std::thread::sleep(Duration::from_millis(200));
        crate::env::failure(format!(
            "the kernel stopped\n{}",
            self.stderr.lock().unwrap()
        ))
    }

    /// Run `code`; `{status: ok|error|stopped, counter, outputs}`. After
    /// `timeout` the cell is interrupted (status `stopped`).
    pub fn execute(&self, code: &str, timeout: Option<Duration>) -> Result<Value> {
        let rx = self.replies.lock().unwrap();
        self.send(json!({"op": "execute", "code": code}))?;
        let start = Instant::now();
        let limit = timeout.unwrap_or(Duration::MAX);
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(v) => return Ok(v),
                Err(RecvTimeoutError::Disconnected) => return Err(self.died()),
                Err(RecvTimeoutError::Timeout) if start.elapsed() >= limit => {
                    self.interrupt()?;
                    return self.reply(&rx, Duration::from_secs(30), "interrupt");
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// Stop the running cell (it finishes with status `stopped`); no-op when idle.
    pub fn interrupt(&self) -> Result<()> {
        self.send(json!({"op": "interrupt"}))
    }

    /// Fresh kernel state (waits for any running cell first: interrupt it to hurry).
    pub fn restart(&self) -> Result<()> {
        let rx = self.replies.lock().unwrap();
        self.send(json!({"op": "restart"}))?;
        self.reply(&rx, START, "restart").map(drop)
    }
}

impl Drop for Session {
    /// Closing stdin makes the bridge shut its kernel down; kill it if it lingers.
    fn drop(&mut self) {
        self.stdin.lock().unwrap().take();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
