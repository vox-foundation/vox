/// Task 2.6 — nucleo command palette fuzzy search.
use vox_term::ui::palette::{Palette, default_commands};

#[test]
fn fuzzy_mdl_matches_model() {
    let palette = Palette::new(default_commands());
    let results = palette.search("mdl");
    assert!(
        results.iter().any(|c| c.name == "/model"),
        "expected /model in results for query 'mdl', got: {results:?}",
        results = results.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

#[test]
fn empty_query_returns_all() {
    let palette = Palette::new(default_commands());
    let cmds = default_commands();
    let results = palette.search("");
    assert_eq!(results.len(), cmds.len());
}

#[test]
fn no_match_returns_empty() {
    let palette = Palette::new(default_commands());
    let results = palette.search("zzzzzzzzz");
    assert!(
        results.is_empty(),
        "expected no results, got {results:?}",
        results = results.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

#[test]
fn palette_includes_live_sessions() {
    let mut palette = Palette::new(default_commands());
    palette.with_sessions(&[vox_terminal_core::SessionInfo {
        id: "term-1".into(),
        blocks: 3,
    }]);
    let results = palette.search("term-1");
    assert!(
        results
            .iter()
            .any(|c| c.name == "/attach term-1" && c.description.contains("3 blocks")),
        "expected /attach term-1 in palette results, got: {results:?}"
    );
}
