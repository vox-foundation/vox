//! v2 index card: golden, grid shape, and the rules for Which model, What's next and the comparison.

use super::support::{
    index_row, ladder_entries, ladder_entries_no_passed, ladder_entries_passed_but_stale,
};
use crate::mens::hub_card::RunRecords;
use crate::mens::hub_card::config::{SpokeStatus, hub_cards};
use crate::mens::hub_card::render_index_readme;
use crate::mens::hub_repo::SIZES;
use crate::mens::hub_sync::{SizeSyncRow, SizeSyncStatus};

type Entries = Vec<(SizeSyncRow, RunRecords)>;

fn golden_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/hub_cards")
}

fn render(entries: &Entries) -> String {
    render_index_readme("ns", entries)
}

fn full_ladder() -> String {
    let tmp = tempfile::tempdir().expect("tempdir");
    render(&ladder_entries(tmp.path()))
}

/// Cells of one markdown table row, outer pipes and padding removed.
fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// The Available models table: header cells, then one cell vector per row.
fn available_table(md: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let body = md
        .split("## Available models\n")
        .nth(1)
        .expect("available models heading");
    let mut lines = body.lines().skip_while(|l| l.is_empty());
    let header = cells(lines.next().expect("header row"));
    lines.next().expect("separator row");
    let rows = lines
        .take_while(|l| l.starts_with('|'))
        .map(cells)
        .collect();
    (header, rows)
}

#[test]
fn index_golden_matches() {
    let got = full_ladder();
    let path = golden_dir().join("index.md");
    if std::env::var_os("VOX_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &got).unwrap();
        return;
    }
    assert_eq!(got, full_ladder(), "two renders differ");
    assert!(
        !got.contains("/Users/") && !got.contains("/home/"),
        "host path leaked"
    );
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run with VOX_UPDATE_GOLDEN=1)", path.display()));
    assert_eq!(
        got, want,
        "golden index drifted; review and run with VOX_UPDATE_GOLDEN=1"
    );
}

#[test]
fn index_frontmatter_parses_with_required_keys() {
    let md = full_ladder();
    let body = md.strip_prefix("---\n").expect("starts with frontmatter");
    let end = body.find("\n---\n").expect("frontmatter is closed");
    let fm: serde_yaml::Value = serde_yaml::from_str(&body[..end]).expect("valid YAML");
    for key in [
        "license",
        "library_name",
        "pipeline_tag",
        "spaces",
        "tags",
        "language",
    ] {
        assert!(!fm[key].is_null(), "missing `{key}`");
    }
    assert!(md.contains("assets/overview.jpg"), "missing overview.jpg");
    assert!(!md.contains(".svg"), "index card must not reference .svg");
    assert!(
        fm["tags"]
            .as_sequence()
            .unwrap()
            .iter()
            .any(|t| t.as_str() == Some("index"))
    );
}

#[test]
fn index_available_models_lists_only_gate_passed_models() {
    let md = full_ladder();
    let (header, rows) = available_table(&md);
    assert_eq!(
        header,
        [
            "Model",
            "Held-out pass@1",
            "Weights",
            "Peak memory",
            "Hub",
            "Repo",
            "Live Demo"
        ]
    );
    // In ladder_entries, only 4B (`SIZES[2]`) has GateState::Passed; unpassed/untrained rungs never leak.
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0][0], "**Vox Mens 4B**");
    assert_eq!(rows[0][4], "published `v2`");
    assert_eq!(
        rows[0][5],
        "[ns/vox-mens-qwen3.5-4b](https://huggingface.co/ns/vox-mens-qwen3.5-4b)"
    );
    for forbidden in [
        "## What's next",
        "[planned]",
        "not evaluated",
        "needs re-gate",
        "no weights",
        "collateral_damage_report.json",
    ] {
        assert!(!md.contains(forbidden), "leaked `{forbidden}` in:\n{md}");
    }
    let cfg = hub_cards().expect("hub-cards.yaml");
    assert!(
        cfg.spokes.iter().all(|s| s.status != SpokeStatus::Planned),
        "hub-cards.yaml must not list planned vaporware spokes"
    );
}

#[test]
fn index_without_passed_sizes_shows_clean_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_no_passed(tmp.path()));
    assert!(
        md.contains("No Vox Mens checkpoint has completed release verification"),
        "{md}"
    );
    assert!(!md.contains("not evaluated") && !md.contains("What's next"));
}

/// The text of one `## ` section, heading excluded, up to the next section.
fn section<'a>(md: &'a str, heading: &str) -> &'a str {
    let start = md
        .split_once(&format!("## {heading}\n"))
        .unwrap_or_else(|| panic!("no `## {heading}` section"))
        .1;
    start.split("\n## ").next().unwrap()
}

const PULL: &str = "vox mens hub pull";

#[test]
fn index_sections_in_spec_order() {
    let md = full_ladder();
    let headings: Vec<&str> = md.lines().filter_map(|l| l.strip_prefix("## ")).collect();
    assert_eq!(
        headings,
        [
            "Available models",
            "Which model should I use",
            "Held-out pass@1 by size",
            "Links"
        ]
    );
}

#[test]
fn pick_is_highest_passed_adapter_rate_ties_to_smaller() {
    use crate::mens::hub_card::facts::{AdapterEvidence::Current, GateState::Passed};
    use crate::mens::hub_card::index_facts::pick_size;
    let row = |i: usize, rate| index_row(SIZES[i], Passed, Current, Some(rate));
    // The highest rate wins.
    let rows = [row(0, 0.30), row(1, 0.50), row(2, 0.40)];
    assert_eq!(pick_size(&rows).unwrap().size, SIZES[1]);
    // A tie goes to the smaller size, in ladder order.
    let rows = [row(1, 0.50), row(2, 0.50), row(3, 0.50)];
    assert_eq!(pick_size(&rows).unwrap().size, SIZES[1]);
    // A size without an adapter score is never picked.
    let rows = [index_row(SIZES[0], Passed, Current, None), row(1, 0.10)];
    assert_eq!(pick_size(&rows).unwrap().size, SIZES[1]);
    assert_eq!(pick_size(&rows[..1]), None);
    assert_eq!(pick_size(&[]), None);
}

#[test]
fn pick_ignores_sizes_whose_gate_is_not_passed_however_high_they_score() {
    use crate::mens::hub_card::facts::{AdapterEvidence::Current, GateState};
    use crate::mens::hub_card::index_facts::pick_size;
    let rows = [
        index_row(SIZES[1], GateState::Passed, Current, Some(0.30)),
        index_row(SIZES[2], GateState::Failed, Current, Some(0.90)),
        index_row(SIZES[3], GateState::NeedsReGate, Current, Some(0.80)),
        index_row(SIZES[4], GateState::NotEvaluated, Current, Some(0.70)),
    ];
    assert_eq!(pick_size(&rows).unwrap().size, SIZES[1]);
}

#[test]
fn pick_requires_current_serving() {
    use crate::mens::hub_card::facts::{AdapterEvidence, GateState::Passed};
    use crate::mens::hub_card::index_facts::pick_size;
    let stale = AdapterEvidence::Stale { recorded: None };
    let rows = [
        index_row(SIZES[1], Passed, stale, Some(0.90)),
        index_row(SIZES[2], Passed, AdapterEvidence::NoCheckpoint, Some(0.80)),
    ];
    assert_eq!(pick_size(&rows), None);

    // On the ladder: 4B passed its gate but its checkpoint predates the serving-path field.
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_passed_but_stale(tmp.path()));
    let which = section(&md, "Which model should I use");
    assert!(!which.contains(PULL), "{which}");
    assert!(
        which.contains("No verified model with a current-serving benchmark score"),
        "{which}"
    );
    assert!(!md.contains(PULL), "no size may carry a pull command");
}

#[test]
fn index_names_no_pick_while_regates_pending() {
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_no_passed(tmp.path()));
    assert!(!md.contains(PULL), "no pull command without a current gate");
    let which = section(&md, "Which model should I use");
    assert!(
        which.contains("No verified model with a current-serving benchmark score"),
        "{which}"
    );

    // With a passed, current size the pick and its pull command appear.
    let md = full_ladder();
    let which = section(&md, "Which model should I use");
    assert!(
        which.contains(&format!("{PULL} --size {} --namespace ns", SIZES[2])),
        "{which}"
    );
    assert!(which.contains(&format!("**Vox Mens {}**", SIZES[2])));
}

#[test]
fn hub_cell_and_row_order_preserve_ladder_invariants() {
    use crate::mens::hub_card::index_facts::{HubCell, index_facts};
    let tmp = tempfile::tempdir().unwrap();
    let mut entries = ladder_entries(tmp.path());
    let want = [
        (
            SizeSyncStatus::HubOnly,
            Some("v3"),
            HubCell::HubOnly {
                version: Some("v3".into()),
            },
        ),
        (
            SizeSyncStatus::InSync,
            None,
            HubCell::Published { version: None },
        ),
        (
            SizeSyncStatus::Diverged,
            None,
            HubCell::Diverged { version: None },
        ),
        (SizeSyncStatus::LocalOnly, None, HubCell::NotPublished),
        (SizeSyncStatus::NotTrained, None, HubCell::NoWeights),
    ];
    for (status, version, cell) in want {
        entries[0].0.sync_status = status;
        entries[0].0.latest_version = version.map(str::to_string);
        assert_eq!(index_facts("ns", &entries).rows[0].hub, cell);
    }
    entries.reverse();
    let sizes: Vec<String> = index_facts("ns", &entries)
        .rows
        .iter()
        .map(|r| r.size.clone())
        .collect();
    assert_eq!(sizes, SIZES);
}

#[test]
fn comparison_uses_passed_current_protocol_only() {
    let md = full_ladder();
    let table = section(&md, "Held-out pass@1 by size");
    let rows: Vec<Vec<String>> = table
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Model"))
        .map(cells)
        .collect();
    let models: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    // Only gate-passed models with current serving evidence appear in the public comparison table.
    assert_eq!(models, [format!("**Vox Mens {}**", SIZES[2]).as_str()]);
    assert!(!rows[0][2].contains('—'), "{:?}", rows[0]);
}

#[test]
fn comparison_with_no_current_report_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let mut entries = ladder_entries(tmp.path());
    for (_, rec) in &mut entries {
        rec.heldout_base = None;
        rec.heldout_adapter = None;
    }
    let table = section(&render(&entries), "Held-out pass@1 by size").to_string();
    assert!(
        table.contains("No release-verified model has a held-out report"),
        "{table}"
    );
    assert!(!table.contains('|'), "{table}");
}
