//! `vox graph history` — first-parent repo history
//! (docs/superpowers/specs/2026-09-22-repo-history-graph-design.md).
use clap::{Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use vox_graph_reader::history::api::{self, HistoryQuery, Paths};
use vox_graph_reader::history::ingest::CatchUp;
use vox_graph_reader::history::query::AreaBy;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum By {
    Crate,
    Dir,
}

impl From<By> for AreaBy {
    fn from(b: By) -> Self {
        match b {
            By::Crate => AreaBy::Crate,
            By::Dir => AreaBy::Dir,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum HistoryCmd {
    /// Changes to a file, following renames/splits/merges back through lineage.
    Log {
        path: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        include_mechanical: bool,
        #[arg(long)]
        json: bool,
    },
    /// Where effort went: non-mechanical churn per area.
    Focus {
        #[arg(long, default_value_t = 30)]
        since_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long)]
        include_mechanical: bool,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Live areas ranked by time since last meaningful change × fan-in.
    Forgotten {
        #[arg(long, default_value_t = 60)]
        min_age_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Search commit subjects, bodies, merged-branch subjects, paths, and symbols.
    Search {
        text: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Top areas and feat/merge highlights per time bucket.
    Timeline {
        #[arg(long, default_value_t = 90)]
        since_days: i64,
        #[arg(long, default_value_t = 7)]
        bucket_days: i64,
        #[arg(long, value_enum, default_value_t = By::Crate)]
        by: By,
        #[arg(long)]
        json: bool,
    },
    /// One SessionStart line from data on disk. Never ingests, never fails.
    Brief,
}

fn to_query(cmd: HistoryCmd) -> (HistoryQuery, bool) {
    match cmd {
        HistoryCmd::Log {
            path,
            limit,
            include_mechanical,
            json,
        } => (
            HistoryQuery::Log {
                path,
                include_mechanical,
                limit,
            },
            json,
        ),
        HistoryCmd::Focus {
            since_days,
            by,
            include_mechanical,
            limit,
            json,
        } => (
            HistoryQuery::Focus {
                since_days,
                by: by.into(),
                include_mechanical,
                limit,
            },
            json,
        ),
        HistoryCmd::Forgotten {
            min_age_days,
            by,
            limit,
            json,
        } => (
            HistoryQuery::Forgotten {
                min_age_days,
                by: by.into(),
                limit,
            },
            json,
        ),
        HistoryCmd::Search { text, limit, json } => (HistoryQuery::Search { text, limit }, json),
        HistoryCmd::Timeline {
            since_days,
            bucket_days,
            by,
            json,
        } => (
            HistoryQuery::Timeline {
                since_days,
                bucket_days,
                by: by.into(),
            },
            json,
        ),
        HistoryCmd::Brief => (HistoryQuery::Brief, false),
    }
}

pub fn run(cmd: HistoryCmd, repo_root: &Path, code_graph: Option<PathBuf>) -> anyhow::Result<()> {
    let (q, json) = to_query(cmd);
    let brief = q == HistoryQuery::Brief;
    let paths = Paths {
        repo_root,
        code_graph: code_graph.as_deref(),
    };
    let mut progress = |done: usize, total: usize| {
        if done == total || done.is_multiple_of(100) {
            eprintln!("history: ingested {done}/{total}");
        }
    };
    match api::answer(
        &paths,
        &q,
        chrono::Utc::now().timestamp(),
        None,
        &mut progress,
    ) {
        Ok(a) => {
            if a.catch_up == CatchUp::Busy {
                eprintln!("history: another ingest is running; showing data on disk");
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&a.value)?);
            } else {
                println!("{}", a.text);
            }
            Ok(())
        }
        Err(e) if brief => {
            println!("history: unavailable ({e})");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

// AMENDED: #3 — tdd-guard needs an in-file test for this file's `pub fn`; `tests/` doesn't count.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_query_maps_flags() {
        let (q, json) = to_query(HistoryCmd::Focus {
            since_days: 7,
            by: By::Dir,
            include_mechanical: true,
            limit: 3,
            json: true,
        });
        assert!(json);
        assert_eq!(
            q,
            HistoryQuery::Focus {
                since_days: 7,
                by: AreaBy::Dir,
                include_mechanical: true,
                limit: 3
            }
        );
        assert_eq!(to_query(HistoryCmd::Brief), (HistoryQuery::Brief, false));
    }
}
