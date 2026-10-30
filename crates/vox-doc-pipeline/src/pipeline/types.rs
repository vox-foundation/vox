//! Doc pipeline data structures.

use std::path::PathBuf;

#[derive(Debug)]
pub struct LintError {
    pub file: PathBuf,
    pub line: usize,
    pub kind: LintKind,
}

#[derive(Debug)]
pub enum LintKind {
    UnclosedCodeFence,
    ShortCodeFence {
        backticks: usize,
        at_line: usize,
    },
    GenericDescription,
    MissingFrontmatter,
    MissingCategory,
    MissingTrainingRationale,
    UnknownCategory {
        value: String,
    },
    UnknownStatus {
        value: String,
    },
    UnknownSchemaType {
        value: String,
    },
    BrokenIncludeAnchor {
        file: String,
        anchor: String,
    },
    BrokenIncludeFile {
        file: String,
    },
    WholeFileIncludeHasTrainingHeader {
        file: String,
    },
    DocTestFailed {
        msg: String,
    },
    UnlabeledCodeFence {
        at_line: usize,
    },
    /// Second YAML frontmatter block detected (usually an accidental merge).
    DuplicateFrontmatter {
        second_block_start_line: usize,
    },
    /// A hand-authored `last_updated:` key is present at all; the pipeline derives this from Git.
    HandAuthoredLastUpdated,
    /// A hand-authored `last_verified:` or `last_verified_commit:` key is present; the pipeline manages this via the Freshness Ledger.
    HandAuthoredLastVerified,
    /// index.mdx's SYNC-FROM-README block content differs from README's matching ANCHOR block,
    /// after known intentional link-scheme transforms are applied.
    ReadmeSyncDrift {
        block: String,
    },
    /// README.md has an ANCHOR block but index.mdx has no matching SYNC-FROM-README block.
    ReadmeSyncMissingBlock {
        block: String,
    },
    /// index.mdx expects a README.md ANCHOR block that doesn't exist there.
    ReadmeSyncMissingAnchor {
        block: String,
    },
    /// One of the two files the README<->index.mdx sync check compares (README.md or
    /// docs/src/index.mdx) could not be read at all — e.g. moved or renamed. Without this,
    /// a missing source file would silently disable the whole check instead of failing loud.
    ReadmeSyncSourceMissing {
        path: String,
    },
    /// A KB facet finding; the rule id is the backlog key.
    Facet(FacetIssue),
    /// A `vox-diagram` fence problem (see `diagrams.rs`).
    Diagram(DiagramIssue),
    /// A legacy-Mermaid ratchet finding (see `mermaid_gate.rs`).
    LegacyMermaid(MermaidIssue),
    /// A ```mermaid fenced block has invalid syntax or unsupported diagram type.
    InvalidMermaid {
        code: &'static str,
        msg: String,
    },
}

/// One finding of the shrink-only legacy-Mermaid baseline gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidIssue {
    /// A file holds more `mermaid` fences than the baseline allows (0 when unlisted).
    Growth {
        /// Live fence count of the file.
        count: usize,
        /// Count the baseline admits.
        allowed: usize,
    },
    /// The baseline admits more fences than the file still has (blocking: run the tighten tool, or the slack could be re-spent).
    Stale {
        /// Live fence count of the file.
        count: usize,
        /// Count the baseline admits.
        allowed: usize,
    },
    /// A fence whose first info token is `mermaid` but whose info string is not exactly that.
    InfoString {
        /// The trimmed info string as written.
        info: String,
    },
    /// The baseline or a doc could not be read or validated (fail closed).
    Unreadable {
        /// Repo-relative path of the unreadable file.
        path: String,
        /// Why it failed.
        error: String,
    },
}

/// One finding about a `vox-diagram` fence; positions are 1-based file coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramIssue {
    /// A `vox_diagram::check` diagnostic mapped onto the file.
    Check {
        /// File line of the diagnostic.
        line: usize,
        /// File column (chars) of the diagnostic.
        col: usize,
        /// Stable `diagram/*` code.
        code: &'static str,
        /// Human-readable description.
        message: String,
        /// `true` for an Error, `false` for a Warning (printed, never blocking).
        error: bool,
    },
    /// A fence whose language token is `vox-diagram` but whose info string is not exactly that.
    InfoString {
        /// File line of the opening fence.
        line: usize,
        /// The trimmed info string as written.
        info: String,
    },
}

/// One facet-lint finding (KB Phase 4). The stable rule id (see `facets.rs`) is also the key
/// the facet backlog uses to excuse a doc from exactly this rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacetIssue {
    /// The taxonomy or backlog contract could not be read or validated (fail closed).
    ContractUnreadable {
        path: String,
        error: String,
    },
    /// A facet value in frontmatter does not deserialize.
    ParseError {
        msg: String,
    },
    MissingTopic,
    MissingDomain,
    UnknownTopic {
        value: String,
        hint: Option<String>,
    },
    UnknownDomain {
        value: String,
        hint: Option<String>,
    },
    /// A retired alias was written as a facet value.
    DeprecatedLabel {
        field: &'static str,
        value: String,
        replaced_by: String,
    },
    DomainMismatch {
        topic: String,
        domain: String,
        expected: String,
    },
    MissingSummary,
    SummaryPlacement {
        reason: String,
    },
    SummaryWordCount {
        words: usize,
    },
    MissingAuthorship,
    MissingAuthorshipModel,
    /// `training_eligible: true` on a page the shared rule forces ineligible.
    TrainingEligibleContradiction,
    MissingSources,
    SourceField {
        index: usize,
        field: &'static str,
        problem: String,
    },
    QuoteOverCap {
        words: usize,
        cap: usize,
        scope: &'static str,
    },
    InvalidDate {
        field: &'static str,
        value: String,
    },
    /// A frontmatter key that looks like a misspelled facet key (WR-03).
    UnknownFacetKey {
        key: String,
        did_you_mean: &'static str,
    },
    /// `human_attested_by` is blank, a placeholder or an AI name (WR-02).
    InvalidAttestation {
        value: String,
    },
    /// The backlog lists a rule the doc no longer violates (warning).
    BacklogStale {
        rule: String,
    },
}
