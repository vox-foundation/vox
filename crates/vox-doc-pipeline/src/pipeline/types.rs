//! Doc pipeline data structures.

use std::path::PathBuf;

#[derive(Debug, Default)]
pub(crate) struct Page {
    pub title: String,
    pub path: String,
    pub sort_order: i32,
    pub description: Option<String>,
    pub last_updated: Option<String>,
    pub status: Option<String>,
    pub schema_type: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct LintError {
    pub file: PathBuf,
    pub line: usize,
    pub kind: LintKind,
}

#[derive(Debug)]
pub(crate) enum LintKind {
    UnclosedCodeFence,
    ShortCodeFence { backticks: usize, at_line: usize },
    GenericDescription,
    MissingFrontmatter,
    MissingCategory,
    MissingTrainingRationale,
    UnknownCategory { value: String },
    UnknownStatus { value: String },
    UnknownSchemaType { value: String },
    BrokenIncludeAnchor { file: String, anchor: String },
    WholeFileIncludeHasTrainingHeader { file: String },
    DocTestFailed { msg: String },
}
