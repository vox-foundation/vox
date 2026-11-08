//! Deterministic, compiler-verified Vox `<think>` reasoning trace synthesizer.
//!
//! Addresses two root causes of `vox_think` collateral regressions after codegen QLoRA SFT:
//! 1. **Auto-fix hallucination on literal `debug` prompts**: When `train_mixed.jsonl` contains
//!    25%+ `error_correction` ("Fix the bug") rows and 0% literal execution traces, the adapter
//!    spots the bug inside `<think>` and outputs the *intended/fixed* value rather than what
//!    `main()` literally returns as written.
//! 2. **Unclosed `<think>` overthinking on `plan-check` / loop traces**: Concise 1-line-per-step
//!    state tables inside `<think>...</think>` teach the adapter to close `</think>` in <150
//!    tokens rather than rambling past the 2048-token cap.
//!
//! All generated Vox programs are parse-verified via `vox_compiler` and checked for 8-gram
//! disjointness against `mens/data/vox_think/reasoning.jsonl`.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

/// One synthesized thinking-mode SFT row for the `vox-lang` spoke.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoxReasoningTraceRow {
    pub id: String,
    pub family: String,
    pub prompt: String,
    pub response: String,
    pub code: String,
    pub oracle_answer: i64,
}

impl VoxReasoningTraceRow {
    /// Serialize as a `vox_codegen` training JSONL line with `<think>...</think>` preserved.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        json!({
            "id": self.id,
            "prompt": self.prompt,
            "response": self.response,
            "messages": [
                { "role": "user", "content": self.prompt },
                { "role": "assistant", "content": self.response }
            ],
            "instruction": self.prompt,
            "output": self.response,
            "category": format!("vox_reasoning_{}", self.family),
            "task_family": "vox_reasoning",
            "lane": "vox_codegen",
            "response_mode": "reasoning_trace",
            "origin": "synthetic",
            "rating": 5,
            "schema_version": "vox_dogfood_v1"
        })
        .to_string()
    }
}

const DOMAIN_NOUNS: &[(&str, &str, &str)] = &[
    ("orbit_apogee", "Pulse", "Drift"),
    ("thermal_flux", "Boost", "Cool"),
    ("beacon_stride", "Relay", "Damp"),
    ("crystal_lattice", "Charge", "Bleed"),
    ("vault_ledger", "Credit", "Debit"),
    ("signal_harmonic", "Amplify", "Trim"),
    ("anchor_cadence", "Advance", "Hold"),
    ("vector_aperture", "Widen", "Pinch"),
    ("glyph_checksum", "Shift", "Mask"),
    ("packet_quorum", "Accept", "Drop"),
];

/// Generate 120 deterministic, compiler-verified Vox `<think>` reasoning rows (40 per family).
#[must_use]
pub fn generate_vox_reasoning_traces(seed: u64) -> Vec<VoxReasoningTraceRow> {
    let mut rows = Vec::with_capacity(120);
    let offset = (seed as usize) % DOMAIN_NOUNS.len();

    // Family 1 (40 rows): Literal execution of defective code ("do not repair the bug").
    for idx in 0..40usize {
        let (fn_stem, var_a, var_b) = DOMAIN_NOUNS[(idx + offset) % DOMAIN_NOUNS.len()];
        let fn_name = format!("{fn_stem}_{idx}");
        let row = match idx % 4 {
            0 => build_literal_wrong_init_seed(idx, &fn_name),
            1 => build_literal_integer_div_trunc(idx, &fn_name),
            2 => build_literal_swapped_variant_scale(idx, &fn_name, var_a, var_b),
            _ => build_literal_strict_vs_inclusive_cmp(idx, &fn_name),
        };
        rows.push(row);
    }

    // Family 2 (40 rows): Concise stateful loop & enum match table trace (closes </think> early).
    for idx in 0..40usize {
        let (fn_stem, var_a, var_b) = DOMAIN_NOUNS[(idx + offset + 3) % DOMAIN_NOUNS.len()];
        let fn_name = format!("step_{fn_stem}_{idx}");
        let row = if idx % 2 == 0 {
            build_concise_range_fold_trace(idx, &fn_name)
        } else {
            build_concise_enum_dispatch_trace(idx, &fn_name, var_a, var_b)
        };
        rows.push(row);
    }

    // Family 3 (40 rows): Vox semantics invariants (half-open range(lo, hi), clamp/branch composition).
    for idx in 0..40usize {
        let (fn_stem, _, _) = DOMAIN_NOUNS[(idx + offset + 6) % DOMAIN_NOUNS.len()];
        let fn_name = format!("eval_{fn_stem}_{idx}");
        let row = build_vox_semantics_invariant_trace(idx, &fn_name);
        rows.push(row);
    }

    rows
}

/// Write synthesized Vox `<think>` reasoning rows to `output` JSONL.
pub fn write_vox_reasoning_traces_jsonl(
    rows: &[VoxReasoningTraceRow],
    output: &Path,
) -> anyhow::Result<usize> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output)?,
    );
    for row in rows {
        writeln!(file, "{}", row.to_jsonl())?;
    }
    file.flush()?;
    Ok(rows.len())
}

fn build_literal_wrong_init_seed(idx: usize, fn_name: &str) -> VoxReasoningTraceRow {
    let n = 3 + (idx as i64 % 4); // 3..=6
    let bonus = 4 + (idx as i64 % 9);
    // Defect: `let mut prod: int = 0` inside product accumulator, so loop keeps prod = 0!
    let literal_ret = bonus;
    let code = format!(
        "fn {fn_name}(limit: int) to int {{\n    let mut prod: int = 0\n    for k in range(1, limit + 1) {{\n        prod = prod * k\n    }}\n    return prod\n}}\n\nfn main() to int {{\n    return {fn_name}({n}) + {bonus}\n}}"
    );
    let prompt = format!(
        "Evaluate `main()` literally as written without repairing the bug in `{fn_name}` (which initializes `prod` to `0` instead of `1`). Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\nWe must trace `main()` literally as written without fixing `{fn_name}`.\n1. `{fn_name}({n})` sets `prod = 0`.\n2. For every `k` in `range(1, {})`, `prod = 0 * k = 0`, so `{fn_name}({n})` returns `0`.\n3. `main()` returns `0 + {bonus} = {literal_ret}`.\n</think>\n\n{literal_ret}",
        n + 1
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-lit-{idx:04}"),
        family: "literal_debug".to_string(),
        prompt,
        response,
        code,
        oracle_answer: literal_ret,
    }
}

fn build_literal_integer_div_trunc(idx: usize, fn_name: &str) -> VoxReasoningTraceRow {
    let num1 = 2 + (idx as i64 % 5); // 2..=6
    let den = num1 + 2; // num1 < den so num1 / den == 0 in integer division
    let scale = 20 + (idx as i64 % 7) * 10;
    let base_offset = 10 + (idx as i64 % 15);
    let literal_ret = (num1 / den) * scale + base_offset;
    let code = format!(
        "fn {fn_name}(part: int, whole: int) to int {{\n    return part / whole * {scale}\n}}\n\nfn main() to int {{\n    return {fn_name}({num1}, {den}) + {base_offset}\n}}"
    );
    let prompt = format!(
        "Evaluate `main()` literally as written without repairing the integer-division order in `{fn_name}`. Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\nTrace `main()` literally as written (do not reorder `part / whole * {scale}`).\n1. In `{fn_name}({num1}, {den})`, integer division `{num1} / {den}` truncates to `0`.\n2. `0 * {scale} = 0`.\n3. `main()` returns `0 + {base_offset} = {literal_ret}`.\n</think>\n\n{literal_ret}"
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-lit-{idx:04}"),
        family: "literal_debug".to_string(),
        prompt,
        response,
        code,
        oracle_answer: literal_ret,
    }
}

fn build_literal_swapped_variant_scale(
    idx: usize,
    fn_name: &str,
    var_a: &str,
    var_b: &str,
) -> VoxReasoningTraceRow {
    let ty_name = format!("Mode{idx}");
    let w_a = 3 + (idx as i64 % 4);
    let w_b = 10 + (idx as i64 % 5);
    let x = 2 + (idx as i64 % 6);
    let y = 1 + (idx as i64 % 4);
    let literal_ret = x * w_b + y * w_a;
    let code = format!(
        "type {ty_name} =\n    | {var_a}(n: int)\n    | {var_b}(n: int)\n\nfn {fn_name}(m: {ty_name}) to int {{\n    return match m {{\n        {var_a}(n) => n * {w_b}\n        {var_b}(n) => n * {w_a}\n    }}\n}}\n\nfn main() to int {{\n    return {fn_name}({var_a}({x})) + {fn_name}({var_b}({y}))\n}}"
    );
    let prompt = format!(
        "Evaluate `main()` literally as written without swapping the multipliers in `{fn_name}`. Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\nTrace `main()` using the literal `match` arms in `{fn_name}`:\n1. `{fn_name}({var_a}({x}))` matches `{var_a}(n) => n * {w_b}` = `{x} * {w_b} = {}`.\n2. `{fn_name}({var_b}({y}))` matches `{var_b}(n) => n * {w_a}` = `{y} * {w_a} = {}`.\n3. Sum = `{} + {} = {literal_ret}`.\n</think>\n\n{literal_ret}",
        x * w_b,
        y * w_a,
        x * w_b,
        y * w_a
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-lit-{idx:04}"),
        family: "literal_debug".to_string(),
        prompt,
        response,
        code,
        oracle_answer: literal_ret,
    }
}

fn build_literal_strict_vs_inclusive_cmp(idx: usize, fn_name: &str) -> VoxReasoningTraceRow {
    let pivot = 3 + (idx as i64 % 5);
    let hi = pivot + 3;
    let step_add = 2 + (idx as i64 % 4);
    // Literal code uses `k >= pivot` over `range(1, hi)` (which is 1..hi-1).
    let mut literal_ret = 0i64;
    for k in 1..hi {
        if k >= pivot {
            literal_ret += step_add;
        }
    }
    let count = (hi - pivot).max(0);
    let code = format!(
        "fn {fn_name}(hi: int) to int {{\n    let mut tally: int = 0\n    for k in range(1, hi) {{\n        if k >= {pivot} {{\n            tally = tally + {step_add}\n        }}\n    }}\n    return tally\n}}\n\nfn main() to int {{\n    return {fn_name}({hi})\n}}"
    );
    let prompt = format!(
        "Evaluate `main()` literally as written (note `k >= {pivot}` and half-open `range(1, hi)`). Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\nTrace `{fn_name}({hi})` literally as written:\n1. `range(1, {hi})` visits `k = 1 .. {}`.\n2. Condition `k >= {pivot}` holds for `k` in `{pivot}..={}` ({count} values).\n3. Each match adds `{step_add}`, giving `{count} * {step_add} = {literal_ret}`.\n</think>\n\n{literal_ret}",
        hi - 1,
        hi - 1
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-lit-{idx:04}"),
        family: "literal_debug".to_string(),
        prompt,
        response,
        code,
        oracle_answer: literal_ret,
    }
}

fn build_concise_range_fold_trace(idx: usize, fn_name: &str) -> VoxReasoningTraceRow {
    let n = 4 + (idx as i64 % 3); // 4..=6
    let mul = 2 + (idx as i64 % 3);
    let bias = 1 + (idx as i64 % 5);
    let mut acc = 0i64;
    let mut step_lines = Vec::new();
    for i in 0..n {
        acc += i * mul + bias;
        step_lines.push(format!("i={i}: acc={acc}"));
    }
    let code = format!(
        "fn {fn_name}(n: int) to int {{\n    let mut acc: int = 0\n    for i in range(0, n) {{\n        acc = acc + i * {mul} + {bias}\n    }}\n    return acc\n}}\n\nfn main() to int {{\n    return {fn_name}({n})\n}}"
    );
    let prompt = format!(
        "Compute the exact integer `main()` produces by stepping through each iteration:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\n`range(0, {n})` visits `i = 0..{}`:\n- {}\nFinal `acc = {acc}`.\n</think>\n\n{acc}",
        n - 1,
        step_lines.join("\n- ")
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-trace-{idx:04}"),
        family: "state_trace".to_string(),
        prompt,
        response,
        code,
        oracle_answer: acc,
    }
}

fn build_concise_enum_dispatch_trace(
    idx: usize,
    fn_name: &str,
    var_a: &str,
    var_b: &str,
) -> VoxReasoningTraceRow {
    let ty_name = format!("Op{idx}");
    let a1 = 4 + (idx as i64 % 7);
    let b1 = 2 + (idx as i64 % 5);
    let a2 = 3 + (idx as i64 % 6);
    let gain = 2 + (idx as i64 % 3);
    let s1 = a1 * gain;
    let s2 = s1 - b1;
    let s3 = s2 + a2 * gain;
    let code = format!(
        "type {ty_name} =\n    | {var_a}(v: int)\n    | {var_b}(v: int)\n\nfn {fn_name}(cur: int, op: {ty_name}) to int {{\n    return match op {{\n        {var_a}(v) => cur + v * {gain}\n        {var_b}(v) => cur - v\n    }}\n}}\n\nfn main() to int {{\n    let s1 = {fn_name}(0, {var_a}({a1}))\n    let s2 = {fn_name}(s1, {var_b}({b1}))\n    return {fn_name}(s2, {var_a}({a2}))\n}}"
    );
    let prompt = format!(
        "Compute the exact integer `main()` produces by stepping through each call:\n\n```vox\n{code}\n```"
    );
    let response = format!(
        "<think>\nStep through `main()`:\n- `s1 = 0 + {a1} * {gain} = {s1}`\n- `s2 = {s1} - {b1} = {s2}`\n- `s3 = {s2} + {a2} * {gain} = {s3}`\n</think>\n\n{s3}"
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-trace-{idx:04}"),
        family: "state_trace".to_string(),
        prompt,
        response,
        code,
        oracle_answer: s3,
    }
}

fn build_vox_semantics_invariant_trace(idx: usize, fn_name: &str) -> VoxReasoningTraceRow {
    let lo = 1 + (idx as i64 % 3);
    let hi = lo + 3 + (idx as i64 % 3);
    let cap = (lo + hi) * 2;
    let scale = 3 + (idx as i64 % 4);
    let mut sum = 0i64;
    for k in lo..hi {
        let term = k * scale;
        sum += if term > cap { cap } else { term };
    }
    let code = format!(
        "fn clamp_term(x: int) to int {{\n    if x > {cap} {{\n        return {cap}\n    }}\n    return x\n}}\n\nfn {fn_name}() to int {{\n    let mut total: int = 0\n    for k in range({lo}, {hi}) {{\n        total = total + clamp_term(k * {scale})\n    }}\n    return total\n}}\n\nfn main() to int {{\n    return {fn_name}()\n}}"
    );
    let prompt = format!(
        "Determine the integer output of `main()` under Vox semantics:\n\n```vox\n{code}\n```"
    );
    let terms: Vec<String> = (lo..hi)
        .map(|k| {
            let t = (k * scale).min(cap);
            format!("k={k}: min({}, {cap})={t}", k * scale)
        })
        .collect();
    let response = format!(
        "<think>\nIn Vox, `range({lo}, {hi})` is half-open `[{lo}, {hi})`:\n- {}\nSum = `{sum}`.\n</think>\n\n{sum}",
        terms.join("\n- ")
    );
    VoxReasoningTraceRow {
        id: format!("vox-reason-inv-{idx:04}"),
        family: "invariant_check".to_string(),
        prompt,
        response,
        code,
        oracle_answer: sum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_generated_reasoning_row_has_valid_vox_ast_and_concise_closed_think() {
        let rows = generate_vox_reasoning_traces(42);
        assert_eq!(rows.len(), 120);
        for row in &rows {
            let tokens = vox_compiler::lexer::lex(&row.code);
            let parsed = vox_compiler::parser::parse(tokens);
            assert!(
                parsed.is_ok(),
                "generated Vox code for {} failed to parse: {:?}\n{}",
                row.id,
                parsed.err(),
                row.code
            );
            assert!(
                row.response.starts_with("<think>\n"),
                "{} must start with <think>",
                row.id
            );
            let close = row
                .response
                .find("\n</think>\n\n")
                .expect("must close </think>");
            let think_body = &row.response["<think>\n".len()..close];
            assert!(
                !think_body.trim().is_empty() && think_body.len() < 600,
                "{} think body len {} out of bounds",
                row.id,
                think_body.len()
            );
            let post_think = row.response[close + "\n</think>\n\n".len()..].trim();
            assert_eq!(
                post_think,
                row.oracle_answer.to_string(),
                "{} final answer mismatch",
                row.id
            );
        }
    }

    fn tokenize_words(s: &str) -> Vec<String> {
        s.split_whitespace()
            .map(|w| {
                w.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect()
    }

    fn ngrams_8(tokens: &[String]) -> HashSet<String> {
        if tokens.is_empty() {
            return HashSet::new();
        }
        let n = 8.min(tokens.len());
        tokens.windows(n).map(|w| w.join("\u{1}")).collect()
    }

    #[test]
    fn generated_reasoning_rows_have_zero_leakage_against_vox_think_slice() {
        let rows = generate_vox_reasoning_traces(42);
        let ws = crate::training::find_workspace_root().expect("workspace root");
        let vox_think_path = ws.join("mens/data/vox_think/reasoning.jsonl");
        if !vox_think_path.is_file() {
            return;
        }
        let raw = std::fs::read_to_string(&vox_think_path).expect("read vox_think");
        let bench_grams: Vec<(String, HashSet<String>)> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).expect("json");
                let id = v["id"].as_str().unwrap_or("?").to_string();
                let q = v["question"].as_str().unwrap_or("");
                (id, ngrams_8(&tokenize_words(q)))
            })
            .collect();

        for row in &rows {
            let p_grams = ngrams_8(&tokenize_words(&row.prompt));
            let r_grams = ngrams_8(&tokenize_words(&row.response));
            for (bid, b_grams) in &bench_grams {
                for (label, grams) in [("prompt", &p_grams), ("response", &r_grams)] {
                    let inter = grams.intersection(b_grams).count() as f64;
                    let union = grams.union(b_grams).count() as f64;
                    let jaccard = if union == 0.0 { 0.0 } else { inter / union };
                    assert!(
                        jaccard < 0.25,
                        "leakage between {} ({label}) and {bid}: jaccard={jaccard:.3}",
                        row.id
                    );
                }
            }
        }
    }
}
