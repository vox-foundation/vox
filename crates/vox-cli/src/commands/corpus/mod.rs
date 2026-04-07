//! `vox corpus` subcommand — native training data pipeline.
//!
//! Replaces the Python scripts with in-process Rust implementations:
//! - `extract` — walk .vox files, emit JSONL corpus
//! - `validate` — re-check entries, dedup, print coverage
//! - `pairs` — generate instruction→response training pairs
//! - `mix` — merge sources per `mens/config/mix.yaml`
//! - `prompt` — auto-generate system prompt from construct reference

mod generate;
mod stats;
mod validate;

use anyhow::{Context, Result};

use clap::Parser;
use crate::commands::ci::bounded_read::read_utf8_path_capped_async;

#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) use stats::{eval_metrics, run_benchmark_gate};

/// Subcommands for invoking native Mens training data pipelines.
#[derive(Parser)]
pub enum CorpusAction {
    /// Generate synthetic training pairs (tools, orchestrator, A2A, etc)
    Generate {
        /// Output JSONL file
        #[arg(short, long, default_value = "mens/data/synthetic.jsonl")]
        output: std::path::PathBuf,
        /// Force regeneration even if corpus fingerprint is fresh
        #[arg(long)]
        force_regen: bool,
        /// Dry-run mode (print pairs generated but do not write output or update fingerprint)
        #[arg(long)]
        dry_run: bool,
    },
    /// Print current corpus fingerprint based on AST/generator files
    Fingerprint,
    /// Extract training corpus from .vox files
    Extract {
        /// Directory containing .vox files (recursive)
        #[arg(required = true)]
        dir: std::path::PathBuf,
        /// Output JSONL file
        #[arg(short, long, default_value = "mens/data/validated.jsonl")]
        output: std::path::PathBuf,
    },
    /// Extract training pairs from Rust source code (.rs)
    ExtractRs {
        /// Root directory (usually `crates/`)
        #[arg(default_value = "crates")]
        dir: std::path::PathBuf,
        /// Output JSONL file
        #[arg(short, long, default_value = "mens/data/mix_sources/rust_source.jsonl")]
        output: std::path::PathBuf,
    },
    /// Extract training pairs from documentation (.md)
    ExtractDocs {
        /// Documentation directory (usually `docs/src/`)
        #[arg(default_value = "docs/src")]
        dir: std::path::PathBuf,
        /// Output JSONL file
        #[arg(short, long, default_value = "mens/data/mix_sources/docs.jsonl")]
        output: std::path::PathBuf,
    },
    /// Validate and deduplicate a corpus JSONL file
    #[command(name = "validate-batch", alias = "validate")]
    Validate {
        /// Input JSONL file
        #[arg(required = true)]
        input: std::path::PathBuf,
        /// Output validated JSONL file (defaults to overwriting input)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Skip re-checking code through the compiler
        #[arg(long)]
        no_recheck: bool,
    },
    /// Generate instruction→response training pairs from validated corpus
    Pairs {
        /// Input validated JSONL file
        #[arg(required = true)]
        input: std::path::PathBuf,
        /// Output training JSONL file
        #[arg(short, long, default_value = "target/dogfood/train.jsonl")]
        output: std::path::PathBuf,
        /// Documentation root(s) for additional fenced `vox` pairs (repeatable; merged)
        #[arg(long, action = clap::ArgAction::Append)]
        docs: Vec<std::path::PathBuf>,
    },
    /// Replay Arca telemetry into Mens training pairs
    Replay {
        /// Emit full multi-turn ChatML sessions (groups events by session_id)
        #[arg(long)]
        chatml: bool,
        /// Minimum session quality score to include (0.0 = all)
        #[arg(long, default_value = "0.0")]
        min_score: f64,
        /// Output JSONL path (auto-picked by `vox mens mix`)
        #[arg(
            short,
            long,
            default_value = "mens/data/mix_sources/autofeedback.jsonl"
        )]
        output: std::path::PathBuf,
        /// Max session/event rows to query
        #[arg(long, default_value = "500")]
        limit: i64,
    },
    /// Auto-generate vox_system_prompt.txt from construct reference
    Prompt {
        /// Output file path
        #[arg(short, long, default_value = "scripts/vox_system_prompt.txt")]
        output: std::path::PathBuf,
    },
    /// Run eval metrics on training data (format, safety, parse, coverage)
    Eval {
        /// Input training JSONL file
        #[arg(required = true)]
        input: std::path::PathBuf,
        /// Output eval results JSON
        #[arg(short, long, default_value = "target/dogfood/eval_results.json")]
        output: std::path::PathBuf,
        /// Print one machine-readable summary line to stdout (for CI logs).
        #[arg(long)]
        print_summary: bool,
    },
    /// Merge corpus sources defined in a mix config (same as `vox mens train` preflight)
    Mix {
        /// Path to mix YAML (default: `mens/config/mix.yaml`)
        #[arg(long, default_value = "mens/config/mix.yaml")]
        config: std::path::PathBuf,
        /// Allow missing or empty required sources (warn only; default is strict fail for operator mixes).
        #[arg(long)]
        allow_missing_sources: bool,
    },
    /// Show corpus statistics (domain mixture, category counts)
    Stats {
        /// Input JSONL file (defaults to `target/dogfood/train.jsonl`)
        #[arg(short, long, default_value = "target/dogfood/train.jsonl")]
        input: std::path::PathBuf,
    },
    /// Export review-derived dataset rows from VoxDB.
    ReviewExport {
        /// Repository id, e.g. `owner/repo`.
        #[arg(long)]
        repository_id: String,
        /// Max findings window.
        #[arg(long, default_value_t = 500)]
        limit: i64,
        /// Output JSONL file.
        #[arg(short, long, default_value = "mens/data/mix_sources/review_findings.jsonl")]
        output: std::path::PathBuf,
    },
    /// Validate review-derived dataset rows.
    ReviewValidate {
        /// Input JSONL file.
        #[arg(required = true)]
        input: std::path::PathBuf,
    },
    /// Show review-derived dataset stats.
    ReviewStats {
        /// Input JSONL file.
        #[arg(short, long, default_value = "mens/data/mix_sources/review_findings.jsonl")]
        input: std::path::PathBuf,
    },
}

/// Execute the native training data extraction or validation logic.
pub async fn run(action: CorpusAction) -> Result<()> {
    match action {
        CorpusAction::Fingerprint => stats::run_fingerprint().await,
        CorpusAction::Generate {
            output,
            force_regen,
            dry_run,
        } => generate::run_generate(output, force_regen, dry_run).await,
        CorpusAction::Extract { dir, output } => generate::run_extract(&dir, &output).await,
        CorpusAction::ExtractRs { dir, output } => {
            let config = vox_corpus::corpus::extract_rs::ExtractRsConfig {
                root: dir,
                ..Default::default()
            };
            let pairs = vox_corpus::corpus::extract_rs::walk_and_extract(&config)?;
            let count = vox_corpus::corpus::extract_rs::write_to_jsonl(&pairs, &output)?;
            println!(
                "✓ Extracted {} Rust source pairs → {}",
                count,
                output.display()
            );
            Ok(())
        }
        CorpusAction::ExtractDocs { dir, output } => {
            let config = vox_corpus::corpus::extract_docs::ExtractDocsConfig {
                root: dir,
                ..Default::default()
            };
            let pairs = vox_corpus::corpus::extract_docs::walk_and_extract_docs(&config)?;
            let count = vox_corpus::corpus::extract_docs::write_docs_to_jsonl(&pairs, &output)?;
            println!(
                "✓ Extracted {} documentation pairs → {}",
                count,
                output.display()
            );
            Ok(())
        }
        CorpusAction::Validate {
            input,
            output,
            no_recheck,
        } => {
            let out = output.as_deref().unwrap_or(&input);
            validate::run_validate(&input, out, !no_recheck).await
        }
        CorpusAction::Pairs {
            input,
            output,
            docs,
        } => generate::run_pairs(&input, &output, &docs).await,
        CorpusAction::Prompt { output } => generate::run_prompt(&output).await,
        CorpusAction::Eval {
            input,
            output,
            print_summary,
        } => stats::run_eval(&input, &output, print_summary).await,
        CorpusAction::Mix {
            config,
            allow_missing_sources,
        } => vox_corpus::corpus::run_mix_with_options(
            &config,
            None,
            vox_corpus::corpus::MixRunOptions {
                strict: !allow_missing_sources,
                write_report: true,
            },
        ),
        CorpusAction::Replay {
            chatml: _chatml,
            min_score: _min_score,
            output: _output,
            limit: _limit,
        } => {
            #[cfg(feature = "database")]
            {
                let db = vox_db::VoxDb::connect_default()
                    .await
                    .expect("Replay requires a database connection (set VOX_DB_URL)");
                let rows =
                    vox_corpus::arca_replay::extract_arca_pairs(&db, _limit, _chatml, _min_score)
                        .await
                        .expect("Failed to extract Arca replay pairs");
                let parent = _output.parent().unwrap_or(std::path::Path::new("."));
                tokio::fs::create_dir_all(parent).await?;
                let mut body = String::new();
                for row in &rows {
                    body.push_str(&serde_json::to_string(row)?);
                    body.push('\n');
                }
                tokio::fs::write(&_output, body)
                    .await
                    .with_context(|| format!("Cannot create output: {}", _output.display()))?;
                println!(
                    "✓ Wrote {} replay pairs → {}",
                    rows.len(),
                    _output.display()
                );
            }
            #[cfg(not(feature = "database"))]
            {
                eprintln!(
                    "Replay requires the `database` feature. Rebuild with --features database"
                );
            }
            Ok(())
        }
        CorpusAction::Stats { input } => stats::run_stats(&input).await,
        CorpusAction::ReviewExport {
            repository_id,
            limit,
            output,
        } => {
            let db = vox_db::VoxDb::connect_default()
                .await
                .context("review-export requires DB connection")?;
            let findings = db
                .list_external_review_findings_for_training_window(&repository_id, limit)
                .await
                .context("list external review findings for export")?;
            let mut rows = Vec::new();
            for f in findings {
                let prompt = format!(
                    "Review finding in {}:{} category={} severity={}: {}",
                    f.file_path.as_deref().unwrap_or("global"),
                    f.line_start.unwrap_or(0),
                    f.category,
                    f.severity,
                    f.title
                );
                let response = f.suggested_fix.clone().unwrap_or_else(|| f.details.clone());
                rows.push(vox_corpus::external_review_replay::ExternalReviewReplayRow {
                    prompt,
                    response,
                    category: f.category,
                    severity: f.severity,
                    placement_kind: f.placement_kind,
                    source_id: f.finding_identity,
                    repository_id: f.repository_id,
                    pr_number: f.pr_number,
                    file_path: f.file_path,
                    line_start: f.line_start,
                    correctness_state: f.status,
                    sample_kind: "review_fix_pairs".to_string(),
                });
            }
            vox_corpus::external_review_replay::validate_external_review_rows(&rows)
                .context("validate extracted review rows")?;
            let parent = output.parent().unwrap_or(std::path::Path::new("."));
            tokio::fs::create_dir_all(parent).await?;
            let mut body = String::new();
            for row in &rows {
                body.push_str(&serde_json::to_string(row)?);
                body.push('\n');
            }
            tokio::fs::write(&output, body).await?;
            println!(
                "✓ Wrote {} review-derived rows -> {}",
                rows.len(),
                output.display()
            );
            Ok(())
        }
        CorpusAction::ReviewValidate { input } => {
            let raw = read_utf8_path_capped_async(&input).await?;
            let mut rows = Vec::new();
            for (idx, line) in raw.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let row: vox_corpus::external_review_replay::ExternalReviewReplayRow =
                    serde_json::from_str(line)
                        .with_context(|| format!("parse review row at line {}", idx + 1))?;
                rows.push(row);
            }
            vox_corpus::external_review_replay::validate_external_review_rows(&rows)?;
            println!("✓ Review dataset valid: {} rows", rows.len());
            Ok(())
        }
        CorpusAction::ReviewStats { input } => stats::run_review_stats(&input).await,
    }
}
