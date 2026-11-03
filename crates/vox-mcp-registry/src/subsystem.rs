//! Bootable daemon subsystems and their dependency graph (capability profiles).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// A daemon boot step a tool can require.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Db,
    Sessions,
    AgentFleet,
    ModelRegistry,
    LlmFacade,
    SearchIndex,
    Memory,
    Research,
    Mesh,
    Attention,
    Clarification,
    Flywheel,
    Compiler,
    Browser,
    Speech,
}

impl Subsystem {
    /// Every variant, in `Ord` order.
    pub const ALL: &'static [Subsystem] = &[
        Subsystem::Db,
        Subsystem::Sessions,
        Subsystem::AgentFleet,
        Subsystem::ModelRegistry,
        Subsystem::LlmFacade,
        Subsystem::SearchIndex,
        Subsystem::Memory,
        Subsystem::Research,
        Subsystem::Mesh,
        Subsystem::Attention,
        Subsystem::Clarification,
        Subsystem::Flywheel,
        Subsystem::Compiler,
        Subsystem::Browser,
        Subsystem::Speech,
    ];

    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Subsystem::Db => "db",
            Subsystem::Sessions => "sessions",
            Subsystem::AgentFleet => "agent_fleet",
            Subsystem::ModelRegistry => "model_registry",
            Subsystem::LlmFacade => "llm_facade",
            Subsystem::SearchIndex => "search_index",
            Subsystem::Memory => "memory",
            Subsystem::Research => "research",
            Subsystem::Mesh => "mesh",
            Subsystem::Attention => "attention",
            Subsystem::Clarification => "clarification",
            Subsystem::Flywheel => "flywheel",
            Subsystem::Compiler => "compiler",
            Subsystem::Browser => "browser",
            Subsystem::Speech => "speech",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|x| x.as_str() == s)
    }

    /// Direct dependencies (must boot first).
    pub const fn deps(self) -> &'static [Subsystem] {
        use Subsystem::*;
        match self {
            Sessions => &[Db],
            AgentFleet => &[Sessions],
            LlmFacade => &[ModelRegistry],
            SearchIndex => &[Db],
            Memory => &[Db, SearchIndex],
            Research => &[LlmFacade, SearchIndex, Db],
            Mesh | Clarification | Flywheel => &[Db],
            Db | ModelRegistry | Attention | Compiler | Browser | Speech => &[],
        }
    }
}

/// Transitive closure including the roots, deterministic order.
pub fn subsystem_closure(roots: &[Subsystem]) -> BTreeSet<Subsystem> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<Subsystem> = roots.to_vec();
    while let Some(s) = stack.pop() {
        if out.insert(s) {
            stack.extend_from_slice(s.deps());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn all_lists_every_variant_once_in_ord_order() {
        assert_eq!(Subsystem::ALL.len(), 15);
        let mut sorted = Subsystem::ALL.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.as_slice(), Subsystem::ALL);
    }

    #[test]
    fn as_str_round_trips_and_matches_serde() {
        for &s in Subsystem::ALL {
            assert_eq!(Subsystem::parse(s.as_str()), Some(s));
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_str())
            );
        }
    }

    #[test]
    fn parse_rejects_non_canonical_spellings() {
        for bad in ["", "AgentFleet", "agent-fleet", "orchestrator", "skills"] {
            assert_eq!(Subsystem::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn deps_match_contract_table() {
        use Subsystem::*;
        assert_eq!(Sessions.deps(), &[Db]);
        assert_eq!(AgentFleet.deps(), &[Sessions]);
        assert_eq!(LlmFacade.deps(), &[ModelRegistry]);
        assert_eq!(SearchIndex.deps(), &[Db]);
        assert_eq!(Memory.deps(), &[Db, SearchIndex]);
        assert_eq!(Research.deps(), &[LlmFacade, SearchIndex, Db]);
        for s in [Mesh, Clarification, Flywheel] {
            assert_eq!(s.deps(), &[Db], "{s:?}");
        }
        for s in [Db, ModelRegistry, Attention, Compiler, Browser, Speech] {
            assert!(s.deps().is_empty(), "{s:?}");
        }
    }

    #[test]
    fn closure_includes_roots_and_transitive_deps() {
        use Subsystem::*;
        assert_eq!(
            subsystem_closure(&[AgentFleet]),
            BTreeSet::from([AgentFleet, Sessions, Db])
        );
        assert_eq!(
            subsystem_closure(&[Research]),
            BTreeSet::from([Research, LlmFacade, ModelRegistry, SearchIndex, Db])
        );
        assert_eq!(
            subsystem_closure(&[ModelRegistry, Compiler]),
            BTreeSet::from([ModelRegistry, Compiler])
        );
        assert!(subsystem_closure(&[]).is_empty());
    }

    #[test]
    fn closure_of_all_is_all_and_closure_is_idempotent() {
        assert_eq!(
            subsystem_closure(Subsystem::ALL).len(),
            Subsystem::ALL.len()
        );
        for &s in Subsystem::ALL {
            let once = subsystem_closure(&[s]);
            let roots: Vec<Subsystem> = once.iter().copied().collect();
            assert_eq!(subsystem_closure(&roots), once, "{s:?}");
        }
    }

    #[test]
    fn dependency_graph_is_acyclic() {
        for &s in Subsystem::ALL {
            assert!(
                !subsystem_closure(s.deps()).contains(&s),
                "{s:?} transitively depends on itself"
            );
        }
    }
}
