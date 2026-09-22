//! `vox mens corpus back-translate` — code → natural-language task
//! (OSS-Instruct / instruction back-translation; pipeline audit 2026-09-20 §6,
//! proposal 1).
//!
//! For each compile-clean code row an LLM writes the task the code solves.
//! A pair is kept only if the instruction is faithful: every top-level declared
//! name appears in it, it contains no code fence, and (with `--round-trip`) a
//! regeneration from the instruction alone also compiles. Replies are cached by
//! hash(prompt version, model, code) so reruns do not re-spend.

use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use super::synth::{self, ChatBackend};
use crate::commands::mens::eval_gate::{BenchTask, leaked_bench_task, load_bench_answers};

/// Bump when either template changes: it is part of the cache key.
pub(crate) const PROMPT_VERSION: &str = "back-translate-v1";

const INSTRUCTION_SYSTEM: &str = "You write programming tasks for a training dataset of the Vox language. \
Given Vox source code, write the natural-language request a user would make that this code fulfils. \
Describe the required behavior, inputs, outputs and edge cases. \
Mention every name in the \"Required names\" list exactly as spelled, with its kind \
(function, component, table, query, ...) and its signature: parameter names and types and the return type. \
Do NOT include code, code blocks, or implementation steps. Reply with the task text only.";

/// Output tokens budgeted per instruction call. 400 was observed truncating
/// mid-sentence for ~15% of real rows (multi-variant types, several
/// functions) — reading the actual `back_translated.jsonl` output on live
/// hardware, not just trusting the pass/fail summary, is what caught this.
/// 800 still truncated the single longest, most multi-function row observed.
const INSTRUCTION_MAX_TOKENS: usize = 1200;
/// Output tokens budgeted per round-trip regeneration call.
const REGEN_MAX_TOKENS: usize = 1024;

pub(crate) struct BackTranslateOpts {
    pub input: PathBuf,
    pub output: PathBuf,
    pub cache: PathBuf,
    pub bench: PathBuf,
    pub max_rows: usize,
    pub round_trip: bool,
    pub max_spend_usd: f64,
    pub usd_per_1k_tokens: f64,
    pub apply: bool,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct BackTranslateSummary {
    pub eligible: usize,
    pub cache_hits: usize,
    pub planned_calls: usize,
    pub estimated_usd: f64,
    pub calls: usize,
    pub spent_usd: f64,
    pub written: usize,
    pub rejected_unfaithful: usize,
    pub rejected_round_trip: usize,
    /// Calls that errored (transport, malformed response, …) rather than
    /// returning a usable reply. The row is skipped, not the whole run: a single
    /// bad response used to abort via `?` and discard every already-verified
    /// pair from earlier in the batch.
    pub call_failed: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct CacheEntry {
    key: String,
    instruction: String,
    model: String,
    #[serde(default)]
    round_trip: Option<bool>,
}

/// Top-level declared names (column-0 declarations), excluding `@test` functions.
pub(crate) fn declared_names(code: &str) -> Vec<String> {
    let re = regex::Regex::new(
        r"^((?:@[\w.]+(?:\([^)]*\))?\s+)*)(?:pub\s+)?(?:fn|component|table|query|mutation|server|tool|resource|form|index|type|actor|workflow|activity|state_machine|module)\s+([A-Za-z_]\w*)",
    )
    .expect("static regex");
    let mut names = Vec::new();
    let mut pending_test = false;
    for line in code.lines() {
        if let Some(c) = re.captures(line) {
            let is_test = pending_test || c[1].contains("@test");
            pending_test = false;
            if !is_test && !names.iter().any(|n| n == &c[2]) {
                names.push(c[2].to_string());
            }
        } else if line.starts_with('@') {
            pending_test |= line.contains("@test");
        } else if !line.trim().is_empty() && !line.trim_start().starts_with("//") {
            pending_test = false;
        }
    }
    names
}

fn mentions_word(text: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// Deterministic faithfulness filter: no code fence, every required name
/// mentioned, and the instruction wasn't cut off mid-sentence by hitting
/// `INSTRUCTION_MAX_TOKENS` (observed on real, complex multi-function tasks
/// even at 1200 tokens — reading actual output, not just the pass/fail
/// summary, is what caught this).
pub(crate) fn is_faithful(instruction: &str, names: &[String]) -> bool {
    let trimmed = instruction.trim();
    !trimmed.is_empty()
        && !trimmed.contains("```")
        && trimmed.ends_with(['.', '!', '?', ')', '"', '`'])
        && names.iter().all(|n| mentions_word(instruction, n))
}

pub(crate) fn instruction_user_prompt(code: &str, names: &[String]) -> String {
    format!(
        "Required names: {}\n\nCode:\n{code}",
        if names.is_empty() {
            "(none)".to_string()
        } else {
            names.join(", ")
        }
    )
}

fn cache_key(model: &str, code: &str) -> String {
    vox_crypto::hash_fast_hex(format!("{PROMPT_VERSION}\0{model}\0{code}").as_bytes())
}

fn load_cache(path: &std::path::Path) -> HashMap<String, CacheEntry> {
    // Later lines win (round-trip results are appended as updated entries).
    synth::read_jsonl(path)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value::<CacheEntry>(v).ok())
        .map(|e| (e.key.clone(), e))
        .collect()
}

struct Candidate {
    code: String,
    source: String,
    difficulty: u64,
    names: Vec<String>,
    key: String,
}

pub(crate) async fn run_back_translate<B: ChatBackend>(
    opts: &BackTranslateOpts,
    backend: &B,
) -> Result<BackTranslateSummary> {
    let rows = synth::read_jsonl(&opts.input)?;
    let bench: Vec<BenchTask> = if opts.bench.is_file() {
        load_bench_answers(&opts.bench)?
    } else {
        Vec::new()
    };
    let model = backend.model_id();
    let mut cache = load_cache(&opts.cache);

    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if candidates.len() >= opts.max_rows {
            break;
        }
        let Some(raw_code) = row
            .get("response")
            .or_else(|| row.get("code"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        // Older extract stages (e.g. `mens/data/validated.jsonl`, one row per whole
        // golden file) never stripped the file's own frontmatter/`ANCHOR` comments —
        // without this, `code` (sent to the LLM AND written as the pair's answer)
        // reproduces `// ---\n// title: ...` blocks, the exact leak the rest of the
        // corpus pipeline was fixed to remove.
        let (code, _training_prompt) = crate::training::split_training_metadata(raw_code);
        let code = code.as_str();
        let key = cache_key(&model, code);
        if !seen.insert(key.clone())
            || leaked_bench_task(code, &bench).is_some()
            || !synth::verify(code, "back_translate", i).pass
        {
            continue;
        }
        candidates.push(Candidate {
            code: code.to_string(),
            source: row
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            difficulty: row.get("difficulty").and_then(|v| v.as_u64()).unwrap_or(5),
            names: declared_names(code),
            key,
        });
    }

    let mut s = BackTranslateSummary {
        eligible: candidates.len(),
        ..Default::default()
    };
    let system_prompt = vox_corpus::training::generate_system_prompt();
    for c in &candidates {
        let hit = cache.get(&c.key);
        if hit.is_some() {
            s.cache_hits += 1;
        }
        if hit.is_none() {
            s.planned_calls += 1;
            s.estimated_usd += synth::estimate_usd(
                INSTRUCTION_SYSTEM.len() + c.code.len(),
                INSTRUCTION_MAX_TOKENS,
                opts.usd_per_1k_tokens,
            );
        }
        if opts.round_trip && hit.and_then(|h| h.round_trip).is_none() {
            s.planned_calls += 1;
            s.estimated_usd += synth::estimate_usd(
                system_prompt.len() + INSTRUCTION_MAX_TOKENS * 4,
                REGEN_MAX_TOKENS.min(c.code.len() / 2),
                opts.usd_per_1k_tokens,
            );
        }
    }

    println!(
        "back-translate plan: {} compile-clean rows, {} cached, {} LLM calls, est ${:.4} (model {model}, cap ${:.2})",
        s.eligible, s.cache_hits, s.planned_calls, s.estimated_usd, opts.max_spend_usd
    );
    if !opts.apply {
        println!("dry run: no calls made; re-run with --apply to spend");
        return Ok(s);
    }

    if let Some(p) = opts.cache.parent()
        && !p.as_os_str().is_empty()
    {
        std::fs::create_dir_all(p)?;
    }
    let mut cache_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&opts.cache)?;
    let mut out = Vec::new();
    for c in &candidates {
        let mut entry = match cache.get(&c.key) {
            Some(e) => e.clone(),
            None => {
                if s.spent_usd >= opts.max_spend_usd {
                    println!("spend cap reached (${:.4}); stopping", s.spent_usd);
                    break;
                }
                let reply = match backend
                    .chat(
                        INSTRUCTION_SYSTEM,
                        &instruction_user_prompt(&c.code, &c.names),
                        0.2,
                    )
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        // A transient failure (transport, malformed response) must
                        // skip this row, not abort the run: every already-verified
                        // pair from earlier in the batch is real spend that would
                        // otherwise be discarded by propagating via `?`.
                        eprintln!("back-translate: call failed for {}: {e}", c.source);
                        s.call_failed += 1;
                        continue;
                    }
                };
                s.calls += 1;
                s.spent_usd += reply.cost_usd;
                CacheEntry {
                    key: c.key.clone(),
                    instruction: reply.text.trim().to_string(),
                    model: reply.model,
                    round_trip: None,
                }
            }
        };
        if !is_faithful(&entry.instruction, &c.names) {
            s.rejected_unfaithful += 1;
        } else if opts.round_trip && entry.round_trip.is_none() {
            if s.spent_usd >= opts.max_spend_usd {
                println!("spend cap reached (${:.4}); stopping", s.spent_usd);
                writeln!(cache_file, "{}", serde_json::to_string(&entry)?)?;
                break;
            }
            match backend.chat(&system_prompt, &entry.instruction, 0.2).await {
                Ok(reply) => {
                    s.calls += 1;
                    s.spent_usd += reply.cost_usd;
                    entry.round_trip =
                        Some(synth::verify(&synth::clean_code(&reply.text), "round_trip", 0).pass);
                }
                Err(e) => {
                    // Cache the entry as-is (round_trip still None) so a rerun
                    // retries only the regeneration call, not the already-paid
                    // instruction call, and skip this row for this run.
                    eprintln!(
                        "back-translate: round-trip call failed for {}: {e}",
                        c.source
                    );
                    s.call_failed += 1;
                    writeln!(cache_file, "{}", serde_json::to_string(&entry)?)?;
                    cache.insert(c.key.clone(), entry.clone());
                    continue;
                }
            }
        }
        writeln!(cache_file, "{}", serde_json::to_string(&entry)?)?;
        cache.insert(c.key.clone(), entry.clone());

        if !is_faithful(&entry.instruction, &c.names) {
            continue;
        }
        if opts.round_trip && entry.round_trip != Some(true) {
            s.rejected_round_trip += 1;
            continue;
        }
        let rating = if entry.round_trip == Some(true) { 5 } else { 4 };
        let mut row = synth::pair_row(
            &entry.instruction,
            &c.code,
            "back_translation",
            &c.source,
            c.difficulty,
            rating,
        );
        row["model"] = entry.model.clone().into();
        row["prompt_template_version"] = PROMPT_VERSION.into();
        row["round_trip"] = entry.round_trip.into();
        out.push(row);
    }
    s.written = out.len();
    synth::write_jsonl(&opts.output, &out)?;
    println!(
        "back-translate: {} calls, ${:.4} spent, {} pairs written ({} unfaithful, {} failed round-trip, {} call errors) -> {}",
        s.calls,
        s.spent_usd,
        s.written,
        s.rejected_unfaithful,
        s.rejected_round_trip,
        s.call_failed,
        opts.output.display()
    );
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::super::synth::test_support::MockBackend;
    use super::*;

    const ADD: &str = "fn add_numbers(a: int, b: int) to int {\n    return a + b\n}\n";
    const BROKEN: &str = "fn broken(a: int) to int {\n    return a +\n";

    fn opts(dir: &std::path::Path, apply: bool, round_trip: bool) -> BackTranslateOpts {
        let input = dir.join("in.jsonl");
        std::fs::write(
            &input,
            [ADD, BROKEN]
                .iter()
                .map(|c| serde_json::json!({"response": c, "source": "x.vox"}).to_string() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        BackTranslateOpts {
            input,
            output: dir.join("out.jsonl"),
            cache: dir.join("cache.jsonl"),
            bench: dir.join("missing_manifest.json"),
            max_rows: 100,
            round_trip,
            max_spend_usd: 10.0,
            usd_per_1k_tokens: 0.002,
            apply,
        }
    }

    #[test]
    fn declared_names_skip_tests_and_nested() {
        let code = "@pure fn one(x: int) to int {\n    fn inner() {}\n    return x\n}\ncomponent Two() {}\n@test\nfn check() to Unit {}\n@test fn check2() to Unit {}\ntable Three { }\n";
        assert_eq!(declared_names(code), vec!["one", "Two", "Three"]);
    }

    #[test]
    fn faithfulness_requires_every_name_as_a_word_and_no_code() {
        let names = vec!["add".to_string(), "Item".to_string()];
        assert!(is_faithful("Write fn `add` returning an Item.", &names));
        assert!(!is_faithful(
            "Write an address book returning an Item.",
            &names
        ));
        assert!(!is_faithful("add Item ```vox\nfn add() {}\n```", &names));
        assert!(!is_faithful("  ", &[]));
    }

    /// Real failure observed on live hardware: a long, multi-function task
    /// hit `INSTRUCTION_MAX_TOKENS` and was cut off mid-sentence with no
    /// closing punctuation. Such instructions must be dropped, not trained on.
    #[test]
    fn faithfulness_rejects_instructions_truncated_mid_sentence() {
        let names = vec!["add".to_string()];
        assert!(!is_faithful(
            "Write fn `add` that sums two ints, handling overflow by",
            &names
        ));
        assert!(is_faithful("Write fn `add` that sums two ints.", &names));
    }

    #[test]
    fn prompt_lists_required_names_and_code() {
        let p = instruction_user_prompt(ADD, &declared_names(ADD));
        assert!(p.starts_with("Required names: add_numbers\n"));
        assert!(p.contains(ADD));
        assert!(INSTRUCTION_SYSTEM.contains("Do NOT include code"));
    }

    #[tokio::test]
    async fn dry_run_makes_zero_calls_and_skips_uncompilable_rows() {
        let dir = tempfile::tempdir().unwrap();
        let o = opts(dir.path(), false, true);
        let mock = MockBackend::new(|_, _| unreachable!("dry run must not call"));
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!(mock.calls(), 0);
        assert_eq!(
            s.eligible, 1,
            "broken row must be dropped by the compile gate"
        );
        assert_eq!(s.planned_calls, 2, "instruction + round-trip");
        assert!(s.estimated_usd > 0.0);
        assert!(!o.output.exists());
    }

    #[tokio::test]
    async fn apply_writes_faithful_pair_and_cache_prevents_respend() {
        let dir = tempfile::tempdir().unwrap();
        let o = opts(dir.path(), true, true);
        let mock = MockBackend::new(|system, user| {
            if system == INSTRUCTION_SYSTEM {
                assert!(user.contains("add_numbers"));
                "Write a function `add_numbers(a: int, b: int) to int` that returns the sum.".into()
            } else {
                format!("```vox\n{ADD}```")
            }
        });
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!((mock.calls(), s.written), (2, 1));
        let rows = synth::read_jsonl(&o.output).unwrap();
        assert_eq!(rows[0]["lane"], "vox_codegen");
        assert_eq!(rows[0]["model"], "mock/model");
        assert_eq!(rows[0]["rating"], 5);
        assert_eq!(rows[0]["round_trip"], true);

        let again = MockBackend::new(|_, _| unreachable!("cached rows must not re-call"));
        let s2 = run_back_translate(&o, &again).await.unwrap();
        assert_eq!((again.calls(), s2.cache_hits, s2.written), (0, 1, 1));
    }

    /// Real defect found by reading actual output: `mens/data/validated.jsonl`
    /// (one row per whole golden file, predates `decl_pairs.rs`'s metadata
    /// stripping) still carries its file's `// ---` frontmatter and `@training_prompt`
    /// line in `response`. Without stripping, that block is both sent to the LLM as
    /// "the code" and, worse, written verbatim as the final pair's *answer* —
    /// reproducing the exact frontmatter-leak defect the rest of this corpus
    /// pipeline was fixed to remove, via a different code path.
    #[tokio::test]
    async fn golden_file_frontmatter_is_stripped_before_use() {
        let dir = tempfile::tempdir().unwrap();
        let raw = format!(
            "// ---\n// title: \"X\"\n// training_eligible: true\n// ---\n// @training_prompt: ignored here\n{ADD}"
        );
        let input = dir.path().join("in.jsonl");
        std::fs::write(
            &input,
            serde_json::json!({"response": raw, "source": "golden.vox"}).to_string() + "\n",
        )
        .unwrap();
        let o = BackTranslateOpts {
            input,
            output: dir.path().join("out.jsonl"),
            cache: dir.path().join("cache.jsonl"),
            bench: dir.path().join("missing_manifest.json"),
            max_rows: 100,
            round_trip: false,
            max_spend_usd: 10.0,
            usd_per_1k_tokens: 0.002,
            apply: true,
        };
        let mock = MockBackend::new(|_, user| {
            assert!(
                !user.contains("training_eligible") && !user.contains("// ---"),
                "frontmatter reached the LLM prompt: {user}"
            );
            "Write a function `add_numbers(a: int, b: int) to int` that returns the sum.".into()
        });
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!(s.written, 1);
        let rows = synth::read_jsonl(&o.output).unwrap();
        assert_eq!(
            rows[0]["response"],
            ADD.trim(),
            "answer must be exactly the stripped code"
        );
    }

    /// Real failure this observed on live hardware: OpenRouter returned a
    /// malformed response mid-batch, and the old `.await?` propagated the
    /// error out of `run_back_translate`, discarding every already-verified
    /// pair from earlier rows (nothing was written; the cache is what saved
    /// the already-spent calls from being paid for twice on retry).
    #[tokio::test]
    async fn a_transient_call_error_skips_one_row_not_the_whole_run() {
        use super::super::synth::test_support::FlakyMockBackend;
        let dir = tempfile::tempdir().unwrap();
        const ADD2: &str = "fn add_two(a: int, b: int) to int {\n    return a + b\n}\n";
        let input = dir.path().join("in.jsonl");
        std::fs::write(
            &input,
            [ADD, ADD2]
                .iter()
                .map(|c| serde_json::json!({"response": c, "source": "x.vox"}).to_string() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        let o = BackTranslateOpts {
            input,
            output: dir.path().join("out.jsonl"),
            cache: dir.path().join("cache.jsonl"),
            bench: dir.path().join("missing_manifest.json"),
            max_rows: 100,
            round_trip: true,
            max_spend_usd: 10.0,
            usd_per_1k_tokens: 0.002,
            apply: true,
        };
        // Calls, in order: row1-instruction(1), row1-round_trip(2, FAILS),
        // row2-instruction(3), row2-round_trip(4).
        let mock = FlakyMockBackend {
            reply: |system, _| {
                if system == INSTRUCTION_SYSTEM {
                    "Write `add_numbers` or `add_two` that returns the sum.".into()
                } else {
                    format!("```vox\n{ADD}```")
                }
            },
            fail_on_call: 2,
            calls: std::cell::RefCell::new(0),
        };
        let s = run_back_translate(&o, &mock)
            .await
            .expect("one bad call must not abort the run");
        assert_eq!(s.call_failed, 1);
        assert_eq!(*mock.calls.borrow(), 4, "both rows were attempted");
        assert_eq!(
            s.written, 1,
            "row2 still produces a pair despite row1's failure"
        );

        // Row1's cache entry survived with round_trip still unknown, so a rerun
        // retries only its round-trip call — not the already-paid instruction call.
        let cache_raw = std::fs::read_to_string(&o.cache).unwrap();
        let row1_entries: Vec<_> = cache_raw
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .filter(|e| e["instruction"].as_str().unwrap().contains("add_numbers"))
            .collect();
        assert!(
            row1_entries.iter().any(|e| e["round_trip"].is_null()),
            "row1 must be retryable, not silently dropped forever: {row1_entries:?}"
        );
    }

    #[tokio::test]
    async fn unfaithful_and_failed_round_trip_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let o = opts(dir.path(), true, false);
        let mock = MockBackend::new(|_, _| "Write a function that sums two numbers.".into());
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!((s.written, s.rejected_unfaithful), (0, 1));

        let dir = tempfile::tempdir().unwrap();
        let o = opts(dir.path(), true, true);
        let mock = MockBackend::new(|system, _| {
            if system == INSTRUCTION_SYSTEM {
                "Write `add_numbers`.".into()
            } else {
                BROKEN.into()
            }
        });
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!((s.written, s.rejected_round_trip), (0, 1));
    }

    #[tokio::test]
    async fn spend_cap_stops_calls() {
        let dir = tempfile::tempdir().unwrap();
        let mut o = opts(dir.path(), true, false);
        o.max_spend_usd = 0.0;
        let mock = MockBackend::new(|_, _| unreachable!());
        let s = run_back_translate(&o, &mock).await.unwrap();
        assert_eq!((mock.calls(), s.written), (0, 0));
    }
}
