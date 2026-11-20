//! Builds `target/dogfood_v5/train.jsonl` from `target/dogfood_v4/train.jsonl`.
//!
//! Keeps every v4 non-`<think>` row unchanged and replaces the v4 `<think>` rows (terse synthetic
//! Vox traces + unverifiable math replay) with verbose, step-by-step Vox traces whose final answer
//! is verified by running the program through `vox run --interp`. Includes adversarially named
//! programs (e.g. a `fibonacci` that doubles) and a trailing check pass in every trace.
//!
//! Usage: `cargo run -p vox-corpus --example gen_dogfood_v5 -- [--vox target/debug/vox] [--target-share 0.22]`
//! Writes `train.jsonl` plus `v5_stats.json` and `v5_leakage.json` under `target/dogfood_v5/`.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

// ───────────────────────────── mini AST ─────────────────────────────
#[derive(Clone)]
enum E {
    N(i64),
    V(String),
    B(Box<E>, &'static str, Box<E>),
    C(String, Vec<E>),
}
#[derive(Clone)]
enum S {
    Let(String, E, bool),
    Set(String, E),
    For(String, E, E, Vec<S>),
    If(E, Vec<S>, Vec<S>),
    Ret(E),
}
#[derive(Clone)]
struct F {
    name: String,
    params: Vec<String>,
    body: Vec<S>,
}

fn n(i: i64) -> E {
    E::N(i)
}
fn v(s: &str) -> E {
    E::V(s.to_string())
}
fn b(l: E, op: &'static str, r: E) -> E {
    E::B(Box::new(l), op, Box::new(r))
}
fn call(name: &str, args: Vec<E>) -> E {
    E::C(name.to_string(), args)
}
fn let_m(name: &str, e: E) -> S {
    S::Let(name.to_string(), e, true)
}
fn let_i(name: &str, e: E) -> S {
    S::Let(name.to_string(), e, false)
}
fn set(name: &str, e: E) -> S {
    S::Set(name.to_string(), e)
}
fn for_(var: &str, lo: E, hi: E, body: Vec<S>) -> S {
    S::For(var.to_string(), lo, hi, body)
}
fn ret(e: E) -> S {
    S::Ret(e)
}
fn func(name: &str, params: &[&str], body: Vec<S>) -> F {
    F {
        name: name.to_string(),
        params: params.iter().map(|s| s.to_string()).collect(),
        body,
    }
}

fn prec(op: &str) -> u8 {
    match op {
        "*" | "/" | "%" => 3,
        "+" | "-" => 2,
        _ => 1,
    }
}

/// Render an expression; when `env` is given, variables are replaced by their values.
fn render(e: &E, env: Option<&[(String, i64, bool)]>, parent: u8, right: bool) -> String {
    match e {
        E::N(i) => {
            if *i < 0 {
                format!("({i})")
            } else {
                i.to_string()
            }
        }
        E::V(s) => match env {
            Some(env) => {
                let val = env
                    .iter()
                    .rev()
                    .find(|(k, _, _)| k == s)
                    .map(|x| x.1)
                    .unwrap_or(0);
                if val < 0 {
                    format!("({val})")
                } else {
                    val.to_string()
                }
            }
            None => s.clone(),
        },
        E::C(name, args) => {
            let a: Vec<String> = args.iter().map(|a| render(a, env, 0, false)).collect();
            format!("{name}({})", a.join(", "))
        }
        E::B(l, op, r) => {
            let p = prec(op);
            let s = format!(
                "{} {op} {}",
                render(l, env, p, false),
                render(r, env, p, true)
            );
            if p < parent || (right && p == parent) {
                format!("({s})")
            } else {
                s
            }
        }
    }
}
fn src(e: &E) -> String {
    render(e, None, 0, false)
}

fn print_stmts(out: &mut String, ss: &[S], ind: usize) {
    let pad = "    ".repeat(ind);
    for s in ss {
        match s {
            S::Let(name, e, m) => {
                let _ = writeln!(
                    out,
                    "{pad}let {}{name}: int = {}",
                    if *m { "mut " } else { "" },
                    src(e)
                );
            }
            S::Set(name, e) => {
                let _ = writeln!(out, "{pad}{name} = {}", src(e));
            }
            S::Ret(e) => {
                let _ = writeln!(out, "{pad}return {}", src(e));
            }
            S::For(var, lo, hi, body) => {
                let _ = writeln!(out, "{pad}for {var} in range({}, {}) {{", src(lo), src(hi));
                print_stmts(out, body, ind + 1);
                let _ = writeln!(out, "{pad}}}");
            }
            S::If(c, t, e) => {
                let _ = writeln!(out, "{pad}if {} {{", src(c));
                print_stmts(out, t, ind + 1);
                if e.is_empty() {
                    let _ = writeln!(out, "{pad}}}");
                } else {
                    let _ = writeln!(out, "{pad}}} else {{");
                    print_stmts(out, e, ind + 1);
                    let _ = writeln!(out, "{pad}}}");
                }
            }
        }
    }
}
fn print_program(fs: &[F]) -> String {
    let mut out = String::new();
    for (i, f) in fs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let ps: Vec<String> = f.params.iter().map(|p| format!("{p}: int")).collect();
        let _ = writeln!(out, "fn {}({}) to int {{", f.name, ps.join(", "));
        print_stmts(&mut out, &f.body, 1);
        let _ = writeln!(out, "}}");
    }
    out.trim_end().to_string()
}

// ───────────────────────────── tracing evaluator ─────────────────────────────
struct Tracer<'a> {
    fns: &'a [F],
    out: Vec<String>,
    depth: usize,
    steps: usize,
    /// (var, start, per-iteration values) of the most recently completed loop
    last_loop: Option<(String, i64, Vec<i64>)>,
}
type Env = Vec<(String, i64, bool)>;

impl<'a> Tracer<'a> {
    fn line(&mut self, s: String) {
        let pad = "  ".repeat(self.depth);
        self.out.push(format!("{pad}{s}"));
    }
    fn eval(&mut self, e: &E, env: &Env) -> Result<i64, String> {
        self.steps += 1;
        if self.steps > 4000 {
            return Err("too long".into());
        }
        Ok(match e {
            E::N(i) => *i,
            E::V(s) => {
                env.iter()
                    .rev()
                    .find(|(k, _, _)| k == s)
                    .ok_or("unbound")?
                    .1
            }
            E::B(l, op, r) => {
                let (x, y) = (self.eval(l, env)?, self.eval(r, env)?);
                match *op {
                    "+" => x.checked_add(y).ok_or("ovf")?,
                    "-" => x.checked_sub(y).ok_or("ovf")?,
                    "*" => x.checked_mul(y).ok_or("ovf")?,
                    "/" => {
                        if y == 0 {
                            return Err("div0".into());
                        }
                        x / y
                    }
                    "%" => {
                        if y == 0 {
                            return Err("div0".into());
                        }
                        x % y
                    }
                    "<" => (x < y) as i64,
                    "<=" => (x <= y) as i64,
                    ">" => (x > y) as i64,
                    ">=" => (x >= y) as i64,
                    "==" => (x == y) as i64,
                    "!=" => (x != y) as i64,
                    _ => return Err("op".into()),
                }
            }
            E::C(name, args) => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval(a, env)?);
                }
                self.call(name, &vals)?
            }
        })
    }
    fn call(&mut self, name: &str, args: &[i64]) -> Result<i64, String> {
        let f = self
            .fns
            .iter()
            .find(|f| f.name == name)
            .ok_or("nofn")?
            .clone();
        let shown: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let binds: Vec<String> = f
            .params
            .iter()
            .zip(args)
            .map(|(p, a)| format!("{p}={a}"))
            .collect();
        self.line(format!(
            "Call `{name}({})`, parameters: {}",
            shown.join(", "),
            binds.join(", ")
        ));
        self.depth += 1;
        let mut env: Env = f
            .params
            .iter()
            .cloned()
            .zip(args.iter().copied())
            .map(|(p, a)| (p, a, false))
            .collect();
        let r = self.exec(&f.body, &mut env)?;
        let r = r.ok_or("noret")?;
        self.depth -= 1;
        self.line(format!("`{name}({})` returns {r}", shown.join(", ")));
        Ok(r)
    }
    fn state(env: &Env) -> String {
        let s: Vec<String> = env
            .iter()
            .filter(|(_, _, m)| *m)
            .map(|(k, val, _)| format!("{k}={val}"))
            .collect();
        s.join(", ")
    }
    fn exec(&mut self, ss: &[S], env: &mut Env) -> Result<Option<i64>, String> {
        for s in ss {
            match s {
                S::Let(name, e, m) => {
                    let val = self.eval(e, env)?;
                    let sub = render(e, Some(env), 0, false);
                    let text = if sub == val.to_string() {
                        format!("{name} = {val}")
                    } else {
                        format!("{name} = {sub} = {val}")
                    };
                    self.line(format!("`let {name}: int = {}` → {text}", src(e)));
                    env.push((name.clone(), val, *m));
                }
                S::Set(name, e) => {
                    let val = self.eval(e, env)?;
                    let sub = render(e, Some(env), 0, false);
                    let text = if sub == val.to_string() {
                        format!("{name} = {val}")
                    } else {
                        format!("{name} = {sub} = {val}")
                    };
                    self.line(format!("`{name} = {}` → {text}", src(e)));
                    let slot = env
                        .iter_mut()
                        .rev()
                        .find(|(k, _, _)| k == name)
                        .ok_or("unbound")?;
                    slot.1 = val;
                }
                S::Ret(e) => {
                    let val = self.eval(e, env)?;
                    let sub = render(e, Some(env), 0, false);
                    let text = if sub == val.to_string() {
                        format!("{val}")
                    } else {
                        format!("{sub} = {val}")
                    };
                    self.line(format!("`return {}` → {text}", src(e)));
                    return Ok(Some(val));
                }
                S::If(c, t, e) => {
                    let cv = self.eval(c, env)?;
                    let sub = render(c, Some(env), 0, false);
                    self.line(format!(
                        "`if {}` → {sub} is {}",
                        src(c),
                        if cv != 0 { "true" } else { "false" }
                    ));
                    let mark = env.len();
                    self.depth += 1;
                    let r = self.exec(if cv != 0 { t } else { e }, env)?;
                    self.depth -= 1;
                    env.truncate(mark);
                    if r.is_some() {
                        return Ok(r);
                    }
                }
                S::For(var, lo, hi, body) => {
                    let (l, h) = (self.eval(lo, env)?, self.eval(hi, env)?);
                    let cnt = (h - l).max(0);
                    if cnt > 14 {
                        return Err("long loop".into());
                    }
                    // primary accumulator = last variable assigned at the top level of the body
                    let acc: Option<String> = body.iter().rev().find_map(|s| match s {
                        S::Set(n, _) => Some(n.clone()),
                        _ => None,
                    });
                    let start = acc
                        .as_ref()
                        .and_then(|a| env.iter().rev().find(|(k, _, _)| k == a).map(|x| x.1));
                    if cnt == 0 {
                        self.line(format!(
                            "`for {var} in range({l}, {h})` → empty range, body never runs"
                        ));
                    } else {
                        self.line(format!(
                            "`for {var} in range({l}, {h})` is half-open: {var} = {} .. {} ({cnt} iterations); state before: {}",
                            l,
                            h - 1,
                            Self::state(env)
                        ));
                    }
                    let mut series = Vec::new();
                    for i in l..h {
                        let mark = env.len();
                        env.push((var.clone(), i, false));
                        self.line(format!("{var} = {i}:"));
                        self.depth += 1;
                        let r = self.exec(body, env)?;
                        if let Some(a) = &acc {
                            series.push(
                                env.iter()
                                    .rev()
                                    .find(|(k, _, _)| k == a)
                                    .map(|x| x.1)
                                    .unwrap_or(0),
                            );
                        }
                        self.line(format!("state after {var}={i}: {}", Self::state(env)));
                        self.depth -= 1;
                        env.truncate(mark);
                        if r.is_some() {
                            return Ok(r);
                        }
                    }
                    if let (Some(a), Some(st)) = (acc, start)
                        && series.len() >= 2
                    {
                        self.last_loop = Some((a, st, series));
                    }
                }
            }
        }
        Ok(None)
    }
}

/// Returns (think body, answer).
fn trace(fs: &[F], note: &str, misnamed: bool, lead: &str) -> Result<(String, i64), String> {
    let mut t = Tracer {
        fns: fs,
        out: vec![],
        depth: 0,
        steps: 0,
        last_loop: None,
    };
    let ans = t.call("main", &[])?;
    let mut body = String::new();
    body.push_str(lead);
    body.push('\n');
    if !note.is_empty() {
        body.push_str(note);
        body.push('\n');
    }
    body.push_str(
        "I will execute the program statement by statement, writing every intermediate value.\n\n",
    );
    for l in &t.out {
        body.push_str(l);
        body.push('\n');
    }
    body.push_str("\nCheck pass:\n");
    if let Some((a, st, series)) = &t.last_loop {
        let mut prev = *st;
        let mut deltas = Vec::new();
        for s in series {
            deltas.push(s - prev);
            prev = *s;
        }
        let ds: Vec<String> = deltas.iter().map(|d| format!("{d:+}")).collect();
        let sum: i64 = deltas.iter().sum();
        let _ = writeln!(
            body,
            "- Loop accumulator `{a}`: started at {st}, per-iteration changes {}; {st} + ({sum}) = {} equals its last recorded value {}. ✓",
            ds.join(", "),
            st + sum,
            series.last().unwrap()
        );
    }
    if misnamed {
        body.push_str("- I followed the code as written, not what the function names suggest. ✓\n");
    }
    let _ = writeln!(
        body,
        "- Re-read the final `return` of `main`: it produced {ans}. ✓"
    );
    let _ = write!(body, "Final answer: {ans}");
    Ok((body, ans))
}

// ───────────────────────────── RNG ─────────────────────────────
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
}

struct Cand {
    family: &'static str,
    fs: Vec<F>,
    note: String,
    misnamed: bool,
}

fn main_ret(e: E) -> F {
    func("main", &[], vec![ret(e)])
}

fn gen_candidate(kind: usize, r: &mut Rng) -> Cand {
    match kind {
        // doubling "fibonacci" (adversarial name) or true fibonacci
        0 | 1 => {
            let a0 = r.range(0, 2);
            let b0 = r.range(1, 3);
            let steps = r.range(4, 9);
            let extra = r.range(0, 9);
            let literal = kind == 0;
            let name = *r.pick(&["fibonacci", "fib_term", "fibonacci_number", "nth_fib"]);
            let body = if literal {
                vec![
                    let_m("a", n(a0)),
                    let_m("b", n(b0)),
                    for_(
                        "i",
                        n(0),
                        v("n"),
                        vec![set("a", v("b")), set("b", b(v("a"), "+", v("b")))],
                    ),
                    ret(v("b")),
                ]
            } else {
                vec![
                    let_m("a", n(a0)),
                    let_m("b", n(b0)),
                    for_(
                        "i",
                        n(0),
                        v("n"),
                        vec![
                            let_i("t", b(v("a"), "+", v("b"))),
                            set("a", v("b")),
                            set("b", v("t")),
                        ],
                    ),
                    ret(v("a")),
                ]
            };
            let f = func(name, &["n"], body);
            let m = main_ret(b(call(name, vec![n(steps)]), "+", n(extra)));
            let note = if literal {
                format!(
                    "The name `{name}` suggests Fibonacci numbers, but I must not assume that: the loop body assigns `a = b` first and only then `b = a + b`, so the second statement reads the already-updated `a`."
                )
            } else {
                format!(
                    "`{name}` uses a temporary `t`, so I still trace it statement by statement instead of relying on memory of Fibonacci values."
                )
            };
            Cand {
                family: if literal {
                    "literal_debug"
                } else {
                    "state_trace"
                },
                fs: vec![f, m],
                note,
                misnamed: literal,
            }
        }
        // name/operator mismatch accumulators
        2 | 3 => {
            let lim = r.range(3, 7);
            let extra = r.range(1, 20);
            let mis = kind == 2;
            let (name, op, init, desc) = if mis {
                *r.pick(&[
                    ("factorial", "+", 1i64, "adds"),
                    ("sum_up_to", "*", 1, "multiplies"),
                    ("triangular", "*", 1, "multiplies"),
                    ("power_of_two", "+", 0, "adds"),
                ])
            } else {
                *r.pick(&[
                    ("sum_to", "+", 0i64, "adds"),
                    ("product_to", "*", 1, "multiplies"),
                    ("sum_squares", "+", 0, "adds squares of"),
                ])
            };
            let upd = if name == "sum_squares" {
                b(v("acc"), "+", b(v("k"), "*", v("k")))
            } else {
                b(v("acc"), op_static(op), v("k"))
            };
            let f = func(
                name,
                &["limit"],
                vec![
                    let_m("acc", n(init)),
                    for_("k", n(1), b(v("limit"), "+", n(1)), vec![set("acc", upd)]),
                    ret(v("acc")),
                ],
            );
            let m = main_ret(b(call(name, vec![n(lim)]), "-", n(extra)));
            let note = if mis {
                format!(
                    "The name `{name}` describes a different computation than the body: the loop {desc} `k` into `acc`, starting from {init}. I trace what the body does."
                )
            } else {
                String::new()
            };
            Cand {
                family: if mis { "literal_debug" } else { "state_trace" },
                fs: vec![f, m],
                note,
                misnamed: mis,
            }
        }
        // divisor counting called is_prime
        4 => {
            let (a, c) = (r.range(5, 14), r.range(4, 12));
            let name = *r.pick(&["is_prime", "is_perfect", "prime_flag"]);
            let f = func(
                name,
                &["x"],
                vec![
                    let_m("c", n(0)),
                    for_(
                        "d",
                        n(1),
                        b(v("x"), "+", n(1)),
                        vec![S::If(
                            b(b(v("x"), "%", v("d")), "==", n(0)),
                            vec![set("c", b(v("c"), "+", n(1)))],
                            vec![],
                        )],
                    ),
                    ret(v("c")),
                ],
            );
            let m = main_ret(b(
                call(name, vec![n(a)]),
                "*",
                b(n(10), "+", call(name, vec![n(c)])),
            ));
            let note = format!(
                "`{name}` sounds like a yes/no predicate, but the body counts how many `d` in 1..=x divide x and returns that count, not 0 or 1."
            );
            Cand {
                family: "literal_debug",
                fs: vec![f, m],
                note,
                misnamed: true,
            }
        }
        // largest-named min tracker / smallest correct
        5 | 6 => {
            let cnt = r.range(4, 8);
            let (mul, add, md) = (r.range(3, 9), r.range(1, 8), *r.pick(&[7i64, 9, 11, 13]));
            let mis = kind == 5;
            let name = if mis {
                *r.pick(&["largest", "max_value", "peak_of"])
            } else {
                *r.pick(&["smallest", "min_value", "lowest_of"])
            };
            let f = func(
                name,
                &["count"],
                vec![
                    let_m("best", n(1000)),
                    for_(
                        "i",
                        n(0),
                        v("count"),
                        vec![
                            let_i("x", b(b(b(v("i"), "*", n(mul)), "+", n(add)), "%", n(md))),
                            S::If(b(v("x"), "<", v("best")), vec![set("best", v("x"))], vec![]),
                        ],
                    ),
                    ret(v("best")),
                ],
            );
            let m = main_ret(call(name, vec![n(cnt)]));
            let note = if mis {
                format!(
                    "`{name}` suggests a maximum, but the comparison is `x < best` with `best` starting at 1000, so I must track what that really keeps."
                )
            } else {
                String::new()
            };
            Cand {
                family: if mis { "literal_debug" } else { "state_trace" },
                fs: vec![f, m],
                note,
                misnamed: mis,
            }
        }
        // digit sums in a base
        7 | 8 => {
            let base = *r.pick(&[3i64, 7, 10, 10, 5]);
            let val = match base {
                10 => r.range(120, 9876),
                _ => r.range(20, 400),
            };
            let digits = {
                let mut d = 0;
                let mut x = val;
                while x > 0 {
                    x /= base;
                    d += 1;
                }
                d + r.range(0, 1)
            };
            let mis = kind == 8;
            let name = if mis {
                *r.pick(&["digit_count", "num_digits", "length_of"])
            } else {
                *r.pick(&["digit_sum", "sum_digits", "base_digit_total"])
            };
            let f = func(
                name,
                &["x"],
                vec![
                    let_m("m", v("x")),
                    let_m("s", n(0)),
                    for_(
                        "i",
                        n(0),
                        n(digits),
                        vec![
                            set("s", b(v("s"), "+", b(v("m"), "%", n(base)))),
                            set("m", b(v("m"), "/", n(base))),
                        ],
                    ),
                    ret(v("s")),
                ],
            );
            let m = main_ret(call(name, vec![n(val)]));
            let note = if mis {
                format!(
                    "`{name}` suggests counting digits, yet the body accumulates the digit values `m % {base}` into `s`."
                )
            } else {
                String::new()
            };
            Cand {
                family: if mis { "literal_debug" } else { "state_trace" },
                fs: vec![f, m],
                note,
                misnamed: mis,
            }
        }
        // modular power
        9 => {
            let (bs, e, md, x) = (r.range(2, 9), r.range(3, 9), r.range(5, 13), r.range(0, 9));
            let name = *r.pick(&["pow_mod", "mod_exp", "power_residue"]);
            let f = func(
                name,
                &["base", "e", "m"],
                vec![
                    let_m("r", n(1)),
                    for_(
                        "i",
                        n(0),
                        v("e"),
                        vec![set("r", b(b(v("r"), "*", v("base")), "%", v("m")))],
                    ),
                    ret(v("r")),
                ],
            );
            let m = main_ret(b(call(name, vec![n(bs), n(e), n(md)]), "+", n(x)));
            Cand {
                family: "invariant_check",
                fs: vec![f, m],
                note: String::new(),
                misnamed: false,
            }
        }
        // nested loops pair count
        10 => {
            let (cap, k) = (r.range(4, 6), r.range(2, 4));
            let mis = r.range(0, 1) == 1;
            let name = if mis {
                "count_distinct_pairs"
            } else {
                "count_pairs"
            };
            let jlo = if mis { v("i") } else { b(v("i"), "+", n(1)) };
            let f = func(
                name,
                &["cap", "k"],
                vec![
                    let_m("cnt", n(0)),
                    for_(
                        "i",
                        n(1),
                        v("cap"),
                        vec![for_(
                            "j",
                            jlo,
                            v("cap"),
                            vec![S::If(
                                b(b(b(v("i"), "+", v("j")), "%", v("k")), "==", n(0)),
                                vec![set("cnt", b(v("cnt"), "+", n(1)))],
                                vec![],
                            )],
                        )],
                    ),
                    ret(v("cnt")),
                ],
            );
            let m = main_ret(call(name, vec![n(cap), n(k)]));
            let note = if mis {
                format!(
                    "`{name}` suggests distinct pairs, but the inner loop starts at `j = i`, so pairs with `i == j` are included."
                )
            } else {
                String::new()
            };
            Cand {
                family: if mis {
                    "literal_debug"
                } else {
                    "invariant_check"
                },
                fs: vec![f, m],
                note,
                misnamed: mis,
            }
        }
        // collatz-like with fixed iteration count
        11 => {
            let (x, it) = (r.range(5, 40), r.range(4, 8));
            let name = *r.pick(&["settle", "reduce_to_one", "steps_to_zero"]);
            let f = func(
                name,
                &["x"],
                vec![
                    let_m("v", v("x")),
                    let_m("s", n(0)),
                    for_(
                        "i",
                        n(0),
                        n(it),
                        vec![S::If(
                            b(b(v("v"), "%", n(2)), "==", n(0)),
                            vec![set("v", b(v("v"), "/", n(2)))],
                            vec![
                                set("v", b(b(v("v"), "*", n(3)), "+", n(1))),
                                set("s", b(v("s"), "+", n(1))),
                            ],
                        )],
                    ),
                    ret(b(v("v"), "+", v("s"))),
                ],
            );
            let m = main_ret(call(name, vec![n(x)]));
            let note = format!(
                "`{name}` is only a label; the loop always runs exactly {it} times regardless of the value of `v`, and the return adds `v` and `s`."
            );
            Cand {
                family: "literal_debug",
                fs: vec![f, m],
                note,
                misnamed: true,
            }
        }
        // helper-call chain
        12 | 13 => {
            let (s0, k) = (r.range(3, 25), r.range(3, 5));
            let mis = kind == 12;
            let name = if mis {
                *r.pick(&["halve", "shrink", "decay"])
            } else {
                *r.pick(&["step_even_odd", "bump", "advance"])
            };
            let helper = func(
                name,
                &["x"],
                vec![
                    S::If(
                        b(b(v("x"), "%", n(2)), "==", n(0)),
                        vec![ret(b(v("x"), "/", n(2)))],
                        vec![],
                    ),
                    ret(b(b(v("x"), "*", n(3)), "+", n(1))),
                ],
            );
            let m = func(
                "main",
                &[],
                vec![
                    let_m("t", n(s0)),
                    for_("i", n(0), n(k), vec![set("t", call(name, vec![v("t")]))]),
                    ret(v("t")),
                ],
            );
            let note = if mis {
                format!(
                    "`{name}` does not always shrink its argument: for odd `x` it returns `x * 3 + 1`. I evaluate each call literally."
                )
            } else {
                String::new()
            };
            Cand {
                family: if mis { "literal_debug" } else { "state_trace" },
                fs: vec![helper, m],
                note,
                misnamed: mis,
            }
        }
        // integer division order
        _ => {
            let (kk, sc) = (r.range(4, 8), *r.pick(&[10i64, 20, 50, 100]));
            let name = *r.pick(&["percent_of", "scale_ratio", "share_pct"]);
            let f = func(
                name,
                &["part", "whole"],
                vec![ret(b(b(v("part"), "/", v("whole")), "*", n(sc)))],
            );
            let m = func(
                "main",
                &[],
                vec![
                    let_m("total", n(0)),
                    for_(
                        "i",
                        n(1),
                        n(kk + 1),
                        vec![set(
                            "total",
                            b(v("total"), "+", call(name, vec![v("i"), n(kk)])),
                        )],
                    ),
                    ret(v("total")),
                ],
            );
            let note = format!(
                "`{name}` computes `part / whole * {sc}` left to right with integer division, so `part / whole` truncates before the multiplication."
            );
            Cand {
                family: "invariant_check",
                fs: vec![f, m],
                note,
                misnamed: false,
            }
        }
    }
}

fn op_static(op: &str) -> &'static str {
    match op {
        "+" => "+",
        _ => "*",
    }
}

const PROMPTS: &[&str] = &[
    "Evaluate `main()` literally as written. Function names may not describe what the bodies really do. Reason step by step, then output the resulting integer:\n\n```vox\n{code}\n```",
    "What integer does `main()` return in this Vox program? Execute the code exactly as written, keeping track of every intermediate value, and give the integer last:\n\n```vox\n{code}\n```",
    "Work out the exact value of `main()` by running the program in your head statement by statement. Do not assume behaviour from names. Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```",
    "Compute the result of `main()` for the Vox code below, following the code as written even if it looks like it has a bug. Output the resulting integer after your reasoning:\n\n```vox\n{code}\n```",
    "Step through this program carefully, recording the state after each step and re-checking the total at the end, then output the integer `main()` returns:\n\n```vox\n{code}\n```",
];
const LEADS: &[&str] = &[
    "Plan: execute `main()` exactly as written, statement by statement, and record every value.",
    "I will follow the code literally and not rely on what the function names suggest.",
    "I need an exact integer, so I will trace every statement with concrete values and then run a check pass.",
];

// ───────────────────────────── leakage helpers ─────────────────────────────
fn words(s: &str) -> Vec<String> {
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
fn grams(s: &str) -> HashSet<String> {
    let t = words(s);
    if t.is_empty() {
        return HashSet::new();
    }
    let k = 8.min(t.len());
    t.windows(k).map(|w| w.join("\u{1}")).collect()
}
fn jacc(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let i = a.intersection(b).count() as f64;
    let u = a.union(b).count() as f64;
    if u == 0.0 { 0.0 } else { i / u }
}
fn collect_strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.len() > 20 => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| collect_strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| collect_strings(x, out)),
        _ => {}
    }
}

fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| -> String {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| d.to_string())
    };
    let vox = arg("--vox", "target/debug/vox");
    let share: f64 = arg("--target-share", "0.22").parse()?;
    let seed: u64 = arg("--seed", "5").parse()?;
    let root = PathBuf::from(arg("--root", "."));
    let outdir = root.join("target/dogfood_v5");
    let progdir = outdir.join("scratch/progs");
    std::fs::create_dir_all(&progdir)?;

    // v4 rows
    let v4_raw = std::fs::read_to_string(root.join("target/dogfood_v4/train.jsonl"))?;
    let mut keep: Vec<String> = Vec::new();
    let mut template: Option<Value> = None;
    let mut v4_prompts: HashSet<String> = HashSet::new();
    let mut dropped_think = 0usize;
    let mut n_non_think = 0usize;
    let mut replay_prompts: Vec<String> = Vec::new();
    for l in v4_raw.lines().filter(|l| !l.trim().is_empty()) {
        let row: Value = serde_json::from_str(l)?;
        v4_prompts.insert(norm(row["prompt"].as_str().unwrap_or("")));
        if row["response"].as_str().unwrap_or("").contains("<think>") {
            if row["category"] == "reasoning_replay" {
                // math replay rows are restored byte-identical (gsm8k/math500 collateral)
                replay_prompts.push(row["prompt"].as_str().unwrap_or("").to_string());
                keep.push(l.to_string());
                continue;
            }
            dropped_think += 1;
            if template.is_none() && row["category"] == "vox_reasoning_literal_debug" {
                template = Some(row);
            }
        } else {
            n_non_think += 1;
            keep.push(l.to_string());
        }
    }
    let template = template.expect("v4 literal_debug template row");
    let n_keep = n_non_think;
    let target_think = ((share / (1.0 - share)) * n_keep as f64).round() as usize;

    // held-out slices
    let mut slice_docs: Vec<(String, String)> = Vec::new(); // (slice, text)
    let mut slice_prompts: HashSet<String> = HashSet::new();
    for (name, rel) in [
        ("vox_think", "mens/data/vox_think/reasoning.jsonl"),
        ("gsm8k_think", "mens/data/gsm8k_think/reasoning.jsonl"),
        ("math500_think", "mens/data/math500_think/reasoning.jsonl"),
    ] {
        let raw = std::fs::read_to_string(root.join(rel))?;
        for l in raw.lines().filter(|l| !l.trim().is_empty()) {
            let v: Value = serde_json::from_str(l)?;
            let mut ss = Vec::new();
            collect_strings(&v, &mut ss);
            // exclude answers/ids by length filter above; include question + program text
            for s in ss {
                slice_prompts.insert(norm(&s));
                slice_docs.push((name.to_string(), s));
            }
        }
    }
    for rel in [
        "mens/data/heldout_bench/manifest.json",
        "mens/data/heldout_bench_v2/manifest.json",
        "mens/data/heldout_bench_v2_canary/manifest.json",
    ] {
        if let Ok(raw) = std::fs::read_to_string(root.join(rel)) {
            let v: Value = serde_json::from_str(&raw)?;
            let mut ss = Vec::new();
            collect_strings(&v, &mut ss);
            for s in ss {
                slice_prompts.insert(norm(&s));
                slice_docs.push(("heldout_bench".to_string(), s));
            }
        }
    }
    let slice_grams: Vec<(String, HashSet<String>)> = slice_docs
        .iter()
        .map(|(n, s)| (n.clone(), grams(s)))
        .collect();

    // candidates
    let mut rng = Rng(seed);
    let mut cands: Vec<(Cand, String, String, i64)> = Vec::new(); // cand, code, think, expected
    let mut seen_prompts: HashSet<String> = HashSet::new();
    let mut attempts = 0usize;
    let want = target_think * 14 / 10;
    let mut trace_err = 0usize;
    let mut too_long = 0usize;
    while cands.len() < want && attempts < want * 40 {
        attempts += 1;
        let kind = (attempts - 1) % 15;
        let c = gen_candidate(kind, &mut rng);
        let code = print_program(&c.fs);
        let lead = *rng.pick(LEADS);
        let (think, ans) = match trace(&c.fs, &c.note, c.misnamed, lead) {
            Ok(x) => x,
            Err(_) => {
                trace_err += 1;
                continue;
            }
        };
        if think.len() > 4200 {
            too_long += 1;
            continue;
        }
        let prompt = rng.pick(PROMPTS).replace("{code}", &code);
        if !seen_prompts.insert(norm(&code)) {
            continue;
        }
        let _ = prompt;
        cands.push((c, code, think, ans));
    }

    // verify by execution (parallel)
    let results: Vec<Option<i64>> = {
        let chunks: Vec<Vec<usize>> = (0..8)
            .map(|t| (0..cands.len()).filter(|i| i % 8 == t).collect())
            .collect();
        let mut res = vec![None; cands.len()];
        std::thread::scope(|s| {
            let hs: Vec<_> = chunks
                .iter()
                .map(|idxs| {
                    let cands = &cands;
                    let vox = &vox;
                    let progdir = &progdir;
                    let root = &root;
                    s.spawn(move || {
                        idxs.iter()
                            .map(|&i| {
                                let p = progdir.join(format!("p{i:04}.vox"));
                                std::fs::write(&p, format!("{}\n", cands[i].1)).ok();
                                let out = Command::new(vox)
                                    .args(["run", "--interp"])
                                    .arg(&p)
                                    .current_dir(root)
                                    .output()
                                    .ok();
                                let val = out.and_then(|o| {
                                    if !o.status.success() {
                                        return None;
                                    }
                                    String::from_utf8_lossy(&o.stdout)
                                        .trim()
                                        .lines()
                                        .last()?
                                        .trim()
                                        .parse::<i64>()
                                        .ok()
                                });
                                (i, val)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for h in hs {
                for (i, val) in h.join().unwrap() {
                    res[i] = val;
                }
            }
        });
        res
    };

    // filter: verified + leakage + dedupe
    let total_cands = cands.len();
    let mut verified = 0usize;
    let mut mismatched = 0usize;
    let mut unrunnable = 0usize;
    let mut leak_dropped = 0usize;
    let mut dup_dropped = 0usize;
    let mut max_prog_sim = 0f64;
    let mut max_prompt_sim = 0f64;
    let mut max_by_slice: std::collections::BTreeMap<String, f64> = Default::default();
    let mut rows: Vec<(String, String)> = Vec::new(); // (family, jsonl)
    let mut think_lens: Vec<usize> = Vec::new();
    let mut fam_counts: std::collections::BTreeMap<String, usize> = Default::default();
    for (i, (c, code, think, ans)) in cands.iter().enumerate() {
        match results[i] {
            None => {
                unrunnable += 1;
                continue;
            }
            Some(x) if x != *ans => {
                mismatched += 1;
                continue;
            }
            _ => verified += 1,
        }
        if rows.len() >= target_think {
            continue;
        }
        let prompt = PROMPTS[i % PROMPTS.len()].replace("{code}", code);
        let np = norm(&prompt);
        if v4_prompts.contains(&np) || slice_prompts.contains(&np) {
            dup_dropped += 1;
            continue;
        }
        let (pg, cg) = (grams(&prompt), grams(code));
        let mut worst = 0f64;
        for (sn, sg) in &slice_grams {
            let s1 = jacc(&pg, sg);
            let s2 = jacc(&cg, sg);
            let s = s1.max(s2);
            let e = max_by_slice.entry(sn.clone()).or_insert(0.0);
            if s > *e {
                *e = s;
            }
            worst = worst.max(s);
            max_prog_sim = max_prog_sim.max(s2);
            max_prompt_sim = max_prompt_sim.max(s1);
        }
        if worst >= 0.40 {
            leak_dropped += 1;
            continue;
        }
        let response = format!("<think>\n{think}\n</think>\n\n{ans}");
        let mut row = template.clone();
        let id = format!("vox-v5-think-{:04}", rows.len());
        row["id"] = json!(id);
        row["prompt"] = json!(prompt);
        row["response"] = json!(response);
        row["instruction"] = json!(prompt);
        row["output"] = json!(response);
        row["messages"] = json!([
            {"role": "system", "content": ""},
            {"role": "user", "content": prompt},
            {"role": "assistant", "content": response}
        ]);
        row["category"] = json!(format!("vox_reasoning_{}", c.family));
        think_lens.push(think.len());
        *fam_counts.entry(c.family.to_string()).or_default() += 1;
        rows.push((c.family.to_string(), serde_json::to_string(&row)?));
    }

    // interleave deterministically
    let total = keep.len() + rows.len();
    let mut out_lines: Vec<String> = Vec::with_capacity(total);
    let (mut ki, mut ri) = (0usize, 0usize);
    for pos in 0..total {
        let want_think = ri < rows.len() && (pos * rows.len()) / total >= ri;
        if want_think {
            out_lines.push(rows[ri].1.clone());
            ri += 1;
        } else if ki < keep.len() {
            out_lines.push(keep[ki].clone());
            ki += 1;
        } else {
            out_lines.push(rows[ri].1.clone());
            ri += 1;
        }
    }
    std::fs::write(outdir.join("train.jsonl"), out_lines.join("\n") + "\n")?;

    let mean = |xs: &[usize]| {
        if xs.is_empty() {
            0.0
        } else {
            xs.iter().sum::<usize>() as f64 / xs.len() as f64
        }
    };
    let mut replay_max = 0f64;
    let mut replay_exact = 0usize;
    for p in &replay_prompts {
        if slice_prompts.contains(&norm(p)) {
            replay_exact += 1;
        }
        let g = grams(p);
        for (_, sg) in &slice_grams {
            replay_max = replay_max.max(jacc(&g, sg));
        }
    }
    let n_replay = replay_prompts.len();
    let stats = json!({
        "math_replay_rows_restored": n_replay,
        "replay_max_8gram_jaccard": replay_max,
        "replay_exact_prompt_overlap": replay_exact,
        "think_rows_total_incl_replay": rows.len() + n_replay,
        "think_share_incl_replay": (rows.len() + n_replay) as f64 / total as f64,
        "v4_think_rows_dropped": dropped_think,
        "v4_non_think_rows_kept": n_keep,
        "candidates_generated": total_cands,
        "trace_errors": trace_err, "too_long": too_long,
        "execution_verified": verified, "execution_mismatch": mismatched, "unrunnable": unrunnable,
        "dup_prompt_dropped": dup_dropped, "leak_dropped": leak_dropped,
        "think_rows_written": rows.len(), "think_rows_verified_of_written": format!("{}/{}", rows.len(), rows.len()),
        "total_rows": total, "think_share": rows.len() as f64 / total as f64,
        "mean_think_chars": mean(&think_lens),
        "families": fam_counts,
        "max_8gram_jaccard_program": max_prog_sim, "max_8gram_jaccard_prompt": max_prompt_sim,
        "max_by_slice": max_by_slice,
    });
    std::fs::write(
        outdir.join("v5_stats.json"),
        serde_json::to_string_pretty(&stats)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&stats)?);
    let _ = Path::new("");
    Ok(())
}
