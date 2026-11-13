//! Stable error codes (contracts/cli.md). Every failure leaves the deck untouched.

use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum QdError {
    #[error("no deck found: {0}")]
    NoDeck(String),
    #[error("could not parse deck: {0}")]
    Parse(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("ambiguous reference: {0}")]
    Ambiguous(String),
    #[error("unsafe edit refused: {0}")]
    UnsafeEdit(String),
    #[error("file changed since it was read: {0}")]
    Conflict(String),
    /// `--expect-text`: the element changed since the caller read it.
    #[error("{what} changed since it was read")]
    StaleElement {
        what: String,
        current_source: String,
        current_hash: String,
    },
    /// A name already in use (a saved layout).
    #[error("{0}")]
    Taken(String),
    #[error("invalid argument: {0}")]
    InvalidArg(String),
    #[error("invalid setting: {0}")]
    InvalidSetting(String),
    #[error("nothing to undo/redo: {0}")]
    NoUndo(String),
    #[error("quarto not found on PATH")]
    NoQuarto,
    #[error("quarto render failed")]
    Render(String),
    #[error("no running `qd serve` for this deck")]
    NoServer,
    #[error("usage: {0}")]
    Usage(String),
    #[error("no browser available: {0}")]
    NoBrowser(String),
    /// History (spec 012) needs the `git` command.
    #[error("git not found on PATH")]
    NoGit,
    /// The deck is not in a git repository (spec 012).
    #[error("the deck is not in a git repository")]
    NoRepo,
    /// The repository is mid-merge, mid-rebase or similar, or on a detached
    /// head: qd doesn't commit (spec 012 research R3).
    #[error("not committing: {0}")]
    RepoBusy(String),
    /// git refused (its message).
    #[error("git: {0}")]
    Git(String),
    /// A format needs TeX (`beamer`, `pdf`) and none was found.
    #[error("{0} needs TeX, and none was found")]
    NoTex(String),
    #[error("download failed: {0}")]
    Download(String),
    /// The offline check failed; the report is printed on stdout.
    #[error("the deck does not work offline ({0} problem(s))")]
    Offline(usize),
    #[error("cannot be exported as one offline file: {0}")]
    OfflineIncompatible(String),
    #[error("no environment for this deck (looked from {0} up to the Quarto project root)")]
    NoEnv(String),
    #[error("{0} not found on PATH")]
    EnvTool(&'static str),
    #[error("the environment's lockfile is out of date with its manifest")]
    LockStale(String),
    #[error("the environment tool failed")]
    Env(String),
    #[error("not a code cell: {0}")]
    NotCode(String),
    /// The deck's engine can't run here: what is missing (spec 013).
    #[error("cannot run {0}")]
    Engine(String),
    /// A cell raised or was stopped; the run's result is printed on stdout.
    #[error("{0} did not finish")]
    Run(String),
    #[error("Zotero's local API is not reachable: {0}")]
    Zotero(String),
    /// Spec 014: a write to a bibliography file outside the deck's folder.
    #[error("read-only: {0}")]
    ReadOnly(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl QdError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoDeck(_) => "E_NO_DECK",
            Self::Parse(_) => "E_PARSE",
            Self::NotFound(_) => "E_NOT_FOUND",
            Self::Ambiguous(_) => "E_AMBIGUOUS",
            Self::UnsafeEdit(_) => "E_UNSAFE_EDIT",
            Self::Conflict(_) | Self::StaleElement { .. } | Self::Taken(_) => "E_CONFLICT",
            Self::InvalidArg(_) => "E_INVALID_ARG",
            Self::InvalidSetting(_) => "E_INVALID_SETTING",
            Self::NoUndo(_) => "E_NO_UNDO",
            Self::NoQuarto => "E_NO_QUARTO",
            Self::Render(_) => "E_RENDER",
            Self::NoServer => "E_NO_SERVER",
            Self::Usage(_) => "E_USAGE",
            Self::NoBrowser(_) => "E_NO_BROWSER",
            Self::NoTex(_) => "E_NO_TEX",
            Self::NoGit => "E_NO_GIT",
            Self::NoRepo => "E_NO_REPO",
            Self::Git(_) => "E_GIT",
            Self::RepoBusy(_) => "E_REPO_BUSY",
            Self::Download(_) => "E_DOWNLOAD",
            Self::Offline(_) => "E_OFFLINE",
            Self::OfflineIncompatible(_) => "E_OFFLINE_INCOMPATIBLE",
            Self::NoEnv(_) => "E_NO_ENV",
            Self::EnvTool(_) => "E_ENV_TOOL",
            Self::LockStale(_) => "E_LOCK_STALE",
            Self::Env(_) => "E_ENV",
            Self::NotCode(_) => "E_NOT_CODE",
            Self::Engine(_) => "E_ENGINE",
            Self::Run(_) => "E_RUN",
            Self::Zotero(_) => "E_ZOTERO",
            Self::ReadOnly(_) => "E_READ_ONLY",
            Self::Io(_) => "E_IO",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::NoDeck(_) => {
                "pass --deck PATH, set QD_DECK, or run in a directory with exactly one .qmd"
            }
            Self::Parse(_) => "run `qd check` to locate the problem",
            Self::NotFound(_) => "run `qd slides` (or `qd show <slide>`) to list valid ids",
            Self::Ambiguous(_) => {
                "ids must be unique; run `qd check`, then address the slide as @N"
            }
            Self::UnsafeEdit(_) => {
                "fix the construct reported by `qd check` (e.g. close the fence) or edit the file by hand"
            }
            Self::Conflict(_) => {
                "re-read with `qd slides` / `qd show` and retry with the new --expect version"
            }
            Self::StaleElement { .. } => {
                "details hold the current source and hash; re-apply your change to it and retry"
            }
            Self::Taken(_) => "choose another name, or replace it (--replace)",
            Self::InvalidArg(_) => "see `qd schema` for argument formats",
            Self::InvalidSetting(_) => {
                "run `qd settings` to see every setting and its allowed values"
            }
            Self::NoUndo(_) => {
                "undo/redo only works right after a qd edit, before the file is changed elsewhere"
            }
            Self::NoQuarto => "install Quarto: https://quarto.org/docs/get-started/",
            Self::Render(_) => "see details for the Quarto log",
            Self::NoServer => "start one with `qd serve`",
            Self::Usage(_) => "run `qd --help` or `qd schema`",
            Self::NoBrowser(_) => {
                "install Google Chrome, set QD_CHROME to a Chrome/Chromium binary, or allow qd to download one (drop --no-download / QD_NO_DOWNLOAD)"
            }
            Self::NoTex(_) => "install TeX: quarto install tinytex",
            Self::NoGit => "install git",
            Self::NoRepo => "start a history with `qd history --init`",
            Self::Git(_) => "see git's message",
            Self::RepoBusy(_) => "finish it (or check out a branch); qd commits at the next pause",
            Self::Download(_) => {
                "check the network connection and retry; a file that fails its checksum is never used"
            }
            Self::Offline(_) => "see the problems in the report on stdout; each has a hint",
            Self::OfflineIncompatible(_) => {
                "remove the feature from the deck (or render that deck as a folder instead)"
            }
            Self::NoEnv(_) => "create one with `qd env init` (uv) or `qd env init --tool pixi`",
            Self::EnvTool("pixi") => {
                "install pixi: https://pixi.sh (curl -fsSL https://pixi.sh/install.sh | sh)"
            }
            Self::EnvTool(_) => {
                "install uv: https://docs.astral.sh/uv/ (curl -LsSf https://astral.sh/uv/install.sh | sh)"
            }
            Self::LockStale(_) => {
                "relock with `qd env sync` (or `qd env add <pkg>`); details hold the tool's output"
            }
            Self::Env(_) => "see details for the tool's output",
            Self::NotCode(_) => "only executable cells (```{python}) run; see `kind` in `qd show`",
            Self::Engine(_) => {
                "install what the message names (a Jupyter kernel, or R with knitr and rmarkdown); Python decks: `qd env init`"
            }
            Self::Run(_) => {
                "the outputs in the result on stdout show why; fix the cell (`qd replace`) and run it again"
            }
            Self::Zotero(_) => {
                "start Zotero 7 and tick Settings > Advanced > \"Allow other applications on this computer to communicate with Zotero\""
            }
            Self::ReadOnly(_) => {
                "qd only reads bibliography files outside the deck's folder: copy it into the folder, or pull from it with `qd bib pull PATH`"
            }
            Self::Io(_) => "check the path and permissions",
        }
    }

    pub fn exit_code(&self) -> i32 {
        if matches!(self, Self::Usage(_)) { 2 } else { 1 }
    }

    pub fn to_json(&self) -> Value {
        let mut e = json!({"code": self.code(), "message": self.to_string(), "hint": self.hint()});
        match self {
            Self::Render(log) | Self::LockStale(log) | Self::Env(log) => e["details"] = json!(log),
            Self::StaleElement {
                current_source,
                current_hash,
                ..
            } => {
                e["details"] =
                    json!({"current_source": current_source, "current_hash": current_hash})
            }
            _ => {}
        }
        json!({ "error": e })
    }
}

pub type Result<T> = std::result::Result<T, QdError>;
