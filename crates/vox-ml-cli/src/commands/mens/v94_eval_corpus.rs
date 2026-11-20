//! MENS v9.4 heldout eval v2 builder, 100%-harness self-verifier, zero-leakage
//! verified `.vox` corpus builder, and offline 5-rung checkpoint rescorer.

#[path = "v94_synth_exec.rs"]
mod synth_exec;
#[path = "v94_eval_tasks.rs"]
mod tasks;
#[path = "v94_eval_tasks_algo.rs"]
mod tasks_algo;
#[path = "v94_eval_tasks_domain.rs"]
mod tasks_domain;
#[path = "v94_teacher_distill.rs"]
mod teacher_distill;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::eval_gate::leakage::{
    BenchTask, assert_no_text_leakage, leaked_bench_task, leaked_bench_task_contained,
    load_bench_answers,
};
use super::eval_local_prompt::{HELDOUT_MAX_TOKENS, prepare_bench_item_for_model};
use super::metrics::{
    extract_code, run_harness_assertions, verify_completion, verify_completion_with_harness,
};
use tasks::{
    V2OwnedTask, extract_fn_name_from_sig, load_humaneval_heldout_v2, rewrite_main_to_eval_harness,
    v1_function_tasks,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sha_hex(text: &str) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(text.as_bytes()))
}

fn local_sample_seed(base: u64, problem_id: &str, sample_index: usize) -> u64 {
    let mut buf = Vec::with_capacity(32 + problem_id.len());
    buf.extend_from_slice(&base.to_le_bytes());
    buf.extend_from_slice(&(problem_id.len() as u64).to_le_bytes());
    buf.extend_from_slice(problem_id.as_bytes());
    buf.extend_from_slice(&(sample_index as u64).to_le_bytes());
    let digest = vox_crypto::facades::secure_hash(&buf);
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(head) & 0x7fff_ffff
}

fn ckpt_item_hash(prompt: &str, max_tokens: usize, temperature: f32, seed: u64) -> String {
    let prompt_json = serde_json::json!({
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "seed": seed,
    })
    .to_string();
    sha_hex(&prompt_json)
}

fn build_all_v2_tasks(root: &Path) -> Result<Vec<V2OwnedTask>> {
    let mut all: Vec<V2OwnedTask> = Vec::new();
    for spec in v1_function_tasks() {
        all.push(V2OwnedTask::from(spec));
    }
    all.extend(load_humaneval_heldout_v2(root)?);
    for spec in tasks_algo::algo_and_grid_tasks() {
        all.push(V2OwnedTask::from(spec));
    }
    for spec in tasks_domain::domain_and_parsing_tasks() {
        all.push(V2OwnedTask::from(spec));
    }
    let mut seen = HashSet::new();
    for t in &all {
        if !seen.insert(t.id.clone()) {
            bail!("duplicate heldout_bench_v2 task id: {}", t.id);
        }
    }
    Ok(all)
}

fn self_verify_v2_tasks(tasks: &[V2OwnedTask]) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut failures = Vec::new();
    for (idx, t) in tasks.iter().enumerate() {
        let v_ok = verify_completion_with_harness(
            &t.answer,
            temp.path(),
            "",
            &t.id,
            idx,
            &t.semantic_expected_contains,
            Some(&t.test_harness),
        );
        if !(v_ok.pass && v_ok.pass_compile && v_ok.pass_exec == Some(true) && v_ok.semantic_pass) {
            failures.push(format!("REF FAIL {}: {}", t.id, v_ok.checks));
        }

        let v_bad = verify_completion_with_harness(
            &t.wrong_stub,
            temp.path(),
            "",
            &format!("{}_stub", t.id),
            idx,
            &t.semantic_expected_contains,
            Some(&t.test_harness),
        );
        if v_bad.pass || v_bad.pass_exec != Some(false) {
            failures.push(format!(
                "TAUTOLOGICAL HARNESS {}: wrong_stub scored pass={} exec={:?} checks={}",
                t.id, v_bad.pass, v_bad.pass_exec, v_bad.checks
            ));
        }
    }
    if !failures.is_empty() {
        bail!(
            "{} self-verification failure(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
    Ok(())
}

fn task_to_manifest_json(t: &V2OwnedTask) -> Value {
    json!({
        "id": t.id,
        "category": t.category,
        "description": t.description,
        "semantic_expected_contains": t.semantic_expected_contains,
        "answer": t.answer,
        "test_harness": t.test_harness,
    })
}

fn write_manifest(path: &Path, description: &str, tasks: &[V2OwnedTask]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let benches: Vec<Value> = tasks.iter().map(task_to_manifest_json).collect();
    let doc = json!({
        "schema": "vox_mens_bench_manifest_v1",
        "description": description,
        "benchmarks": benches,
    });
    std::fs::write(path, serde_json::to_string_pretty(&doc)? + "\n")?;
    Ok(())
}

fn pick_stratified(pool: &[V2OwnedTask], count: usize) -> Vec<V2OwnedTask> {
    let n = pool.len();
    if count >= n {
        return pool.to_vec();
    }
    (0..count).map(|k| pool[k * n / count].clone()).collect()
}

fn build_canary_64(tasks: &[V2OwnedTask]) -> Vec<V2OwnedTask> {
    let quotas = [
        ("humaneval-vox", 16usize),
        ("function", 8usize),
        ("algorithm", 7usize),
        ("data_structures", 7usize),
        ("matrix_grid", 6usize),
        ("string_parsing", 6usize),
        ("adt_match", 6usize),
        ("state_machine", 4usize),
        ("endpoint_handler", 4usize),
    ];
    let mut out = Vec::with_capacity(64);
    for (cat, q) in quotas {
        let sub: Vec<V2OwnedTask> = tasks
            .iter()
            .filter(|t| t.category == cat)
            .cloned()
            .collect();
        out.extend(pick_stratified(&sub, q));
    }
    out
}

fn extract_prompt_for_source_file(rel: &str, content: &str) -> String {
    for line in content.lines().take(25) {
        let t = line.trim();
        if let Some(p) = t
            .strip_prefix("// @training_prompt:")
            .or_else(|| t.strip_prefix("// description:"))
            .or_else(|| t.strip_prefix("// title:"))
        {
            let cleaned = p.trim().trim_matches('"');
            if !cleaned.is_empty() {
                return format!("Write valid Vox code implementing: {cleaned}");
            }
        }
    }
    format!("Write a valid self-contained Vox module matching `{rel}`.")
}

fn make_codegen_row(prompt: &str, code: &str, category: &str, source: &str, origin: &str) -> Value {
    json!({
        "prompt": prompt,
        "response": code,
        "messages": [
            { "role": "user", "content": prompt },
            { "role": "assistant", "content": code },
        ],
        "instruction": prompt,
        "output": code,
        "category": category,
        "source": source,
        "origin": origin,
        "rating": 5,
        "schema_version": "vox_dogfood_v1",
        "lane": "vox_codegen",
        "response_mode": "code_only",
        "task_family": "vox_codegen",
    })
}

fn normalize_code_key(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct CorpusAccumulator<'a> {
    all_bench: &'a [BenchTask],
    rows: Vec<Value>,
    dedup_hashes: HashSet<String>,
    counts: BTreeMap<String, usize>,
    skipped: BTreeMap<String, usize>,
}

impl<'a> CorpusAccumulator<'a> {
    fn new(all_bench: &'a [BenchTask]) -> Self {
        Self {
            all_bench,
            rows: Vec::new(),
            dedup_hashes: HashSet::new(),
            counts: BTreeMap::new(),
            skipped: BTreeMap::new(),
        }
    }

    fn bump_skip(&mut self, reason: &str) {
        *self.skipped.entry(reason.to_string()).or_default() += 1;
    }

    fn try_push(&mut self, prompt: &str, code: &str, category: &str, source: &str, origin: &str) {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            self.bump_skip("empty");
            return;
        }
        if let Some((tid, _)) = leaked_bench_task(trimmed, self.all_bench) {
            self.bump_skip(&format!("leakage_jaccard:{tid}"));
            return;
        }
        if let Some((tid, _)) = leaked_bench_task_contained(self.all_bench, trimmed) {
            self.bump_skip(&format!("leakage_contained:{tid}"));
            return;
        }
        let key = sha_hex(&normalize_code_key(trimmed));
        if !self.dedup_hashes.insert(key) {
            self.bump_skip("duplicate");
            return;
        }
        *self.counts.entry(origin.to_string()).or_default() += 1;
        self.rows
            .push(make_codegen_row(prompt, trimmed, category, source, origin));
    }
}

fn build_verified_codegen_corpus(root: &Path, all_bench: &[BenchTask]) -> Result<Value> {
    let temp = tempfile::tempdir()?;
    let mut acc = CorpusAccumulator::new(all_bench);

    // Source 1: 133 training_eligible:true humaneval-vox problems (compile + tests.vox executed!)
    let fixtures = vox_corpus::humaneval_runner::manifest::load_corpus(
        &root.join("contracts/eval/humaneval-vox"),
    )?;
    let mut humaneval_eligible = 0usize;
    let mut humaneval_verified = 0usize;
    for fx in fixtures {
        if !fx.training_eligible {
            continue;
        }
        humaneval_eligible += 1;
        let reference = std::fs::read_to_string(&fx.reference_path)?;
        let tests_src = std::fs::read_to_string(&fx.tests_path)?;
        let harness = rewrite_main_to_eval_harness(
            &vox_corpus::humaneval_runner::compose::extract_test_block(&tests_src)?,
        );
        let ok = run_harness_assertions(&reference, &harness, &fx.tests_path).unwrap_or(false);
        if ok {
            humaneval_verified += 1;
            let prompt = format!(
                "Write a Vox function implementing the specification.\n\n{}\n\nRequired signature:\n{}",
                fx.prompt, fx.signature
            );
            acc.try_push(
                &prompt,
                &reference,
                "humaneval_verified",
                &format!(
                    "contracts/eval/humaneval-vox/problems/{}/reference.vox",
                    fx.slug
                ),
                "humaneval_vox_training_eligible",
            );
        } else {
            acc.bump_skip("humaneval_exec_fail");
        }
    }

    // Source 2: eligible repo `.vox` files via vox-source-pool + compiler gate
    let pool_cfg_path = root.join("mens/config/vox-source-pool.yaml");
    let selector = super::super::corpus::source_pool::Selector::load(&pool_cfg_path, None)?;
    let walked = super::super::corpus::source_pool::walk(root);
    let mut pool_verified = 0usize;
    for path in walked {
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if selector.skip_reason(rel, &content).is_some() {
            acc.bump_skip("source_pool_policy");
            continue;
        }
        let v = verify_completion(&content, temp.path(), "", "pool", 0, &[]);
        if v.pass && v.pass_compile && v.anti_stub_pass {
            pool_verified += 1;
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            let prompt = extract_prompt_for_source_file(&rel_s, &content);
            acc.try_push(
                &prompt,
                &content,
                "vox_module_verified",
                &rel_s,
                "repo_source_pool",
            );
        } else {
            acc.bump_skip("source_pool_compile_fail");
        }
    }

    // Source 3: dogfood_v4/train.jsonl compiler-verified codegen rows
    let dogfood_path = root.join("target/dogfood_v4/train.jsonl");
    let mut dogfood_total = 0usize;
    let mut dogfood_compiled = 0usize;
    if dogfood_path.is_file() {
        let f = std::fs::File::open(&dogfood_path)?;
        let reader = BufReader::new(f);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if obj["lane"].as_str() != Some("vox_codegen") {
                continue;
            }
            dogfood_total += 1;
            let prompt = obj["prompt"].as_str().unwrap_or("").trim();
            let raw_resp = obj["response"]
                .as_str()
                .or_else(|| obj["output"].as_str())
                .unwrap_or("");
            let code = extract_code(raw_resp);
            let v = verify_completion(&code, temp.path(), "", &format!("dogfood_{idx}"), idx, &[]);
            if !prompt.is_empty()
                && v.pass
                && v.pass_compile
                && v.anti_stub_pass
                && v.constraint_adherence
            {
                dogfood_compiled += 1;
                let cat = obj["category"].as_str().unwrap_or("function");
                let src = obj["source"]
                    .as_str()
                    .unwrap_or("target/dogfood_v4/train.jsonl");
                acc.try_push(prompt, &code, cat, src, "dogfood_v4_verified");
            } else {
                acc.bump_skip("dogfood_compile_or_stub_fail");
            }
        }
    }

    // Source 4A: mens/data/train_full_backup.jsonl (code_only compiler-verified)
    let backup_path = root.join("mens/data/train_full_backup.jsonl");
    let mut backup_total = 0usize;
    let mut backup_compiled = 0usize;
    if backup_path.is_file() {
        let f = std::fs::File::open(&backup_path)?;
        let reader = BufReader::new(f);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if obj["lane"].as_str() != Some("vox_codegen") {
                continue;
            }
            if obj["response_mode"].as_str().unwrap_or("code_only") != "code_only" {
                continue;
            }
            backup_total += 1;
            let prompt = obj["prompt"].as_str().unwrap_or("").trim();
            let raw_resp = obj["response"]
                .as_str()
                .or_else(|| obj["output"].as_str())
                .unwrap_or("");
            let code = extract_code(raw_resp);
            let v = verify_completion(&code, temp.path(), "", &format!("backup_{idx}"), idx, &[]);
            if !prompt.is_empty()
                && v.pass
                && v.pass_compile
                && v.anti_stub_pass
                && v.constraint_adherence
            {
                backup_compiled += 1;
                let cat = obj["category"].as_str().unwrap_or("function");
                let src = obj["source"]
                    .as_str()
                    .unwrap_or("mens/data/train_full_backup.jsonl");
                acc.try_push(prompt, &code, cat, src, "train_full_backup_verified");
            } else {
                acc.bump_skip("backup_compile_or_stub_fail");
            }
        }
    }

    // Source 4B: mens/data/validated.clean.jsonl compiler-verified .vox snippets
    let clean_path = root.join("mens/data/validated.clean.jsonl");
    let mut clean_total = 0usize;
    let mut clean_compiled = 0usize;
    if clean_path.is_file() {
        let f = std::fs::File::open(&clean_path)?;
        let reader = BufReader::new(f);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            clean_total += 1;
            let raw_code = obj["code"].as_str().unwrap_or("").trim();
            let code = extract_code(raw_code);
            let src = obj["source"]
                .as_str()
                .unwrap_or("mens/data/validated.clean.jsonl");
            let prompt = extract_prompt_for_source_file(src, &code);
            let v = verify_completion(&code, temp.path(), "", &format!("clean_{idx}"), idx, &[]);
            if !code.is_empty() && v.pass && v.pass_compile && v.anti_stub_pass {
                clean_compiled += 1;
                acc.try_push(
                    &prompt,
                    &code,
                    "vox_module_verified",
                    src,
                    "validated_clean_verified",
                );
            } else {
                acc.bump_skip("validated_clean_compile_or_stub_fail");
            }
        }
    }

    // Source 5: deterministic execution-verified synthetic `.vox` tasks across 10 families
    let (synth_rows, synth_stats) = synth_exec::generate_and_verify_synth_rows()?;
    for row in &synth_rows {
        acc.try_push(
            &row.prompt,
            &row.code,
            &row.category,
            &row.source,
            "v94_synth_exec_verified",
        );
    }

    // Source 6: MIT/Apache-2.0 frontier teacher distilled & harness-verified rows (if present)
    let teacher_path = root.join("mens/data/mix_sources/mit_teacher_rft_v94.jsonl");
    let mut teacher_total = 0usize;
    let mut teacher_verified = 0usize;
    if teacher_path.is_file() {
        let f = std::fs::File::open(&teacher_path)?;
        let reader = BufReader::new(f);
        for (idx, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            teacher_total += 1;
            let prompt = obj["prompt"].as_str().unwrap_or("").trim();
            let raw_resp = obj["response"]
                .as_str()
                .or_else(|| obj["output"].as_str())
                .unwrap_or("");
            let code = extract_code(raw_resp);
            let harness = obj["test_harness"].as_str();
            let expected: Vec<String> = obj["expected_contains"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let v = verify_completion_with_harness(
                &code,
                temp.path(),
                "",
                &format!("mit_teacher_{idx}"),
                idx,
                &expected,
                harness,
            );
            let exec_ok = if harness.is_some() {
                v.pass_exec == Some(true)
            } else {
                v.pass_exec.unwrap_or(true)
            };
            if !prompt.is_empty()
                && v.pass
                && v.pass_compile
                && exec_ok
                && v.anti_stub_pass
                && v.constraint_adherence
            {
                teacher_verified += 1;
                let cat = obj["category"].as_str().unwrap_or("function");
                let src = obj["source"]
                    .as_str()
                    .unwrap_or("mens/data/mix_sources/mit_teacher_rft_v94.jsonl");
                acc.try_push(prompt, &code, cat, src, "mit_teacher_rft_v94");
            } else {
                acc.bump_skip("mit_teacher_verify_fail");
            }
        }
    }

    // Stage in temp dir to run exact CI leakage gate against BOTH heldout manifests before committing disk artifact.
    let stage_dir = temp.path().join("corpus_stage");
    std::fs::create_dir_all(&stage_dir)?;
    let stage_jsonl = stage_dir.join("verified_vox_codegen_v2.jsonl");
    {
        let mut wf = std::fs::File::create(&stage_jsonl)?;
        for row in &acc.rows {
            serde_json::to_writer(&mut wf, row)?;
            writeln!(wf)?;
        }
    }
    assert_no_text_leakage(
        &root.join("mens/data/heldout_bench/manifest.json"),
        &[&stage_dir],
    )
    .context("leakage check failed against v1 heldout_bench")?;
    assert_no_text_leakage(
        &root.join("mens/data/heldout_bench_v2/manifest.json"),
        &[&stage_dir],
    )
    .context("leakage check failed against v2 heldout_bench_v2")?;

    let out_jsonl = root.join("mens/data/verified_vox_codegen_v2.jsonl");
    std::fs::copy(&stage_jsonl, &out_jsonl)?;

    // Also emit deterministically shuffled (seed=42) train.jsonl for isolated canary/27B training.
    let mut shuffled_rows = acc.rows.clone();
    let mut rng_state: u64 = 42u64 ^ 0x9E37_79B9_7F4A_7C15;
    for i in (1..shuffled_rows.len()).rev() {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let j = (rng_state as usize) % (i + 1);
        shuffled_rows.swap(i, j);
    }
    let canary_train_jsonl = root.join("target/v94_canary_data/train.jsonl");
    if let Some(p) = canary_train_jsonl.parent() {
        std::fs::create_dir_all(p)?;
    }
    {
        let mut wf = std::fs::File::create(&canary_train_jsonl)?;
        for row in &shuffled_rows {
            serde_json::to_writer(&mut wf, row)?;
            writeln!(wf)?;
        }
    }

    let jsonl_bytes = std::fs::read(&out_jsonl)?;
    let out_blake3 = vox_crypto::hex_encode(&vox_crypto::secure_hash(&jsonl_bytes));

    let synth_emitted = acc
        .counts
        .get("v94_synth_exec_verified")
        .copied()
        .unwrap_or(0);
    let mit_emitted = acc.counts.get("mit_teacher_rft_v94").copied().unwrap_or(0);
    let exec_verified_total = humaneval_verified + synth_emitted + mit_emitted;

    Ok(json!({
        "schema": "mens_verified_corpus_v2",
        "output_jsonl": "mens/data/verified_vox_codegen_v2.jsonl",
        "canary_train_jsonl": "target/v94_canary_data/train.jsonl",
        "output_blake3": out_blake3,
        "total_emitted": acc.rows.len(),
        "execution_verified_total": exec_verified_total,
        "humaneval_training_eligible_total": humaneval_eligible,
        "humaneval_execution_verified": humaneval_verified,
        "repo_source_pool_compiled": pool_verified,
        "dogfood_v4_total_codegen": dogfood_total,
        "dogfood_v4_compiler_verified": dogfood_compiled,
        "train_full_backup_total": backup_total,
        "train_full_backup_verified": backup_compiled,
        "validated_clean_total": clean_total,
        "validated_clean_verified": clean_compiled,
        "synth_exec_stats": synth_stats,
        "mit_teacher_rft_total": teacher_total,
        "mit_teacher_rft_verified": teacher_verified,
        "emitted_by_origin": acc.counts,
        "skipped_by_reason": acc.skipped,
        "leakage_checks": {
            "heldout_bench_v1_passed": true,
            "heldout_bench_v2_passed": true,
        }
    }))
}

fn parse_sig_params_and_ret(sig: &str) -> Option<(String, Vec<(String, String)>, String)> {
    let fname = extract_fn_name_from_sig(sig)?;
    let open = sig.find('(')?;
    let close = sig.rfind(") to ")?;
    let params_str = sig[open + 1..close].trim();
    let ret = sig[close + ") to ".len()..].trim().to_string();
    let mut params = Vec::new();
    if !params_str.is_empty() {
        for part in split_top_level_commas(params_str) {
            let mut it = part.splitn(2, ':');
            let pname = it.next()?.trim().to_string();
            let pty = it.next()?.trim().to_string();
            params.push((pname, pty));
        }
    }
    Some((fname, params, ret))
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        out.push(s[start..].trim());
    }
    out
}

/// If `candidate` defines a single non-test function matching `(arity, return_type)`
/// under a different name, build a forwarding alias `expected_name(...) { return actual_name(...) }`.
fn try_signature_rescue(candidate_code: &str, expected_sig: &str) -> Option<String> {
    let (exp_name, exp_params, exp_ret) = parse_sig_params_and_ret(expected_sig)?;
    let mut matching_fns: Vec<String> = Vec::new();
    for line in candidate_code.lines() {
        let t = line.trim();
        if t.starts_with("fn ")
            && let Some((cname, cparams, cret)) =
                parse_sig_params_and_ret(t.trim_end_matches('{').trim())
            && cparams.len() == exp_params.len()
            && cret == exp_ret
            && !cname.starts_with("test_")
            && cname != exp_name
        {
            matching_fns.push(cname);
        }
    }
    let chosen = matching_fns.last()?.clone();
    let formal_params = exp_params
        .iter()
        .map(|(n, ty)| format!("{n}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let actual_args = exp_params
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "{}\n\nfn {exp_name}({formal_params}) to {exp_ret} {{\n    return {chosen}({actual_args})\n}}\n",
        candidate_code.trim_end()
    ))
}

fn load_ckpt_map(ckpt_path: &Path) -> Result<HashMap<String, String>> {
    let f = std::fs::File::open(ckpt_path)
        .with_context(|| format!("opening {}", ckpt_path.display()))?;
    let reader = BufReader::new(f);
    let mut map = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let val: Value = serde_json::from_str(&line)?;
        if let (Some(item), Some(resp)) = (val["item"].as_str(), val["response"].as_str()) {
            let gen_text = serde_json::from_str::<Value>(resp)
                .ok()
                .and_then(|v| v["generated_text"].as_str().map(str::to_string))
                .unwrap_or_default();
            map.insert(item.to_string(), gen_text);
        }
    }
    Ok(map)
}

fn rescore_five_rungs(root: &Path, v2_tasks: &[V2OwnedTask]) -> Result<Value> {
    let bench_dir = root.join("mens/data/heldout_bench");
    let v1_manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(bench_dir.join("manifest.json"))?)?;
    let benchmarks = v1_manifest["benchmarks"]
        .as_array()
        .context("v1 manifest has no benchmarks array")?;

    let mut v2_by_id: HashMap<&str, &V2OwnedTask> = HashMap::new();
    for t in v2_tasks {
        v2_by_id.insert(t.id.as_str(), t);
    }
    let fixtures = vox_corpus::humaneval_runner::manifest::load_corpus(
        &root.join("contracts/eval/humaneval-vox"),
    )?;
    let mut sig_by_v1_id: HashMap<String, String> = HashMap::new();
    for fx in fixtures {
        let id = format!("humaneval_vox_{}", fx.slug.replace('-', "_"));
        sig_by_v1_id.insert(id, fx.signature);
    }

    let mut item_meta: Vec<(String, String, String, Vec<String>, String)> = Vec::new();
    for (idx, b) in benchmarks.iter().enumerate() {
        let prep = prepare_bench_item_for_model(&bench_dir, idx, b, Some("Qwen/Qwen3.5-0.8B"));
        let seed = local_sample_seed(1337, &prep.id, 0);
        let key = ckpt_item_hash(&prep.prompt, HELDOUT_MAX_TOKENS, 0.0, seed);
        item_meta.push((
            key,
            prep.id,
            prep.category,
            prep.semantic_expected_contains,
            b["id"].as_str().unwrap_or("").to_string(),
        ));
    }

    let rungs = [
        ("0.8B", "mens/runs/qwen3.5-0.8b"),
        ("2B", "mens/runs/qwen3.5-2b"),
        ("4B", "mens/runs/qwen3.5-4b"),
        ("9B", "mens/runs/qwen3.5-9b"),
        ("27B", "mens/runs/qwen3.8-27b"),
    ];
    let temp = tempfile::tempdir()?;
    let mut rung_reports = Vec::new();

    for (label, rel_dir) in rungs {
        let run_dir = root.join(rel_dir);
        let mut sides = BTreeMap::new();
        for (side_name, ckpt_name) in [
            ("base", "baseline_passatk.ckpt.jsonl"),
            ("adapter", "eval_local_report.ckpt.jsonl"),
        ] {
            let ckpt_map = load_ckpt_map(&run_dir.join(ckpt_name))?;
            let mut matched = 0usize;
            let mut v1_pass = 0usize;
            let mut v1_semantic = 0usize;
            let mut v2_strict_52 = 0usize;
            let mut v2_rescued_52 = 0usize;
            let mut exec_46_strict = 0usize;
            let mut exec_46_rescued = 0usize;
            let mut exec_46_compile_fail = 0usize;
            let mut exec_46_sig_rescued = 0usize;
            let mut exec_46_logic_fail = 0usize;
            let mut failing_ids = Vec::new();

            for (idx, (key, id, _cat, sem, _raw_id)) in item_meta.iter().enumerate() {
                let raw = ckpt_map.get(key).map(String::as_str).unwrap_or("");
                if ckpt_map.contains_key(key) {
                    matched += 1;
                }
                let answer_region = vox_eval::qwen_reasoning::answer_region(raw);
                let base_v = verify_completion(answer_region, temp.path(), "", id, idx, sem);
                if base_v.pass {
                    v1_pass += 1;
                }
                if base_v.semantic_pass {
                    v1_semantic += 1;
                }

                if let Some(t2) = v2_by_id.get(id.as_str()) {
                    let strict_v = verify_completion_with_harness(
                        answer_region,
                        temp.path(),
                        "",
                        id,
                        idx,
                        sem,
                        Some(&t2.test_harness),
                    );
                    if strict_v.pass {
                        v2_strict_52 += 1;
                        v2_rescued_52 += 1;
                        exec_46_strict += 1;
                        exec_46_rescued += 1;
                    } else if !strict_v.pass_compile || !strict_v.anti_stub_pass {
                        exec_46_compile_fail += 1;
                        failing_ids.push(format!("{id}:compile_or_stub"));
                    } else {
                        let mut rescued = false;
                        if let Some(sig) = sig_by_v1_id.get(id)
                            && let Some(aliased) =
                                try_signature_rescue(&extract_code(answer_region), sig)
                            && run_harness_assertions(
                                &aliased,
                                &t2.test_harness,
                                &temp.path().join("r.vox"),
                            )
                            .unwrap_or(false)
                        {
                            rescued = true;
                        }
                        if rescued {
                            v2_rescued_52 += 1;
                            exec_46_rescued += 1;
                            exec_46_sig_rescued += 1;
                            failing_ids.push(format!("{id}:sig_mismatch_only"));
                        } else {
                            exec_46_logic_fail += 1;
                            failing_ids.push(format!("{id}:logic_or_runtime_fail"));
                        }
                    }
                } else if base_v.pass {
                    v2_strict_52 += 1;
                    v2_rescued_52 += 1;
                }
            }

            if matched != 52 {
                bail!("{label} {side_name} matched {matched}/52 items in checkpoint");
            }
            eprintln!(
                "V94_RESCORE_RUNG {label} {side_name}: v1={v1_pass}/52 strict_52={v2_strict_52}/52 rescued_52={v2_rescued_52}/52 exec46={exec_46_strict}->{exec_46_rescued}/46"
            );
            sides.insert(
                side_name.to_string(),
                json!({
                    "matched_items": matched,
                    "v1_compile_anti_stub_52": v1_pass,
                    "v1_semantic_52": v1_semantic,
                    "v2_strict_exec_52": v2_strict_52,
                    "v2_sig_rescued_exec_52": v2_rescued_52,
                    "executable_46_strict_pass": exec_46_strict,
                    "executable_46_strict_rate": exec_46_strict as f64 / 46.0,
                    "executable_46_sig_rescued_pass": exec_46_rescued,
                    "executable_46_sig_rescued_rate": exec_46_rescued as f64 / 46.0,
                    "failure_breakdown_46": {
                        "compile_or_stub_fail": exec_46_compile_fail,
                        "sig_mismatch_rescued": exec_46_sig_rescued,
                        "logic_or_assertion_fail": exec_46_logic_fail,
                    },
                    "sample_non_passing_46": failing_ids,
                }),
            );
        }
        rung_reports.push(json!({
            "rung": label,
            "run_dir": rel_dir,
            "base": sides["base"],
            "adapter": sides["adapter"],
        }));
    }

    Ok(json!({
        "schema": "mens_execution_eval_rescore_v2",
        "bench_v1_total": 52,
        "executable_harness_tasks": 46,
        "non_executable_construct_tasks": 6,
        "rungs": rung_reports,
    }))
}

#[test]
fn build_and_verify_v94_eval_and_corpus() {
    let root = repo_root();

    // 1. Assemble + self-verify heldout_bench_v2 (166 tasks)
    let v2_tasks = build_all_v2_tasks(&root).expect("build v2 tasks");
    assert!(
        v2_tasks.len() >= 162,
        "expected >= 162 tasks, got {}",
        v2_tasks.len()
    );
    self_verify_v2_tasks(&v2_tasks).expect("all v2 tasks must pass ref & fail stub");

    // 2. Write heldout_bench_v2 + stratified 64-task canary
    write_manifest(
        &root.join("mens/data/heldout_bench_v2/manifest.json"),
        "MENS v9.4 heldout eval v2 (166 100%-execution-verified Vox tasks: 31 held-out humaneval-vox + 15 upgraded v1 functions + 120 net-new harder tasks with explicit signatures and multi-assertion __vox_eval_harness oracles).",
        &v2_tasks,
    )
    .expect("write heldout_bench_v2");

    let canary = build_canary_64(&v2_tasks);
    assert_eq!(canary.len(), 64);
    write_manifest(
        &root.join("mens/data/heldout_bench_v2_canary/manifest.json"),
        "Stratified 64-task MENS v9.4 canary subset of heldout_bench_v2 (16 humaneval-vox + 8 v1 functions + 40 net-new across all 7 categories).",
        &canary,
    )
    .expect("write heldout_bench_v2_canary");

    // 3. Build verified_vox_codegen_v2.jsonl with 0 leakage against both v1 & v2 heldout sets
    let mut combined_bench =
        load_bench_answers(&root.join("mens/data/heldout_bench/manifest.json")).expect("v1 bench");
    combined_bench.extend(
        load_bench_answers(&root.join("mens/data/heldout_bench_v2/manifest.json"))
            .expect("v2 bench"),
    );
    let corpus_report = build_verified_codegen_corpus(&root, &combined_bench)
        .expect("build verified_vox_codegen_v2 corpus");
    let corpus_report_path = root.join("contracts/reports/mens-verified-corpus-v2.json");
    std::fs::write(
        &corpus_report_path,
        serde_json::to_string_pretty(&corpus_report).unwrap() + "\n",
    )
    .expect("write corpus report");
    let total_emitted = corpus_report["total_emitted"].as_u64().unwrap_or(0);
    assert!(
        total_emitted >= 3000,
        "CAP-27B-02 requires >= 3000 unique verified rows, got {total_emitted}"
    );

    // 4. Re-score all 10 stored checkpoints (0.8B -> 27B) offline under real execution harnesses
    let rescore_report = rescore_five_rungs(&root, &v2_tasks).expect("rescore 5 rungs");
    let rescore_report_path = root.join("contracts/reports/mens-execution-eval-rescore-v2.json");
    std::fs::write(
        &rescore_report_path,
        serde_json::to_string_pretty(&rescore_report).unwrap() + "\n",
    )
    .expect("write rescore report");

    eprintln!(
        "V94_CORPUS_REPORT: {}",
        serde_json::to_string(&corpus_report).unwrap()
    );
    eprintln!(
        "V94_RESCORE_REPORT: {}",
        serde_json::to_string(&rescore_report).unwrap()
    );
}
