//! Doc pipeline data structures.

use std::path::PathBuf;

#[derive(Debug, Default)]
pub(crate) struct Page {
    pub title: String,
    pub path: String,
    pub sort_order: i32,
    pub description: Option<String>,
    pub last_updated: Option<String>,
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
    UnknownCategory { value: String },
    UnknownStatus { value: String },
    RawVoxCodeBlock,
    BrokenIncludeAnchor { file: String, anchor: String },
    WholeFileIncludeHasTrainingHeader { file: String },
}
