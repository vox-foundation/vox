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

/// The grid: header cells, then one cell vector per row.
fn grid(md: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let body = md
        .split("## Sizes and spokes\n")
        .nth(1)
        .expect("grid heading");
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
        "tags",
        "language",
    ] {
        assert!(!fm[key].is_null(), "missing `{key}`");
    }
    assert!(
        fm["tags"]
            .as_sequence()
            .unwrap()
            .iter()
            .any(|t| t.as_str() == Some("index"))
    );
}

#[test]
fn index_grid_has_one_row_per_size_and_one_column_per_spoke() {
    let (header, rows) = grid(&full_ladder());
    let cfg = hub_cards().expect("hub-cards.yaml");
    let spoke_labels: Vec<&str> = cfg.spokes.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(header.first().map(String::as_str), Some("Size"));
    assert_eq!(
        header[1..=spoke_labels.len()],
        spoke_labels,
        "one column per spoke, in yaml order"
    );
    assert_eq!(
        header[spoke_labels.len() + 1..],
        ["Hub".to_string(), "Repo".to_string()]
    );
    let sizes: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert_eq!(sizes, SIZES, "one row per size, in ladder order");
    for row in &rows {
        assert_eq!(row.len(), header.len(), "{}: ragged row", row[0]);
        for (spoke, cell) in cfg.spokes.iter().zip(&row[1..]) {
            match spoke.status {
                SpokeStatus::Planned => {
                    let link = spoke.link.as_deref().unwrap();
                    assert_eq!(cell, &format!("[planned]({link})"), "{}", row[0]);
                    assert!(link.ends_with("#hub-and-spoke"));
                }
                SpokeStatus::Trained => assert!(
                    [
                        "passed",
                        "needs re-gate",
                        "failed",
                        "not evaluated",
                        "not trained"
                    ]
                    .contains(&cell.as_str()),
                    "{}: {cell}",
                    row[0]
                ),
            }
        }
    }
    let by_size = |s: &str| rows.iter().find(|r| r[0] == s).unwrap().clone();
    let trained = 1; // the one trained spoke column
    assert_eq!(by_size("0.6B")[trained], "failed");
    assert_eq!(by_size("1.7B")[trained], "needs re-gate");
    assert_eq!(by_size("4B")[trained], "passed");
    assert_eq!(by_size("8B")[trained], "needs re-gate");
    assert_eq!(by_size("14B")[trained], "not evaluated");
    assert_eq!(by_size("32B")[trained], "not trained");
}

#[test]
fn index_hub_and_repo_cells_keep_their_wording() {
    let (header, rows) = grid(&full_ladder());
    let hub = header.len() - 2;
    let row = |s: &str| rows.iter().find(|r| r[0] == s).unwrap().clone();
    assert_eq!(row("0.6B")[hub], "`v5` on Hub; local run differs");
    assert_eq!(row("4B")[hub], "published `v2`");
    assert_eq!(row("14B")[hub], "not published");
    assert_eq!(row("32B")[hub], "no weights");
    assert_eq!(
        row("4B")[hub + 1],
        "[ns/vox-mens-qwen3-4b](https://huggingface.co/ns/vox-mens-qwen3-4b)"
    );
}

#[test]
fn hub_cell_covers_every_sync_state() {
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
}

#[test]
fn index_rows_follow_ladder_order_whatever_the_input_order() {
    use crate::mens::hub_card::index_facts::index_facts;
    let tmp = tempfile::tempdir().unwrap();
    let mut entries = ladder_entries(tmp.path());
    entries.reverse();
    let sizes: Vec<String> = index_facts("ns", &entries)
        .rows
        .iter()
        .map(|r| r.size.clone())
        .collect();
    assert_eq!(sizes, SIZES);
}

#[test]
fn index_without_passed_sizes_still_renders_every_row() {
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_no_passed(tmp.path()));
    let (_, rows) = grid(&md);
    assert_eq!(rows.len(), SIZES.len());
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_passed_but_stale(tmp.path()));
    assert_eq!(grid(&md).1.len(), SIZES.len());
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
            "Sizes and spokes",
            "Which model should I use",
            "What's next",
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
        which.contains("Sizes that need a held-out re-evaluation under the current serving path: ")
            && which.contains(SIZES[2]),
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
        which.contains("No size has a current gate, so this index recommends none"),
        "{which}"
    );
    // Every trained size whose gate is not passed is named, in ladder order.
    let trained = [SIZES[0], SIZES[1], SIZES[2], SIZES[3], SIZES[4]];
    assert!(
        which.contains(&format!(
            "Sizes whose gate is not passed: {}.",
            trained.join(", ")
        )),
        "{which}"
    );
    assert!(
        !which.contains(SIZES[5]),
        "the untrained size is not pending"
    );

    // With a passed, current size the pick and its pull command appear.
    let md = full_ladder();
    let which = section(&md, "Which model should I use");
    assert!(
        which.contains(&format!("{PULL} --size {} --namespace ns", SIZES[2])),
        "{which}"
    );
    assert!(which.contains(&format!("| {} | ", SIZES[2])));
}

#[test]
fn whats_next_from_records() {
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries(tmp.path()));
    let next = section(&md, "What's next");
    let items: Vec<&str> = next.split("\n- ").skip(1).collect();
    let cfg = hub_cards().unwrap();
    let planned: Vec<_> = cfg
        .spokes
        .iter()
        .filter(|s| s.status == SpokeStatus::Planned)
        .collect();
    // One item per pending trained size, then one per planned spoke.
    assert_eq!(items.len(), 4 + planned.len(), "{next}");
    let item_of = |size: &str| {
        items
            .iter()
            .find(|i| i.starts_with(&format!("**{size}**:")))
            .unwrap_or_else(|| panic!("no item for {size}: {next}"))
    };
    let (failed, stale, passed_stale, re_gate, missing) =
        (SIZES[0], SIZES[1], SIZES[2], SIZES[3], SIZES[4]);
    assert!(
        items
            .iter()
            .all(|i| !i.starts_with(&format!("**{passed_stale}**")))
    );

    // 1.7B: the re-gate and the re-evaluation, with its yaml estimate.
    let it = item_of(stale);
    assert!(
        it.contains("vox mens eval-collateral-damage") && it.contains("vox mens eval-local"),
        "{it}"
    );
    let eta = cfg.sizes[stale].regate_eta.as_ref().unwrap();
    assert!(
        it.contains(&format!(
            "about {:.1} hours, or {:.1} hours if slower",
            eta.hours_expected, eta.hours_conservative
        )) && it.contains("This is an estimate, not a measurement")
            && it.contains(&format!("Basis: {}", eta.basis)),
        "{it}"
    );
    // 8B: only the re-gate (its adapter evidence is current).
    let it = item_of(re_gate);
    assert!(
        it.contains("vox mens eval-collateral-damage") && !it.contains("eval-local"),
        "{it}"
    );
    assert!(it.contains("This is an estimate"), "{it}");
    // 14B: the missing records, with its own estimate.
    let it = item_of(missing);
    assert!(
        it.contains("produce the records that are missing for")
            && it.contains("`collateral_damage_report.json`"),
        "{it}"
    );
    assert!(it.contains("This is an estimate"), "{it}");
    // 0.6B: a gate that measured the adapter and failed it has no re-gate estimate.
    let it = item_of(failed);
    assert!(
        it.contains("review the gate")
            && it.contains(&format!("There is no estimate for {failed}.")),
        "{it}"
    );
    // Planned spokes: label, purpose, link, no estimate.
    for s in planned {
        let it = items
            .iter()
            .find(|i| i.starts_with(&format!("**{}**", s.label)))
            .unwrap_or_else(|| panic!("no item for {}", s.label));
        assert!(
            it.contains(&s.purpose) && it.contains(s.link.as_deref().unwrap()),
            "{it}"
        );
        assert!(it.contains("there is no estimate for it"), "{it}");
    }
    // What's next names actions, never a gate label.
    for label in [
        "passed",
        "needs re-gate",
        "failed",
        "not evaluated",
        "not trained",
    ] {
        assert!(
            !next.contains(label),
            "`{label}` restated in What's next: {next}"
        );
    }
}

#[test]
fn whats_next_names_a_stale_passed_size_with_only_the_re_evaluation() {
    let tmp = tempfile::tempdir().unwrap();
    let md = render(&ladder_entries_passed_but_stale(tmp.path()));
    let next = section(&md, "What's next");
    let item = next
        .split("\n- ")
        .find(|i| i.starts_with(&format!("**{}**:", SIZES[2])))
        .expect("4B item");
    assert!(
        item.contains("vox mens eval-local") && !item.contains("eval-collateral-damage"),
        "{item}"
    );
    assert!(
        item.contains(&format!("There is no estimate for {}.", SIZES[2])),
        "{item}"
    );
}

#[test]
fn whats_next_is_empty_of_sizes_when_nothing_is_pending() {
    let tmp = tempfile::tempdir().unwrap();
    let mut entries = ladder_entries(tmp.path());
    entries.retain(|(r, _)| r.size == SIZES[2] || r.size == SIZES[5]);
    let next = section(&render(&entries), "What's next").to_string();
    assert!(!next.contains(&format!("**{}**", SIZES[2])), "{next}");
    assert!(next.contains("- **harness: tool-selection**"), "{next}");
}

#[test]
fn comparison_uses_current_protocol_only() {
    let tmp = tempfile::tempdir().unwrap();
    let mut entries = ladder_entries(tmp.path());
    // 8B's held-out reports are rewritten under a retired protocol and reloaded.
    let run = tmp
        .path()
        .join(format!("qwen3-{}", SIZES[3].to_ascii_lowercase()));
    for f in ["heldout_base_nothink.json", "heldout_adapter_nothink.json"] {
        let p = run.join("evidence").join(f);
        let mut v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        v["prompt_protocol"] = serde_json::json!("think128");
        std::fs::write(&p, v.to_string()).unwrap();
    }
    let e = entries
        .iter_mut()
        .find(|(r, _)| r.size == SIZES[3])
        .unwrap();
    e.1 = RunRecords::load(&run, SIZES[3], &e.0.base_pin);
    let md = render(&entries);
    let table = section(&md, "Held-out pass@1 by size");
    let rows: Vec<Vec<String>> = table
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Size"))
        .map(cells)
        .collect();
    let sizes: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    // 8B (retired protocol) and 32B (no reports) are absent; every row names its size.
    assert_eq!(sizes, [SIZES[0], SIZES[1], SIZES[2], SIZES[4]]);
    let row = |s: &str| rows.iter().find(|r| r[0] == s).unwrap();
    // 1.7B's adapter checkpoint predates the serving-path field: base cell, dash.
    assert_ne!(row(SIZES[1])[1], "not recorded");
    assert_eq!(row(SIZES[1])[2], "—");
    assert_ne!(row(SIZES[2])[2], "—");
    assert!(
        table
            .contains("an adapter score recorded under an earlier serving path is shown as a dash"),
        "{table}"
    );
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
    assert!(table.contains("so there is nothing to compare"), "{table}");
    assert!(!table.contains('|'), "{table}");
}
