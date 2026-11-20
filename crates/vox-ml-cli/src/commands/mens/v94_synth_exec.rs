//! Deterministic generator + parallel execution oracle verifier for ~2,100+
//! genuinely distinct, idiomatic `.vox` tasks across 10 algorithmic & domain families.

use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::path::Path;

use super::super::metrics::verify_completion_with_harness;

pub(super) struct SynthExecRow {
    pub category: String,
    pub prompt: String,
    pub code: String,
    pub source: String,
}

#[derive(serde::Serialize)]
pub(super) struct SynthExecStats {
    pub generated: usize,
    pub verified: usize,
    pub failed: usize,
    pub by_category: BTreeMap<String, usize>,
}

struct SynthCand {
    id: String,
    category: String,
    prompt: String,
    code: String,
    harness: String,
    expected: Vec<String>,
    source: String,
}

struct TaskBuilder {
    items: Vec<SynthCand>,
}

impl TaskBuilder {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn push(
        &mut self,
        id: &str,
        category: &str,
        prose: &str,
        signature: &str,
        code: String,
        asserts: &[String],
        expected: Vec<String>,
    ) {
        let idx = self.items.len();
        let prompt = format_prompt(idx, prose, signature);
        let mut harness = String::from("fn __vox_eval_harness() to str {\n");
        for a in asserts {
            harness.push_str("    assert(");
            harness.push_str(a);
            harness.push_str(")\n");
        }
        harness.push_str("    return \"ok\"\n}");
        self.items.push(SynthCand {
            id: id.to_string(),
            category: category.to_string(),
            prompt,
            code,
            harness,
            expected,
            source: format!("v94_synth_exec/{category}/{id}.vox"),
        });
    }
}

fn format_prompt(idx: usize, prose: &str, signature: &str) -> String {
    if idx % 2 == 0 {
        let lead = match (idx / 2) % 4 {
            0 => "Implement in Vox:",
            1 => "Write a self-contained Vox solution:",
            2 => "Create clean Vox code meeting this specification:",
            _ => "Write a Vox function/module for the following task:",
        };
        format!("{lead}\n\n{prose}\n\nRequired signature:\n{signature}")
    } else {
        let sig_summary = signature
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("fn ")
                    || t.starts_with("query ")
                    || t.starts_with("mutation ")
                    || t.starts_with("type ")
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "{prose}\n\nName and types to use in Vox: {sig_summary}.\nReturn only valid Vox code (no prose or markdown)."
        )
    }
}

fn fmt_int_list(xs: &[i64]) -> String {
    let parts: Vec<String> = xs.iter().map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

fn fmt_str_list(xs: &[&str]) -> String {
    let parts: Vec<String> = xs.iter().map(|s| fmt_str_lit(s)).collect();
    format!("[{}]", parts.join(", "))
}

fn fmt_matrix(m: &[Vec<i64>]) -> String {
    let parts: Vec<String> = m.iter().map(|r| fmt_int_list(r)).collect();
    format!("[{}]", parts.join(", "))
}

fn fmt_str_lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(super) fn generate_and_verify_synth_rows() -> Result<(Vec<SynthExecRow>, SynthExecStats)> {
    let mut tb = TaskBuilder::new();
    gen_math_modular(&mut tb);
    gen_list_window(&mut tb);
    gen_sorting_intervals(&mut tb);
    gen_adt_match(&mut tb);
    gen_state_machine(&mut tb);
    gen_string_parsing(&mut tb);
    gen_matrix_grid(&mut tb);
    gen_data_structures(&mut tb);
    gen_multi_fn_module(&mut tb);
    gen_endpoint_handler(&mut tb);

    let generated = tb.items.len();
    verify_all_parallel(tb.items, generated)
}

fn verify_all_parallel(
    items: Vec<SynthCand>,
    generated: usize,
) -> Result<(Vec<SynthExecRow>, SynthExecStats)> {
    let workers = 8usize;
    let total = items.len();
    let chunk_size = total.div_ceil(workers).max(1);
    let mut indexed: Vec<(usize, SynthCand)> = items.into_iter().enumerate().collect();
    let mut shards: Vec<Vec<(usize, SynthCand)>> = Vec::with_capacity(workers);
    while !indexed.is_empty() {
        let take = chunk_size.min(indexed.len());
        let tail = indexed.split_off(take);
        shards.push(indexed);
        indexed = tail;
    }

    let mut handles = Vec::with_capacity(shards.len());
    for shard in shards {
        let h = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(
                move || -> Result<(Vec<(usize, SynthExecRow)>, Vec<(String, String)>)> {
                    let tmp = tempfile::tempdir()?;
                    verify_shard(shard, tmp.path())
                },
            )?;
        handles.push(h);
    }

    let mut ok_rows: Vec<(usize, SynthExecRow)> = Vec::with_capacity(total);
    let mut failures: Vec<(String, String)> = Vec::new();
    for h in handles {
        let (rows, errs) = h
            .join()
            .map_err(|_| anyhow::anyhow!("synth_exec worker thread panicked"))??;
        ok_rows.extend(rows);
        failures.extend(errs);
    }

    ok_rows.sort_by_key(|(idx, _)| *idx);
    let mut by_category: BTreeMap<String, usize> = BTreeMap::new();
    let mut rows = Vec::with_capacity(ok_rows.len());
    for (_, r) in ok_rows {
        *by_category.entry(r.category.clone()).or_default() += 1;
        rows.push(r);
    }

    let failed = failures.len();
    for (id, diag) in failures.iter().take(12) {
        eprintln!("SYNTH_EXEC_FAIL [{id}]: {diag}");
    }
    let stats = SynthExecStats {
        generated,
        verified: rows.len(),
        failed,
        by_category,
    };
    if rows.len() < 1800 || failed * 50 > generated {
        bail!(
            "synth_exec verification below bar: verified={} generated={} failed={failed}",
            rows.len(),
            generated
        );
    }
    Ok((rows, stats))
}

fn verify_shard(
    shard: Vec<(usize, SynthCand)>,
    root: &Path,
) -> Result<(Vec<(usize, SynthExecRow)>, Vec<(String, String)>)> {
    let mut ok = Vec::with_capacity(shard.len());
    let mut errs = Vec::new();
    for (idx, cand) in shard {
        let v = verify_completion_with_harness(
            &cand.code,
            root,
            "",
            &cand.id,
            idx,
            &cand.expected,
            Some(&cand.harness),
        );
        if v.pass && v.pass_compile && v.pass_exec == Some(true) && v.anti_stub_pass {
            ok.push((
                idx,
                SynthExecRow {
                    category: cand.category,
                    prompt: cand.prompt,
                    code: cand.code,
                    source: cand.source,
                },
            ));
        } else {
            errs.push((cand.id, v.checks.to_string()));
        }
    }
    Ok((ok, errs))
}

// ============================================================================
// Family 1: math_modular (~220 tasks)
// ============================================================================

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn mod_pow_i64(mut base: i64, mut exp: i64, modulus: i64) -> i64 {
    if modulus <= 1 {
        return 0;
    }
    let mut acc = 1i64;
    base = ((base % modulus) + modulus) % modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            acc = (acc * base) % modulus;
        }
        base = (base * base) % modulus;
        exp /= 2;
    }
    acc
}

fn gen_math_modular(tb: &mut TaskBuilder) {
    // 1A: 30 coprime / gcd-threshold / lcm-aggregate tasks
    let coprime_specs: &[(&str, i64, i64)] = &[
        ("beacon_coprime_count", 1, 1),
        ("orbital_sync_lcm_sum", 2, 1),
        ("cadence_gcd_max", 3, 2),
        ("pulse_gcd_filter_sum", 2, 3),
        ("slot_coprime_weight", 4, 1),
        ("phase_gcd_even_count", 2, 2),
        ("ring_lcm_bounded_count", 3, 4),
        ("tick_gcd_excess_sum", 1, 5),
        ("sector_coprime_product_mod", 5, 1),
        ("frame_gcd_threshold_hits", 4, 3),
        ("harmonic_lcm_floor_sum", 6, 2),
        ("grid_ratio_gcd_total", 7, 1),
        ("channel_coprime_odd_sum", 3, 3),
        ("drift_gcd_pair_score", 5, 2),
        ("valve_lcm_cap_sum", 8, 3),
    ];
    for &(base_name, k1, k2) in coprime_specs {
        for variant in 0..2 {
            let fname = format!("{base_name}_v{}", variant + 1);
            let mult = k1 + variant as i64 * 2;
            let bias = k2 + variant as i64;
            let sig = format!("fn {fname}(a: int, b: int, limit: int) to int");
            let prose = format!(
                "Implement `{fname}(a, b, limit)` iterating `i` from `1` through `limit` (inclusive), finding `g = gcd(a, i)` and adding `(b * mult + i) if g <= bias else (g + mult)` using `mult={mult}`, `bias={bias}` (or `0` when `limit <= 0`)."
            );
            let code = format!(
                "fn {fname}(a: int, b: int, limit: int) to int {{\n    if limit <= 0 {{ return 0 }}\n    let mut total: int = 0\n    let mut i: int = 1\n    while i <= limit {{\n        let mut x: int = a\n        if x < 0 {{ x = 0 - x }}\n        let mut y: int = i\n        while y != 0 {{\n            let rem: int = x % y\n            x = y\n            y = rem\n        }}\n        if x <= {bias} {{\n            total = total + b * {mult} + i\n        }} else {{\n            total = total + x + {mult}\n        }}\n        i = i + 1\n    }}\n    return total\n}}"
            );
            let eval = |a: i64, b: i64, limit: i64| -> i64 {
                if limit <= 0 {
                    return 0;
                }
                let mut total = 0i64;
                for i in 1..=limit {
                    let g = gcd_i64(a, i);
                    if g <= bias {
                        total += b * mult + i;
                    } else {
                        total += g + mult;
                    }
                }
                total
            };
            let asserts = vec![
                format!("{fname}(12, 3, 6) == {}", eval(12, 3, 6)),
                format!("{fname}(15, 2, 10) == {}", eval(15, 2, 10)),
                format!("{fname}(-9, 4, 0) == {}", eval(-9, 4, 0)),
            ];
            tb.push(
                &format!("math_{fname}"),
                "math_modular",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        }
    }

    // 1B: 30 modular exponentiation & LCG sequence tasks
    let mod_specs: &[(&str, i64, i64)] = &[
        ("cipher_orbit_mod_sum", 97, 3),
        ("token_digest_step_mod", 101, 5),
        ("shard_hash_ring_eval", 131, 7),
        ("epoch_nonce_fold", 199, 11),
        ("beacon_exp_series", 251, 2),
        ("nonce_chain_acc", 307, 13),
        ("ledger_mod_kernel", 401, 17),
        ("gate_pow_residue", 503, 19),
        ("packet_poly_mod", 607, 4),
        ("salt_lcg_checksum", 701, 9),
        ("epoch_power_ladder", 809, 6),
        ("shard_residue_orbit", 907, 12),
        ("frame_seed_mix", 997, 15),
        ("ring_pow_trace", 1009, 8),
        ("wire_mod_accum", 1021, 14),
    ];
    for &(base_name, m_val, stride) in mod_specs {
        for v in 0..2 {
            let fname = format!("{base_name}_r{}", v + 1);
            let m = m_val + v as i64 * 12;
            let inc = stride + v as i64 * 3;
            let sig = format!("fn {fname}(seed: int, steps: int) to int");
            let prose = format!(
                "Implement `{fname}(seed, steps)` computing `acc` summed across `step = 0..steps-1` where state evolves via `(state * state + {inc}) % {m}` for each step starting at `((seed % {m}) + {m}) % {m}`, returning `0` when `steps <= 0`."
            );
            let code = format!(
                "fn {fname}(seed: int, steps: int) to int {{\n    if steps <= 0 {{ return 0 }}\n    let mut cur: int = ((seed % {m}) + {m}) % {m}\n    let mut total: int = 0\n    let mut step: int = 0\n    while step < steps {{\n        cur = (cur * cur + {inc}) % {m}\n        total = (total + cur) % {m}\n        step = step + 1\n    }}\n    return total\n}}"
            );
            let eval = |seed: i64, steps: i64| -> i64 {
                if steps <= 0 {
                    return 0;
                }
                let mut cur = ((seed % m) + m) % m;
                let mut total = 0i64;
                for _ in 0..steps {
                    cur = (cur * cur + inc) % m;
                    total = (total + cur) % m;
                }
                total
            };
            let asserts = vec![
                format!("{fname}(5, 6) == {}", eval(5, 6)),
                format!("{fname}(-3, 4) == {}", eval(-3, 4)),
                format!("{fname}(12, 0) == 0"),
            ];
            tb.push(
                &format!("math_{fname}"),
                "math_modular",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        }
    }

    // 1C: 40 filtered divisor / prime / totient arithmetic tasks
    for idx in 0..40 {
        let div_a = 3 + (idx % 5) as i64;
        let div_b = 4 + ((idx / 5) % 4) as i64;
        let weight = 2 + (idx % 3) as i64;
        let mode = idx % 2;
        let fname = format!("arith_filter_{}_{}_{}", div_a, div_b, idx);
        let sig = format!("fn {fname}(lo: int, hi: int) to int");
        let (prose, code, ans_fn): (String, String, Box<dyn Fn(i64, i64) -> i64>) = if mode == 0 {
            let p = format!(
                "Implement `{fname}(lo, hi)` counting the weighted sum of integers `x` in `[lo, hi]` divisible by `{div_a}` or `{div_b}`, adding `{weight} * x` when divisible by both and `x` when divisible by one (`0` if `lo > hi`)."
            );
            let c = format!(
                "fn {fname}(lo: int, hi: int) to int {{\n    if lo > hi {{ return 0 }}\n    let mut acc: int = 0\n    let mut x: int = lo\n    while x <= hi {{\n        let a_ok: bool = x % {div_a} == 0\n        let b_ok: bool = x % {div_b} == 0\n        if a_ok and b_ok {{\n            acc = acc + x * {weight}\n        }} else if a_ok or b_ok {{\n            acc = acc + x\n        }}\n        x = x + 1\n    }}\n    return acc\n}}"
            );
            let f = Box::new(move |lo: i64, hi: i64| -> i64 {
                if lo > hi {
                    return 0;
                }
                let mut acc = 0i64;
                for x in lo..=hi {
                    let a = x % div_a == 0;
                    let b = x % div_b == 0;
                    if a && b {
                        acc += x * weight;
                    } else if a || b {
                        acc += x;
                    }
                }
                acc
            });
            (p, c, f)
        } else {
            let p = format!(
                "Implement `{fname}(lo, hi)` summing the count of proper divisors strictly below `n` (with each factor `d` >= 2 dividing `n`) across integers `n` in `[lo, hi]` filtered by `n % {div_a} != 0` (`lo >= 2`)."
            );
            let c = format!(
                "fn {fname}(lo: int, hi: int) to int {{\n    if lo > hi {{ return 0 }}\n    let mut total: int = 0\n    let mut n: int = lo\n    if n < 2 {{ n = 2 }}\n    while n <= hi {{\n        if n % {div_a} != 0 {{\n            let mut d: int = 2\n            while d * d <= n {{\n                if n % d == 0 {{\n                    total = total + 1\n                    if d * d != n {{ total = total + 1 }}\n                }}\n                d = d + 1\n            }}\n        }}\n        n = n + 1\n    }}\n    return total\n}}"
            );
            let f = Box::new(move |lo: i64, hi: i64| -> i64 {
                if lo > hi {
                    return 0;
                }
                let mut total = 0i64;
                let mut n = lo.max(2);
                while n <= hi {
                    if n % div_a != 0 {
                        let mut d = 2i64;
                        while d * d <= n {
                            if n % d == 0 {
                                total += 1;
                                if d * d != n {
                                    total += 1;
                                }
                            }
                            d += 1;
                        }
                    }
                    n += 1;
                }
                total
            });
            (p, c, f)
        };
        let asserts = vec![
            format!("{fname}(1, 18) == {}", ans_fn(1, 18)),
            format!("{fname}(6, 30) == {}", ans_fn(6, 30)),
            format!("{fname}(12, 5) == {}", ans_fn(12, 5)),
        ];
        tb.push(
            &format!("math_{fname}"),
            "math_modular",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 1D: 40 base-k digit checksum / palindrome / reversal tasks
    let bases = [3i64, 5, 6, 7, 8, 9, 10, 11, 12, 16];
    for idx in 0..40 {
        let base = bases[idx % bases.len()];
        let mode = idx / bases.len(); // 0..4
        let fname = format!("base{base}_digit_op_{mode}_{idx}");
        let sig = format!("fn {fname}(n: int) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(i64) -> i64>) = match mode {
            0 => {
                let p = format!(
                    "Implement `{fname}(n)` summing squares of digits of `|n|` when expressed in base `{base}`."
                );
                let c = format!(
                    "fn {fname}(n: int) to int {{\n    let mut x: int = n\n    if x < 0 {{ x = 0 - x }}\n    if x == 0 {{ return 0 }}\n    let mut s: int = 0\n    while x > 0 {{\n        let d: int = x % {base}\n        s = s + d * d\n        x = x / {base}\n    }}\n    return s\n}}"
                );
                let f = Box::new(move |n: i64| -> i64 {
                    let mut x = n.abs();
                    let mut s = 0;
                    while x > 0 {
                        let d = x % base;
                        s += d * d;
                        x /= base;
                    }
                    s
                });
                (p, c, f)
            }
            1 => {
                let p = format!(
                    "Implement `{fname}(n)` reversing the base-`{base}` digits of `n` (returning `0` for `n <= 0`)."
                );
                let c = format!(
                    "fn {fname}(n: int) to int {{\n    if n <= 0 {{ return 0 }}\n    let mut x: int = n\n    let mut rev: int = 0\n    while x > 0 {{\n        rev = rev * {base} + (x % {base})\n        x = x / {base}\n    }}\n    return rev\n}}"
                );
                let f = Box::new(move |n: i64| -> i64 {
                    if n <= 0 {
                        return 0;
                    }
                    let mut x = n;
                    let mut rev = 0;
                    while x > 0 {
                        rev = rev * base + (x % base);
                        x /= base;
                    }
                    rev
                });
                (p, c, f)
            }
            2 => {
                let p = format!(
                    "Implement `{fname}(n)` computing the multiplicative weight `sum((pos + 1) * digit)` over base-`{base}` digits of `|n|` from least-significant (`pos=0`)."
                );
                let c = format!(
                    "fn {fname}(n: int) to int {{\n    let mut x: int = n\n    if x < 0 {{ x = 0 - x }}\n    let mut pos: int = 1\n    let mut acc: int = 0\n    while x > 0 {{\n        acc = acc + pos * (x % {base})\n        x = x / {base}\n        pos = pos + 1\n    }}\n    return acc\n}}"
                );
                let f = Box::new(move |n: i64| -> i64 {
                    let mut x = n.abs();
                    let mut pos = 1;
                    let mut acc = 0;
                    while x > 0 {
                        acc += pos * (x % base);
                        x /= base;
                        pos += 1;
                    }
                    acc
                });
                (p, c, f)
            }
            _ => {
                let p = format!(
                    "Implement `{fname}(n)` returning the maximum digit in the base-`{base}` expansion of `|n|`."
                );
                let c = format!(
                    "fn {fname}(n: int) to int {{\n    let mut x: int = n\n    if x < 0 {{ x = 0 - x }}\n    let mut best: int = 0\n    while x > 0 {{\n        let d: int = x % {base}\n        if d > best {{ best = d }}\n        x = x / {base}\n    }}\n    return best\n}}"
                );
                let f = Box::new(move |n: i64| -> i64 {
                    let mut x = n.abs();
                    let mut best = 0;
                    while x > 0 {
                        let d = x % base;
                        if d > best {
                            best = d;
                        }
                        x /= base;
                    }
                    best
                });
                (p, c, f)
            }
        };
        let asserts = vec![
            format!("{fname}(0) == {}", eval(0)),
            format!("{fname}(1234) == {}", eval(1234)),
            format!("{fname}(49) == {}", eval(49)),
        ];
        tb.push(
            &format!("math_{fname}"),
            "math_modular",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 1E: 40 piecewise tariff / quota bracket functions
    for idx in 0..40 {
        let t1 = 100 + (idx as i64 % 5) * 50;
        let t2 = t1 + 200 + (idx as i64 % 4) * 100;
        let r1 = 2 + (idx as i64 % 4);
        let r2 = r1 + 3 + ((idx / 4) as i64 % 3);
        let r3 = r2 + 4;
        let base_fee = 25 + (idx as i64 % 6) * 15;
        let fname = format!("tariff_tier_{idx}_calc");
        let sig = format!("fn {fname}(units: int) to int");
        let prose = format!(
            "Implement `{fname}(units)` returning integer cents for `units` consumed (`0` if `units <= 0`): base fee `{base_fee}` plus `{r1}`/unit for `1..={t1}`, `{r2}`/unit for `{t1}+1..={t2}`, and `{r3}`/unit beyond `{t2}`."
        );
        let code = format!(
            "fn {fname}(units: int) to int {{\n    if units <= 0 {{ return 0 }}\n    let mut cost: int = {base_fee}\n    if units <= {t1} {{\n        return cost + units * {r1}\n    }}\n    cost = cost + {t1} * {r1}\n    if units <= {t2} {{\n        return cost + (units - {t1}) * {r2}\n    }}\n    return cost + ({t2} - {t1}) * {r2} + (units - {t2}) * {r3}\n}}"
        );
        let eval = |u: i64| -> i64 {
            if u <= 0 {
                return 0;
            }
            if u <= t1 {
                return base_fee + u * r1;
            }
            if u <= t2 {
                return base_fee + t1 * r1 + (u - t1) * r2;
            }
            base_fee + t1 * r1 + (t2 - t1) * r2 + (u - t2) * r3
        };
        let asserts = vec![
            format!("{fname}(-5) == 0"),
            format!("{fname}({}) == {}", t1, eval(t1)),
            format!("{fname}({}) == {}", t1 + 50, eval(t1 + 50)),
            format!("{fname}({}) == {}", t2 + 40, eval(t2 + 40)),
        ];
        tb.push(
            &format!("math_{fname}"),
            "math_modular",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 1F: 40 iterative recurrences & power-mod tasks
    for idx in 0..40 {
        let ca = 1 + (idx as i64 % 3);
        let cb = 1 + ((idx / 3) as i64 % 4);
        let md = 97 + (idx as i64 % 7) * 10;
        let kind = idx % 2;
        let fname = format!("seq_recur_{idx}_{ca}_{cb}");
        let sig = format!("fn {fname}(n: int) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(i64) -> i64>) = if kind == 0 {
            let p = format!(
                "Implement `{fname}(n)` computing term `a(n)` of recurrence `a(0)=1, a(1)=2, a(k) = ({ca}*a(k-1) + {cb}*a(k-2)) % {md}` (`0` for `n < 0`)."
            );
            let c = format!(
                "fn {fname}(n: int) to int {{\n    if n < 0 {{ return 0 }}\n    if n == 0 {{ return 1 }}\n    if n == 1 {{ return 2 }}\n    let mut p2: int = 1\n    let mut p1: int = 2\n    let mut i: int = 2\n    while i <= n {{\n        let nxt: int = ({ca} * p1 + {cb} * p2) % {md}\n        p2 = p1\n        p1 = nxt\n        i = i + 1\n    }}\n    return p1\n}}"
            );
            let f = Box::new(move |n: i64| -> i64 {
                if n < 0 {
                    return 0;
                }
                if n == 0 {
                    return 1;
                }
                if n == 1 {
                    return 2;
                }
                let (mut p2, mut p1) = (1i64, 2i64);
                for _ in 2..=n {
                    let nxt = (ca * p1 + cb * p2) % md;
                    p2 = p1;
                    p1 = nxt;
                }
                p1
            });
            (p, c, f)
        } else {
            let p = format!(
                "Implement `{fname}(n)` summing `(i^k) % {md}` where `k = {ca} + (i % {cb})` across `i = 1..=n` (`0` when `n <= 0`)."
            );
            let c = format!(
                "fn {fname}(n: int) to int {{\n    if n <= 0 {{ return 0 }}\n    let mut acc: int = 0\n    let mut i: int = 1\n    while i <= n {{\n        let mut b: int = i % {md}\n        let mut e: int = {ca} + (i % {cb})\n        let mut pw: int = 1\n        while e > 0 {{\n            if e % 2 == 1 {{ pw = (pw * b) % {md} }}\n            b = (b * b) % {md}\n            e = e / 2\n        }}\n        acc = (acc + pw) % {md}\n        i = i + 1\n    }}\n    return acc\n}}"
            );
            let f = Box::new(move |n: i64| -> i64 {
                if n <= 0 {
                    return 0;
                }
                let mut acc = 0i64;
                for i in 1..=n {
                    let e = ca + (i % cb);
                    acc = (acc + mod_pow_i64(i, e, md)) % md;
                }
                acc
            });
            (p, c, f)
        };
        let asserts = vec![
            format!("{fname}(0) == {}", eval(0)),
            format!("{fname}(5) == {}", eval(5)),
            format!("{fname}(12) == {}", eval(12)),
        ];
        tb.push(
            &format!("math_{fname}"),
            "math_modular",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }
}

// ============================================================================
// Family 2: list_window (~220 tasks)
// ============================================================================

fn gen_list_window(tb: &mut TaskBuilder) {
    // 2A: 45 fixed-width sliding window scans
    for idx in 0..45 {
        let mode = idx % 3;
        let k = 2 + (idx as i64 % 3);
        let thresh = 4 + (idx as i64 % 7) * 2;
        let fname = format!("win_scan_{idx}_k{k}");
        let sig = format!("fn {fname}(xs: list[int]) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[i64]) -> i64>) = match mode {
            0 => {
                let p = format!(
                    "Implement `{fname}(xs)` finding the maximum sum of any contiguous window of `{k}` elements (`0` if `xs.len() < {k}`)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    if xs.len() < {k} {{ return 0 }}\n    let mut w: int = 0\n    let mut i: int = 0\n    while i < {k} {{\n        w = w + xs[i].unwrap()\n        i = i + 1\n    }}\n    let mut best: int = w\n    while i < xs.len() {{\n        w = w + xs[i].unwrap() - xs[i - {k}].unwrap()\n        if w > best {{ best = w }}\n        i = i + 1\n    }}\n    return best\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    if xs.len() < k as usize {
                        return 0;
                    }
                    let mut w: i64 = xs[..k as usize].iter().sum();
                    let mut best = w;
                    for i in (k as usize)..xs.len() {
                        w += xs[i] - xs[i - k as usize];
                        if w > best {
                            best = w;
                        }
                    }
                    best
                });
                (p, c, f)
            }
            1 => {
                let p = format!(
                    "Implement `{fname}(xs)` counting contiguous windows of length `{k}` whose average floor (`sum / {k}`) is `>= {thresh}`."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    if xs.len() < {k} {{ return 0 }}\n    let mut w: int = 0\n    let mut i: int = 0\n    while i < {k} {{\n        w = w + xs[i].unwrap()\n        i = i + 1\n    }}\n    let mut hits: int = 0\n    if w >= {k} * {thresh} {{ hits = hits + 1 }}\n    while i < xs.len() {{\n        w = w + xs[i].unwrap() - xs[i - {k}].unwrap()\n        if w >= {k} * {thresh} {{ hits = hits + 1 }}\n        i = i + 1\n    }}\n    return hits\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    if xs.len() < k as usize {
                        return 0;
                    }
                    let mut w: i64 = xs[..k as usize].iter().sum();
                    let mut hits = if w >= k * thresh { 1 } else { 0 };
                    for i in (k as usize)..xs.len() {
                        w += xs[i] - xs[i - k as usize];
                        if w >= k * thresh {
                            hits += 1;
                        }
                    }
                    hits
                });
                (p, c, f)
            }
            _ => {
                let p = format!(
                    "Implement `{fname}(xs)` finding the maximum `(window_max - window_min)` span across all `{k}`-length contiguous windows (`0` if fewer than `{k}` elements)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    if xs.len() < {k} {{ return 0 }}\n    let mut best: int = 0\n    let mut i: int = 0\n    while i + {k} <= xs.len() {{\n        let mut lo: int = xs[i].unwrap()\n        let mut hi: int = lo\n        let mut j: int = 1\n        while j < {k} {{\n            let v: int = xs[i + j].unwrap()\n            if v < lo {{ lo = v }}\n            if v > hi {{ hi = v }}\n            j = j + 1\n        }}\n        let span: int = hi - lo\n        if i == 0 or span > best {{ best = span }}\n        i = i + 1\n    }}\n    return best\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    if xs.len() < k as usize {
                        return 0;
                    }
                    let mut best = 0i64;
                    for i in 0..=(xs.len() - k as usize) {
                        let sl = &xs[i..i + k as usize];
                        let span = sl.iter().max().unwrap() - sl.iter().min().unwrap();
                        if i == 0 || span > best {
                            best = span;
                        }
                    }
                    best
                });
                (p, c, f)
            }
        };
        let a1 = [1, 10, 3, 12, 8, 2];
        let a2 = [5, 2];
        let a3 = [-4, 15, 0, 11, 20, -2];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_int_list(&a1), eval(&a1)),
            format!("{fname}({}) == {}", fmt_int_list(&a2), eval(&a2)),
            format!("{fname}({}) == {}", fmt_int_list(&a3), eval(&a3)),
        ];
        tb.push(
            &format!("list_{fname}"),
            "list_window",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 2B: 45 contiguous runs & equilibrium index scans
    for idx in 0..45 {
        let mode = idx % 3;
        let bound = 3 + (idx as i64 % 7);
        let fname = format!("run_streak_{idx}_m{mode}");
        let sig = format!("fn {fname}(xs: list[int]) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[i64]) -> i64>) = match mode {
            0 => {
                let p = format!(
                    "Implement `{fname}(xs)` returning the length of the longest contiguous run where every element is strictly greater than `{bound}`."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    let mut best: int = 0\n    let mut cur: int = 0\n    for v in xs {{\n        if v > {bound} {{\n            cur = cur + 1\n            if cur > best {{ best = cur }}\n        }} else {{\n            cur = 0\n        }}\n    }}\n    return best\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    let (mut best, mut cur) = (0i64, 0i64);
                    for &v in xs {
                        if v > bound {
                            cur += 1;
                            best = best.max(cur);
                        } else {
                            cur = 0;
                        }
                    }
                    best
                });
                (p, c, f)
            }
            1 => {
                let p = format!(
                    "Implement `{fname}(xs)` returning the longest contiguous subarray length where adjacent elements differ by at most `{bound}` (`0` if empty)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    if xs.len() == 0 {{ return 0 }}\n    let mut best: int = 1\n    let mut cur: int = 1\n    let mut i: int = 1\n    while i < xs.len() {{\n        let mut diff: int = xs[i].unwrap() - xs[i - 1].unwrap()\n        if diff < 0 {{ diff = 0 - diff }}\n        if diff <= {bound} {{\n            cur = cur + 1\n            if cur > best {{ best = cur }}\n        }} else {{\n            cur = 1\n        }}\n        i = i + 1\n    }}\n    return best\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    if xs.is_empty() {
                        return 0;
                    }
                    let (mut best, mut cur) = (1i64, 1i64);
                    for i in 1..xs.len() {
                        if (xs[i] - xs[i - 1]).abs() <= bound {
                            cur += 1;
                            best = best.max(cur);
                        } else {
                            cur = 1;
                        }
                    }
                    best
                });
                (p, c, f)
            }
            _ => {
                let p = format!(
                    "Implement `{fname}(xs)` counting indices `i` where `sum(xs[0..i]) + {bound} == sum(xs[i+1..])`."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    let mut total: int = 0\n    for v in xs {{ total = total + v }}\n    let mut left: int = 0\n    let mut hits: int = 0\n    for v in xs {{\n        let right: int = total - left - v\n        if left + {bound} == right {{ hits = hits + 1 }}\n        left = left + v\n    }}\n    return hits\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> i64 {
                    let total: i64 = xs.iter().sum();
                    let (mut left, mut hits) = (0i64, 0i64);
                    for &v in xs {
                        if left + bound == total - left - v {
                            hits += 1;
                        }
                        left += v;
                    }
                    hits
                });
                (p, c, f)
            }
        };
        let a1 = [1, 8, 9, 10, 2, 7, 11];
        let a2 = [0, 0, bound + 1, bound + 2, -bound];
        let a3: [i64; 0] = [];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_int_list(&a1), eval(&a1)),
            format!("{fname}({}) == {}", fmt_int_list(&a2), eval(&a2)),
            format!("{fname}({}) == {}", fmt_int_list(&a3), eval(&a3)),
        ];
        tb.push(
            &format!("list_{fname}"),
            "list_window",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 2C: 65 list-returning filter/prefix/delta transforms
    for idx in 0..65 {
        let mode = idx % 5;
        let p_arg = 2 + (idx as i64 % 6);
        let fname = format!("seq_tx_{idx}_m{mode}");
        let sig = format!("fn {fname}(xs: list[int]) to list[int]");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[i64]) -> Vec<i64>>) = match mode {
            0 => {
                let p = format!(
                    "Implement `{fname}(xs)` returning a new list replacing each value with the running cumulative maximum clamped at least `-{p_arg}`."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    if xs.len() == 0 {{ return out }}\n    let mut best: int = -{p_arg}\n    for v in xs {{\n        if v > best {{ best = v }}\n        out = out.push(best)\n    }}\n    return out\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> Vec<i64> {
                    if xs.is_empty() {
                        return vec![];
                    }
                    let mut best = -p_arg;
                    xs.iter()
                        .map(|&v| {
                            if v > best {
                                best = v;
                            }
                            best
                        })
                        .collect()
                });
                (p, c, f)
            }
            1 => {
                let p = format!(
                    "Implement `{fname}(xs)` returning adjacent forward differences `xs[i+1] - xs[i] + {p_arg}` (empty if `xs.len() < 2`)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    let mut i: int = 1\n    while i < xs.len() {{\n        out = out.push(xs[i].unwrap() - xs[i - 1].unwrap() + {p_arg})\n        i = i + 1\n    }}\n    return out\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> Vec<i64> {
                    xs.windows(2).map(|w| w[1] - w[0] + p_arg).collect()
                });
                (p, c, f)
            }
            2 => {
                let p = format!(
                    "Implement `{fname}(xs)` keeping elements strictly divisible by `{p_arg}`, doubling each retained positive entry."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    for v in xs {{\n        if v % {p_arg} == 0 {{\n            if v > 0 {{ out = out.push(v * 2) }} else {{ out = out.push(v) }}\n        }}\n    }}\n    return out\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> Vec<i64> {
                    xs.iter()
                        .filter(|&&v| v % p_arg == 0)
                        .map(|&v| if v > 0 { v * 2 } else { v })
                        .collect()
                });
                (p, c, f)
            }
            3 => {
                let p = format!(
                    "Implement `{fname}(xs)` rotating `xs` left by `{p_arg}` steps modulo `xs.len()` (`[]` if empty)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let n: int = xs.len()\n    if n == 0 {{ return [] }}\n    let k: int = {p_arg} % n\n    let mut out: list[int] = []\n    let mut i: int = 0\n    while i < n {{\n        out = out.push(xs[(i + k) % n].unwrap())\n        i = i + 1\n    }}\n    return out\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> Vec<i64> {
                    let n = xs.len();
                    if n == 0 {
                        return vec![];
                    }
                    let k = (p_arg as usize) % n;
                    (0..n).map(|i| xs[(i + k) % n]).collect()
                });
                (p, c, f)
            }
            _ => {
                let p = format!(
                    "Implement `{fname}(xs)` summing non-overlapping chunks of size `{p_arg}` (including a shorter trailing chunk)."
                );
                let c = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    let mut i: int = 0\n    while i < xs.len() {{\n        let mut s: int = 0\n        let mut j: int = 0\n        while j < {p_arg} and i + j < xs.len() {{\n            s = s + xs[i + j].unwrap()\n            j = j + 1\n        }}\n        out = out.push(s)\n        i = i + {p_arg}\n    }}\n    return out\n}}"
                );
                let f = Box::new(move |xs: &[i64]| -> Vec<i64> {
                    xs.chunks(p_arg as usize).map(|c| c.iter().sum()).collect()
                });
                (p, c, f)
            }
        };
        let a1 = [3, -6, 9, 1, 12, -4, 6];
        let a2 = [p_arg];
        let a3: [i64; 0] = [];
        let asserts = vec![
            format!(
                "{fname}({}) == {}",
                fmt_int_list(&a1),
                fmt_int_list(&eval(&a1))
            ),
            format!(
                "{fname}({}) == {}",
                fmt_int_list(&a2),
                fmt_int_list(&eval(&a2))
            ),
            format!(
                "{fname}({}) == {}",
                fmt_int_list(&a3),
                fmt_int_list(&eval(&a3))
            ),
        ];
        tb.push(
            &format!("list_{fname}"),
            "list_window",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 2D: 65 two-list pairing & bounded subarray scans
    for idx in 0..65 {
        let bias = (idx as i64 % 9) - 3;
        let factor = 2 + (idx as i64 % 4);
        let mode = idx % 2;
        let fname = format!("pair_fold_{idx}_m{mode}");
        let sig = format!("fn {fname}(a: list[int], b: list[int]) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[i64], &[i64]) -> i64>) = if mode == 0
        {
            let p = format!(
                "Implement `{fname}(a, b)` counting pairs `(i, j)` with `a[i] + {factor} * b[j] == {bias}`."
            );
            let c = format!(
                "fn {fname}(a: list[int], b: list[int]) to int {{\n    let mut hits: int = 0\n    for x in a {{\n        for y in b {{\n            if x + {factor} * y == {bias} {{ hits = hits + 1 }}\n        }}\n    }}\n    return hits\n}}"
            );
            let f = Box::new(move |a: &[i64], b: &[i64]| -> i64 {
                let mut hits = 0i64;
                for &x in a {
                    for &y in b {
                        if x + factor * y == bias {
                            hits += 1;
                        }
                    }
                }
                hits
            });
            (p, c, f)
        } else {
            let p = format!(
                "Implement `{fname}(a, b)` computing the weighted dot product across shared length `min(a.len(), b.len())` adding `a[i]*b[i]` when `i % {factor} == 0` else `a[i] + b[i] + {bias}`."
            );
            let c = format!(
                "fn {fname}(a: list[int], b: list[int]) to int {{\n    let mut n: int = a.len()\n    if b.len() < n {{ n = b.len() }}\n    let mut acc: int = 0\n    let mut i: int = 0\n    while i < n {{\n        let x: int = a[i].unwrap()\n        let y: int = b[i].unwrap()\n        if i % {factor} == 0 {{\n            acc = acc + x * y\n        }} else {{\n            acc = acc + x + y + {bias}\n        }}\n        i = i + 1\n    }}\n    return acc\n}}"
            );
            let f = Box::new(move |a: &[i64], b: &[i64]| -> i64 {
                let n = a.len().min(b.len());
                let mut acc = 0i64;
                for i in 0..n {
                    if (i as i64) % factor == 0 {
                        acc += a[i] * b[i];
                    } else {
                        acc += a[i] + b[i] + bias;
                    }
                }
                acc
            });
            (p, c, f)
        };
        let l1a = [bias - factor, 2, bias - 2 * factor, 5];
        let l1b = [1, 2, -1];
        let l2a = [0, 3];
        let l2b = [4, 5, 6];
        let asserts = vec![
            format!(
                "{fname}({}, {}) == {}",
                fmt_int_list(&l1a),
                fmt_int_list(&l1b),
                eval(&l1a, &l1b)
            ),
            format!(
                "{fname}({}, {}) == {}",
                fmt_int_list(&l2a),
                fmt_int_list(&l2b),
                eval(&l2a, &l2b)
            ),
            format!("{fname}([], [1, 2]) == 0"),
        ];
        tb.push(
            &format!("list_{fname}"),
            "list_window",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }
}

// ============================================================================
// Family 3: sorting_intervals (~220 tasks)
// ============================================================================

fn sort_vec(mut v: Vec<i64>, desc: bool) -> Vec<i64> {
    if desc {
        v.sort_by(|a, b| b.cmp(a));
    } else {
        v.sort();
    }
    v
}

fn gen_sorting_intervals(tb: &mut TaskBuilder) {
    // 3A: 55 iterative sorting & order-statistic tasks
    for idx in 0..55 {
        let mode = idx % 3;
        let k = 1 + (idx as i64 % 3);
        let pivot = 5 + (idx as i64 % 7) * 2;
        let fname = format!("order_rank_{idx}_m{mode}");
        let sig = if mode == 2 {
            format!("fn {fname}(xs: list[int]) to int")
        } else {
            format!("fn {fname}(xs: list[int]) to list[int]")
        };
        match mode {
            0 => {
                let desc = idx % 2 == 1;
                let dir_word = if desc { "descending" } else { "ascending" };
                let cmp_op = if desc { ">" } else { "<" };
                let prose = format!(
                    "Implement `{fname}(xs)` returning `xs` sorted in `{dir_word}` order using an iterative selection pass (without mutating empty inputs)."
                );
                let code = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    for v in xs {{ out = out.push(v) }}\n    let mut res: list[int] = []\n    while out.len() > 0 {{\n        let mut best_i: int = 0\n        let mut best_v: int = out[0].unwrap()\n        let mut i: int = 1\n        while i < out.len() {{\n            let cand: int = out[i].unwrap()\n            if cand {cmp_op} best_v {{\n                best_v = cand\n                best_i = i\n            }}\n            i = i + 1\n        }}\n        res = res.push(best_v)\n        out = out.remove_at(best_i)\n    }}\n    return res\n}}"
                );
                let a1 = [9, -2, 7, 7, 1, 14];
                let a2 = [pivot, -pivot, 0];
                let asserts = vec![
                    format!(
                        "{fname}({}) == {}",
                        fmt_int_list(&a1),
                        fmt_int_list(&sort_vec(a1.to_vec(), desc))
                    ),
                    format!(
                        "{fname}({}) == {}",
                        fmt_int_list(&a2),
                        fmt_int_list(&sort_vec(a2.to_vec(), desc))
                    ),
                    format!("{fname}([]) == []"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
            1 => {
                let prose = format!(
                    "Implement `{fname}(xs)` partitioning `xs` into `[less_than_{pivot}..., ge_{pivot}...]` while preserving relative order inside each half."
                );
                let code = format!(
                    "fn {fname}(xs: list[int]) to list[int] {{\n    let mut lo: list[int] = []\n    let mut hi: list[int] = []\n    for v in xs {{\n        if v < {pivot} {{ lo = lo.push(v) }} else {{ hi = hi.push(v) }}\n    }}\n    for v in hi {{ lo = lo.push(v) }}\n    return lo\n}}"
                );
                let eval = |xs: &[i64]| -> Vec<i64> {
                    let mut lo: Vec<i64> = xs.iter().copied().filter(|&x| x < pivot).collect();
                    lo.extend(xs.iter().copied().filter(|&x| x >= pivot));
                    lo
                };
                let a1 = [pivot + 3, pivot - 1, pivot, 0, pivot + 9];
                let a2 = [pivot - 4, pivot - 2];
                let asserts = vec![
                    format!(
                        "{fname}({}) == {}",
                        fmt_int_list(&a1),
                        fmt_int_list(&eval(&a1))
                    ),
                    format!(
                        "{fname}({}) == {}",
                        fmt_int_list(&a2),
                        fmt_int_list(&eval(&a2))
                    ),
                    format!("{fname}([]) == []"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
            _ => {
                let prose = format!(
                    "Implement `{fname}(xs)` returning the sum of the `{k}` largest distinct-position values after sorting descending (`sum(all)` when `xs.len() <= {k}`)."
                );
                let code = format!(
                    "fn {fname}(xs: list[int]) to int {{\n    let mut pool: list[int] = []\n    for v in xs {{ pool = pool.push(v) }}\n    let mut sum_top: int = 0\n    let mut picks: int = 0\n    while pool.len() > 0 and picks < {k} {{\n        let mut best_i: int = 0\n        let mut best_v: int = pool[0].unwrap()\n        let mut i: int = 1\n        while i < pool.len() {{\n            if pool[i].unwrap() > best_v {{\n                best_v = pool[i].unwrap()\n                best_i = i\n            }}\n            i = i + 1\n        }}\n        sum_top = sum_top + best_v\n        pool = pool.remove_at(best_i)\n        picks = picks + 1\n    }}\n    return sum_top\n}}"
                );
                let eval = |xs: &[i64]| -> i64 {
                    let s = sort_vec(xs.to_vec(), true);
                    s.into_iter().take(k as usize).sum()
                };
                let a1 = [4, 15, 2, 19, 15, 7];
                let a2 = [8];
                let asserts = vec![
                    format!("{fname}({}) == {}", fmt_int_list(&a1), eval(&a1)),
                    format!("{fname}({}) == {}", fmt_int_list(&a2), eval(&a2)),
                    format!("{fname}([]) == 0"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
        }
    }

    // 3B: 55 two sorted lists set-algebra & k-th merge
    for idx in 0..55 {
        let mode = idx % 3;
        let offset = (idx as i64 % 5) + 1;
        let fname = format!("sorted_merge_{idx}_m{mode}");
        let sig = if mode == 2 {
            format!("fn {fname}(a: list[int], b: list[int]) to int")
        } else {
            format!("fn {fname}(a: list[int], b: list[int]) to list[int]")
        };
        match mode {
            0 => {
                let prose = format!(
                    "Implement `{fname}(a, b)` returning the sorted deduplicated union of two already-sorted lists `a` and `b`."
                );
                let code = format!(
                    "fn push_unique(acc: list[int], v: int) to list[int] {{\n    if acc.len() == 0 {{ return [v] }}\n    if acc[acc.len() - 1].unwrap() == v {{ return acc }}\n    return acc.push(v)\n}}\n\nfn {fname}(a: list[int], b: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    let mut i: int = 0\n    let mut j: int = 0\n    while i < a.len() and j < b.len() {{\n        let av: int = a[i].unwrap()\n        let bv: int = b[j].unwrap()\n        if av <= bv {{\n            out = push_unique(out, av)\n            i = i + 1\n        }} else {{\n            out = push_unique(out, bv)\n            j = j + 1\n        }}\n    }}\n    while i < a.len() {{ out = push_unique(out, a[i].unwrap())\n i = i + 1 }}\n    while j < b.len() {{ out = push_unique(out, b[j].unwrap())\n j = j + 1 }}\n    return out\n}}"
                );
                let eval = |a: &[i64], b: &[i64]| -> Vec<i64> {
                    let mut v: Vec<i64> = a.iter().chain(b.iter()).copied().collect();
                    v.sort();
                    v.dedup();
                    v
                };
                let a1 = [1, 2, 2, 5, 9];
                let b1 = [2, 3, 5, 11];
                let a2: [i64; 0] = [];
                let b2 = [4, 4, 7];
                let asserts = vec![
                    format!(
                        "{fname}({}, {}) == {}",
                        fmt_int_list(&a1),
                        fmt_int_list(&b1),
                        fmt_int_list(&eval(&a1, &b1))
                    ),
                    format!(
                        "{fname}({}, {}) == {}",
                        fmt_int_list(&a2),
                        fmt_int_list(&b2),
                        fmt_int_list(&eval(&a2, &b2))
                    ),
                    format!("{fname}([1, 2], [1, 2]) == [1, 2]"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec!["fn push_unique".to_string(), format!("fn {fname}")],
                );
            }
            1 => {
                let prose = format!(
                    "Implement `{fname}(a, b)` returning sorted unique elements present in sorted list `a` that are NOT in sorted list `b`."
                );
                let code = format!(
                    "fn {fname}(a: list[int], b: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    let mut i: int = 0\n    let mut j: int = 0\n    while i < a.len() {{\n        let av: int = a[i].unwrap()\n        while j < b.len() and b[j].unwrap() < av {{ j = j + 1 }}\n        if j == b.len() or b[j].unwrap() != av {{\n            if out.len() == 0 or out[out.len() - 1].unwrap() != av {{\n                out = out.push(av)\n            }}\n        }}\n        i = i + 1\n    }}\n    return out\n}}"
                );
                let eval = |a: &[i64], b: &[i64]| -> Vec<i64> {
                    let mut out = Vec::new();
                    for &x in a {
                        if !b.contains(&x) && out.last() != Some(&x) {
                            out.push(x);
                        }
                    }
                    out
                };
                let a1 = [1, 2, 2, 4, 7, 9];
                let b1 = [2, 5, 7];
                let asserts = vec![
                    format!(
                        "{fname}({}, {}) == {}",
                        fmt_int_list(&a1),
                        fmt_int_list(&b1),
                        fmt_int_list(&eval(&a1, &b1))
                    ),
                    format!("{fname}([3, 4], [1, 3, 4, 8]) == []"),
                    format!("{fname}([1, 5], []) == [1, 5]"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
            _ => {
                let prose = format!(
                    "Implement `{fname}(a, b)` returning the `(0`-indexed `{offset}`-th, or last if shorter) value of merging sorted lists `a` and `b` (`-1` if total elements `< {offset} + 1`)."
                );
                let code = format!(
                    "fn {fname}(a: list[int], b: list[int]) to int {{\n    if a.len() + b.len() <= {offset} {{ return -1 }}\n    let mut i: int = 0\n    let mut j: int = 0\n    let mut seen: int = 0\n    while i < a.len() or j < b.len() {{\n        let mut pick: int = 0\n        if i < a.len() and (j >= b.len() or a[i].unwrap() <= b[j].unwrap()) {{\n            pick = a[i].unwrap()\n            i = i + 1\n        }} else {{\n            pick = b[j].unwrap()\n            j = j + 1\n        }}\n        if seen == {offset} {{ return pick }}\n        seen = seen + 1\n    }}\n    return -1\n}}"
                );
                let eval = |a: &[i64], b: &[i64]| -> i64 {
                    let mut v: Vec<i64> = a.iter().chain(b.iter()).copied().collect();
                    v.sort();
                    v.get(offset as usize).copied().unwrap_or(-1)
                };
                let a1 = [1, 4, 9, 12];
                let b1 = [2, 3, 8, 15];
                let asserts = vec![
                    format!(
                        "{fname}({}, {}) == {}",
                        fmt_int_list(&a1),
                        fmt_int_list(&b1),
                        eval(&a1, &b1)
                    ),
                    format!("{fname}([10], [20]) == {}", eval(&[10], &[20])),
                    format!(
                        "{fname}([], [1, 2, 3, 4, 5, 6]) == {}",
                        eval(&[], &[1, 2, 3, 4, 5, 6])
                    ),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
        }
    }

    // 3C: 55 interval tasks (sorted/unsorted list[list[int]] [start, end])
    for idx in 0..55 {
        let mode = idx % 3;
        let point_q = 4 + (idx as i64 % 9);
        let fname = format!("span_ops_{idx}_m{mode}");
        let sig = if mode == 0 {
            format!("fn {fname}(spans: list[list[int]]) to int")
        } else {
            format!("fn {fname}(spans: list[list[int]], x: int) to int")
        };
        match mode {
            0 => {
                let prose = format!(
                    "Implement `{fname}(spans)` merging overlapping `[start, end]` integer intervals (inclusive endpoints with length `end - start`, `start <= end`) and returning total covered integer length (`{point_q}` bonus per merged island)."
                );
                let code = format!(
                    "fn sort_spans(spans: list[list[int]]) to list[list[int]] {{\n    let mut pool: list[list[int]] = []\n    for s in spans {{ pool = pool.push(s) }}\n    let mut out: list[list[int]] = []\n    while pool.len() > 0 {{\n        let mut best_i: int = 0\n        let mut best_s: int = pool[0].unwrap()[0].unwrap()\n        let mut best_e: int = pool[0].unwrap()[1].unwrap()\n        let mut i: int = 1\n        while i < pool.len() {{\n            let s: int = pool[i].unwrap()[0].unwrap()\n            let e: int = pool[i].unwrap()[1].unwrap()\n            if s < best_s or (s == best_s and e < best_e) {{\n                best_s = s\n                best_e = e\n                best_i = i\n            }}\n            i = i + 1\n        }}\n        out = out.push(pool[best_i].unwrap())\n        pool = pool.remove_at(best_i)\n    }}\n    return out\n}}\n\nfn {fname}(spans: list[list[int]]) to int {{\n    if spans.len() == 0 {{ return 0 }}\n    let sorted: list[list[int]] = sort_spans(spans)\n    let mut cs: int = sorted[0].unwrap()[0].unwrap()\n    let mut ce: int = sorted[0].unwrap()[1].unwrap()\n    let mut total: int = {point_q}\n    let mut i: int = 1\n    while i < sorted.len() {{\n        let ns: int = sorted[i].unwrap()[0].unwrap()\n        let ne: int = sorted[i].unwrap()[1].unwrap()\n        if ns <= ce {{\n            if ne > ce {{ ce = ne }}\n        }} else {{\n            total = total + (ce - cs) + {point_q}\n            cs = ns\n            ce = ne\n        }}\n        i = i + 1\n    }}\n    return total + (ce - cs)\n}}"
                );
                let eval = |spans: &[[i64; 2]]| -> i64 {
                    if spans.is_empty() {
                        return 0;
                    }
                    let mut s = spans.to_vec();
                    s.sort();
                    let (mut cs, mut ce) = (s[0][0], s[0][1]);
                    let mut total = point_q;
                    for &[ns, ne] in &s[1..] {
                        if ns <= ce {
                            ce = ce.max(ne);
                        } else {
                            total += (ce - cs) + point_q;
                            cs = ns;
                            ce = ne;
                        }
                    }
                    total + (ce - cs)
                };
                let s1 = [[5, 8], [1, 4], [3, 6], [12, 15]];
                let s2 = [[0, 2], [4, 7]];
                let m1: Vec<Vec<i64>> = s1.iter().map(|r| r.to_vec()).collect();
                let m2: Vec<Vec<i64>> = s2.iter().map(|r| r.to_vec()).collect();
                let asserts = vec![
                    format!("{fname}({}) == {}", fmt_matrix(&m1), eval(&s1)),
                    format!("{fname}({}) == {}", fmt_matrix(&m2), eval(&s2)),
                    format!("{fname}([]) == 0"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec!["fn sort_spans".to_string(), format!("fn {fname}")],
                );
            }
            1 => {
                let prose = format!(
                    "Implement `{fname}(spans, x)` counting how many intervals `[s, e]` contain `x` (inclusive) with length at least `e - s >= {point_q}`."
                );
                let code = format!(
                    "fn {fname}(spans: list[list[int]], x: int) to int {{\n    let mut count: int = 0\n    for sp in spans {{\n        let s: int = sp[0].unwrap()\n        let e: int = sp[1].unwrap()\n        if x >= s and x <= e and (e - s) >= {point_q} {{\n            count = count + 1\n        }}\n    }}\n    return count\n}}"
                );
                let eval = |spans: &[[i64; 2]], x: i64| -> i64 {
                    spans
                        .iter()
                        .filter(|[s, e]| x >= *s && x <= *e && (e - s) >= point_q)
                        .count() as i64
                };
                let s1 = [[0, 12], [3, 5], [2, 2 + point_q], [6, 20]];
                let m1: Vec<Vec<i64>> = s1.iter().map(|r| r.to_vec()).collect();
                let asserts = vec![
                    format!("{fname}({}, 4) == {}", fmt_matrix(&m1), eval(&s1, 4)),
                    format!("{fname}({}, 10) == {}", fmt_matrix(&m1), eval(&s1, 10)),
                    format!("{fname}([], 5) == 0"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {fname}")],
                );
            }
            _ => {
                let prose = format!(
                    "Implement `{fname}(spans, x)` returning the total width of gaps between non-overlapping sorted spans strictly exceeding threshold `x` (starting from the span ends after sorting by start ascending, adding `(gap - x)` per qualifying gap)."
                );
                let code = format!(
                    "fn sort_by_start(spans: list[list[int]]) to list[list[int]] {{\n    let mut pool: list[list[int]] = []\n    for sp in spans {{ pool = pool.push(sp) }}\n    let mut out: list[list[int]] = []\n    while pool.len() > 0 {{\n        let mut bi: int = 0\n        let mut bs: int = pool[0].unwrap()[0].unwrap()\n        let mut i: int = 1\n        while i < pool.len() {{\n            let s: int = pool[i].unwrap()[0].unwrap()\n            if s < bs {{ bs = s\n bi = i }}\n            i = i + 1\n        }}\n        out = out.push(pool[bi].unwrap())\n        pool = pool.remove_at(bi)\n    }}\n    return out\n}}\n\nfn {fname}(spans: list[list[int]], x: int) to int {{\n    if spans.len() < 2 {{ return 0 }}\n    let sorted: list[list[int]] = sort_by_start(spans)\n    let mut max_end: int = sorted[0].unwrap()[1].unwrap()\n    let mut excess: int = 0\n    let mut i: int = 1\n    while i < sorted.len() {{\n        let ns: int = sorted[i].unwrap()[0].unwrap()\n        let ne: int = sorted[i].unwrap()[1].unwrap()\n        if ns > max_end {{\n            let gap: int = ns - max_end\n            if gap > x {{ excess = excess + (gap - x) }}\n        }}\n        if ne > max_end {{ max_end = ne }}\n        i = i + 1\n    }}\n    return excess\n}}"
                );
                let eval = |spans: &[[i64; 2]], x: i64| -> i64 {
                    if spans.len() < 2 {
                        return 0;
                    }
                    let mut s = spans.to_vec();
                    s.sort_by_key(|a| a[0]);
                    let mut max_end = s[0][1];
                    let mut excess = 0;
                    for &[ns, ne] in &s[1..] {
                        if ns > max_end {
                            let gap = ns - max_end;
                            if gap > x {
                                excess += gap - x;
                            }
                        }
                        max_end = max_end.max(ne);
                    }
                    excess
                };
                let s1 = [[10, 14], [0, 3], [5, 7], [20, 22]];
                let m1: Vec<Vec<i64>> = s1.iter().map(|r| r.to_vec()).collect();
                let asserts = vec![
                    format!("{fname}({}, 2) == {}", fmt_matrix(&m1), eval(&s1, 2)),
                    format!("{fname}({}, 5) == {}", fmt_matrix(&m1), eval(&s1, 5)),
                    format!("{fname}([[1, 5]], 1) == 0"),
                ];
                tb.push(
                    &format!("sort_{fname}"),
                    "sorting_intervals",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec!["fn sort_by_start".to_string(), format!("fn {fname}")],
                );
            }
        }
    }

    // 3D: 55 binary search / range / monotonicity queries
    for idx in 0..55 {
        let mode = idx % 2;
        let offset = (idx as i64 % 5) * 10;
        let fname = format!("bsearch_range_{idx}_m{mode}");
        let sig = format!("fn {fname}(sorted: list[int], target: int) to int");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[i64], i64) -> i64>) = if mode == 0 {
            let p = format!(
                "Implement `{fname}(sorted, target)` returning the count of elements strictly equal to `target` in ascending `sorted` using lower/upper binary search (plus `{offset}` if count > 0)."
            );
            let c = format!(
                "fn {fname}(sorted: list[int], target: int) to int {{\n    let n: int = sorted.len()\n    let mut lo: int = 0\n    let mut hi: int = n\n    while lo < hi {{\n        let mid: int = lo + (hi - lo) / 2\n        if sorted[mid].unwrap() < target {{ lo = mid + 1 }} else {{ hi = mid }}\n    }}\n    let first: int = lo\n    lo = first\n    hi = n\n    while lo < hi {{\n        let mid: int = lo + (hi - lo) / 2\n        if sorted[mid].unwrap() <= target {{ lo = mid + 1 }} else {{ hi = mid }}\n    }}\n    let cnt: int = lo - first\n    if cnt == 0 {{ return 0 }}\n    return cnt + {offset}\n}}"
            );
            let f = Box::new(move |s: &[i64], target: i64| -> i64 {
                let cnt = s.iter().filter(|&&v| v == target).count() as i64;
                if cnt == 0 { 0 } else { cnt + offset }
            });
            (p, c, f)
        } else {
            let p = format!(
                "Implement `{fname}(sorted, target)` returning the index of the greatest element `<= target` (`-1` if all elements exceed `target` or empty), offset by `{offset}` when found."
            );
            let c = format!(
                "fn {fname}(sorted: list[int], target: int) to int {{\n    let mut lo: int = 0\n    let mut hi: int = sorted.len()\n    while lo < hi {{\n        let mid: int = lo + (hi - lo) / 2\n        if sorted[mid].unwrap() <= target {{ lo = mid + 1 }} else {{ hi = mid }}\n    }}\n    if lo == 0 {{ return -1 }}\n    return (lo - 1) + {offset}\n}}"
            );
            let f = Box::new(move |s: &[i64], target: i64| -> i64 {
                match s.iter().rposition(|&v| v <= target) {
                    Some(pos) => pos as i64 + offset,
                    None => -1,
                }
            });
            (p, c, f)
        };
        let s1 = [1, 3, 5, 5, 5, 9, 14];
        let asserts = vec![
            format!("{fname}({}, 5) == {}", fmt_int_list(&s1), eval(&s1, 5)),
            format!("{fname}({}, 0) == {}", fmt_int_list(&s1), eval(&s1, 0)),
            format!("{fname}({}, 14) == {}", fmt_int_list(&s1), eval(&s1, 14)),
        ];
        tb.push(
            &format!("sort_{fname}"),
            "sorting_intervals",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }
}

// ============================================================================
// Family 4: adt_match (~220 tasks)
// ============================================================================

fn gen_adt_match(tb: &mut TaskBuilder) {
    // Generate 55 domain ADTs x 4 behavioral rule combinations = 220 tasks
    let domains: &[(&str, &str, &str, &str)] = &[
        ("BillingUnit", "Metered", "FixedSlot", "Waived"),
        ("TelemetryFrame", "Pulse", "Burst", "Silence"),
        ("DispatchOrder", "Express", "Standard", "Hold"),
        ("CacheDirective", "WarmHit", "ColdFetch", "Purge"),
        ("SensorReading", "Nominal", "Spike", "Dropout"),
        ("RouteHop", "DirectLink", "RelayLink", "Blocked"),
        ("AccessGrant", "ReadScope", "WriteScope", "Revoked"),
        ("BudgetDelta", "Reserve", "Spend", "Refunded"),
        ("QueueTicket", "PriorityJob", "BulkJob", "Parked"),
        ("PowerMode", "TurboDraw", "EcoDraw", "Sleep"),
        ("ShaderOp", "UniformLoad", "TextureSample", "Discard"),
        ("LedgerItem", "CreditEntry", "DebitEntry", "VoidEntry"),
        ("AlertSignal", "WarnTick", "CritTick", "Cleared"),
        ("MeshPacket", "AckFrame", "DataFrame", "DropFrame"),
        ("StorageTier", "HotBlock", "ColdBlock", "Archive"),
        ("QuotaEvent", "ConsumeUnit", "Replenish", "Throttle"),
        ("RuleOutcome", "AllowRule", "LimitRule", "DenyRule"),
        ("BuildStage", "CompileUnit", "LinkUnit", "SkipUnit"),
        ("ProbeResult", "PassProbe", "FlakeProbe", "Timeout"),
        ("SyncAction", "PushDelta", "PullDelta", "NoopSync"),
        ("TraceSpan", "CpuWork", "IoWait", "IdleGap"),
        ("TariffRule", "PeakRate", "OffPeakRate", "FreeWindow"),
        ("FlightLeg", "ClimbSeg", "CruiseSeg", "HoldPattern"),
        ("ParcelClass", "AirFreight", "GroundBox", "Returned"),
        ("LockState", "SharedLock", "ExclusiveLock", "Unlocked"),
        ("AudioBand", "GainBoost", "Attenuate", "Muted"),
        ("GridTile", "RoadCost", "TollCost", "Impassable"),
        ("QueryCost", "IndexScan", "FullScan", "CachedPlan"),
        ("CheckItem", "SyntaxPass", "WarningHit", "FatalErr"),
        ("EpochEvent", "StepCommit", "BackoffStep", "AbortEpoch"),
        ("VaultOp", "SealBlob", "UnsealBlob", "ShredBlob"),
        ("TicketTier", "VipPass", "DayPass", "CompPass"),
        ("FleetTask", "TowLoad", "LiftLoad", "Standby"),
        ("ClockTick", "FastClock", "SlowClock", "PausedClock"),
        ("MailFilter", "InboxTag", "QuarantineTag", "TrashTag"),
        ("GraphEdge", "ForwardEdge", "BackEdge", "SelfLoop"),
        ("StockMove", "RestockLot", "ShipLot", "WriteOff"),
        ("EvalScore", "BonusPts", "PenaltyPts", "NeutralPts"),
        ("DiskOp", "SeqWrite", "RandSeek", "FsyncFlush"),
        ("PodEvent", "ScaleUp", "ScaleDown", "EvictPod"),
        ("DnsRecord", "CacheHit", "UpstreamHop", "NxDomain"),
        ("WebhookTry", "Delivered", "RetryAfter", "DeadLetter"),
        ("LeaseState", "Renewed", "GracePeriod", "Expired"),
        ("MetricSample", "CounterInc", "GaugeDelta", "ResetZero"),
        ("RelaySignal", "BoostAmp", "DampenAmp", "Cutoff"),
        ("TokenGrant", "MintCoin", "BurnCoin", "FreezeCoin"),
        ("SlotAlloc", "PinCore", "ShareCore", "IdleCore"),
        ("StageGate", "ApproveGate", "HoldGate", "RejectGate"),
        ("PortPolicy", "AllowPort", "RatePort", "BlockPort"),
        ("LogAction", "AppendLog", "CompactLog", "TruncateLog"),
        ("SliceRule", "HeadSlice", "TailSlice", "EmptySlice"),
        ("FrameSync", "LockFrame", "DropFrame", "BlankFrame"),
        ("StreamOp", "EmitBatch", "BufferBatch", "FlushEnd"),
        ("WorkerCmd", "RunWork", "SleepWork", "StopWork"),
        ("AuditNote", "CleanPass", "MinorFlag", "MajorFlag"),
    ];
    for (d_idx, &(ty, v1, v2, v3)) in domains.iter().enumerate() {
        for r in 0..4 {
            let w1 = 2 + ((d_idx + r) as i64 % 5);
            let w2 = 3 + ((d_idx * 3 + r) as i64 % 4);
            let penalty = 5 + (r as i64 * 4);
            let cap = 20 + (d_idx as i64 % 6) * 10;
            let ty_name = format!("{ty}{}", r + 1);
            let fn_name = format!("score_{}_{}", ty.to_lowercase(), r + 1);
            let type_def = format!(
                "type {ty_name} =\n    | {v1}(x: int)\n    | {v2}(a: int, b: int)\n    | {v3}"
            );
            let sig = format!("{type_def}\n\nfn {fn_name}(items: list[{ty_name}]) to int");
            let prose = match r % 2 {
                0 => format!(
                    "Define algebraic type `{ty_name}` (`{v1}(x: int)`, `{v2}(a: int, b: int)`, `{v3}`) and implement `{fn_name}(items)` summing contributions (`{v1}(x)` adds `x * {w1}`, `{v2}(a, b)` adds `a * b + {w2}`, `{v3}` subtracts `{penalty}` clamped at min `0`)."
                ),
                _ => format!(
                    "Define algebraic type `{ty_name}` (`{v1}(x: int)`, `{v2}(a: int, b: int)`, `{v3}`) and implement `{fn_name}(items)` accumulating a score (halting accumulation when `{v3}` is encountered): `{v1}(x)` adds `min(x, {cap}) * {w1}`, `{v2}(a, b)` adds `a * {w2} - b`."
                ),
            };
            let code = if r % 2 == 0 {
                format!(
                    "{type_def}\n\nfn {fn_name}(items: list[{ty_name}]) to int {{\n    let mut acc: int = 0\n    for it in items {{\n        match it {{\n            {v1}(x) => {{ acc = acc + x * {w1} }}\n            {v2}(a, b) => {{ acc = acc + a * b + {w2} }}\n            {v3} => {{\n                acc = acc - {penalty}\n                if acc < 0 {{ acc = 0 }}\n            }}\n        }}\n    }}\n    return acc\n}}"
                )
            } else {
                format!(
                    "{type_def}\n\nfn {fn_name}(items: list[{ty_name}]) to int {{\n    let mut acc: int = 0\n    let mut halted: bool = false\n    for it in items {{\n        if halted {{ continue }}\n        match it {{\n            {v1}(x) => {{\n                let mut ux: int = x\n                if ux > {cap} {{ ux = {cap} }}\n                acc = acc + ux * {w1}\n            }}\n            {v2}(a, b) => {{ acc = acc + a * {w2} - b }}\n            {v3} => {{ halted = true }}\n        }}\n    }}\n    return acc\n}}"
                )
            };
            let eval_fn = |seq: &[(u8, i64, i64)]| -> i64 {
                if r % 2 == 0 {
                    let mut acc = 0i64;
                    for &(tag, a, b) in seq {
                        match tag {
                            1 => acc += a * w1,
                            2 => acc += a * b + w2,
                            _ => acc = (acc - penalty).max(0),
                        }
                    }
                    acc
                } else {
                    let mut acc = 0i64;
                    for &(tag, a, b) in seq {
                        match tag {
                            1 => acc += a.min(cap) * w1,
                            2 => acc += a * w2 - b,
                            _ => break,
                        }
                    }
                    acc
                }
            };
            let s1 = [(1, 4, 0), (2, 3, 2), (3, 0, 0), (1, 5, 0)];
            let s2 = [(2, 4, 1), (1, cap + 10, 0)];
            let lit1 = format!("[{v1}(4), {v2}(3, 2), {v3}, {v1}(5)]");
            let lit2 = format!("[{v2}(4, 1), {v1}({})]", cap + 10);
            let asserts = vec![
                format!("{fn_name}({lit1}) == {}", eval_fn(&s1)),
                format!("{fn_name}({lit2}) == {}", eval_fn(&s2)),
                format!("{fn_name}([]) == 0"),
            ];
            tb.push(
                &format!("adt_{fn_name}"),
                "adt_match",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("type {ty_name}"), format!("fn {fn_name}")],
            );
        }
    }
}

// ============================================================================
// Family 5: state_machine (~220 tasks)
// ============================================================================

fn parse_signed_vox_helper() -> &'static str {
    "fn parse_int_val(s: str) to int {\n    let cs: list[str] = s.trim().chars()\n    if cs.len() == 0 { return 0 }\n    let mut sign: int = 1\n    let mut i: int = 0\n    if cs[0].unwrap() == \"-\" {\n        sign = -1\n        i = 1\n    }\n    let mut acc: int = 0\n    while i < cs.len() {\n        let d: int = cs[i].unwrap().ord() - 48\n        if d >= 0 and d <= 9 {\n            acc = acc * 10 + d\n        }\n        i = i + 1\n    }\n    return acc * sign\n}"
}

fn gen_state_machine(tb: &mut TaskBuilder) {
    // 5A: 55 Register VM / Ledger / Counter machines
    for idx in 0..55 {
        let cap = 50 + (idx as i64 % 7) * 25;
        let init_v = (idx as i64 % 5) * 3;
        let step_mul = 2 + (idx as i64 % 3);
        let fname = format!("vm_trace_{idx}_run");
        let sig = format!("fn {fname}(ops: list[str]) to int");
        let prose = format!(
            "Implement `{fname}(ops)` executing command strings starting from accumulator `{init_v}` (clamped to `[-{cap}, {cap}]` after every step): `\"add:<n>\"`, `\"sub:<n>\"`, `\"mul\"` (multiplies by `{step_mul}`), `\"reset\"` (`0`), returning the final accumulator."
        );
        let code = format!(
            "{}\n\nfn clamp_{idx}(v: int) to int {{\n    if v < -{cap} {{ return -{cap} }}\n    if v > {cap} {{ return {cap} }}\n    return v\n}}\n\nfn {fname}(ops: list[str]) to int {{\n    let mut acc: int = {init_v}\n    for op in ops {{\n        if op == \"mul\" {{\n            acc = clamp_{idx}(acc * {step_mul})\n        }} else if op == \"reset\" {{\n            acc = 0\n        }} else if op.starts_with(\"add:\") {{\n            let parts: list[str] = op.split(\":\")\n            acc = clamp_{idx}(acc + parse_int_val(parts[1].unwrap()))\n        }} else if op.starts_with(\"sub:\") {{\n            let parts: list[str] = op.split(\":\")\n            acc = clamp_{idx}(acc - parse_int_val(parts[1].unwrap()))\n        }}\n    }}\n    return acc\n}}",
            parse_signed_vox_helper()
        );
        let clamp = |v: i64| v.clamp(-cap, cap);
        let eval = |ops: &[&str]| -> i64 {
            let mut acc = init_v;
            for &op in ops {
                if op == "mul" {
                    acc = clamp(acc * step_mul);
                } else if op == "reset" {
                    acc = 0;
                } else if let Some(rest) = op.strip_prefix("add:") {
                    acc = clamp(acc + rest.parse::<i64>().unwrap_or(0));
                } else if let Some(rest) = op.strip_prefix("sub:") {
                    acc = clamp(acc - rest.parse::<i64>().unwrap_or(0));
                }
            }
            acc
        };
        let o1 = ["add:10", "mul", "sub:4"];
        let o2 = ["add:999", "sub:15", "reset", "add:-5", "mul"];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_list(&o1), eval(&o1)),
            format!("{fname}({}) == {}", fmt_str_list(&o2), eval(&o2)),
            format!("{fname}([]) == {init_v}"),
        ];
        tb.push(
            &format!("fsm_{fname}"),
            "state_machine",
            &prose,
            &sig,
            code,
            &asserts,
            vec!["fn parse_int_val".to_string(), format!("fn {fname}")],
        );
    }

    // 5B: 55 Grid / Rover navigation state machines
    for idx in 0..55 {
        let bound = 4 + (idx as i64 % 6);
        let mode = idx % 2;
        let fname = format!("rover_nav_{idx}_b{bound}");
        let sig = format!("fn {fname}(cmds: list[str]) to int");
        let prose = format!(
            "Implement `{fname}(cmds)` tracking a bounded 2D rover on `[-{bound}, {bound}] x [-{bound}, {bound}]` starting at `(0, 0)` with commands `\"N:<d>\"`, `\"S:<d>\"`, `\"E:<d>\"`, `\"W:<d>\"`, `\"HOME\"` (reset to `0,0`), returning `{}`.",
            if mode == 0 {
                "`|x| + |y|` Manhattan distance from origin"
            } else {
                "total clamped coordinate moves accepted (`|dx| + |dy|`) before stopping"
            }
        );
        let code = format!(
            "{}\n\nfn bound_{idx}(v: int) to int {{\n    if v < -{bound} {{ return -{bound} }}\n    if v > {bound} {{ return {bound} }}\n    return v\n}}\n\nfn {fname}(cmds: list[str]) to int {{\n    let mut x: int = 0\n    let mut y: int = 0\n    let mut moved: int = 0\n    for c in cmds {{\n        if c == \"HOME\" {{\n            x = 0\n            y = 0\n        }} else {{\n            let ps: list[str] = c.split(\":\")\n            let d: int = parse_int_val(ps[1].unwrap())\n            let dir: str = ps[0].unwrap()\n            let mut nx: int = x\n            let mut ny: int = y\n            if dir == \"N\" {{ ny = bound_{idx}(y + d) }}\n            if dir == \"S\" {{ ny = bound_{idx}(y - d) }}\n            if dir == \"E\" {{ nx = bound_{idx}(x + d) }}\n            if dir == \"W\" {{ nx = bound_{idx}(x - d) }}\n            let mut dx: int = nx - x\n            if dx < 0 {{ dx = 0 - dx }}\n            let mut dy: int = ny - y\n            if dy < 0 {{ dy = 0 - dy }}\n            moved = moved + dx + dy\n            x = nx\n            y = ny\n        }}\n    }}\n    if {mode} == 0 {{\n        let mut ax: int = x\n        if ax < 0 {{ ax = 0 - ax }}\n        let mut ay: int = y\n        if ay < 0 {{ ay = 0 - ay }}\n        return ax + ay\n    }}\n    return moved\n}}",
            parse_signed_vox_helper()
        );
        let eval = |cmds: &[&str]| -> i64 {
            let (mut x, mut y, mut moved) = (0i64, 0i64, 0i64);
            for &c in cmds {
                if c == "HOME" {
                    x = 0;
                    y = 0;
                } else {
                    let mut sp = c.split(':');
                    let dir = sp.next().unwrap_or("");
                    let d = sp.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
                    let (mut nx, mut ny) = (x, y);
                    match dir {
                        "N" => ny = (y + d).clamp(-bound, bound),
                        "S" => ny = (y - d).clamp(-bound, bound),
                        "E" => nx = (x + d).clamp(-bound, bound),
                        "W" => nx = (x - d).clamp(-bound, bound),
                        _ => {}
                    }
                    moved += (nx - x).abs() + (ny - y).abs();
                    x = nx;
                    y = ny;
                }
            }
            if mode == 0 { x.abs() + y.abs() } else { moved }
        };
        let c1 = ["N:3", "E:10", "S:1", "W:2"];
        let c2 = ["E:2", "HOME", "S:9"];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_list(&c1), eval(&c1)),
            format!("{fname}({}) == {}", fmt_str_list(&c2), eval(&c2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("fsm_{fname}"),
            "state_machine",
            &prose,
            &sig,
            code,
            &asserts,
            vec!["fn parse_int_val".to_string(), format!("fn {fname}")],
        );
    }

    // 5C: 55 Thermostat / Lock / Alarm FSM
    for idx in 0..55 {
        let lo = 15 + (idx as i64 % 5) * 2;
        let hi = lo + 4 + ((idx / 5) as i64 % 4) * 2;
        let fname = format!("hvac_fsm_{idx}_band");
        let sig = format!("fn {fname}(temps: list[int]) to int");
        let prose = format!(
            "Implement `{fname}(temps)` simulating a stateful HVAC controller initially OFF (`0`): switch ON (`1`) when reading `<= {lo}` (if heating) or `>= {hi}`? Specifically: start state `0` (idle); enter heating (`1`) when `t < {lo}` and stay heating until `t >= {hi}`; count total steps spent in state `1`."
        );
        let code = format!(
            "fn {fname}(temps: list[int]) to int {{\n    let mut active: bool = false\n    let mut on_steps: int = 0\n    for t in temps {{\n        if not active and t < {lo} {{\n            active = true\n        }} else if active and t >= {hi} {{\n            active = false\n        }}\n        if active {{ on_steps = on_steps + 1 }}\n    }}\n    return on_steps\n}}"
        );
        let eval = |temps: &[i64]| -> i64 {
            let mut active = false;
            let mut on_steps = 0;
            for &t in temps {
                if !active && t < lo {
                    active = true;
                } else if active && t >= hi {
                    active = false;
                }
                if active {
                    on_steps += 1;
                }
            }
            on_steps
        };
        let t1 = [lo + 2, lo - 1, lo + 1, hi - 1, hi, lo];
        let t2 = [hi + 3, lo - 2, lo + 1];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_int_list(&t1), eval(&t1)),
            format!("{fname}({}) == {}", fmt_int_list(&t2), eval(&t2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("fsm_{fname}"),
            "state_machine",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 5D: 55 Door / Lock / Breaker state trackers with cooldowns
    for idx in 0..55 {
        let trip_k = 2 + (idx as i64 % 3);
        let cool_k = 2 + ((idx / 3) as i64 % 3);
        let fname = format!("lockout_fsm_{idx}_t{trip_k}");
        let sig = format!("fn {fname}(events: list[str]) to int");
        let prose = format!(
            "Implement `{fname}(events)` tracking a security gate (`\"ok\"`, `\"fail\"`, `\"tick\"`): `{trip_k}` consecutive `\"fail\"`s lock the gate for `{cool_k}` `\"tick\"`s (ignoring `\"ok\"`/`\"fail\"` while locked); count total accepted `\"ok\"` logins."
        );
        let code = format!(
            "fn {fname}(events: list[str]) to int {{\n    let mut streak: int = 0\n    let mut cooldown: int = 0\n    let mut ok_count: int = 0\n    for ev in events {{\n        if cooldown > 0 {{\n            if ev == \"tick\" {{ cooldown = cooldown - 1 }}\n        }} else if ev == \"ok\" {{\n            streak = 0\n            ok_count = ok_count + 1\n        }} else if ev == \"fail\" {{\n            streak = streak + 1\n            if streak >= {trip_k} {{\n                cooldown = {cool_k}\n                streak = 0\n            }}\n        }}\n    }}\n    return ok_count\n}}"
        );
        let eval = |evs: &[&str]| -> i64 {
            let (mut streak, mut cool, mut oks) = (0i64, 0i64, 0i64);
            for &ev in evs {
                if cool > 0 {
                    if ev == "tick" {
                        cool -= 1;
                    }
                } else if ev == "ok" {
                    streak = 0;
                    oks += 1;
                } else if ev == "fail" {
                    streak += 1;
                    if streak >= trip_k {
                        cool = cool_k;
                        streak = 0;
                    }
                }
            }
            oks
        };
        let e1 = [
            "fail", "fail", "fail", "ok", "tick", "tick", "tick", "tick", "ok",
        ];
        let e2 = ["ok", "fail", "ok", "fail", "fail", "fail", "ok"];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_list(&e1), eval(&e1)),
            format!("{fname}({}) == {}", fmt_str_list(&e2), eval(&e2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("fsm_{fname}"),
            "state_machine",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }
}

// ============================================================================
// Family 6: string_parsing (~220 tasks)
// ============================================================================

fn gen_string_parsing(tb: &mut TaskBuilder) {
    // 6A: 55 KV / delimited record parsers
    let seps = [";", "|", ",", "&", "/"];
    for idx in 0..55 {
        let sep = seps[idx % seps.len()];
        let eq = if idx % 2 == 0 { "=" } else { ":" };
        let mult = 1 + (idx as i64 % 4);
        let fname = format!("kv_sum_key_{idx}");
        let sig = format!("fn {fname}(record: str, prefix: str) to int");
        let prose = format!(
            "Implement `{fname}(record, prefix)` splitting `record` on `{sep}` into `{eq}`-separated `key{eq}int_val` pairs, summing `{mult} * int_val` for every trimmed entry whose key starts with `prefix`."
        );
        let sep_lit = fmt_str_lit(sep);
        let eq_lit = fmt_str_lit(eq);
        let code = format!(
            "{}\n\nfn {fname}(record: str, prefix: str) to int {{\n    let mut total: int = 0\n    for entry in record.split({sep_lit}) {{\n        let t: str = entry.trim()\n        if t.len() == 0 {{ continue }}\n        let kv: list[str] = t.split({eq_lit})\n        if kv.len() == 2 {{\n            let k: str = kv[0].unwrap().trim()\n            if k.starts_with(prefix) {{\n                total = total + parse_int_val(kv[1].unwrap()) * {mult}\n            }}\n        }}\n    }}\n    return total\n}}",
            parse_signed_vox_helper()
        );
        let eval = |rec: &str, pfx: &str| -> i64 {
            let mut total = 0i64;
            for entry in rec.split(sep) {
                let t = entry.trim();
                if t.is_empty() {
                    continue;
                }
                let mut kv = t.split(eq);
                if let (Some(k), Some(v), None) = (kv.next(), kv.next(), kv.next()) {
                    if k.trim().starts_with(pfx) {
                        total += v.trim().parse::<i64>().unwrap_or(0) * mult;
                    }
                }
            }
            total
        };
        let r1 = format!(" cpu{eq}12 {sep} mem{eq}-3 {sep} cpu_peak{eq}8 {sep} disk{eq}5 ");
        let r2 = format!("net{eq}4{sep}bad_entry{sep}net_tx{eq}9");
        let asserts = vec![
            format!(
                "{fname}({}, \"cpu\") == {}",
                fmt_str_lit(&r1),
                eval(&r1, "cpu")
            ),
            format!(
                "{fname}({}, \"net\") == {}",
                fmt_str_lit(&r2),
                eval(&r2, "net")
            ),
            format!("{fname}(\"\", \"cpu\") == 0"),
        ];
        tb.push(
            &format!("str_{fname}"),
            "string_parsing",
            &prose,
            &sig,
            code,
            &asserts,
            vec!["fn parse_int_val".to_string(), format!("fn {fname}")],
        );
    }

    // 6B: 55 delimiter/bracket/indent/tag balancing & depth tasks
    let pairs = [("(", ")"), ("[", "]"), ("{", "}"), ("<", ">"), ("(", "]")];
    for idx in 0..55 {
        let (open_ch, close_ch) = pairs[idx % pairs.len()];
        let bonus = (idx as i64 % 5) + 1;
        let fname = format!("delim_depth_{idx}");
        let sig = format!("fn {fname}(text: str) to int");
        let prose = format!(
            "Implement `{fname}(text)` scanning characters to measure nesting of `{open_ch}` and `{close_ch}`: return `max_depth + {bonus}` when balanced (and depth never negative), or `-1` if unbalanced."
        );
        let o_lit = fmt_str_lit(open_ch);
        let c_lit = fmt_str_lit(close_ch);
        let code = format!(
            "fn {fname}(text: str) to int {{\n    let mut depth: int = 0\n    let mut max_d: int = 0\n    for ch in text.chars() {{\n        if ch == {o_lit} {{\n            depth = depth + 1\n            if depth > max_d {{ max_d = depth }}\n        }} else if ch == {c_lit} {{\n            depth = depth - 1\n            if depth < 0 {{ return -1 }}\n        }}\n    }}\n    if depth != 0 {{ return -1 }}\n    return max_d + {bonus}\n}}"
        );
        let eval = |txt: &str| -> i64 {
            let o = open_ch.chars().next().unwrap();
            let c = close_ch.chars().next().unwrap();
            let (mut d, mut md) = (0i64, 0i64);
            for ch in txt.chars() {
                if ch == o {
                    d += 1;
                    md = md.max(d);
                } else if ch == c {
                    d -= 1;
                    if d < 0 {
                        return -1;
                    }
                }
            }
            if d != 0 { -1 } else { md + bonus }
        };
        let s1 = format!("a{open_ch}{open_ch}x{close_ch}{open_ch}y{close_ch}{close_ch}b");
        let s2 = format!("{close_ch}{open_ch}");
        let s3 = format!("{open_ch}{open_ch}");
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_lit(&s1), eval(&s1)),
            format!("{fname}({}) == -1", fmt_str_lit(&s2)),
            format!("{fname}({}) == -1", fmt_str_lit(&s3)),
            format!("{fname}(\"plain\") == {bonus}"),
        ];
        tb.push(
            &format!("str_{fname}"),
            "string_parsing",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 6C: 55 slug/kebab/title/case normalization transforms
    for idx in 0..55 {
        let delim = ["-", "_", "."][idx % 3];
        let upper_first = idx % 2 == 0;
        let fname = format!("fmt_ident_norm_{idx}");
        let sig = format!("fn {fname}(raw: str) to str");
        let d_lit = fmt_str_lit(delim);
        let prose = format!(
            "Implement `{fname}(raw)` splitting `raw` into maximal alphanumeric runs, {}joining non-empty runs with `{delim}`.",
            if upper_first {
                "uppercasing the first character of each run and lowercasing the rest, "
            } else {
                "lowercasing every run, "
            }
        );
        let word_xform = if upper_first {
            "cur.slice(0, 1).to_upper() + cur.slice(1, cur.chars_count()).to_lower()"
        } else {
            "cur.to_lower()"
        };
        let code = format!(
            "fn {fname}(raw: str) to str {{\n    let mut parts: list[str] = []\n    let mut cur: str = \"\"\n    for ch in raw.chars() {{\n        if ch.is_alnum() {{\n            cur = cur + ch\n        }} else if cur.len() > 0 {{\n            parts = parts.push({word_xform})\n            cur = \"\"\n        }}\n    }}\n    if cur.len() > 0 {{\n        parts = parts.push({word_xform})\n    }}\n    return parts.join({d_lit})\n}}"
        );
        let eval = |raw: &str| -> String {
            let mut parts: Vec<String> = Vec::new();
            let mut cur = String::new();
            for ch in raw.chars() {
                if ch.is_alphanumeric() {
                    cur.push(ch);
                } else if !cur.is_empty() {
                    parts.push(if upper_first {
                        let mut cs = cur.chars();
                        let first = cs.next().unwrap().to_uppercase().to_string();
                        let rest = cs.as_str().to_lowercase();
                        format!("{first}{rest}")
                    } else {
                        cur.to_lowercase()
                    });
                    cur.clear();
                }
            }
            if !cur.is_empty() {
                parts.push(if upper_first {
                    let mut cs = cur.chars();
                    let first = cs.next().unwrap().to_uppercase().to_string();
                    let rest = cs.as_str().to_lowercase();
                    format!("{first}{rest}")
                } else {
                    cur.to_lowercase()
                });
            }
            parts.join(delim)
        };
        let t1 = "  Alpha__Beta 42! ";
        let t2 = "foo/BAR/baz";
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_lit(t1), fmt_str_lit(&eval(t1))),
            format!("{fname}({}) == {}", fmt_str_lit(t2), fmt_str_lit(&eval(t2))),
            format!("{fname}(\"---\") == \"\""),
        ];
        tb.push(
            &format!("str_{fname}"),
            "string_parsing",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 6D: 55 numeric/hex/binary string decoders & semver pack
    for idx in 0..55 {
        let mode = idx % 2;
        let weight = (idx as i64 % 5) + 1;
        let fname = format!("str_decode_op_{idx}_m{mode}");
        let sig = format!("fn {fname}(s: str) to int");
        if mode == 0 {
            let prose = format!(
                "Implement `{fname}(s)` parsing a packed dotted tuple `\"A.B.C\"` into `A * 10000 + B * 100 + C + {weight}` (`-1` if not 3 non-empty digit segments)."
            );
            let code = format!(
                "fn {fname}(s: str) to int {{\n    let ps: list[str] = s.trim().split(\".\")\n    if ps.len() != 3 {{ return -1 }}\n    let a: str = ps[0].unwrap()\n    let b: str = ps[1].unwrap()\n    let c: str = ps[2].unwrap()\n    if a.len() == 0 or b.len() == 0 or c.len() == 0 {{ return -1 }}\n    if not a.is_digit() or not b.is_digit() or not c.is_digit() {{ return -1 }}\n    return a.to_int().unwrap() * 10000 + b.to_int().unwrap() * 100 + c.to_int().unwrap() + {weight}\n}}"
            );
            let eval = |s: &str| -> i64 {
                let ps: Vec<&str> = s.trim().split('.').collect();
                if ps.len() != 3
                    || ps
                        .iter()
                        .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
                {
                    return -1;
                }
                ps[0].parse::<i64>().unwrap() * 10000
                    + ps[1].parse::<i64>().unwrap() * 100
                    + ps[2].parse::<i64>().unwrap()
                    + weight
            };
            let asserts = vec![
                format!("{fname}(\"2.14.3\") == {}", eval("2.14.3")),
                format!("{fname}(\"1.2\") == -1"),
                format!("{fname}(\"1.a.3\") == -1"),
            ];
            tb.push(
                &format!("str_{fname}"),
                "string_parsing",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        } else {
            let shift = 1 + (idx as i64 % 9);
            let prose = format!(
                "Implement `{fname}(s)` summing weighted ASCII codes over characters of `s`: adding `(ch.ord() + {shift})` for digits (`is_digit`), `ch.ord() * {weight}` for alphanumeric non-digits (`is_alnum`), and ignoring punctuation/whitespace."
            );
            let code = format!(
                "fn {fname}(s: str) to int {{\n    let mut acc: int = 0\n    for ch in s.chars() {{\n        if ch.is_digit() {{\n            acc = acc + ch.ord() + {shift}\n        }} else if ch.is_alnum() {{\n            acc = acc + ch.ord() * {weight}\n        }}\n    }}\n    return acc\n}}"
            );
            let eval = |s: &str| -> i64 {
                s.chars()
                    .map(|c| {
                        if c.is_ascii_digit() {
                            (c as i64) + shift
                        } else if c.is_alphanumeric() {
                            (c as i64) * weight
                        } else {
                            0
                        }
                    })
                    .sum()
            };
            let asserts = vec![
                format!("{fname}(\"v9-ok!\") == {}", eval("v9-ok!")),
                format!("{fname}(\"0A\") == {}", eval("0A")),
                format!("{fname}(\"---\") == 0"),
            ];
            tb.push(
                &format!("str_{fname}"),
                "string_parsing",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        }
    }
}

// ============================================================================
// Family 7: matrix_grid (~200 tasks)
// ============================================================================

fn gen_matrix_grid(tb: &mut TaskBuilder) {
    // 7A: 50 scalar grid reductions (border, diagonals, 2x2 max, dominant cells)
    for idx in 0..50 {
        let mode = idx % 2;
        let mul = 1 + (idx as i64 % 5);
        let fname = format!("grid_metric_{idx}_m{mode}");
        let sig = format!("fn {fname}(grid: list[list[int]]) to int");
        if mode == 0 {
            let prose = format!(
                "Implement `{fname}(grid)` returning `{mul}` times the sum of all boundary (perimeter) cells in rectangular `grid` (without double-counting corners; `0` if empty)."
            );
            let code = format!(
                "fn {fname}(grid: list[list[int]]) to int {{\n    let r: int = grid.len()\n    if r == 0 {{ return 0 }}\n    let c: int = grid[0].unwrap().len()\n    if c == 0 {{ return 0 }}\n    let mut sum: int = 0\n    let mut i: int = 0\n    while i < r {{\n        let mut j: int = 0\n        while j < c {{\n            if i == 0 or i == r - 1 or j == 0 or j == c - 1 {{\n                sum = sum + grid[i].unwrap()[j].unwrap()\n            }}\n            j = j + 1\n        }}\n        i = i + 1\n    }}\n    return sum * {mul}\n}}"
            );
            let eval = |g: &[Vec<i64>]| -> i64 {
                let r = g.len();
                if r == 0 || g[0].is_empty() {
                    return 0;
                }
                let c = g[0].len();
                let mut s = 0i64;
                for i in 0..r {
                    for j in 0..c {
                        if i == 0 || i == r - 1 || j == 0 || j == c - 1 {
                            s += g[i][j];
                        }
                    }
                }
                s * mul
            };
            let g1 = vec![vec![1, 2, 3], vec![4, 99, 5], vec![6, 7, 8]];
            let g2 = vec![vec![5, -2]];
            let asserts = vec![
                format!("{fname}({}) == {}", fmt_matrix(&g1), eval(&g1)),
                format!("{fname}({}) == {}", fmt_matrix(&g2), eval(&g2)),
                format!("{fname}([]) == 0"),
            ];
            tb.push(
                &format!("grid_{fname}"),
                "matrix_grid",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        } else {
            let prose = format!(
                "Implement `{fname}(grid)` returning the maximum sum of any `2x2` submatrix multiplied by `{mul}` (`0` if rows `< 2` or cols `< 2`)."
            );
            let code = format!(
                "fn {fname}(grid: list[list[int]]) to int {{\n    let r: int = grid.len()\n    if r < 2 {{ return 0 }}\n    let c: int = grid[0].unwrap().len()\n    if c < 2 {{ return 0 }}\n    let mut best: int = 0\n    let mut first: bool = true\n    let mut i: int = 0\n    while i + 1 < r {{\n        let mut j: int = 0\n        while j + 1 < c {{\n            let s: int = grid[i].unwrap()[j].unwrap() + grid[i].unwrap()[j + 1].unwrap() + grid[i + 1].unwrap()[j].unwrap() + grid[i + 1].unwrap()[j + 1].unwrap()\n            if first or s > best {{\n                best = s\n                first = false\n            }}\n            j = j + 1\n        }}\n        i = i + 1\n    }}\n    return best * {mul}\n}}"
            );
            let eval = |g: &[Vec<i64>]| -> i64 {
                if g.len() < 2 || g[0].len() < 2 {
                    return 0;
                }
                let mut best = i64::MIN;
                for i in 0..g.len() - 1 {
                    for j in 0..g[0].len() - 1 {
                        let s = g[i][j] + g[i][j + 1] + g[i + 1][j] + g[i + 1][j + 1];
                        best = best.max(s);
                    }
                }
                best * mul
            };
            let g1 = vec![vec![1, -5, 4], vec![2, 8, 6], vec![3, 1, 0]];
            let g2 = vec![vec![1, 2, 3]];
            let asserts = vec![
                format!("{fname}({}) == {}", fmt_matrix(&g1), eval(&g1)),
                format!("{fname}({}) == 0", fmt_matrix(&g2)),
                format!("{fname}([]) == 0"),
            ];
            tb.push(
                &format!("grid_{fname}"),
                "matrix_grid",
                &prose,
                &sig,
                code,
                &asserts,
                vec![format!("fn {fname}")],
            );
        }
    }

    // 7B: 50 matrix transforms returning list[list[int]] (transpose, rotate, flip, scale)
    for idx in 0..50 {
        let mode = idx % 3;
        let k = (idx as i64 % 5) + 1;
        let fname = format!("mat_xform_{idx}_m{mode}");
        let sig = format!("fn {fname}(m: list[list[int]]) to list[list[int]]");
        let (prose, code, eval): (String, String, Box<dyn Fn(&[Vec<i64>]) -> Vec<Vec<i64>>>) =
            match mode {
                0 => {
                    let p = format!(
                        "Implement `{fname}(m)` returning the transposed rectangular matrix with each element incremented by `{k}`."
                    );
                    let c = format!(
                        "fn {fname}(m: list[list[int]]) to list[list[int]] {{\n    if m.len() == 0 {{ return [] }}\n    let r: int = m.len()\n    let c: int = m[0].unwrap().len()\n    let mut out: list[list[int]] = []\n    let mut j: int = 0\n    while j < c {{\n        let mut row: list[int] = []\n        let mut i: int = 0\n        while i < r {{\n            row = row.push(m[i].unwrap()[j].unwrap() + {k})\n            i = i + 1\n        }}\n        out = out.push(row)\n        j = j + 1\n    }}\n    return out\n}}"
                    );
                    let f = Box::new(move |m: &[Vec<i64>]| -> Vec<Vec<i64>> {
                        if m.is_empty() {
                            return vec![];
                        }
                        let (r, c) = (m.len(), m[0].len());
                        (0..c)
                            .map(|j| (0..r).map(|i| m[i][j] + k).collect())
                            .collect()
                    });
                    (p, c, f)
                }
                1 => {
                    let p = format!(
                        "Implement `{fname}(m)` rotating an `R x C` grid 90 degrees clockwise and multiplying values by `{k}`."
                    );
                    let c = format!(
                        "fn {fname}(m: list[list[int]]) to list[list[int]] {{\n    if m.len() == 0 {{ return [] }}\n    let r: int = m.len()\n    let c: int = m[0].unwrap().len()\n    let mut out: list[list[int]] = []\n    let mut j: int = 0\n    while j < c {{\n        let mut row: list[int] = []\n        let mut i: int = r - 1\n        while i >= 0 {{\n            row = row.push(m[i].unwrap()[j].unwrap() * {k})\n            i = i - 1\n        }}\n        out = out.push(row)\n        j = j + 1\n    }}\n    return out\n}}"
                    );
                    let f = Box::new(move |m: &[Vec<i64>]| -> Vec<Vec<i64>> {
                        if m.is_empty() {
                            return vec![];
                        }
                        let (r, c) = (m.len(), m[0].len());
                        (0..c)
                            .map(|j| (0..r).rev().map(|i| m[i][j] * k).collect())
                            .collect()
                    });
                    (p, c, f)
                }
                _ => {
                    let p = format!(
                        "Implement `{fname}(m)` reversing rows top-to-bottom and negating every cell if its sum index `(i + j) % 2 == 1`, adding `{k}` to others."
                    );
                    let c = format!(
                        "fn {fname}(m: list[list[int]]) to list[list[int]] {{\n    let mut out: list[list[int]] = []\n    let mut i: int = m.len() - 1\n    while i >= 0 {{\n        let src: list[int] = m[i].unwrap()\n        let mut row: list[int] = []\n        let mut j: int = 0\n        while j < src.len() {{\n            let v: int = src[j].unwrap()\n            if (i + j) % 2 == 1 {{ row = row.push(0 - v) }} else {{ row = row.push(v + {k}) }}\n            j = j + 1\n        }}\n        out = out.push(row)\n        i = i - 1\n    }}\n    return out\n}}"
                    );
                    let f = Box::new(move |m: &[Vec<i64>]| -> Vec<Vec<i64>> {
                        let mut out = Vec::new();
                        for i in (0..m.len()).rev() {
                            let row = (0..m[i].len())
                                .map(|j| {
                                    if (i + j) % 2 == 1 {
                                        -m[i][j]
                                    } else {
                                        m[i][j] + k
                                    }
                                })
                                .collect();
                            out.push(row);
                        }
                        out
                    });
                    (p, c, f)
                }
            };
        let g1 = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let g2 = vec![vec![7]];
        let asserts = vec![
            format!(
                "{}({}) == {}",
                fname,
                fmt_matrix(&g1),
                fmt_matrix(&eval(&g1))
            ),
            format!(
                "{}({}) == {}",
                fname,
                fmt_matrix(&g2),
                fmt_matrix(&eval(&g2))
            ),
            format!("{fname}([]) == []"),
        ];
        tb.push(
            &format!("grid_{fname}"),
            "matrix_grid",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 7C: 50 grid DP path sums (min/max right-down DP)
    for idx in 0..50 {
        let minimize = idx % 2 == 0;
        let step_cost = idx as i64 % 4;
        let fname = format!("grid_dp_path_{idx}");
        let sig = format!("fn {fname}(grid: list[list[int]]) to int");
        let cmp = if minimize { "<" } else { ">" };
        let goal = if minimize { "minimum" } else { "maximum" };
        let prose = format!(
            "Implement `{fname}(grid)` computing the `{goal}` path sum from `(0, 0)` to `(R-1, C-1)` moving only right or down, adding `{step_cost}` step penalty per cell (`0` if empty)."
        );
        let code = format!(
            "fn {fname}(grid: list[list[int]]) to int {{\n    let r: int = grid.len()\n    if r == 0 {{ return 0 }}\n    let c: int = grid[0].unwrap().len()\n    if c == 0 {{ return 0 }}\n    let mut dp: list[int] = []\n    let mut i: int = 0\n    while i < r {{\n        let mut next_dp: list[int] = []\n        let mut j: int = 0\n        while j < c {{\n            let cell: int = grid[i].unwrap()[j].unwrap() + {step_cost}\n            if i == 0 and j == 0 {{\n                next_dp = next_dp.push(cell)\n            }} else if i == 0 {{\n                next_dp = next_dp.push(next_dp[j - 1].unwrap() + cell)\n            }} else if j == 0 {{\n                next_dp = next_dp.push(dp[0].unwrap() + cell)\n            }} else {{\n                let up: int = dp[j].unwrap()\n                let left: int = next_dp[j - 1].unwrap()\n                let mut best: int = up\n                if left {cmp} best {{ best = left }}\n                next_dp = next_dp.push(best + cell)\n            }}\n            j = j + 1\n        }}\n        dp = next_dp\n        i = i + 1\n    }}\n    return dp[c - 1].unwrap()\n}}"
        );
        let eval = |g: &[Vec<i64>]| -> i64 {
            if g.is_empty() || g[0].is_empty() {
                return 0;
            }
            let (r, c) = (g.len(), g[0].len());
            let mut dp = vec![0i64; c];
            for i in 0..r {
                for j in 0..c {
                    let cell = g[i][j] + step_cost;
                    dp[j] = match (i, j) {
                        (0, 0) => cell,
                        (0, _) => dp[j - 1] + cell,
                        (_, 0) => dp[0] + cell,
                        _ => {
                            let b = if minimize {
                                dp[j].min(dp[j - 1])
                            } else {
                                dp[j].max(dp[j - 1])
                            };
                            b + cell
                        }
                    };
                }
            }
            dp[c - 1]
        };
        let g1 = vec![vec![1, 3, 1], vec![1, 5, 1], vec![4, 2, 1]];
        let g2 = vec![vec![2, 9], vec![4, 1]];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_matrix(&g1), eval(&g1)),
            format!("{fname}({}) == {}", fmt_matrix(&g2), eval(&g2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("grid_{fname}"),
            "matrix_grid",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 7D: 50 grid neighbor / island / connected count tasks (iterative BFS using flat queue)
    for idx in 0..50 {
        let min_val = 1 + (idx as i64 % 3);
        let weight = 1 + ((idx / 3) as i64 % 4);
        let fname = format!("grid_region_{idx}_v{min_val}");
        let sig = format!("fn {fname}(grid: list[list[int]]) to int");
        let prose = format!(
            "Implement `{fname}(grid)` counting cells `grid[i][j] >= {min_val}` scaled by `{weight}` plus count of 4-orthogonal neighbors also `>= {min_val}` (`0` if empty)."
        );
        let code = format!(
            "fn {fname}(grid: list[list[int]]) to int {{\n    let r: int = grid.len()\n    if r == 0 {{ return 0 }}\n    let c: int = grid[0].unwrap().len()\n    let mut total: int = 0\n    let mut i: int = 0\n    while i < r {{\n        let mut j: int = 0\n        while j < c {{\n            if grid[i].unwrap()[j].unwrap() >= {min_val} {{\n                total = total + {weight}\n                if i > 0 and grid[i - 1].unwrap()[j].unwrap() >= {min_val} {{ total = total + 1 }}\n                if i + 1 < r and grid[i + 1].unwrap()[j].unwrap() >= {min_val} {{ total = total + 1 }}\n                if j > 0 and grid[i].unwrap()[j - 1].unwrap() >= {min_val} {{ total = total + 1 }}\n                if j + 1 < c and grid[i].unwrap()[j + 1].unwrap() >= {min_val} {{ total = total + 1 }}\n            }}\n            j = j + 1\n        }}\n        i = i + 1\n    }}\n    return total\n}}"
        );
        let eval = |g: &[Vec<i64>]| -> i64 {
            if g.is_empty() {
                return 0;
            }
            let (r, c) = (g.len(), g[0].len());
            let mut total = 0i64;
            for i in 0..r {
                for j in 0..c {
                    if g[i][j] >= min_val {
                        total += weight;
                        if i > 0 && g[i - 1][j] >= min_val {
                            total += 1;
                        }
                        if i + 1 < r && g[i + 1][j] >= min_val {
                            total += 1;
                        }
                        if j > 0 && g[i][j - 1] >= min_val {
                            total += 1;
                        }
                        if j + 1 < c && g[i][j + 1] >= min_val {
                            total += 1;
                        }
                    }
                }
            }
            total
        };
        let g1 = vec![vec![0, 3, 2], vec![1, 3, 0], vec![2, 0, 4]];
        let g2 = vec![vec![0, 0], vec![0, 0]];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_matrix(&g1), eval(&g1)),
            format!("{fname}({}) == 0", fmt_matrix(&g2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("grid_{fname}"),
            "matrix_grid",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }
}

// ============================================================================
// Family 8: data_structures (~200 tasks)
// ============================================================================

fn gen_data_structures(tb: &mut TaskBuilder) {
    // 8A: 50 Stack + Min/Max accumulator simulations
    for idx in 0..50 {
        let cap = 3 + (idx as i64 % 5);
        let bonus = idx as i64 % 7;
        let fname = format!("stack_eval_{idx}_c{cap}");
        let sig = format!("fn {fname}(ops: list[str]) to int");
        let prose = format!(
            "Implement `{fname}(ops)` executing integer stack operations (`\"push:<n>\"`, `\"pop\"`, `\"add2\"`, `\"dup\"`, ignored cleanly when impossible) up to capacity `{cap}` (extra pushes dropped), returning `sum(stack) + {bonus} * stack.len()`."
        );
        let code = format!(
            "{}\n\nfn {fname}(ops: list[str]) to int {{\n    let mut st: list[int] = []\n    for op in ops {{\n        if op.starts_with(\"push:\") {{\n            if st.len() < {cap} {{\n                let ps: list[str] = op.split(\":\")\n                st = st.push(parse_int_val(ps[1].unwrap()))\n            }}\n        }} else if op == \"pop\" {{\n            if st.len() > 0 {{ st = st.remove_at(st.len() - 1) }}\n        }} else if op == \"dup\" {{\n            if st.len() > 0 and st.len() < {cap} {{\n                st = st.push(st[st.len() - 1].unwrap())\n            }}\n        }} else if op == \"add2\" {{\n            if st.len() >= 2 {{\n                let a: int = st[st.len() - 1].unwrap()\n                let b: int = st[st.len() - 2].unwrap()\n                st = st.remove_at(st.len() - 1)\n                st = st.remove_at(st.len() - 1)\n                st = st.push(a + b)\n            }}\n        }}\n    }}\n    let mut sum: int = 0\n    for v in st {{ sum = sum + v }}\n    return sum + {bonus} * st.len()\n}}",
            parse_signed_vox_helper()
        );
        let eval = |ops: &[&str]| -> i64 {
            let mut st: Vec<i64> = Vec::new();
            for &op in ops {
                if let Some(rest) = op.strip_prefix("push:") {
                    if st.len() < cap as usize {
                        st.push(rest.parse::<i64>().unwrap_or(0));
                    }
                } else if op == "pop" {
                    st.pop();
                } else if op == "dup" {
                    if !st.is_empty() && st.len() < cap as usize {
                        st.push(*st.last().unwrap());
                    }
                } else if op == "add2" && st.len() >= 2 {
                    let a = st.pop().unwrap();
                    let b = st.pop().unwrap();
                    st.push(a + b);
                }
            }
            st.iter().sum::<i64>() + bonus * (st.len() as i64)
        };
        let o1 = [
            "push:4", "dup", "add2", "push:7", "push:9", "push:11", "push:13", "pop",
        ];
        let o2 = ["pop", "add2", "push:-3", "dup"];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_list(&o1), eval(&o1)),
            format!("{fname}({}) == {}", fmt_str_list(&o2), eval(&o2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("ds_{fname}"),
            "data_structures",
            &prose,
            &sig,
            code,
            &asserts,
            vec!["fn parse_int_val".to_string(), format!("fn {fname}")],
        );
    }

    // 8B: 50 LRU / Move-To-Front cache hit counters
    for idx in 0..50 {
        let cap = 2 + (idx as usize % 4);
        let hit_pts = 5 + (idx as i64 % 5);
        let fname = format!("lru_cache_{idx}_c{cap}");
        let sig = format!("fn {fname}(keys: list[int]) to int");
        let prose = format!(
            "Implement `{fname}(keys)` simulating an LRU cache of capacity `{cap}`: return `hits * {hit_pts} + final_cache_size`."
        );
        let code = format!(
            "fn {fname}(keys: list[int]) to int {{\n    let mut q: list[int] = []\n    let mut hits: int = 0\n    for k in keys {{\n        let mut found_i: int = -1\n        let mut i: int = 0\n        while i < q.len() {{\n            if q[i].unwrap() == k {{ found_i = i }}\n            i = i + 1\n        }}\n        if found_i >= 0 {{\n            hits = hits + 1\n            q = q.remove_at(found_i)\n            q = q.push(k)\n        }} else {{\n            if q.len() == {cap} {{ q = q.remove_at(0) }}\n            q = q.push(k)\n        }}\n    }}\n    return hits * {hit_pts} + q.len()\n}}"
        );
        let eval = |keys: &[i64]| -> i64 {
            let mut q: Vec<i64> = Vec::new();
            let mut hits = 0i64;
            for &k in keys {
                if let Some(pos) = q.iter().position(|&x| x == k) {
                    hits += 1;
                    q.remove(pos);
                    q.push(k);
                } else {
                    if q.len() == cap {
                        q.remove(0);
                    }
                    q.push(k);
                }
            }
            hits * hit_pts + q.len() as i64
        };
        let k1 = [1, 2, 3, 2, 4, 1, 2, 5];
        let k2 = [7, 7, 7];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_int_list(&k1), eval(&k1)),
            format!("{fname}({}) == {}", fmt_int_list(&k2), eval(&k2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("ds_{fname}"),
            "data_structures",
            &prose,
            &sig,
            code,
            &asserts,
            vec![format!("fn {fname}")],
        );
    }

    // 8C: 50 Parallel-list KV Map interpreters
    for idx in 0..50 {
        let default_v = (idx as i64 % 6) * 10;
        let fname = format!("kv_map_exec_{idx}");
        let sig = format!("fn {fname}(ops: list[str]) to int");
        let prose = format!(
            "Implement `{fname}(ops)` maintaining an in-memory map (`keys: list[str], vals: list[int]`) over `\"put:k:v\"` (insert or overwrite), `\"add:k:v\"` (increment existing or insert starting from `{default_v}` + v), `\"del:k\"`, returning the sum of remaining values."
        );
        let code = format!(
            "{}\n\nfn find_kv(ks: list[str], target: str) to int {{\n    let mut i: int = 0\n    while i < ks.len() {{\n        if ks[i].unwrap() == target {{ return i }}\n        i = i + 1\n    }}\n    return -1\n}}\n\nfn {fname}(ops: list[str]) to int {{\n    let mut ks: list[str] = []\n    let mut vs: list[int] = []\n    for op in ops {{\n        let ps: list[str] = op.split(\":\")\n        let cmd: str = ps[0].unwrap()\n        let k: str = ps[1].unwrap()\n        let pos: int = find_kv(ks, k)\n        if cmd == \"put\" {{\n            let v: int = parse_int_val(ps[2].unwrap())\n            if pos >= 0 {{\n                vs = vs.remove_at(pos)\n                ks = ks.remove_at(pos)\n            }}\n            ks = ks.push(k)\n            vs = vs.push(v)\n        }} else if cmd == \"add\" {{\n            let v: int = parse_int_val(ps[2].unwrap())\n            if pos >= 0 {{\n                let cur: int = vs[pos].unwrap()\n                vs = vs.remove_at(pos)\n                ks = ks.remove_at(pos)\n                ks = ks.push(k)\n                vs = vs.push(cur + v)\n            }} else {{\n                ks = ks.push(k)\n                vs = vs.push({default_v} + v)\n            }}\n        }} else if cmd == \"del\" and pos >= 0 {{\n            ks = ks.remove_at(pos)\n            vs = vs.remove_at(pos)\n        }}\n    }}\n    let mut total: int = 0\n    for v in vs {{ total = total + v }}\n    return total\n}}",
            parse_signed_vox_helper()
        );
        let eval = |ops: &[&str]| -> i64 {
            let mut m = std::collections::BTreeMap::<String, i64>::new();
            for &op in ops {
                let ps: Vec<&str> = op.split(':').collect();
                match ps[0] {
                    "put" => {
                        m.insert(ps[1].to_string(), ps[2].parse::<i64>().unwrap_or(0));
                    }
                    "add" => {
                        let d = ps[2].parse::<i64>().unwrap_or(0);
                        let e = m.entry(ps[1].to_string()).or_insert(default_v);
                        *e += d;
                    }
                    "del" => {
                        m.remove(ps[1]);
                    }
                    _ => {}
                }
            }
            m.values().sum()
        };
        let o1 = ["put:a:12", "add:b:5", "add:a:3", "del:a", "put:c:4"];
        let o2 = ["add:x:1", "add:x:2", "put:y:-5"];
        let asserts = vec![
            format!("{fname}({}) == {}", fmt_str_list(&o1), eval(&o1)),
            format!("{fname}({}) == {}", fmt_str_list(&o2), eval(&o2)),
            format!("{fname}([]) == 0"),
        ];
        tb.push(
            &format!("ds_{fname}"),
            "data_structures",
            &prose,
            &sig,
            code,
            &asserts,
            vec!["fn find_kv".to_string(), format!("fn {fname}")],
        );
    }

    // 8D: 50 Iterative Graph BFS reachability & connected component trackers
    for idx in 0..50 {
        let mode = idx % 2;
        let bonus = idx as i64 % 5;
        let fname = format!("graph_walk_{idx}_m{mode}");
        let sig = if mode == 0 {
            format!("fn {fname}(n: int, edges: list[list[int]], src: int) to int")
        } else {
            format!("fn {fname}(n: int, edges: list[list[int]]) to int")
        };
        if mode == 0 {
            let prose = format!(
                "Implement `{fname}(n, edges, src)` running an iterative undirected BFS over nodes `0..n-1` from `src` and returning `reachable_count + {bonus}` (`0` if `src < 0 or src >= n`)."
            );
            let code = format!(
                "fn list_has(xs: list[int], target: int) to bool {{\n    for v in xs {{ if v == target {{ return true }} }}\n    return false\n}}\n\nfn {fname}(n: int, edges: list[list[int]], src: int) to int {{\n    if src < 0 or src >= n {{ return 0 }}\n    let mut visited: list[int] = [src]\n    let mut q: list[int] = [src]\n    let mut head: int = 0\n    while head < q.len() {{\n        let u: int = q[head].unwrap()\n        head = head + 1\n        for e in edges {{\n            let a: int = e[0].unwrap()\n            let b: int = e[1].unwrap()\n            if a == u and not list_has(visited, b) {{\n                visited = visited.push(b)\n                q = q.push(b)\n            }} else if b == u and not list_has(visited, a) {{\n                visited = visited.push(a)\n                q = q.push(a)\n            }}\n        }}\n    }}\n    return visited.len() + {bonus}\n}}"
            );
            let eval = |n: i64, edges: &[[i64; 2]], src: i64| -> i64 {
                if src < 0 || src >= n {
                    return 0;
                }
                let mut vis = vec![src];
                let mut head = 0;
                while head < vis.len() {
                    let u = vis[head];
                    head += 1;
                    for &[a, b] in edges {
                        if a == u && !vis.contains(&b) {
                            vis.push(b);
                        } else if b == u && !vis.contains(&a) {
                            vis.push(a);
                        }
                    }
                }
                vis.len() as i64 + bonus
            };
            let e1 = [[0, 1], [1, 2], [3, 4], [2, 0]];
            let m1: Vec<Vec<i64>> = e1.iter().map(|r| r.to_vec()).collect();
            let asserts = vec![
                format!("{fname}(5, {}, 0) == {}", fmt_matrix(&m1), eval(5, &e1, 0)),
                format!("{fname}(5, {}, 3) == {}", fmt_matrix(&m1), eval(5, &e1, 3)),
                format!("{fname}(5, {}, 9) == 0", fmt_matrix(&m1)),
            ];
            tb.push(
                &format!("ds_{fname}"),
                "data_structures",
                &prose,
                &sig,
                code,
                &asserts,
                vec!["fn list_has".to_string(), format!("fn {fname}")],
            );
        } else {
            let prose = format!(
                "Implement `{fname}(n, edges)` computing `{bonus}` plus the number of isolated/component groups across `n` vertices `0..n-1` (`0` if `n <= 0`)."
            );
            let code = format!(
                "fn has_item(xs: list[int], target: int) to bool {{\n    for v in xs {{ if v == target {{ return true }} }}\n    return false\n}}\n\nfn {fname}(n: int, edges: list[list[int]]) to int {{\n    if n <= 0 {{ return 0 }}\n    let mut seen: list[int] = []\n    let mut comps: int = 0\n    let mut start: int = 0\n    while start < n {{\n        if not has_item(seen, start) {{\n            comps = comps + 1\n            seen = seen.push(start)\n            let mut q: list[int] = [start]\n            let mut head: int = 0\n            while head < q.len() {{\n                let u: int = q[head].unwrap()\n                head = head + 1\n                for e in edges {{\n                    let a: int = e[0].unwrap()\n                    let b: int = e[1].unwrap()\n                    if a == u and not has_item(seen, b) {{\n                        seen = seen.push(b)\n                        q = q.push(b)\n                    }} else if b == u and not has_item(seen, a) {{\n                        seen = seen.push(a)\n                        q = q.push(a)\n                    }}\n                }}\n            }}\n        }}\n        start = start + 1\n    }}\n    return comps + {bonus}\n}}"
            );
            let eval = |n: i64, edges: &[[i64; 2]]| -> i64 {
                if n <= 0 {
                    return 0;
                }
                let mut seen = Vec::new();
                let mut comps = 0i64;
                for s in 0..n {
                    if !seen.contains(&s) {
                        comps += 1;
                        seen.push(s);
                        let mut q = vec![s];
                        let mut head = 0;
                        while head < q.len() {
                            let u = q[head];
                            head += 1;
                            for &[a, b] in edges {
                                if a == u && !seen.contains(&b) {
                                    seen.push(b);
                                    q.push(b);
                                } else if b == u && !seen.contains(&a) {
                                    seen.push(a);
                                    q.push(a);
                                }
                            }
                        }
                    }
                }
                comps + bonus
            };
            let e1 = [[0, 1], [1, 2], [3, 4]];
            let m1: Vec<Vec<i64>> = e1.iter().map(|r| r.to_vec()).collect();
            let asserts = vec![
                format!("{fname}(6, {}) == {}", fmt_matrix(&m1), eval(6, &e1)),
                format!("{fname}(4, []) == {}", 4 + bonus),
                format!("{fname}(0, []) == 0"),
            ];
            tb.push(
                &format!("ds_{fname}"),
                "data_structures",
                &prose,
                &sig,
                code,
                &asserts,
                vec!["fn has_item".to_string(), format!("fn {fname}")],
            );
        }
    }
}

// ============================================================================
// Family 9: multi_fn_module (~200 tasks)
// ============================================================================

fn gen_multi_fn_module(tb: &mut TaskBuilder) {
    // 50 domain pipeline builders x 4 variants = 200 multi-function Vox modules
    for idx in 0..200 {
        let mode = idx % 4;
        let lo = 5 + (idx as i64 % 5) * 3;
        let hi = lo + 25 + (idx as i64 % 7) * 5;
        let scale = 2 + (idx as i64 % 3);
        let h_fn = format!("mod_step_{idx}");
        let m_fn = format!("mod_pipeline_{idx}");
        match mode {
            0 => {
                let sig = format!("fn {h_fn}(x: int) to int\nfn {m_fn}(xs: list[int]) to int");
                let prose = format!(
                    "Write a 2-function Vox module: `{h_fn}(x)` clamping `x` into `[{lo}, {hi}]` and multiplying by `{scale}`, plus `{m_fn}(xs)` summing `{h_fn}` over every positive entry."
                );
                let code = format!(
                    "fn {h_fn}(x: int) to int {{\n    let mut v: int = x\n    if v < {lo} {{ v = {lo} }}\n    if v > {hi} {{ v = {hi} }}\n    return v * {scale}\n}}\n\nfn {m_fn}(xs: list[int]) to int {{\n    let mut acc: int = 0\n    for x in xs {{\n        if x > 0 {{ acc = acc + {h_fn}(x) }}\n    }}\n    return acc\n}}"
                );
                let h = |x: i64| x.clamp(lo, hi) * scale;
                let eval =
                    |xs: &[i64]| -> i64 { xs.iter().filter(|&&x| x > 0).map(|&x| h(x)).sum() };
                let xs1 = [-3, 1, lo + 4, hi + 10];
                let asserts = vec![
                    format!("{h_fn}(1) == {}", h(1)),
                    format!("{m_fn}({}) == {}", fmt_int_list(&xs1), eval(&xs1)),
                    format!("{m_fn}([]) == 0"),
                ];
                tb.push(
                    &format!("mod_{m_fn}"),
                    "multi_fn_module",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {h_fn}"), format!("fn {m_fn}")],
                );
            }
            1 => {
                let sig = format!("fn {h_fn}(s: str) to int\nfn {m_fn}(lines: list[str]) to int");
                let prose = format!(
                    "Write a 2-function Vox module: `{h_fn}(s)` returning trimmed token length scaled by `{scale}` (`0` if empty or starts with `#`), plus `{m_fn}(lines)` returning the maximum `{h_fn}` across `lines`."
                );
                let code = format!(
                    "fn {h_fn}(s: str) to int {{\n    let t: str = s.trim()\n    if t.len() == 0 or t.starts_with(\"#\") {{ return 0 }}\n    return t.len() * {scale}\n}}\n\nfn {m_fn}(lines: list[str]) to int {{\n    let mut best: int = 0\n    for ln in lines {{\n        let sc: int = {h_fn}(ln)\n        if sc > best {{ best = sc }}\n    }}\n    return best\n}}"
                );
                let h = |s: &str| -> i64 {
                    let t = s.trim();
                    if t.is_empty() || t.starts_with('#') {
                        0
                    } else {
                        t.len() as i64 * scale
                    }
                };
                let ls1 = ["  # comment  ", " alpha ", "longer_item"];
                let asserts = vec![
                    format!("{h_fn}(\"# skip\") == 0"),
                    format!("{h_fn}(\"vox\") == {}", h("vox")),
                    format!(
                        "{m_fn}({}) == {}",
                        fmt_str_list(&ls1),
                        ls1.iter().map(|s| h(s)).max().unwrap()
                    ),
                ];
                tb.push(
                    &format!("mod_{m_fn}"),
                    "multi_fn_module",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {h_fn}"), format!("fn {m_fn}")],
                );
            }
            2 => {
                let sig = format!(
                    "fn {h_fn}(a: int, b: int) to bool\nfn {m_fn}(xs: list[int]) to list[int]"
                );
                let prose = format!(
                    "Write a 2-function Vox module: `{h_fn}(a, b)` returning true when `|a - b| <= {lo}`, plus `{m_fn}(xs)` deduplicating adjacent runs where `{h_fn}(kept_last, current)` holds."
                );
                let code = format!(
                    "fn {h_fn}(a: int, b: int) to bool {{\n    let mut d: int = a - b\n    if d < 0 {{ d = 0 - d }}\n    return d <= {lo}\n}}\n\nfn {m_fn}(xs: list[int]) to list[int] {{\n    let mut out: list[int] = []\n    for v in xs {{\n        if out.len() == 0 or not {h_fn}(out[out.len() - 1].unwrap(), v) {{\n            out = out.push(v)\n        }}\n    }}\n    return out\n}}"
                );
                let h = |a: i64, b: i64| (a - b).abs() <= lo;
                let eval = |xs: &[i64]| -> Vec<i64> {
                    let mut out = Vec::new();
                    for &v in xs {
                        if out.last().is_none_or(|&prev| !h(prev, v)) {
                            out.push(v);
                        }
                    }
                    out
                };
                let xs1 = [10, 12, 10 + lo, 100, 100 + lo + 1];
                let asserts = vec![
                    format!("{h_fn}(10, {}) == true", 10 + lo),
                    format!(
                        "{m_fn}({}) == {}",
                        fmt_int_list(&xs1),
                        fmt_int_list(&eval(&xs1))
                    ),
                    format!("{m_fn}([]) == []"),
                ];
                tb.push(
                    &format!("mod_{m_fn}"),
                    "multi_fn_module",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("fn {h_fn}"), format!("fn {m_fn}")],
                );
            }
            _ => {
                let ty = format!("BandClass{idx}");
                let norm_fn = format!("classify_{idx}");
                let sig = format!(
                    "type {ty} =\n    | LowBand(val: int)\n    | MidBand(val: int)\n    | HighBand(val: int)\n\nfn {norm_fn}(x: int) to {ty}\nfn {m_fn}(xs: list[int]) to int"
                );
                let prose = format!(
                    "Define ADT `{ty}` (`LowBand(val)`, `MidBand(val)`, `HighBand(val)`), classifier `{norm_fn}(x)` splitting at `< {lo}` and `> {hi}`, and reducer `{m_fn}(xs)` summing `val` (`LowBand`), `val * {scale}` (`MidBand`), `val * {scale} + {lo}` (`HighBand`)."
                );
                let code = format!(
                    "type {ty} =\n    | LowBand(val: int)\n    | MidBand(val: int)\n    | HighBand(val: int)\n\nfn {norm_fn}(x: int) to {ty} {{\n    if x < {lo} {{ return LowBand(x) }}\n    if x > {hi} {{ return HighBand(x) }}\n    return MidBand(x)\n}}\n\nfn {m_fn}(xs: list[int]) to int {{\n    let mut total: int = 0\n    for x in xs {{\n        match {norm_fn}(x) {{\n            LowBand(v) => {{ total = total + v }}\n            MidBand(v) => {{ total = total + v * {scale} }}\n            HighBand(v) => {{ total = total + v * {scale} + {lo} }}\n        }}\n    }}\n    return total\n}}"
                );
                let eval = |xs: &[i64]| -> i64 {
                    xs.iter()
                        .map(|&x| {
                            if x < lo {
                                x
                            } else if x > hi {
                                x * scale + lo
                            } else {
                                x * scale
                            }
                        })
                        .sum()
                };
                let xs1 = [2, lo + 1, hi + 5];
                let asserts = vec![
                    format!("{m_fn}({}) == {}", fmt_int_list(&xs1), eval(&xs1)),
                    format!("{m_fn}([0]) == 0"),
                    format!("{m_fn}([]) == 0"),
                ];
                tb.push(
                    &format!("mod_{m_fn}"),
                    "multi_fn_module",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![
                        format!("type {ty}"),
                        format!("fn {norm_fn}"),
                        format!("fn {m_fn}"),
                    ],
                );
            }
        }
    }
}

// ============================================================================
// Family 10: endpoint_handler (~200 tasks, Vox query & mutation declarations)
// ============================================================================

fn gen_endpoint_handler(tb: &mut TaskBuilder) {
    // 50 backend service policies x 4 flavors (2 query, 2 mutation) = 200 tasks
    for idx in 0..200 {
        let mode = idx % 4;
        let base = 100 + (idx as i64 % 7) * 50;
        let pct = 5 + (idx as i64 % 5) * 5;
        let cap = 20 + (idx as i64 % 6) * 10;
        match mode {
            0 => {
                let qname = format!("quote_service_fee_{idx}");
                let sig = format!("query {qname}(units: int, rush: bool) to int");
                let prose = format!(
                    "Define a Vox `query` named `{qname}(units: int, rush: bool) to int` returning `0` if `units <= 0`, otherwise `{base} + units * {pct}` (doubled if `rush` is true)."
                );
                let code = format!(
                    "query {qname}(units: int, rush: bool) to int {{\n    if units <= 0 {{ return 0 }}\n    let mut amt: int = {base} + units * {pct}\n    if rush {{ amt = amt * 2 }}\n    return amt\n}}"
                );
                let eval = |u: i64, r: bool| -> i64 {
                    if u <= 0 {
                        0
                    } else {
                        let a = base + u * pct;
                        if r { a * 2 } else { a }
                    }
                };
                let asserts = vec![
                    format!("{qname}(0, true) == 0"),
                    format!("{qname}(4, false) == {}", eval(4, false)),
                    format!("{qname}(4, true) == {}", eval(4, true)),
                ];
                tb.push(
                    &format!("ep_{qname}"),
                    "endpoint_handler",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("query {qname}")],
                );
            }
            1 => {
                let qname = format!("page_slice_bounds_{idx}");
                let sig = format!("query {qname}(total: int, page: int, size: int) to list[int]");
                let prose = format!(
                    "Define a Vox `query` named `{qname}(total: int, page: int, size: int) to list[int]` returning 1-indexed pagination `[start, end, {cap}]` clamped to `[0, total]` (`[0, 0, 0]` when invalid or past end)."
                );
                let code = format!(
                    "query {qname}(total: int, page: int, size: int) to list[int] {{\n    if total <= 0 or page < 1 or size <= 0 {{ return [0, 0, 0] }}\n    let start: int = (page - 1) * size\n    if start >= total {{ return [0, 0, 0] }}\n    let mut end: int = start + size\n    if end > total {{ end = total }}\n    return [start, end, {cap}]\n}}"
                );
                let eval = |t: i64, p: i64, s: i64| -> Vec<i64> {
                    if t <= 0 || p < 1 || s <= 0 {
                        return vec![0, 0, 0];
                    }
                    let st = (p - 1) * s;
                    if st >= t {
                        return vec![0, 0, 0];
                    }
                    vec![st, (st + s).min(t), cap]
                };
                let asserts = vec![
                    format!("{qname}(25, 2, 10) == {}", fmt_int_list(&eval(25, 2, 10))),
                    format!("{qname}(25, 3, 10) == {}", fmt_int_list(&eval(25, 3, 10))),
                    format!("{qname}(25, 4, 10) == [0, 0, 0]"),
                ];
                tb.push(
                    &format!("ep_{qname}"),
                    "endpoint_handler",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("query {qname}")],
                );
            }
            2 => {
                let mname = format!("reserve_quota_tx_{idx}");
                let sig = format!("mutation {mname}(req: int, used: int, limit: int) to int");
                let prose = format!(
                    "Define a Vox `mutation` named `{mname}(req: int, used: int, limit: int) to int` granting up to `min(req, max(0, limit - used))` capped at `{cap}` (`0` when `req <= 0`)."
                );
                let code = format!(
                    "mutation {mname}(req: int, used: int, limit: int) to int {{\n    if req <= 0 or used >= limit {{ return 0 }}\n    let rem: int = limit - used\n    let mut grant: int = req\n    if rem < grant {{ grant = rem }}\n    if {cap} < grant {{ grant = {cap} }}\n    return grant\n}}"
                );
                let eval = |req: i64, used: i64, limit: i64| -> i64 {
                    if req <= 0 || used >= limit {
                        0
                    } else {
                        req.min(limit - used).min(cap)
                    }
                };
                let asserts = vec![
                    format!("{mname}(5, 10, 50) == {}", eval(5, 10, 50)),
                    format!(
                        "{mname}({}, 0, 1000) == {}",
                        cap + 15,
                        eval(cap + 15, 0, 1000)
                    ),
                    format!("{mname}(10, 50, 50) == 0"),
                ];
                tb.push(
                    &format!("ep_{mname}"),
                    "endpoint_handler",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("mutation {mname}")],
                );
            }
            _ => {
                let mname = format!("reconcile_tier_tx_{idx}");
                let sig = format!("mutation {mname}(tier: str, cents: int) to int");
                let prose = format!(
                    "Define a Vox `mutation` named `{mname}(tier: str, cents: int) to int` adjusting positive `cents`: `\"gold\"` reduces by `{pct}%`, `\"silver\"` reduces by `min(cents, {base})`, `\"freeze\"` sets `0`, others leave `cents` unchanged (`0` if `cents <= 0`)."
                );
                let code = format!(
                    "mutation {mname}(tier: str, cents: int) to int {{\n    if cents <= 0 or tier == \"freeze\" {{ return 0 }}\n    if tier == \"gold\" {{ return cents - (cents * {pct}) / 100 }}\n    if tier == \"silver\" {{\n        if cents <= {base} {{ return 0 }}\n        return cents - {base}\n    }}\n    return cents\n}}"
                );
                let eval = |tier: &str, c: i64| -> i64 {
                    if c <= 0 || tier == "freeze" {
                        return 0;
                    }
                    match tier {
                        "gold" => c - (c * pct) / 100,
                        "silver" => (c - base).max(0),
                        _ => c,
                    }
                };
                let asserts = vec![
                    format!("{mname}(\"gold\", 1000) == {}", eval("gold", 1000)),
                    format!(
                        "{mname}(\"silver\", {}) == {}",
                        base + 40,
                        eval("silver", base + 40)
                    ),
                    format!("{mname}(\"freeze\", 500) == 0"),
                ];
                tb.push(
                    &format!("ep_{mname}"),
                    "endpoint_handler",
                    &prose,
                    &sig,
                    code,
                    &asserts,
                    vec![format!("mutation {mname}")],
                );
            }
        }
    }
}
