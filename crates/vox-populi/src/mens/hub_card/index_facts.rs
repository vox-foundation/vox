//! The one input the index template reads (spec decision D4, section 5).
//!
//! [`index_facts`] turns every size's sync row and records into typed facts. It reads no file and
//! writes no sentence: every word the index says lives in `templates/hub/index.md`. A size's gate
//! comes from [`gate_of`] and its adapter evidence from [`adapter_evidence`], the same functions
//! the model cards use, so the two cannot disagree.

use super::RunRecords;
use super::card_facts::{DOCS, HeldOut, held_out};
use super::config::{RegateEta, SpokeEntry, SpokeStatus, hub_cards};
use super::facts::{
    AdapterEvidence, Blocker, GateState, PROMPT_PROTOCOL, active_spoke_id, adapter_evidence,
    gate_of,
};
use crate::mens::hub_repo::SIZES;
use crate::mens::hub_sync::{SizeSyncRow, SizeSyncStatus};

/// Hub column of the grid, from the sync row (one case per state the sync classifier reports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HubCell {
    Published { version: Option<String> },
    Diverged { version: Option<String> },
    HubOnly { version: Option<String> },
    NotPublished,
    NoWeights,
}

/// One grid cell: a size under one spoke.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GridCell {
    /// The size is trained for this spoke: its gate.
    Gate(GateState),
    NotTrained,
    Planned {
        link: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct IndexRow {
    pub size: String,
    pub repo_id: String,
    pub trained: bool,
    pub spoke_id: Option<String>,
    pub gate: GateState,
    pub adapter_evidence: AdapterEvidence,
    pub hub: HubCell,
    pub peak_mb: Option<u64>,
    /// Held-out pass@1 under the current prompt protocol; adapter fields only for `Current` evidence.
    pub heldout: Option<HeldOut>,
    /// One cell per spoke, in `hub-cards.yaml` order.
    pub cells: Vec<GridCell>,
}

impl IndexRow {
    pub fn adapter_cell(&self) -> String {
        self.heldout
            .as_ref()
            .map_or_else(|| "—".to_string(), HeldOut::adapter_cell)
    }

    pub fn peak_text(&self) -> String {
        self.peak_mb
            .map_or_else(|| "not recorded".to_string(), |mb| format!("{mb} MiB"))
    }
}

/// The size the "Which model" rule picks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pick {
    pub size: String,
    pub rate: f64,
    pub peak_mb: Option<u64>,
}

impl Pick {
    pub fn rate_text(&self) -> String {
        format!("{:.1}%", self.rate * 100.0)
    }
}

/// A trained size with work left: its gate is not passed, or its adapter score is not from the
/// current serving path.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub size: String,
    /// Why the gate is not passed; `None` when only the evidence is stale.
    pub blocker: Option<Blocker>,
    /// The adapter evidence is not `Current`.
    pub reevaluate: bool,
    /// The re-gate estimate, for the blockers a re-run can clear.
    pub eta: Option<RegateEta>,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexFacts {
    pub namespace: String,
    pub docs: &'static str,
    pub protocol: &'static str,
    /// The `hub-cards.yaml` error text when the file did not load.
    pub config_error: Option<String>,
    /// Where the declared spoke of every trained size comes from.
    pub spoke_source: Option<String>,
    pub spokes: Vec<SpokeEntry>,
    pub rows: Vec<IndexRow>,
    pub pick: Option<Pick>,
    pub pending: Vec<Pending>,
    pub planned_spokes: Vec<SpokeEntry>,
    /// Held-out pass@1 per size, only for sizes with a current-protocol report.
    pub comparison: Vec<(String, HeldOut)>,
}

impl IndexFacts {
    /// Rows whose gate is passed.
    pub fn passed_rows(&self) -> Vec<&IndexRow> {
        self.rows
            .iter()
            .filter(|r| r.gate == GateState::Passed)
            .collect()
    }

    /// Pending sizes whose gate is not passed.
    pub fn not_passed(&self) -> Vec<&Pending> {
        self.pending
            .iter()
            .filter(|p| p.blocker.is_some())
            .collect()
    }

    /// Pending sizes whose adapter score must be re-measured under the current serving path.
    pub fn needs_reevaluation(&self) -> Vec<&Pending> {
        self.pending.iter().filter(|p| p.reevaluate).collect()
    }
}

fn hub_cell(row: &SizeSyncRow) -> HubCell {
    let version = row.latest_version.clone();
    match row.sync_status {
        SizeSyncStatus::InSync => HubCell::Published { version },
        SizeSyncStatus::Diverged => HubCell::Diverged { version },
        SizeSyncStatus::HubOnly => HubCell::HubOnly { version },
        SizeSyncStatus::LocalOnly => HubCell::NotPublished,
        SizeSyncStatus::NotTrained => HubCell::NoWeights,
    }
}

/// Highest held-out adapter pass@1 among sizes whose gate is passed and whose adapter evidence is
/// `Current`; a tie goes to the smaller size (rows are in `SIZES` order, so the first one kept wins).
pub(crate) fn pick_size(rows: &[IndexRow]) -> Option<Pick> {
    let mut best: Option<Pick> = None;
    for r in rows {
        if r.gate != GateState::Passed || r.adapter_evidence != AdapterEvidence::Current {
            continue;
        }
        let Some(rate) = r.heldout.as_ref().and_then(|h| h.adapter_rate) else {
            continue;
        };
        if best.as_ref().is_none_or(|b| rate > b.rate) {
            best = Some(Pick {
                size: r.size.clone(),
                rate,
                peak_mb: r.peak_mb,
            });
        }
    }
    best
}

fn grid_cell(row: &IndexRow, s: &SpokeEntry) -> GridCell {
    match s.status {
        SpokeStatus::Planned => GridCell::Planned {
            link: s
                .link
                .clone()
                .unwrap_or_else(|| format!("{DOCS}#hub-and-spoke")),
        },
        SpokeStatus::Trained if row.trained && row.spoke_id.as_deref() == Some(s.id.as_str()) => {
            GridCell::Gate(row.gate)
        }
        SpokeStatus::Trained => GridCell::NotTrained,
    }
}

/// Build the index facts from every size's sync row and records, in `SIZES` order.
pub(crate) fn index_facts(namespace: &str, entries: &[(SizeSyncRow, RunRecords)]) -> IndexFacts {
    let cfg = hub_cards();
    let (spokes, default_spoke, spoke_source, config_error) = match &cfg {
        Ok(c) => (
            c.spokes.clone(),
            Some(c.default_spoke.clone()),
            Some(c.default_spoke_source.clone()),
            None,
        ),
        Err(e) => (Vec::new(), None, None, Some(e.clone())),
    };
    let mut sorted: Vec<&(SizeSyncRow, RunRecords)> = entries.iter().collect();
    sorted.sort_by_key(|(r, _)| {
        SIZES
            .iter()
            .position(|s| *s == r.size)
            .unwrap_or(SIZES.len())
    });

    let mut rows = Vec::new();
    let mut pending = Vec::new();
    for (sync, rec) in sorted {
        let gate = gate_of(rec);
        let evidence = adapter_evidence(rec);
        let spoke_id = gate
            .trained
            .then(|| active_spoke_id(rec, default_spoke.as_deref()))
            .flatten();
        let peak_mb = rec
            .train_record
            .as_ref()
            .filter(|_| gate.trained)
            .and_then(|t| t["peak_mb"].as_u64());
        let heldout = held_out(rec, &evidence).filter(|_| gate.trained);
        let reevaluate = evidence != AdapterEvidence::Current;
        if gate.trained && (gate.state != GateState::Passed || reevaluate) {
            let eta = match gate.blocker {
                Some(Blocker::Stale { .. } | Blocker::Missing(_)) => cfg
                    .as_ref()
                    .ok()
                    .and_then(|c| c.sizes.get(&sync.size))
                    .and_then(|e| e.regate_eta.clone()),
                _ => None,
            };
            pending.push(Pending {
                size: sync.size.clone(),
                blocker: gate.blocker.clone(),
                reevaluate,
                eta,
            });
        }
        let mut row = IndexRow {
            size: sync.size.clone(),
            repo_id: sync.repo_id.clone(),
            trained: gate.trained,
            spoke_id,
            gate: gate.state,
            adapter_evidence: evidence,
            hub: hub_cell(sync),
            peak_mb,
            heldout,
            cells: Vec::new(),
        };
        row.cells = spokes.iter().map(|s| grid_cell(&row, s)).collect();
        rows.push(row);
    }
    let comparison = rows
        .iter()
        .filter_map(|r| Some((r.size.clone(), r.heldout.clone()?)))
        .collect();
    IndexFacts {
        namespace: namespace.to_string(),
        docs: DOCS,
        protocol: PROMPT_PROTOCOL,
        config_error,
        spoke_source,
        planned_spokes: spokes
            .iter()
            .filter(|s| s.status == SpokeStatus::Planned)
            .cloned()
            .collect(),
        pick: pick_size(&rows),
        spokes,
        rows,
        pending,
        comparison,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mens::hub_card::tests::support::{index_row, ladder_entries};

    #[test]
    fn row_and_pick_cells_read_the_facts() {
        let mut r = index_row(SIZES[2], GateState::Passed, AdapterEvidence::Current, None);
        assert_eq!(
            (r.adapter_cell().as_str(), r.peak_text().as_str()),
            ("—", "not recorded")
        );
        r.peak_mb = Some(34460);
        assert_eq!(r.peak_text(), "34460 MiB");
        let pick = Pick {
            size: SIZES[2].to_string(),
            rate: 0.5,
            peak_mb: None,
        };
        assert_eq!(pick.rate_text(), "50.0%");
    }

    #[test]
    fn pending_filters_split_by_reason() {
        let tmp = tempfile::tempdir().unwrap();
        let f = index_facts("ns", &ladder_entries(tmp.path()));
        let sizes = |v: Vec<&Pending>| v.iter().map(|p| p.size.clone()).collect::<Vec<_>>();
        assert_eq!(
            f.passed_rows()
                .iter()
                .map(|r| r.size.as_str())
                .collect::<Vec<_>>(),
            [SIZES[2]]
        );
        assert_eq!(
            sizes(f.not_passed()),
            [SIZES[0], SIZES[1], SIZES[3], SIZES[4]]
        );
        assert_eq!(sizes(f.needs_reevaluation()), [SIZES[1]]);
    }
}
