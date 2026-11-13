//! Per-deck reproducible environments (spec 004 R1–R3): **uv** by default,
//! **pixi** with PyPI dependencies as the alternative; one lockfile per deck.
//! Code-executing commands run inside the environment with `--locked`, so a
//! stale lock is refused instead of silently re-resolved. qd never calls pip.

use crate::error::{QdError, Result};
use serde_json::{Value, json};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// What Quarto's Jupyter engine and the kernel bridge need (research R2).
pub const BASE: [&str; 4] = ["ipykernel", "nbclient", "nbformat", "pyyaml"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Tool {
    Uv,
    Pixi,
}

impl Tool {
    fn name(self) -> &'static str {
        match self {
            Tool::Uv => "uv",
            Tool::Pixi => "pixi",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Env {
    pub tool: Tool,
    pub root: PathBuf,
}

fn is_project(d: &Path) -> bool {
    d.join("_quarto.yml").is_file() || d.join("_quarto.yaml").is_file()
}

/// Folder a new environment goes in: the Quarto project root, else the deck's folder.
pub fn home(deck_dir: &Path) -> PathBuf {
    deck_dir
        .ancestors()
        .find(|d| is_project(d))
        .unwrap_or(deck_dir)
        .to_path_buf()
}

/// The nearest environment from the deck's folder up to its Quarto project root.
pub fn find(deck_dir: &Path) -> Option<Env> {
    for d in deck_dir.ancestors() {
        let pyproject = std::fs::read_to_string(d.join("pyproject.toml")).ok();
        let tool = if d.join("pixi.toml").is_file()
            || pyproject
                .as_deref()
                .is_some_and(|t| t.contains("[tool.pixi"))
        {
            Some(Tool::Pixi)
        } else if pyproject.is_some() || d.join("uv.lock").is_file() {
            Some(Tool::Uv)
        } else {
            None
        };
        if let Some(tool) = tool {
            let root = d.to_path_buf();
            return Some(Env { tool, root });
        }
        if is_project(d) {
            break;
        }
    }
    None
}

/// `find`, or `E_NO_ENV`.
pub fn require(deck_dir: &Path) -> Result<Env> {
    find(deck_dir).ok_or_else(|| QdError::NoEnv(deck_dir.display().to_string()))
}

/// The tool found on `path` (`$PATH`), or `E_ENV_TOOL`.
pub(crate) fn tool_cmd(tool: Tool, path: Option<OsString>) -> Result<Command> {
    let bin = crate::render::which(tool.name(), path).ok_or(QdError::EnvTool(tool.name()))?;
    Ok(Command::new(bin))
}

impl Env {
    /// How uv/pixi are told which environment: `--project` / `--manifest-path`.
    fn flag(&self) -> &'static str {
        if self.tool == Tool::Uv {
            "--project"
        } else {
            "--manifest-path"
        }
    }

    /// Arguments that run `program` inside the environment, refusing a stale lock.
    fn run_args(&self, program: &OsStr) -> Vec<OsString> {
        let root = self.root.as_os_str();
        [
            "run".as_ref(),
            self.flag().as_ref(),
            root,
            "--locked".as_ref(),
            program,
        ]
        .map(OsString::from)
        .into()
    }

    /// A command that runs `program` inside the environment (`uv run --locked …`),
    /// with qd's kernel spec ([`KERNEL`]) first on Jupyter's search path.
    pub fn wrap(&self, program: &Path) -> Result<Command> {
        let mut c = tool_cmd(self.tool, std::env::var_os("PATH"))?;
        c.args(self.run_args(program.as_os_str()));
        let mut path = vec![kernel_dir()?];
        path.extend(
            std::env::var_os("JUPYTER_PATH")
                .iter()
                .flat_map(std::env::split_paths),
        );
        c.env(
            "JUPYTER_PATH",
            std::env::join_paths(path).map_err(std::io::Error::other)?,
        );
        Ok(c)
    }

    /// A uv/pixi subcommand aimed at this environment: `uv <sub> --project <root> <args>`.
    fn tool(&self, sub: &[&str], args: &[&str]) -> Result<String> {
        let mut c = tool_cmd(self.tool, std::env::var_os("PATH"))?;
        c.args(sub).arg(self.flag()).arg(&self.root).args(args);
        output(&mut c)
    }

    pub fn manifest(&self) -> PathBuf {
        let pixi = self.root.join("pixi.toml");
        if self.tool == Tool::Pixi && pixi.is_file() {
            pixi
        } else {
            self.root.join("pyproject.toml")
        }
    }

    pub fn lockfile(&self) -> PathBuf {
        self.root.join(format!("{}.lock", self.tool.name()))
    }

    /// `qd env`: where the environment is and whether its lock matches the manifest.
    pub fn report(&self) -> Value {
        let current = self.lockfile().is_file() && self.tool(&["lock", "--check"], &[]).is_ok();
        json!({
            "tool": self.tool.name(), "root": self.root, "manifest": self.manifest(),
            "lockfile": self.lockfile(), "lock_current": current,
        })
    }

    /// `qd env add`: add PyPI packages and relock (`uv add` / `pixi add --pypi`).
    pub fn add(&self, pkgs: &[String]) -> Result<String> {
        if let Some(p) = pkgs.iter().find(|p| p.starts_with('-')) {
            return Err(QdError::InvalidArg(format!("'{p}' is not a package name")));
        }
        let pkgs: Vec<&str> = pkgs.iter().map(String::as_str).collect();
        match self.tool {
            Tool::Uv => self.tool(&["add"], &pkgs),
            Tool::Pixi => self.tool(&["add", "--pypi"], &pkgs),
        }
    }

    /// `qd env sync`: install the locked versions (relocking only if the manifest changed).
    pub fn sync(&self) -> Result<String> {
        match self.tool {
            Tool::Uv => self.tool(&["sync"], &[]),
            Tool::Pixi => self.tool(&["install"], &[]),
        }
    }
}

/// Name of qd's kernel spec: `python` from `PATH`, i.e. the environment's own
/// Python (uv/pixi put it first). Without it Quarto may pick any Python kernel
/// installed for the user, outside the environment.
pub const KERNEL: &str = "qd-python";

/// The Jupyter data folder holding qd's kernel spec (written on first use).
fn kernel_dir() -> Result<PathBuf> {
    let dir = crate::store::cache_root().join("jupyter");
    let spec = dir.join("kernels").join(KERNEL).join("kernel.json");
    if !spec.is_file() {
        std::fs::create_dir_all(spec.parent().unwrap())?;
        let json = json!({
            "argv": ["python", "-m", "ipykernel_launcher", "-f", "{connection_file}"],
            "display_name": "Python (deck environment)", "language": "python",
        });
        std::fs::write(&spec, json.to_string())?;
    }
    Ok(dir)
}

/// `qd env init`: a new environment with the base packages, in `home(deck_dir)`.
pub fn init(deck_dir: &Path, tool: Tool) -> Result<Env> {
    if let Some(e) = find(deck_dir) {
        return Err(QdError::InvalidArg(format!(
            "an environment already exists: {}",
            e.manifest().display()
        )));
    }
    let root = home(deck_dir);
    let mut c = tool_cmd(tool, std::env::var_os("PATH"))?;
    match tool {
        // --name: the folder name may not be a valid package name
        Tool::Uv => c.args(["init", "--bare", "--no-workspace", "--name", "slides"]),
        Tool::Pixi => c.arg("init"),
    };
    output(c.arg(&root))?;
    let env = Env { tool, root };
    if tool == Tool::Pixi {
        // pixi takes Python itself from conda-forge; every package is from PyPI
        env.tool(&["add"], &["python"])?;
    }
    env.add(&BASE.map(String::from))?;
    Ok(env)
}

fn output(c: &mut Command) -> Result<String> {
    let out = c.output()?;
    let log = crate::render::log(&out);
    if out.status.success() {
        Ok(log)
    } else {
        Err(failure(log))
    }
}

/// `E_LOCK_STALE` when uv/pixi refused because the lock is out of date, else `E_ENV`.
pub fn failure(log: String) -> QdError {
    if is_stale(&log) {
        QdError::LockStale(log)
    } else {
        QdError::Env(log)
    }
}

pub fn is_stale(log: &str) -> bool {
    log.contains("needs to be updated, but `--locked`") || log.contains("not up-to-date")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let p = d.path().join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        d
    }

    #[test]
    fn discovery() {
        let d = dir(&[("pyproject.toml", "[project]\n"), ("talks/a/deck.qmd", "")]);
        let found = find(&d.path().join("talks/a")).unwrap();
        assert_eq!((found.tool, found.root), (Tool::Uv, d.path().to_path_buf()));

        let d = dir(&[("uv.lock", ""), ("deck.qmd", "")]);
        assert_eq!(find(d.path()).unwrap().tool, Tool::Uv);
        let d = dir(&[("pixi.toml", ""), ("pyproject.toml", "[project]\n")]);
        assert_eq!(find(d.path()).unwrap().tool, Tool::Pixi);
        let d = dir(&[("pyproject.toml", "[project]\n[tool.pixi.workspace]\n")]);
        assert_eq!(find(d.path()).unwrap().tool, Tool::Pixi);

        // the Quarto project root bounds the search; a new environment goes there
        let d = dir(&[
            ("pyproject.toml", ""),
            ("proj/_quarto.yml", ""),
            ("proj/s/deck.qmd", ""),
        ]);
        let deck = d.path().join("proj/s");
        assert_eq!(find(&deck), None);
        assert!(matches!(require(&deck), Err(QdError::NoEnv(_))));
        assert_eq!(home(&deck), d.path().join("proj"));
        let d = dir(&[("deck.qmd", "")]);
        assert_eq!(home(d.path()), d.path());
    }

    #[test]
    fn wrapping_runs_locked_inside_the_environment() {
        let root = PathBuf::from("/talk");
        let args = |tool| {
            Env {
                tool,
                root: root.clone(),
            }
            .run_args(OsStr::new("quarto"))
            .join(OsStr::new(" "))
        };
        assert_eq!(args(Tool::Uv), "run --project /talk --locked quarto");
        assert_eq!(
            args(Tool::Pixi),
            "run --manifest-path /talk --locked quarto"
        );
    }

    #[test]
    fn missing_tool_and_stale_lock_are_clear_errors() {
        let e = tool_cmd(Tool::Pixi, Some(OsString::from("/nonexistent"))).unwrap_err();
        assert_eq!(e.code(), "E_ENV_TOOL");
        assert!(e.hint().contains("pixi"));
        let uv =
            "error: The lockfile at `uv.lock` needs to be updated, but `--locked` was provided.";
        assert_eq!(failure(uv.into()).code(), "E_LOCK_STALE");
        assert_eq!(failure("boom".into()).code(), "E_ENV");
    }
}
