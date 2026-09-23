//! Edge resolution for `rebuild.rs`. Split out to stay under the TOESTUB 500-line cap;
//! no behavior of its own.

use crate::ast::ExtractedEdge;

/// Resolve each call target to a qualified definition id. Target shapes (see `ast`):
/// an exact node id (`Self::f()`) is kept; `Type::f` matches methods whose container
/// self type is `Type`; a bare `f` matches non-method definitions only (a bare call
/// cannot reach a method). Among candidates, preference: same file; else the unique
/// same-crate definition (beats any cross-crate homonym); else the unique global
/// definition. Ambiguous, unresolved, and self-edges are dropped (honesty rule: never
/// invent an edge).
pub(crate) fn resolve_edges(
    nodes: &[crate::ast::ExtractedNode],
    edges: &[crate::ast::ExtractedEdge],
) -> Vec<crate::ast::ExtractedEdge> {
    use std::collections::HashMap;
    /// The file path: ids are `<path>::<container>*::<name>` and paths never hold `::`.
    fn module_of(id: &str) -> &str {
        id.split("::").next().unwrap_or("")
    }
    /// Name without a `#n` duplicate-disambiguation suffix (a TS `#private` name keeps
    /// its `#` because the suffix must be all digits).
    fn bare_name(id: &str) -> &str {
        let tail = id.rsplit("::").next().unwrap_or(id);
        match tail.rsplit_once('#') {
            Some((name, n)) if !name.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => name,
            _ => tail,
        }
    }
    /// `Type::name` for a method id: the container's self type (`Foo`, or `Foo` out of
    /// `<Foo as Trait>`; a trait's own name for trait method declarations).
    fn method_key(id: &str) -> Option<String> {
        let mut segs = id.rsplit("::");
        let name = bare_name(segs.next()?);
        let container = segs.next()?;
        let self_ty = container
            .strip_prefix('<')
            .and_then(|c| c.split_once(" as "))
            .map_or(container, |(ty, _)| ty);
        Some(format!("{self_ty}::{name}"))
    }
    /// "crates/<name>/…::sym" -> "crates/<name>"; anything else -> "" (no scoping).
    fn crate_scope(id: &str) -> &str {
        let path = id.split("::").next().unwrap_or("");
        let mut it = path.splitn(3, '/');
        match (it.next(), it.next()) {
            (Some("crates"), Some(name)) if !name.is_empty() => {
                &path[.."crates/".len() + name.len()]
            }
            _ => "",
        }
    }
    // Bare names -> non-method defs; `Type::name` -> methods.
    let mut defs_by_name: HashMap<String, Vec<String>> = HashMap::new();
    for n in nodes {
        let key = if n.kind == "method" {
            match method_key(&n.id) {
                Some(k) => k,
                None => continue,
            }
        } else {
            bare_name(&n.id).to_string()
        };
        defs_by_name.entry(key).or_default().push(n.id.clone());
    }
    use std::collections::HashSet;
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    edges
        .iter()
        .filter_map(|e| {
            // Boundary targets (cmd:/tool:/surface:) are dead-end-preserving: keep the
            // edge whether or not the target node exists. If it is missing, downgrade the
            // confidence to "dangling" so coverage can find the dead-end.
            if e.target.starts_with("cmd:")
                || e.target.starts_with("tool:")
                || e.target.starts_with("surface:")
            {
                let confidence = if node_ids.contains(e.target.as_str()) {
                    e.confidence.clone()
                } else {
                    "dangling".to_string()
                };
                return Some(ExtractedEdge {
                    source: e.source.clone(),
                    target: e.target.clone(),
                    confidence,
                });
            }
            if node_ids.contains(e.target.as_str()) {
                return (e.target != e.source).then(|| e.clone());
            }
            let candidates = defs_by_name.get(&e.target)?;
            let src_mod = module_of(&e.source);
            let same: Vec<&String> = candidates
                .iter()
                .filter(|id| module_of(id) == src_mod)
                .collect();
            let src_crate = crate_scope(&e.source);
            let same_crate: Vec<&String> = if src_crate.is_empty() {
                Vec::new()
            } else {
                candidates
                    .iter()
                    .filter(|id| crate_scope(id) == src_crate)
                    .collect()
            };
            let target = if same.len() == 1 {
                same[0].clone()
            } else if same_crate.len() == 1 {
                // A bare name defined once in the caller's own crate is far more
                // likely the intended callee than any cross-crate homonym.
                same_crate[0].clone()
            } else if candidates.len() == 1 {
                candidates[0].clone()
            } else {
                return None;
            };
            if target == e.source {
                return None; // self-edge
            }
            Some(ExtractedEdge {
                source: e.source.clone(),
                target,
                confidence: e.confidence.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use crate::ast::{ExtractedEdge, ExtractedNode};

    fn n(id: &str) -> ExtractedNode {
        ExtractedNode {
            id: id.to_string(),
            label: id.rsplit("::").next().unwrap_or(id).to_string(),
            kind: "fn".to_string(),
        }
    }

    fn e(source: &str, target: &str) -> ExtractedEdge {
        ExtractedEdge {
            source: source.to_string(),
            target: target.to_string(),
            confidence: "resolved".to_string(),
        }
    }

    fn m(id: &str) -> ExtractedNode {
        ExtractedNode {
            kind: "method".to_string(),
            ..n(id)
        }
    }

    #[test]
    fn bare_call_never_resolves_to_a_method() {
        // A bare `run()` cannot call a method; methods named `run` must not make
        // the free fn ambiguous.
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            n("crates/aaa/src/y.rs::run"),
            m("crates/aaa/src/x.rs::Foo::run"),
            m("crates/aaa/src/y.rs::<Bar as Go>::run"),
        ];
        let edges = vec![e("crates/aaa/src/x.rs::caller", "run")];
        let resolved = resolve_edges(&nodes, &edges);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, "crates/aaa/src/y.rs::run");
    }

    #[test]
    fn type_qualified_hint_resolves_to_method() {
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            m("crates/aaa/src/y.rs::Foo::new"),
            m("crates/aaa/src/y.rs::Bar::new"),
            m("crates/bbb/src/z.rs::Foo::new"),
            m("crates/aaa/src/y.rs::<Foo as Go>::run"),
        ];
        let edges = vec![
            e("crates/aaa/src/x.rs::caller", "Foo::new"),
            e("crates/aaa/src/x.rs::caller", "Foo::run"),
            e("crates/aaa/src/x.rs::caller", "Baz::new"),
        ];
        let targets: Vec<String> = resolve_edges(&nodes, &edges)
            .into_iter()
            .map(|e| e.target)
            .collect();
        assert_eq!(
            targets,
            vec![
                "crates/aaa/src/y.rs::Foo::new",
                "crates/aaa/src/y.rs::<Foo as Go>::run"
            ]
        );
    }

    #[test]
    fn exact_qualified_target_is_kept() {
        let nodes = vec![m("a.rs::Foo::new"), m("a.rs::Foo::build")];
        let edges = vec![e("a.rs::Foo::new", "a.rs::Foo::build")];
        let out = resolve_edges(&nodes, &edges);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].target, "a.rs::Foo::build");
    }

    #[test]
    fn disambiguated_duplicates_are_both_candidates() {
        // cfg-gated `plat` / `plat#2`: which one a caller hits is unknowable, so the
        // same-module call is ambiguous and dropped (honesty rule).
        let nodes = vec![n("a.rs::caller"), n("a.rs::plat"), n("a.rs::plat#2")];
        let edges = vec![e("a.rs::caller", "plat")];
        assert!(resolve_edges(&nodes, &edges).is_empty());
    }

    #[test]
    fn resolver_recovers_same_crate_ambiguous_target() {
        // `run` is defined in the caller's crate AND another crate (ambiguous
        // globally). Old behavior: dropped. New: resolves to the same-crate def.
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            n("crates/aaa/src/y.rs::run"),
            n("crates/bbb/src/z.rs::run"),
        ];
        let edges = vec![e("crates/aaa/src/x.rs::caller", "run")];
        let resolved = resolve_edges(&nodes, &edges);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, "crates/aaa/src/y.rs::run");
    }

    #[test]
    fn resolver_same_module_still_wins_over_same_crate() {
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            n("crates/aaa/src/x.rs::run"),
            n("crates/aaa/src/y.rs::run"),
        ];
        let edges = vec![e("crates/aaa/src/x.rs::caller", "run")];
        let resolved = resolve_edges(&nodes, &edges);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, "crates/aaa/src/x.rs::run");
    }

    #[test]
    fn resolver_drops_ambiguous_within_same_crate() {
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            n("crates/aaa/src/y.rs::run"),
            n("crates/aaa/src/z.rs::run"),
        ];
        let edges = vec![e("crates/aaa/src/x.rs::caller", "run")];
        assert!(resolve_edges(&nodes, &edges).is_empty());
    }

    #[test]
    fn resolver_unique_global_cross_crate_still_resolves() {
        let nodes = vec![
            n("crates/aaa/src/x.rs::caller"),
            n("crates/bbb/src/z.rs::unique_fn"),
        ];
        let edges = vec![e("crates/aaa/src/x.rs::caller", "unique_fn")];
        let resolved = resolve_edges(&nodes, &edges);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, "crates/bbb/src/z.rs::unique_fn");
    }

    #[test]
    fn same_module_unique_resolves_and_drops_ambiguous() {
        let nodes = vec![
            ExtractedNode {
                id: "m.rs::a".into(),
                label: "a".into(),
                kind: "fn".into(),
            },
            ExtractedNode {
                id: "m.rs::b".into(),
                label: "b".into(),
                kind: "fn".into(),
            },
        ];
        let edges = vec![ExtractedEdge {
            source: "m.rs::a".into(),
            target: "b".into(),
            confidence: "resolved".into(),
        }];
        let out = resolve_edges(&nodes, &edges);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].target, "m.rs::b");
    }

    #[test]
    fn prefixed_target_resolves_or_dangles() {
        use crate::ast::{ExtractedEdge, ExtractedNode};
        let nodes = vec![ExtractedNode {
            id: "cmd:real".into(),
            label: "real".into(),
            kind: "command".into(),
        }];
        let edges = vec![
            ExtractedEdge {
                source: "S.tsx::a".into(),
                target: "cmd:real".into(),
                confidence: "declared".into(),
            },
            ExtractedEdge {
                source: "S.tsx::b".into(),
                target: "cmd:gone".into(),
                confidence: "declared".into(),
            },
        ];
        let out = resolve_edges(&nodes, &edges);
        assert!(
            out.iter()
                .any(|e| e.target == "cmd:real" && e.confidence == "declared")
        );
        let dangling = out
            .iter()
            .find(|e| e.target == "cmd:gone")
            .expect("dead-end edge must survive");
        assert_eq!(dangling.confidence, "dangling");
    }
}
