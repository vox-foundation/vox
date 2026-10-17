//! Importer id rules: the DSL id rule, plus the words the pinned Mermaid parser rejects or
//! misreads as ids. Mermaid's lexers are position-sensitive, so each list below applies only
//! where it was probed to fail (word by word, position by position, mermaid 11.17.2): a word
//! is refused exactly there and imports everywhere else.

use crate::limits::MAX_LABEL_CHARS;
use crate::resolve::RESERVED;

type Check = Result<(), &'static str>;

/// `Ok` when `s` is a legal DSL id; otherwise the construct name.
pub(super) fn valid_id(s: &str) -> Check {
    let mut chars = s.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !first_ok
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || s.ends_with('-')
    {
        return Err("invalid-id");
    }
    if s.len() > MAX_LABEL_CHARS {
        // The DSL caps an id at the label length; `fmt` would refuse the whole block as `internal`.
        return Err("id-too-long");
    }
    if RESERVED.contains(&s) {
        return Err("reserved-id");
    }
    Ok(())
}

/// The part of `s` before its first `-` (how a lexer sees `end-x`: the keyword `end`, then `-x`).
fn head(s: &str) -> &str {
    s.split('-').next().unwrap_or(s)
}

fn reserved(hit: bool) -> Check {
    if hit { Err("reserved-id") } else { Ok(()) }
}

/// Flowchart words rejected as an id, alone or before a `-` (exact case), wherever a node goes.
const FLOW_WORDS: [&str; 13] = [
    "end",
    "style",
    "class",
    "graph",
    "flowchart",
    "subgraph",
    "interpolate",
    "classDef",
    "linkStyle",
    "_self",
    "_blank",
    "_parent",
    "_top",
];

/// Flowchart words rejected as a bare id, but fine in front of a shape (`click["x"]`). The bare
/// `click --> b` is worse than rejected: Mermaid accepts it as an empty graph.
const FLOW_BARE_WORDS: [&str; 3] = ["click", "href", "call"];

/// A flowchart node reference; `after` is the text right behind the id.
pub(super) fn flow_ref(id: &str, after: &str) -> Check {
    valid_id(id)?;
    let shaped = after.starts_with(['[', '(', '{']);
    reserved(FLOW_WORDS.contains(&head(id)) || (!shaped && FLOW_BARE_WORDS.contains(&id)))
}

/// Subgraph words rejected before a `-` (`default-x` too), and ones rejected only as the whole id.
const SUBGRAPH_HEADS: [&str; 6] = [
    "interpolate",
    "default",
    "_self",
    "_blank",
    "_parent",
    "_top",
];
const SUBGRAPH_WORDS: [&str; 3] = ["click", "href", "call"];

/// A `subgraph` id. Words that are statements elsewhere (`style`, `class`, `classDef`, ...) are
/// fine here, and `end` is fine only when the subgraph has a title.
pub(super) fn subgraph_id(id: &str, titled: bool) -> Check {
    valid_id(id)?;
    reserved(
        SUBGRAPH_HEADS.contains(&head(id))
            || SUBGRAPH_WORDS.contains(&id)
            || (id == "end" && !titled),
    )
}

/// Where a state id sits in a transition.
#[derive(Clone, Copy)]
pub(super) enum StateAt {
    Source,
    /// A target with nothing behind it.
    Target,
    /// A target followed by `: label`.
    Labelled,
}

/// Words that are no state id in any position (any case).
const STATE_WORDS: [&str; 3] = ["click", "href", "default"];
/// Statement words (any case): rejected as a source or a bare target, text in front of a label.
/// `state --> A` parses but declares something else, so it is refused too.
const STATE_STATEMENTS: [&str; 6] = ["style", "class", "classdef", "note", "scale", "state"];
/// Accessibility words (any case): rejected only in front of a label.
const STATE_ACC: [&str; 2] = ["acctitle", "accdescr"];

/// A state id (Mermaid's state ids have no `-`). `root_start` / `root_end` (exact case) are the
/// ids Mermaid gives `[*]`, so a state of that name would be merged with the terminals.
pub(super) fn state_id(id: &str, at: StateAt) -> Check {
    valid_id(id)?;
    if id.contains('-') {
        return Err("invalid-id");
    }
    let w = id.to_ascii_lowercase();
    let w = w.as_str();
    reserved(
        STATE_WORDS.contains(&w)
            || matches!(id, "root_start" | "root_end")
            || match at {
                StateAt::Source | StateAt::Target => STATE_STATEMENTS.contains(&w),
                StateAt::Labelled => STATE_ACC.contains(&w),
            },
    )
}

/// Where a sequence id sits.
#[derive(Clone, Copy)]
pub(super) enum SeqAt {
    /// `participant <id>`: Mermaid takes any word, it just cannot be used afterwards.
    Declared,
    /// Before the arrow.
    Sender,
    /// After the arrow, or in a `Note over`.
    Receiver,
}

/// Block and statement keywords (any case): rejected alone or before a `-`.
const SEQ_WORDS: [&str; 25] = [
    "loop",
    "alt",
    "else",
    "opt",
    "par",
    "and",
    "critical",
    "option",
    "break",
    "rect",
    "end",
    "actor",
    "participant",
    "autonumber",
    "box",
    "create",
    "destroy",
    "activate",
    "deactivate",
    "note",
    "over",
    "link",
    "links",
    "properties",
    "details",
];

/// A sequence id. `title` is only a statement when followed by a space, so it is rejected as a
/// whole receiver only; `accTitle` / `accDescr` likewise. `-x` is the cross arrow wherever it sits.
pub(super) fn seq_id(id: &str, at: SeqAt) -> Check {
    valid_id(id)?;
    if matches!(at, SeqAt::Declared) {
        return Ok(());
    }
    let w = id.to_ascii_lowercase();
    if w.contains("-x") {
        return Err("invalid-id");
    }
    let receiver = matches!(at, SeqAt::Receiver);
    reserved(
        SEQ_WORDS.contains(&head(&w))
            || (receiver && matches!(w.as_str(), "title" | "acctitle" | "accdescr")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rule() {
        assert_eq!(valid_id("a-b_1"), Ok(()));
        assert_eq!(valid_id("1a"), Err("invalid-id"));
        assert_eq!(valid_id("node"), Err("reserved-id"));
    }

    #[test]
    fn positions_decide() {
        assert_eq!(flow_ref("click", "[x]"), Ok(()));
        assert_eq!(flow_ref("click", " --> b"), Err("reserved-id"));
        assert_eq!(subgraph_id("end", true), Ok(()));
        assert_eq!(subgraph_id("end", false), Err("reserved-id"));
        assert_eq!(state_id("note", StateAt::Labelled), Ok(()));
        assert_eq!(state_id("note", StateAt::Target), Err("reserved-id"));
        assert_eq!(seq_id("title", SeqAt::Sender), Ok(()));
        assert_eq!(seq_id("title", SeqAt::Receiver), Err("reserved-id"));
        assert_eq!(seq_id("loop", SeqAt::Declared), Ok(()));
    }
}
