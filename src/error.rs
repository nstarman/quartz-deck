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
    #[error("invalid argument: {0}")]
    InvalidArg(String),
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
            Self::Conflict(_) => "E_CONFLICT",
            Self::InvalidArg(_) => "E_INVALID_ARG",
            Self::NoUndo(_) => "E_NO_UNDO",
            Self::NoQuarto => "E_NO_QUARTO",
            Self::Render(_) => "E_RENDER",
            Self::NoServer => "E_NO_SERVER",
            Self::Usage(_) => "E_USAGE",
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
            Self::InvalidArg(_) => "see `qd schema` for argument formats",
            Self::NoUndo(_) => {
                "undo/redo only works right after a qd edit, before the file is changed elsewhere"
            }
            Self::NoQuarto => "install Quarto: https://quarto.org/docs/get-started/",
            Self::Render(_) => "see details for the Quarto log",
            Self::NoServer => "start one with `qd serve`",
            Self::Usage(_) => "run `qd --help` or `qd schema`",
            Self::Io(_) => "check the path and permissions",
        }
    }

    pub fn exit_code(&self) -> i32 {
        if matches!(self, Self::Usage(_)) { 2 } else { 1 }
    }

    pub fn to_json(&self) -> Value {
        let mut e = json!({"code": self.code(), "message": self.to_string(), "hint": self.hint()});
        if let Self::Render(log) = self {
            e["details"] = json!(log);
        }
        json!({ "error": e })
    }
}

pub type Result<T> = std::result::Result<T, QdError>;
