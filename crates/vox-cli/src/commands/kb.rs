//! `vox kb` — knowledge-base operations: `eval`, `status`, `reindex`.
//!
//! `status` and `reindex` go through the orchestrator daemon (`orch.tool_call`) so the CLI
//! never opens a second Tantivy writer: the daemon's `vox_kb_reindex` is the single writer.

use std::fmt::Write as _;

use serde_json::{Value, json};

use super::daemon_tool::{daemon_client, tool_call};

pub mod eval;
pub(crate) mod gate;

/// Knowledge-base operations (`eval`, `status`, `reindex`).
#[derive(clap::Subcommand, Debug)]
pub enum KbCmd {
    /// Retrieval eval over the golden set, pooling queries, heading listing and corpus snapshot.
    Eval(eval::KbEvalArgs),
    /// Library index state from the daemon: commit, corpus root, counts, backend, errors.
    Status {
        /// Print the raw JSON instead of the rendered report.
        #[arg(long)]
        json: bool,
    },
    /// Rebuild the Library index through the daemon (single writer) and print what changed.
    Reindex {
        /// Print the raw JSON instead of the rendered report.
        #[arg(long)]
        json: bool,
    },
}

/// Dispatch a `vox kb` subcommand.
pub async fn run(cmd: KbCmd) -> anyhow::Result<()> {
    match cmd {
        KbCmd::Eval(args) => eval::run(&args),
        KbCmd::Status { json } => daemon_report("vox_kb_status", json, render_kb_status).await,
        KbCmd::Reindex { json } => daemon_report("vox_kb_reindex", json, render_index_report).await,
    }
}

/// Call a no-argument KB tool through the daemon and print its JSON or rendered report.
/// A tool error (for example "reindex already running") propagates as a non-zero exit.
async fn daemon_report(tool: &str, json: bool, render: fn(&Value) -> String) -> anyhow::Result<()> {
    let client = daemon_client().await?;
    let data = tool_call(&client, tool, json!({})).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print!("{}", render(&data));
    }
    Ok(())
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("-")
}

fn n(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_u64)
        .map_or_else(|| "-".into(), |x| x.to_string())
}

/// One `  - <path>: <error>` line per entry; entries are `{path|rel_path, error}` objects.
fn push_errors(out: &mut String, errors: Option<&Vec<Value>>) {
    for e in errors.into_iter().flatten() {
        let path = e
            .get("path")
            .or_else(|| e.get("rel_path"))
            .and_then(Value::as_str)
            .unwrap_or("?");
        let _ = writeln!(out, "  - {path}: {}", s(e, "error"));
    }
}

/// The legacy VoxKB row counts and the findings-inbox counts, printed only when the daemon sent
/// them (an older daemon sends neither, and the report is then unchanged). Until the Phase 12
/// Inbox GUI exists, this is the interim way to see findings filed by the GUI side doors.
fn push_phase3_counts(out: &mut String, v: &Value) {
    if let Some(l) = v.get("legacy_voxkb") {
        let _ = writeln!(
            out,
            "  legacy VoxKB:        {} knowledge bases, {} entries, {} routing rules",
            n(l, "knowledge_bases"),
            n(l, "kb_entries"),
            n(l, "kb_routing_rules")
        );
    }
    if let Some(i) = v.get("inbox") {
        let _ = writeln!(
            out,
            "  inbox:               {} pending of {}",
            n(i, "pending"),
            n(i, "total")
        );
    }
}

/// Render the `vox_kb_status` data payload (pure).
pub(crate) fn render_kb_status(v: &Value) -> String {
    if v.get("indexed").and_then(Value::as_bool) != Some(true) {
        let mut out = String::from("KB Library: not indexed. Run `vox kb reindex` to build it.\n");
        push_phase3_counts(&mut out, v);
        return out;
    }
    let errors = v.get("errors").and_then(Value::as_array);
    let mut out = String::from("KB Library index\n");
    let _ = writeln!(out, "  last-indexed commit: {}", s(v, "corpus_commit"));
    let _ = writeln!(out, "  corpus root:         {}", s(v, "corpus_root"));
    let _ = writeln!(out, "  indexed at:          {}", s(v, "indexed_at"));
    let _ = writeln!(out, "  docs:                {}", n(v, "doc_count"));
    let _ = writeln!(out, "  chunks:              {}", n(v, "chunk_count"));
    let _ = writeln!(out, "  cards:               {}", n(v, "card_count"));
    let _ = writeln!(out, "  chunker version:     {}", s(v, "chunker_version"));
    let _ = writeln!(out, "  lexical backend:     {}", s(v, "lexical_backend"));
    let _ = writeln!(out, "  embed model:         {}", s(v, "embed"));
    let _ = writeln!(out, "  trust tier:          {}", s(v, "trust_tier"));
    let _ = writeln!(out, "  index dir:           {}", s(v, "index_dir"));
    let _ = writeln!(out, "  errors: {}", errors.map_or(0, Vec::len));
    push_errors(&mut out, errors);
    let verdict = if v.get("consistent").and_then(Value::as_bool) == Some(true) {
        "consistent".to_string()
    } else {
        format!(
            "NOT consistent (db chunks {}, lexical docs {}); run `vox kb reindex`",
            n(v, "db_chunk_count"),
            n(v, "lexical_docs")
        )
    };
    let _ = writeln!(out, "  index consistency:   {verdict}");
    push_phase3_counts(&mut out, v);
    out
}

/// Render the `vox_kb_reindex` data payload (the index report; pure).
pub(crate) fn render_index_report(v: &Value) -> String {
    let errors = v.get("errors").and_then(Value::as_array);
    let mut out = String::from("KB Library reindex\n");
    let _ = writeln!(
        out,
        "  docs:   {} seen, {} unchanged, {} changed, {} pruned",
        n(v, "docs_seen"),
        n(v, "docs_unchanged"),
        n(v, "docs_changed"),
        n(v, "docs_pruned")
    );
    let _ = writeln!(
        out,
        "  chunks: {} inserted, {} deleted, {} kept",
        n(v, "chunks_inserted"),
        n(v, "chunks_deleted"),
        n(v, "chunks_kept")
    );
    let _ = writeln!(out, "  cards:  {}", n(v, "cards"));
    let _ = writeln!(out, "  corpus commit:   {}", s(v, "corpus_commit"));
    let _ = writeln!(out, "  lexical backend: {}", s(v, "lexical_backend"));
    let _ = writeln!(out, "  duration: {} ms", n(v, "duration_ms"));
    let _ = writeln!(out, "  errors: {}", errors.map_or(0, Vec::len));
    push_errors(&mut out, errors);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Harness {
        #[command(subcommand)]
        cmd: KbCmd,
    }

    fn parse(args: &[&str]) -> Result<Harness, clap::Error> {
        Harness::try_parse_from(std::iter::once("kb").chain(args.iter().copied()))
    }

    fn eval_args(args: &[&str]) -> eval::KbEvalArgs {
        match parse(args).unwrap().cmd {
            KbCmd::Eval(a) => a,
            other => panic!("expected eval, got {other:?}"),
        }
    }

    #[test]
    fn kb_cli_status_and_reindex_parse() {
        for (arg, want) in [(vec!["status"], false), (vec!["status", "--json"], true)] {
            assert!(
                matches!(parse(&arg).unwrap().cmd, KbCmd::Status { json } if json == want),
                "{arg:?}"
            );
        }
        for (arg, want) in [(vec!["reindex"], false), (vec!["reindex", "--json"], true)] {
            assert!(
                matches!(parse(&arg).unwrap().cmd, KbCmd::Reindex { json } if json == want),
                "{arg:?}"
            );
        }
        // No path arguments exist: roots are fixed server-side.
        assert!(parse(&["reindex", "docs/src"]).is_err());
    }

    fn status_json() -> Value {
        json!({
            "indexed": true, "index_dir": "/d/kb/library",
            "corpus_root": "/repo", "corpus_commit": "abc1234",
            "indexed_at": "2026-10-05T00:00:00Z",
            "doc_count": 3, "chunk_count": 12, "card_count": 3,
            "chunker_version": "chunker-v7", "lexical_backend": "tantivy-0.22",
            "embed": "none (lexical-only)", "trust_tier": "trusted",
            "errors": [], "duration_ms": 41,
            "db_chunk_count": 12, "lexical_docs": 12, "consistent": true,
        })
    }

    #[test]
    fn render_kb_status_not_indexed_points_at_reindex() {
        let out = render_kb_status(&json!({"indexed": false, "consistent": false}));
        assert!(
            out.contains("not indexed") && out.contains("vox kb reindex"),
            "{out}"
        );
    }

    #[test]
    fn render_kb_status_full_report() {
        let out = render_kb_status(&status_json());
        for want in [
            "abc1234",
            "/repo",
            "docs:",
            "chunks:",
            "cards:",
            "chunker-v7",
            "tantivy-0.22",
            "lexical-only",
            "trusted",
            "errors: 0",
            "consistent",
        ] {
            assert!(out.contains(want), "missing {want:?} in:\n{out}");
        }
        assert!(!out.contains("NOT consistent"), "{out}");
    }

    fn phase3_counts() -> Value {
        json!({
            "legacy_voxkb": {"knowledge_bases": 1, "kb_entries": 2, "kb_routing_rules": 1},
            "inbox": {"total": 3, "pending": 3},
        })
    }

    #[test]
    fn kb_status_renders_legacy_and_inbox_lines_when_not_indexed() {
        let mut v = json!({"indexed": false, "consistent": false});
        v.as_object_mut()
            .unwrap()
            .extend(phase3_counts().as_object().unwrap().clone());
        let out = render_kb_status(&v);
        assert!(
            out.contains("not indexed") && out.contains("vox kb reindex"),
            "{out}"
        );
        assert!(
            out.contains("legacy VoxKB")
                && out.contains("1 knowledge bases, 2 entries, 1 routing rules"),
            "{out}"
        );
        assert!(
            out.contains("inbox") && out.contains("3 pending of 3"),
            "{out}"
        );
    }

    #[test]
    fn kb_status_renders_legacy_and_inbox_lines_when_indexed() {
        let mut v = status_json();
        v.as_object_mut()
            .unwrap()
            .extend(phase3_counts().as_object().unwrap().clone());
        let out = render_kb_status(&v);
        assert!(
            out.contains("legacy VoxKB")
                && out.contains("1 knowledge bases, 2 entries, 1 routing rules"),
            "{out}"
        );
        assert!(
            out.contains("inbox") && out.contains("3 pending of 3"),
            "{out}"
        );
        // The Phase 2 report is still there, ahead of the added lines.
        assert!(
            out.contains("index consistency:") && out.contains("tantivy-0.22"),
            "{out}"
        );
    }

    #[test]
    fn kb_status_without_phase3_keys_renders_as_before() {
        let not_indexed = render_kb_status(&json!({"indexed": false, "consistent": false}));
        assert_eq!(
            not_indexed,
            "KB Library: not indexed. Run `vox kb reindex` to build it.\n"
        );
        let indexed = render_kb_status(&status_json());
        assert!(
            !indexed.contains("legacy VoxKB") && !indexed.contains("inbox"),
            "{indexed}"
        );
        assert!(indexed.trim_end().ends_with("consistent"), "{indexed}");
    }

    #[test]
    fn render_kb_status_lists_errors_and_flags_inconsistency() {
        let mut v = status_json();
        v["errors"] = json!([{"path": "docs/src/bad.md", "error": "read failed"}]);
        v["consistent"] = json!(false);
        v["lexical_docs"] = json!(9);
        let out = render_kb_status(&v);
        assert!(
            out.contains("errors: 1") && out.contains("docs/src/bad.md"),
            "{out}"
        );
        assert!(
            out.contains("NOT consistent") && out.contains("lexical docs 9"),
            "{out}"
        );
    }

    #[test]
    fn render_index_report_prints_counts_and_errors() {
        let out = render_index_report(&json!({
            "docs_seen": 5, "docs_unchanged": 3, "docs_changed": 2, "docs_pruned": 1,
            "chunks_inserted": 8, "chunks_deleted": 4, "chunks_kept": 20, "cards": 5,
            "errors": [{"path": "docs/src/x.md", "error": "bad utf8"}],
            "lexical_backend": "tantivy-0.22", "lexical_docs": 28,
            "corpus_commit": "deadbee", "duration_ms": 77,
        }));
        for want in [
            "5 seen",
            "3 unchanged",
            "2 changed",
            "1 pruned",
            "8 inserted",
            "4 deleted",
            "20 kept",
            "cards:  5",
            "deadbee",
            "tantivy-0.22",
            "77 ms",
            "errors: 1",
            "docs/src/x.md: bad utf8",
        ] {
            assert!(out.contains(want), "missing {want:?} in:\n{out}");
        }
    }

    #[test]
    fn plain_eval_defaults_to_dev() {
        let a = eval_args(&["eval"]);
        assert_eq!(a.split, eval::SplitArg::Dev);
        assert!(!a.json && a.query.is_none() && !a.snapshot_corpus);
    }

    #[test]
    fn top_requires_query() {
        assert!(parse(&["eval", "--top", "5"]).is_err());
        assert!(parse(&["eval", "--query", "x", "--top", "5"]).is_ok());
    }

    #[test]
    fn ratchet_modes_reject_explicit_split() {
        for mode in ["--gate", "--tighten"] {
            assert!(parse(&["eval", mode, "--split", "test"]).is_err(), "{mode}");
            assert!(parse(&["eval", mode, "--split", "dev"]).is_err(), "{mode}");
            assert!(parse(&["eval", mode]).is_ok(), "{mode}");
        }
        let wb = ["--write-baseline", "--epoch-note", "n"];
        assert!(parse(&[&["eval"], &wb[..], &["--split", "dev"]].concat()).is_err());
        // Ratchet modes never mix with each other or the special modes.
        assert!(parse(&["eval", "--gate", "--tighten"]).is_err());
        assert!(parse(&["eval", "--gate", "--query", "x"]).is_err());
        assert!(parse(&["eval", "--tighten", "--snapshot-corpus"]).is_err());
        assert!(parse(&[&["eval", "--gate"], &wb[..]].concat()).is_err());
    }

    #[test]
    fn write_baseline_requires_epoch_note() {
        assert!(parse(&["eval", "--write-baseline"]).is_err());
        assert!(parse(&["eval", "--epoch-note", "n"]).is_err());
        let a = eval_args(&["eval", "--write-baseline", "--epoch-note", "n"]);
        assert!(a.write_baseline && a.epoch_note.as_deref() == Some("n"));
    }

    #[test]
    fn base_requires_gate() {
        assert!(parse(&["eval", "--base", "origin/main"]).is_err());
        let a = eval_args(&["eval", "--gate", "--base", "x"]);
        assert_eq!(a.base, "x");
        let a = eval_args(&["eval", "--gate"]);
        assert_eq!(a.base, "origin/main");
    }

    #[test]
    fn special_modes_are_mutually_exclusive() {
        assert!(parse(&["eval", "--query", "x", "--headings", "docs/src/a.md"]).is_err());
        assert!(parse(&["eval", "--query", "x", "--snapshot-corpus"]).is_err());
        assert!(parse(&["eval", "--headings", "d", "--snapshot-corpus"]).is_err());
    }
}
