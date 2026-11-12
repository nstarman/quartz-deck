//! A live session for a deck: a Jupyter kernel through the embedded Python
//! bridge (`kernel_bridge.py`, spec 004 R4; any kernel since 013), or R through
//! `kernel_bridge.R` for knitr decks (013 R4). qd talks to either in the same
//! JSON lines. One request at a time; `interrupt` may be sent while a cell runs.

use crate::deck::{Block, BlockKind, Deck, code_open, version_of};
use crate::engine::{Engine, Kind};
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
const BRIDGE_R: &str = include_str!("kernel_bridge.R");

/// Cold start measured at ~8 s (environment + kernel); allow for a first sync.
const START: Duration = Duration::from_secs(180);

/// The language of a code block (`python` for ```` ```{python} ````).
pub fn cell_lang<'a>(d: &'a Deck, b: &Block) -> &'a str {
    let info = d
        .slice(b.span)
        .lines()
        .next()
        .and_then(code_open)
        .map_or("", |(_, _, i)| i);
    let lang = info.trim_start_matches([' ', '{']);
    &lang[..lang.find([' ', ',', '}']).unwrap_or(lang.len())]
}

/// The code of cell `b` to execute, or None for `#| eval: false`;
/// `E_NOT_CODE` for anything but an executable cell.
pub fn cell_code(d: &Deck, b: &Block) -> Result<Option<String>> {
    if b.kind != BlockKind::Cell {
        return Err(QdError::NotCode(b.id.clone()));
    }
    let body: Vec<&str> = d.slice(b.span).lines().skip(1).collect();
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

/// Whether `engine` runs cell `b` (`E_ENGINE` if it is in another language).
pub fn runnable(d: &Deck, b: &Block, engine: &Engine) -> Result<Option<String>> {
    let code = cell_code(d, b)?;
    let lang = cell_lang(d, b);
    if !engine.runs(lang) {
        return Err(QdError::Engine(format!(
            "{lang} cells in a deck that runs {}",
            engine.label()
        )));
    }
    Ok(code)
}

/// The cells to run, in order: the picked `cells` (in the order given), or
/// every cell (`all`), or from the top through the furthest picked one (`fresh`,
/// a new session has no state). Run all / fresh skip what can't run here;
/// picked cells must be code that `engine` runs.
pub fn plan(
    d: &Deck,
    engine: &Engine,
    cells: &[String],
    all: bool,
    fresh: bool,
) -> Result<Vec<Planned>> {
    let mut picked = vec![];
    for r in cells {
        let (_, b) = d.element(r)?;
        runnable(d, b, engine)?;
        picked.push(b.id.clone());
    }
    let last = d
        .blocks()
        .enumerate()
        .filter(|(_, b)| picked.contains(&b.id))
        .map(|(i, _)| i)
        .max();
    let mut plan = vec![];
    for (i, b) in d.blocks().enumerate() {
        let wanted = all || picked.contains(&b.id) || (fresh && last.is_some_and(|l| i <= l));
        if b.kind != BlockKind::Cell || !wanted {
            continue;
        }
        if let Ok(Some(code)) = runnable(d, b, engine) {
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
    pub engine: Engine,
    child: Child,
    stdin: Mutex<Option<ChildStdin>>,
    /// Replies in request order; holding the lock = the session is busy.
    replies: Mutex<Receiver<Value>>,
    stderr: Arc<Mutex<String>>,
}

impl Session {
    /// Start `engine`'s bridge (cwd `dir`) and wait until it is ready.
    /// Jupyter: in the deck's environment when it has one (004), else with
    /// uv's throwaway `jupyter_client` for a kernel named or found by language
    /// (013 R3). knitr: R running `kernel_bridge.R` (R4).
    pub fn start(engine: &Engine, dir: &Path) -> Result<Session> {
        let path = std::env::var_os("PATH");
        let mut cmd = match engine.kind {
            Kind::Jupyter => {
                let env = crate::env::find(dir);
                let mut c = match &env {
                    Some(env) => env.wrap(Path::new("python"))?,
                    // a Python deck without a named kernel needs its environment (004)
                    None if engine.kernel.is_none() && engine.language == "python" => {
                        return Err(QdError::NoEnv(dir.display().to_string()));
                    }
                    None => {
                        let mut c = crate::env::tool_cmd(crate::env::Tool::Uv, path)?;
                        c.args([
                            "run",
                            "-q",
                            "--no-project",
                            "--with",
                            "jupyter_client",
                            "python",
                        ]);
                        c
                    }
                };
                let own = env.is_some() && engine.language == "python";
                let kernel = engine
                    .kernel
                    .as_deref()
                    .or(own.then_some(crate::env::KERNEL));
                c.args(["-I", "-c", BRIDGE])
                    .env("QD_KERNEL", kernel.unwrap_or_default())
                    .env("QD_LANGUAGE", &engine.language);
                c
            }
            Kind::Knitr => {
                let r = crate::render::which("Rscript", path).ok_or_else(|| {
                    QdError::Engine(
                        "r cells: R is not installed (https://cran.r-project.org)".into(),
                    )
                })?;
                let mut c = std::process::Command::new(r);
                c.args(["-e", BRIDGE_R]);
                let [w, h, dpi] = engine.fig;
                c.env("QD_FIG", format!("{w} {h} {dpi}"));
                c
            }
        };
        let mut child = cmd
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
            engine: engine.clone(),
            stdin: Mutex::new(child.stdin.take()),
            child,
            replies: Mutex::new(rx),
            stderr,
        };
        let ready = s.reply(&s.replies.lock().unwrap(), START, "start")?;
        // the bridge says what is missing (a kernel, an R package) and exits
        match ready["error"].as_str() {
            Some(e) => Err(QdError::Engine(e.into())),
            None => Ok(s),
        }
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
    /// R is interrupted by a signal (SIGINT), only while a cell runs.
    pub fn interrupt(&self) -> Result<()> {
        if self.engine.kind == Kind::Jupyter {
            return self.send(json!({"op": "interrupt"}));
        }
        if self.replies.try_lock().is_err() {
            // ponytail: `kill` the command, not a libc dependency
            std::process::Command::new("kill")
                .args(["-INT", &self.child.id().to_string()])
                .status()?;
        }
        Ok(())
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
