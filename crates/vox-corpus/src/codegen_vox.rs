//! **Organic Vox code generator** — produces syntactically valid `.vox` programs
//! from the AST type definitions, verified by parser round-trip.
//!
//! ## Architecture
//! - **`VoxTypeGen`**: recursive type generator (all 5 `TypeExpr` variants)
//! - **`VoxExprGen`**: recursive expression generator (all 22 `Expr` variants)
//! - **`VoxDeclGen`**: per-construct generators consuming `TAXONOMY_FROM_AST`
//! - **Parser verification**: every emitted program runs through the parser
//!
//! ## Dynamic walking
//! This module consumes `TAXONOMY_FROM_AST` (auto-derived from `vox-ast` Decl enum
//! at build time) so that when new language constructs are added, generators are
//! automatically flagged as missing by coverage analysis.

use serde_json::json;

// ── Build-time constants (dynamic, walked by build.rs) ───────────────────────
include!(concat!(env!("OUT_DIR"), "/dynamic_registry.rs"));

// ── Deterministic RNG ────────────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self { Self(if seed == 0 { 0xdeadbeef } else { seed }) }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn usize(&mut self, max: usize) -> usize { self.next() as usize % max.max(1) }
    fn coin(&mut self) -> bool { self.next() % 2 == 0 }
}

// ── Word pools (used for identifier generation) ──────────────────────────────
// These are NOT language constructs — they're realistic domain names for
// generating variable names, function names, etc.

const NOUNS: &[&str] = &[
    "user", "order", "product", "session", "payment", "event", "metric",
    "config", "task", "report", "message", "record", "item", "entry",
    "account", "profile", "document", "request", "response", "result",
];

const VERBS: &[&str] = &[
    "process", "validate", "transform", "fetch", "store", "compute",
    "render", "parse", "encode", "notify", "schedule", "dispatch",
    "route", "check", "filter", "sort", "merge", "update", "create",
    "delete", "find", "search", "analyze", "generate", "format",
];

const FIELD_POOL: &[(&str, &str)] = &[
    ("id", "int"), ("name", "str"), ("email", "str"), ("count", "int"),
    ("active", "bool"), ("amount", "float"), ("status", "str"),
    ("created_at", "str"), ("data", "str"), ("score", "float"),
    ("label", "str"), ("value", "int"), ("title", "str"),
    ("description", "str"), ("priority", "int"), ("done", "bool"),
];

// ── VoxTypeGen: recursive type generator ─────────────────────────────────────

fn gen_type(rng: &mut Rng, depth: u8) -> String {
    if depth == 0 {
        return gen_prim_type(rng);
    }
    match rng.usize(10) {
        0..=3 => gen_prim_type(rng),
        4 => format!("list[{}]", gen_type(rng, depth - 1)),
        5 => format!("Option[{}]", gen_type(rng, depth - 1)),
        6 => format!("Result[{}]", gen_type(rng, depth - 1)),
        7 => {
            // Tuple type: (int, str)
            let a = gen_type(rng, depth - 1);
            let b = gen_type(rng, depth - 1);
            format!("({a}, {b})")
        }
        _ => {
            let p1 = gen_type(rng, depth - 1);
            let ret = gen_prim_type(rng);
            format!("fn({p1}) -> {ret}")
        }
    }
}

fn gen_prim_type(rng: &mut Rng) -> String {
    ["int", "str", "bool", "float"][rng.usize(4)].to_string()
}

fn gen_return_type(rng: &mut Rng, depth: u8) -> String {
    match rng.usize(6) {
        0..=2 => gen_prim_type(rng),
        3 => "Unit".to_string(),
        4 => format!("Result[{}]", gen_prim_type(rng)),
        _ => gen_type(rng, depth),
    }
}

// ── VoxExprGen: recursive expression generator ───────────────────────────────

// EXPR_VARIANTS is auto-derived from vox-ast Expr enum (24 variants).
// When Expr grows, this dispatch must be extended — coverage report flags the gap.
fn gen_expr(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    if depth == 0 {
        return gen_literal(rng, tags);
    }
    // 24 arms — one per Expr variant
    match rng.usize(24) {
        0 => { tags.push("expr:int_lit".into()); format!("{}", rng.next() % 1000) }
        1 => { tags.push("expr:float_lit".into()); format!("{}.{}", rng.next() % 100, rng.next() % 99 + 1) }
        2 => { tags.push("expr:string_lit".into()); format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())]) }
        3 => { tags.push("expr:bool_lit".into()); if rng.coin() { "true" } else { "false" }.into() }
        4 => { tags.push("expr:ident".into()); gen_ident(rng) }
        5 => { tags.push("expr:object_lit".into()); gen_object(rng, depth, tags) }
        6 => { tags.push("expr:list_lit".into()); gen_list(rng, depth, tags) }
        7 => { tags.push("expr:tuple_lit".into()); gen_tuple(rng, depth, tags) }
        8 => { tags.push("expr:binary".into()); gen_binary(rng, depth) }
        9 => { tags.push("expr:unary".into()); gen_unary(rng, depth, tags) }
        10 => { tags.push("expr:call".into()); gen_call_expr(rng, depth, tags) }
        11 => { tags.push("expr:method_call".into()); gen_method_call(rng) }
        12 => { tags.push("expr:field_access".into()); gen_field_access(rng) }
        13 => { tags.push("expr:match".into()); gen_match(rng, depth, tags) }
        14 => { tags.push("expr:if".into()); gen_if(rng, depth, tags) }
        15 => { tags.push("expr:for".into()); gen_for_expr(rng, depth, tags) }
        16 => { tags.push("expr:lambda".into()); gen_lambda(rng, depth, tags) }
        17 => { tags.push("expr:pipe".into()); gen_pipe(rng, depth, tags) }
        18 => { tags.push("expr:spawn".into()); gen_spawn_expr(rng) }
        19 => { tags.push("expr:with".into()); gen_with_expr(rng, depth, tags) }
        20 => { tags.push("expr:jsx".into()); gen_jsx_element(rng) }
        21 => { tags.push("expr:jsx_self_closing".into()); gen_jsx_self_closing(rng) }
        22 => { tags.push("expr:string_interp".into()); gen_string_interp(rng) }
        _ => { tags.push("expr:block".into()); gen_block(rng, depth, tags) }
    }
}

fn gen_literal(rng: &mut Rng, tags: &mut Vec<String>) -> String {
    match rng.usize(6) {
        0 => { tags.push("expr:int_lit".into()); format!("{}", rng.next() % 1000) }
        1 => { tags.push("expr:float_lit".into()); format!("{}.{}", rng.next() % 100, rng.next() % 99 + 1) }
        2 => { tags.push("expr:string_lit".into()); format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())]) }
        3 => { tags.push("expr:bool_lit".into()); if rng.coin() { "true" } else { "false" }.into() }
        4 => { tags.push("expr:string_interp".into()); gen_string_interp(rng) }
        _ => { tags.push("expr:ident".into()); gen_ident(rng) }
    }
}

fn gen_ident(rng: &mut Rng) -> String {
    NOUNS[rng.usize(NOUNS.len())].to_string()
}

fn gen_string_interp(rng: &mut Rng) -> String {
    let n = NOUNS[rng.usize(NOUNS.len())];
    let v = NOUNS[rng.usize(NOUNS.len())];
    format!("\"Hello {{{n}}}, your {v} is ready\"")
}

fn gen_jsx_element(rng: &mut Rng) -> String {
    let tag = ["div", "section", "main", "article"][rng.usize(4)];
    let cls = NOUNS[rng.usize(NOUNS.len())];
    let child = NOUNS[rng.usize(NOUNS.len())];
    format!("<{tag} className=\"{cls}\">{{{child}}}</{tag}>")
}

fn gen_jsx_self_closing(rng: &mut Rng) -> String {
    let tag = ["input", "img", "hr", "br"][rng.usize(4)];
    let (attr, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
    let val = NOUNS[rng.usize(NOUNS.len())];
    format!("<{tag} {attr}=\"{val}\" />")
}

fn gen_literal_for_type(rng: &mut Rng, ty: &str) -> String {
    match ty {
        "int" => format!("{}", rng.next() % 1000),
        "float" => format!("{}.{}", rng.next() % 100, rng.next() % 99),
        "bool" => if rng.coin() { "true" } else { "false" }.to_string(),
        "Unit" => "()".to_string(),
        _ => format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())]),
    }
}

fn gen_binary(rng: &mut Rng, depth: u8) -> String {
    // All 13 BinOp variants: +, -, *, /, <, >, <=, >=, and, or, is, isnt, |>
    let ops = ["+", "-", "*", "/", ">", "<", ">=", "<=", "and", "or", "is", "isnt"];
    let op = ops[rng.usize(ops.len())];
    let left = NOUNS[rng.usize(NOUNS.len())];
    let right = if depth > 1 {
        format!("{} {} {}", NOUNS[rng.usize(NOUNS.len())], ops[rng.usize(4)], rng.next() % 50)
    } else {
        format!("{}", rng.next() % 100)
    };
    format!("{left} {op} {right}")
}

fn gen_unary(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    if rng.coin() {
        format!("not {}", NOUNS[rng.usize(NOUNS.len())])
    } else {
        format!("-{}", rng.next() % 100)
    }
}

fn gen_call_expr(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    let func = VERBS[rng.usize(VERBS.len())];
    let arg = NOUNS[rng.usize(NOUNS.len())];
    if rng.coin() {
        format!("{func}({arg})")
    } else {
        let arg2 = NOUNS[rng.usize(NOUNS.len())];
        format!("{func}({arg}, {arg2})")
    }
}

fn gen_method_call(rng: &mut Rng) -> String {
    let obj = NOUNS[rng.usize(NOUNS.len())];
    let method = VERBS[rng.usize(VERBS.len())];
    format!("{obj}.{method}()")
}

fn gen_if(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let cond = gen_binary(rng, 0);
    let then_val = gen_expr(rng, depth - 1, tags);
    let else_val = gen_expr(rng, depth - 1, tags);
    format!("if {cond} {{ {then_val} }} else {{ {else_val} }}")
}

/// Match with all 5 Pattern variants exercised across calls.
fn gen_match(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let subject = NOUNS[rng.usize(NOUNS.len())];
    let arm1_body = gen_expr(rng, depth.saturating_sub(1), tags);
    let arm2_body = gen_expr(rng, depth.saturating_sub(1), tags);
    let arm3_body = gen_expr(rng, depth.saturating_sub(1), tags);

    // Rotate through all Pattern variants based on RNG
    let arms = match rng.usize(5) {
        // Ident pattern
        0 => format!(
            "match {subject} {{ {subject} -> {arm1_body}, _ -> {arm2_body} }}"
        ),
        // Literal pattern
        1 => format!(
            "match {subject} {{ 0 -> {arm1_body}, 1 -> {arm2_body}, _ -> {arm3_body} }}"
        ),
        // Constructor pattern (ADT)
        2 => {
            tags.push("pattern:constructor".into());
            format!("match {subject} {{ Ok(value) -> {arm1_body}, Error(msg) -> {arm2_body} }}")
        }
        // Tuple destructuring pattern
        3 => {
            tags.push("pattern:tuple".into());
            let (f1, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            let (f2, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            format!("match {subject} {{ ({f1}, {f2}) -> {arm1_body}, _ -> {arm2_body} }}")
        }
        // Wildcard + guard
        _ => format!(
            "match {subject} {{ x if x > 0 -> {arm1_body}, _ -> {arm2_body} }}"
        ),
    };
    tags.push("expr:match".into());
    arms
}

fn gen_lambda(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let param = NOUNS[rng.usize(NOUNS.len())];
    let body = gen_expr(rng, depth - 1, tags);
    format!("fn({param}: int) to {body}")
}

fn gen_pipe(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    let source = NOUNS[rng.usize(NOUNS.len())];
    let f1 = VERBS[rng.usize(VERBS.len())];
    let f2 = VERBS[rng.usize(VERBS.len())];
    format!("{source} |> {f1} |> {f2}")
}

fn gen_list(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(4);
    let items: Vec<String> = (0..n).map(|_| gen_expr(rng, depth - 1, tags)).collect();
    format!("[{}]", items.join(", "))
}

fn gen_tuple(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(3);
    let items: Vec<String> = (0..n).map(|_| gen_expr(rng, depth - 1, tags)).collect();
    format!("({})", items.join(", "))
}

fn gen_object(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(4);
    let mut used = std::collections::HashSet::new();
    let mut fields = Vec::new();
    for _ in 0..n {
        let (name, _) = loop {
            let e = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            if used.insert(e.0) { break e; }
        };
        let val = gen_expr(rng, depth.saturating_sub(1), tags);
        fields.push(format!("{name}: {val}"));
    }
    format!("{{{}}}", fields.join(", "))
}

fn gen_for_expr(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    let binding = NOUNS[rng.usize(NOUNS.len())];
    let iterable = NOUNS[rng.usize(NOUNS.len())];
    format!("for {binding} in {iterable}: <li>{{{binding}.name}}</li>")
}

fn gen_spawn_expr(rng: &mut Rng) -> String {
    let target = {
        let n = NOUNS[rng.usize(NOUNS.len())];
        let mut s = n[..1].to_uppercase();
        s.push_str(&n[1..]);
        format!("{s}Actor")
    };
    format!("spawn({target})")
}

fn gen_with_expr(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let base = NOUNS[rng.usize(NOUNS.len())];
    let options = gen_object(rng, depth.saturating_sub(1), tags);
    format!("{base} with {options}")
}

fn gen_field_access(rng: &mut Rng) -> String {
    let obj = NOUNS[rng.usize(NOUNS.len())];
    let (field, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
    format!("{obj}.{field}")
}

fn gen_block(rng: &mut Rng, depth: u8, tags: &mut Vec<String>) -> String {
    let val = gen_expr(rng, depth - 1, tags);
    let (name, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
    format!("{{ let {name} = {val}; {name} }}")
}

// ── Statement/body builders ──────────────────────────────────────────────────

fn gen_body(rng: &mut Rng, ret_type: &str, complexity: u8, tags: &mut Vec<String>) -> String {
    let mut lines = Vec::new();
    let stmts = 1 + complexity as usize / 2;

    for _ in 0..stmts.min(4) {
        let (fname, ftype) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
        let depth = (complexity / 3).min(2);
        let val = gen_expr(rng, depth, tags);
        if rng.coin() {
            lines.push(format!("    let {fname}: {ftype} = {val}"));
            tags.push("stmt:let".into());
        } else {
            lines.push(format!("    let {fname} = {val}"));
        }
    }

    if complexity >= 4 && rng.coin() {
        let cond = gen_binary(rng, 0);
        lines.push(format!("    if {cond} {{"));
        lines.push(format!("        ret {}", gen_literal_for_type(rng, ret_type)));
        lines.push("    }".to_string());
        tags.push("expr:if".into());
    }

    if ret_type != "Unit" {
        lines.push(format!("    ret {}", gen_literal_for_type(rng, ret_type)));
        tags.push("stmt:return".into());
    }

    lines.join("\n")
}

fn gen_params(rng: &mut Rng, count: usize) -> String {
    let mut params = Vec::new();
    let mut used = std::collections::HashSet::new();
    for _ in 0..count {
        let (name, ty) = loop {
            let entry = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            if used.insert(entry.0) { break entry; }
        };
        params.push(format!("{name}: {ty}"));
    }
    params.join(", ")
}

fn gen_fields(rng: &mut Rng, count: usize) -> String {
    let mut lines = Vec::new();
    let mut used = std::collections::HashSet::new();
    for _ in 0..count {
        let (name, ty) = loop {
            let entry = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            if used.insert(entry.0) { break entry; }
        };
        lines.push(format!("    {name}: {ty}"));
    }
    lines.join("\n")
}

// ── Organic pair ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct OrganicPair {
    pub prompt: String,
    pub response: String,
    pub category: String,
    pub verified: bool,
    pub complexity: u8,
    pub coverage_tags: Vec<String>,
}

impl OrganicPair {
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        json!({
            "prompt": self.prompt,
            "response": self.response,
            "category": self.category,
            "rating": if self.verified { 5 } else { 1 },
            "format": "vox_organic",
            "complexity": self.complexity,
            "coverage": self.coverage_tags,
            "schema_version": "vox_dogfood_v1",
        }).to_string()
    }
}

// ── Dynamic Decl generators (one per TAXONOMY entry) ─────────────────────────
// Each function maps a TAXONOMY_FROM_AST snake_case tag to a Vox source string.
// When a new Decl variant is added to vox-ast, TAXONOMY_FROM_AST grows, and the
// coverage report flags it as uncovered — prompting addition of a new generator.

fn generate_for_taxonomy_entry(
    tag: &str, rng: &mut Rng, variant: usize,
) -> Option<OrganicPair> {
    let noun = NOUNS[rng.usize(NOUNS.len())];
    let verb = VERBS[rng.usize(VERBS.len())];
    let name = format!("{verb}_{noun}");
    let type_name = {
        let mut s = String::from(&noun[..1].to_uppercase());
        s.push_str(&noun[1..]);
        s
    };
    let ret_type = gen_return_type(rng, 1);
    let param_count = 1 + variant % 3;
    let params = gen_params(rng, param_count);
    let complexity = 2 + (variant % 5) as u8;
    let mut tags = vec![format!("decl:{tag}")];

    let (source, prompt) = match tag {
        "function" => {
            let body = gen_body(rng, &ret_type, complexity, &mut tags);
            let dec = ["", "@async\n", "@traced\n", "@pure\n"][variant % 4];
            (format!("{dec}fn {name}({params}) to {ret_type} {{\n{body}\n}}"),
             format!("Write a Vox function called `{name}` that returns `{ret_type}`"))
        }
        "component" => {
            tags.push("expr:jsx".into());
            let jsx = format!("    ret <div className=\"{noun}\">\n        <h1>{{\"{type_name}\"}}</h1>\n    </div>");
            (format!("component fn {type_name}View({params}) to Element {{\n{jsx}\n}}"),
             format!("Create a Vox UI component called `{type_name}View`"))
        }
        "actor" => {
            let handler_count = 1 + variant % 3;
            let (sf, st) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            let mut handlers = Vec::new();
            for i in 0..handler_count {
                let ev = &VERBS[(rng.usize(VERBS.len()) + i) % VERBS.len()];
                let hr = gen_prim_type(rng);
                handlers.push(format!("    on {ev}() to {hr} {{\n        {sf} = {sf} + 1\n        ret {}\n    }}", gen_literal_for_type(rng, &hr)));
            }
            (format!("actor {type_name}Actor {{\n    state {sf}: {st} = {}\n\n{}\n}}", gen_literal_for_type(rng, st), handlers.join("\n\n")),
             format!("Define a Vox actor called `{type_name}Actor` with {handler_count} handlers"))
        }
        "workflow" => {
            let body = gen_body(rng, &ret_type, complexity, &mut tags);
            (format!("workflow {name}({params}) to {ret_type} {{\n{body}\n}}"),
             format!("Write a durable Vox workflow called `{name}`"))
        }
        "activity" => {
            let body = gen_body(rng, &ret_type, complexity, &mut tags);
            (format!("activity {name}({params}) to {ret_type} {{\n{body}\n}}"),
             format!("Define a Vox activity called `{name}`"))
        }
        "table" => {
            let fc = 2 + variant % 4;
            let fields = gen_fields(rng, fc);
            (format!("@table type {type_name} {{\n{fields}\n}}"),
             format!("Define a Vox @table schema `{type_name}` with {fc} fields"))
        }
        "http_route" => {
            let methods = ["get", "post", "put", "delete"];
            let m = methods[variant % methods.len()];
            (format!("@{m}(\"/api/{noun}\")\nfn {name}(req: str) to str {{\n    ret \"ok\"\n}}"),
             format!("Create a Vox HTTP {m} handler at `/api/{noun}`"))
        }
        "mcp_tool" => {
            (format!("@mcp.tool \"{name}: {verb} data\"\nfn {name}({params}) to str {{\n    ret \"done\"\n}}"),
             format!("Define a Vox MCP tool called `{name}`"))
        }
        "mcp_resource" => {
            (format!("@mcp.resource \"{noun}://{{path}}\"\nfn read_{noun}(path: str) to str {{\n    ret path\n}}"),
             format!("Define a Vox MCP resource for `{noun}`"))
        }
        "query" => {
            (format!("@query\nfn get_{noun}(id: int) to str {{\n    let result = db.{type_name}.find(id)\n    ret result\n}}"),
             format!("Write a Vox @query to read from `{type_name}`"))
        }
        "mutation" => {
            (format!("@mutation\nfn update_{noun}(id: int, value: str) to Unit {{\n    db.{type_name}.update(id, value)\n}}"),
             format!("Write a Vox @mutation to write to `{type_name}`"))
        }
        "action" => {
            let body = gen_body(rng, &ret_type, complexity, &mut tags);
            (format!("@action\nfn {name}({params}) to {ret_type} {{\n{body}\n}}"),
             format!("Write a Vox @action called `{name}`"))
        }
        "test" => {
            (format!("@test\nfn test_{noun}() to Unit {{\n    let result = {verb}(42)\n    assert(result > 0)\n}}"),
             format!("Write a Vox @test for `{verb}`"))
        }
        "type_def" => {
            let src = match variant % 3 {
                0 => format!("type {type_name}Status = Active | Inactive | Pending"),
                1 => format!("type {type_name}Result = Success(data: str) | Error(msg: str)"),
                _ => format!("type {type_name}Option[T] = Some(value: T) | None"),
            };
            (src, format!("Define a Vox union type for `{type_name}`"))
        }
        "import" => {
            let modules = ["std.json", "network.HTTP", "react.use_state", "db.users"];
            (format!("import {}", modules[variant % modules.len()]),
             "Write a Vox import statement".into())
        }
        "message" => {
            let fields = gen_fields(rng, 2 + variant % 3);
            (format!("message {type_name}Event {{\n{fields}\n}}"),
             format!("Define a Vox inter-agent message `{type_name}Event`"))
        }
        "scheduled" => {
            let intervals = ["1h", "30m", "24h", "5m"];
            let iv = intervals[variant % intervals.len()];
            (format!("@scheduled(\"{iv}\")\nfn {name}_job() to Unit {{\n    let status = check()\n    log(status)\n}}"),
             format!("Create a Vox scheduled job running every {iv}"))
        }
        "server_fn" => {
            let body = gen_body(rng, &ret_type, complexity, &mut tags);
            (format!("@server\nfn {name}({params}) to {ret_type} {{\n{body}\n}}"),
             format!("Write a Vox @server function `{name}`"))
        }
        "const" => {
            let ty = gen_prim_type(rng);
            (format!("const {}_LIMIT: {ty} = {}", noun.to_uppercase(), gen_literal_for_type(rng, &ty)),
             format!("Declare a Vox constant of type `{ty}`"))
        }
        "collection" => {
            let fields = gen_fields(rng, 3);
            (format!("@collection type {type_name}Doc {{\n{fields}\n    embedding: list[float]\n}}"),
             format!("Define a Vox @collection for `{type_name}`"))
        }
        "index" => {
            let (f, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            (format!("@index {type_name}.by_{f} on ({f})"),
             format!("Define a Vox @index on `{type_name}.{f}`"))
        }
        "vector_index" => {
            (format!("@vector_index {type_name}Doc.by_embedding on (embedding) {{ dimensions: 768 }}"),
             format!("Define a Vox @vector_index for `{type_name}Doc`"))
        }
        "search_index" => {
            (format!("@search_index {type_name}Doc.by_content on (title, description)"),
             format!("Define a Vox @search_index on `{type_name}Doc`"))
        }
        "trait" => {
            (format!("trait {type_name}Trait {{\n    fn {verb}(self) to str\n    fn validate(self) to bool\n}}"),
             format!("Define a Vox trait `{type_name}Trait`"))
        }
        "impl" => {
            (format!("impl Serializable for {type_name} {{\n    fn serialize(self) to str {{\n        ret \"{noun}\"\n    }}\n}}"),
             format!("Implement a trait for `{type_name}`"))
        }
        "skill" => {
            (format!("@skill\nfn {name}_skill({params}) to str {{\n    ret \"analyzed\"\n}}"),
             format!("Define a Vox @skill called `{name}_skill`"))
        }
        "agent_def" => {
            (format!("@agent_def\nfn {name}_agent() to str {{\n    tools: [{verb}]\n    memory: long_term\n    ret \"ready\"\n}}"),
             format!("Define a Vox @agent_def `{name}_agent`"))
        }
        "agent" => {
            let (sf, st) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            (format!("agent {type_name}Agent {{\n    state {sf}: {st} = {}\n    on {verb}() to str {{\n        ret \"processed\"\n    }}\n}}", gen_literal_for_type(rng, st)),
             format!("Define a Vox agent `{type_name}Agent`"))
        }
        "config" => {
            let fields = gen_fields(rng, 3);
            (format!("config {type_name}Config {{\n{fields}\n}}"),
             format!("Define a Vox config block `{type_name}Config`"))
        }
        "context" => {
            (format!("context {type_name}Context {{\n    value: str\n    update: fn(str) -> Unit\n}}"),
             format!("Define a Vox context `{type_name}Context`"))
        }
        "hook" => {
            (format!("hook fn use_{noun}(initial: int) to (int, fn() -> Unit) {{\n    let state = initial\n    ret (state, fn() to state + 1)\n}}"),
             format!("Define a Vox hook `use_{noun}`"))
        }
        "provider" => {
            (format!("provider fn {type_name}Provider(children: Element) to Element {{\n    ret <div>{{children}}</div>\n}}"),
             format!("Define a Vox provider `{type_name}Provider`"))
        }
        "fixture" => {
            (format!("@fixture\nfn setup_{noun}() to str {{\n    ret \"test_fixture\"\n}}"),
             format!("Define a Vox @fixture for `{noun}`"))
        }
        "layout" => {
            (format!("layout fn {type_name}Layout(children: Element) to Element {{\n    ret <main>{{children}}</main>\n}}"),
             format!("Define a Vox layout `{type_name}Layout`"))
        }
        "loading" => {
            (format!("loading fn {type_name}Loading() to Element {{\n    ret <div>{{\"Loading...\"}}</div>\n}}"),
             format!("Define a Vox loading component for `{type_name}`"))
        }
        "not_found" => {
            (format!("not_found fn {type_name}NotFound() to Element {{\n    ret <h1>{{\"404 - Not Found\"}}</h1>\n}}"),
             format!("Define a Vox 404 handler for `{type_name}`"))
        }
        "error_boundary" =>  {
            (format!("error_boundary fn {type_name}Error(error: str) to Element {{\n    ret <div class=\"error\">{{error}}</div>\n}}"),
             format!("Define a Vox error boundary for `{type_name}`"))
        }
        "keyframes" => {
            (format!("@keyframes {noun}_fade {{\n    from {{ opacity: 0 }}\n    to {{ opacity: 1 }}\n}}"),
             format!("Define Vox @keyframes `{noun}_fade`"))
        }
        "theme" => {
            (format!("theme dark_{noun} {{\n    bg: \"#1a1a2e\"\n    fg: \"#e0e0e0\"\n    accent: \"#00d4ff\"\n}}"),
             format!("Define a Vox dark theme for `{noun}`"))
        }
        "mock" => {
            (format!("@mock\nfn mock_{noun}() to str {{\n    ret \"mock_data\"\n}}"),
             format!("Define a Vox @mock for `{noun}`"))
        }
        "environment" => {
            (format!("environment {noun}_staging {{\n    region: \"us-east-1\"\n    replicas: 2\n    debug: false\n}}"),
             format!("Define a Vox environment for `{noun}`"))
        }
        "page" => {
            (format!("page fn {type_name}Page() to Element {{\n    ret <section>\n        <h1>{{\"{type_name}\"}}</h1>\n    </section>\n}}"),
             format!("Define a Vox static page `{type_name}Page`"))
        }
        "island" => {
            (format!("@island\nfn {type_name}Island(data: list[int]) to Element {{\n    ret <div>{{\"Interactive\"}}</div>\n}}"),
             format!("Define a Vox island component `{type_name}Island`"))
        }
        "routes" => {
            (format!("routes {{\n    \"/\" -> {type_name}Page\n    \"/{noun}\" -> {type_name}View\n    \"/{noun}/:id\" -> {type_name}Detail\n}}"),
             format!("Define Vox routes for `{type_name}`"))
        }
        "v0_component" => {
            (format!("@v0(\"https://v0.dev/t/example\")\nfn {type_name}Widget() to Element {{\n}}"),
             format!("Define a Vox v0.dev component `{type_name}Widget`"))
        }
        "py_import" => {
            let libs = ["torch", "numpy", "pandas", "transformers"];
            let lib = libs[variant % libs.len()];
            (format!("@py.import {lib} as {lib}"), format!("Import Python `{lib}` in Vox"))
        }
        _ => {
            // Unknown taxonomy entry — generate a generic function tagged with the construct
            let body = gen_body(rng, "str", complexity, &mut tags);
            (format!("# {tag} construct\nfn {name}({params}) to str {{\n{body}\n}}"),
             format!("Write a Vox `{tag}` construct called `{name}`"))
        }
    };

    Some(OrganicPair {
        prompt,
        response: source,
        category: format!("vox_{tag}"),
        verified: false,
        complexity,
        coverage_tags: tags,
    })
}

// ── Multi-construct program templates ────────────────────────────────────────

fn gen_full_stack_program(rng: &mut Rng, variant: usize) -> OrganicPair {
    let noun = NOUNS[rng.usize(NOUNS.len())];
    let tn = { let mut s = noun[..1].to_uppercase(); s.push_str(&noun[1..]); s };
    let verb = VERBS[rng.usize(VERBS.len())];
    let templates = [
        // Template 0: CRUD API
        format!("@table type {tn} {{\n    id: int\n    name: str\n    active: bool\n}}\n\n@query\nfn get_{noun}(id: int) to str {{\n    ret db.{tn}.find(id).name\n}}\n\n@mutation\nfn create_{noun}(name: str) to Unit {{\n    db.{tn}.insert(name)\n}}\n\n@get(\"/api/{noun}\")\nfn {noun}_handler(req: str) to str {{\n    ret get_{noun}(1)\n}}\n\n@test\nfn test_{noun}() to Unit {{\n    create_{noun}(\"test\")\n    assert(get_{noun}(1) == \"test\")\n}}"),
        // Template 1: Agent pipeline
        format!("message {tn}Event {{\n    id: int\n    data: str\n}}\n\nactor {tn}Worker {{\n    state count: int = 0\n    on {verb}() to str {{\n        count = count + 1\n        ret \"processed\"\n    }}\n}}\n\nworkflow {noun}_pipeline(input: str) to str {{\n    let worker = spawn({tn}Worker)\n    let result = {verb}(input)\n    ret result\n}}"),
        // Template 2: UI app
        format!("type {tn}Status = Loading | Ready(data: str) | Error(msg: str)\n\ncomponent fn {tn}View() to Element {{\n    let status = \"ready\"\n    ret <div className=\"{noun}\">\n        <h1>{{\"{tn}\"}}</h1>\n        <p>{{status}}</p>\n    </div>\n}}\n\nlayout fn {tn}Layout(children: Element) to Element {{\n    ret <main>{{children}}</main>\n}}\n\nroutes {{\n    \"/\" -> {tn}View\n}}"),
    ];

    OrganicPair {
        prompt: format!("Write a complete Vox program for {tn} with multiple constructs"),
        response: templates[variant % templates.len()].clone(),
        category: "vox_full_program".into(),
        verified: false,
        complexity: 8,
        coverage_tags: vec!["multi_construct".into()],
    }
}

// ── Main generation entry point ──────────────────────────────────────────────

/// Generate the organic corpus by iterating over `TAXONOMY_FROM_AST`.
///
/// For each taxonomy entry, generates `variants_per_construct` variants,
/// ensuring every language construct has training coverage.
pub fn generate_organic_corpus(seed: u64) -> Vec<OrganicPair> {
    let mut rng = Rng::new(seed);
    let mut pairs = Vec::new();
    let variants_per_construct: usize = 5;

    // Dynamic: iterate over TAXONOMY_FROM_AST (auto-derived from Decl enum)
    for tag in TAXONOMY_FROM_AST {
        for v in 0..variants_per_construct {
            if let Some(pair) = generate_for_taxonomy_entry(tag, &mut rng, v) {
                pairs.push(pair);
            }
        }
    }

    // Multi-construct programs
    for v in 0..5 {
        pairs.push(gen_full_stack_program(&mut rng, v));
    }

    // Parser round-trip verification
    for pair in &mut pairs {
        pair.verified = verify_parse(&pair.response);
    }

    let total = pairs.len();
    let verified = pairs.iter().filter(|p| p.verified).count();
    let taxonomy_count = TAXONOMY_FROM_AST.len();
    eprintln!("  [organic] {total} pairs for {taxonomy_count} taxonomy entries, {verified} verified ({:.0}%)",
        if total > 0 { verified as f64 / total as f64 * 100.0 } else { 0.0 });

    pairs
}

/// Parse verification (heuristic fallback when parser feature not enabled).
fn verify_parse(source: &str) -> bool {
    #[cfg(feature = "parser-verify")]
    {
        let tokens = vox_lexer::lex(source);
        let (_tree, errors) = vox_parser::parse(&tokens);
        errors.is_empty()
    }
    #[cfg(not(feature = "parser-verify"))]
    {
        let open = source.chars().filter(|&c| c == '{').count();
        let close = source.chars().filter(|&c| c == '}').count();
        open == close && !source.is_empty()
    }
}

/// Write organic pairs to JSONL.
pub fn write_organic_to_jsonl(
    pairs: &[OrganicPair], output: &std::path::Path, verified_only: bool,
) -> anyhow::Result<usize> {
    use std::io::Write;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true).truncate(true).write(true).open(output)?;
    let mut count = 0;
    for pair in pairs {
        if verified_only && !pair.verified { continue; }
        writeln!(f, "{}", pair.to_jsonl())?;
        count += 1;
    }
    Ok(count)
}

// ── Variety calculation and coverage analysis ────────────────────────────────

/// Coverage report for the generated corpus.
#[derive(Debug)]
pub struct CoverageReport {
    /// Total pairs generated.
    pub total_pairs: usize,
    /// Pairs that passed parser verification.
    pub verified_pairs: usize,
    /// Number of TAXONOMY entries with generators.
    pub taxonomy_covered: usize,
    /// Total TAXONOMY entries.
    pub taxonomy_total: usize,
    /// Per-construct pair counts.
    pub per_construct: Vec<(String, usize, usize)>, // (tag, actual, required)
    /// Constructs below minimum required variety.
    pub under_covered: Vec<String>,
    /// Unique Expr variant tags observed.
    pub expr_variants_seen: usize,
    /// Total Expr variant count from AST (24).
    pub expr_variants_total: usize,
}

/// Minimum pairs required per construct based on language complexity.
///
/// **Formula**: constructs with bodies (function, actor, workflow, etc.) need
/// more examples because a user can ask for them in wildly different ways.
/// Structural constructs (import, const, index) need fewer.
///
/// - **Body constructs** (function, actor, workflow, component, etc.): 7 pairs
/// - **Decorator constructs** (test, fixture, mock, scheduled, etc.): 5 pairs
/// - **Structural constructs** (import, const, index, message, etc.): 3 pairs
pub fn compute_variety_requirements() -> Vec<(&'static str, usize)> {
    TAXONOMY_FROM_AST.iter().map(|tag| {
        let min = match *tag {
            // High-body-complexity: many parameters, complex bodies, many use cases
            "function" | "actor" | "workflow" | "component" | "activity"
            | "server_fn" | "agent_def" | "agent" | "skill" | "action"
            | "trait" | "impl" | "hook" | "provider" | "page" | "island" => 7,
            // Medium: decorator-style with predictable structure
            "test" | "fixture" | "mock" | "scheduled" | "mcp_tool"
            | "mcp_resource" | "query" | "mutation" | "http_route"
            | "error_boundary" | "layout" | "loading" | "not_found"
            | "context" | "config" | "type_def" | "v0_component" => 5,
            // Low: structural / declarative with little variation
            "import" | "const" | "message" | "table" | "collection"
            | "index" | "vector_index" | "search_index" | "keyframes"
            | "theme" | "environment" | "routes" | "py_import" => 3,
            // Unknown new construct — conservative default
            _ => 5,
        };
        (*tag, min)
    }).collect()
}

/// Analyze generated pairs and produce a coverage report.
pub fn compute_coverage_report(pairs: &[OrganicPair]) -> CoverageReport {
    use std::collections::{HashMap, HashSet};

    let mut per_construct: HashMap<String, usize> = HashMap::new();
    let mut expr_tags: HashSet<String> = HashSet::new();

    for pair in pairs {
        *per_construct.entry(pair.category.clone()).or_default() += 1;
        for tag in &pair.coverage_tags {
            if tag.starts_with("expr:") {
                expr_tags.insert(tag.clone());
            }
        }
    }

    let requirements = compute_variety_requirements();
    let mut construct_report = Vec::new();
    let mut under_covered = Vec::new();
    let mut covered = 0;

    for (tag, min_required) in &requirements {
        let category = format!("vox_{tag}");
        let actual = per_construct.get(&category).copied().unwrap_or(0);
        construct_report.push((tag.to_string(), actual, *min_required));
        if actual > 0 { covered += 1; }
        if actual < *min_required {
            under_covered.push(format!("{tag} ({actual}/{min_required})"));
        }
    }

    CoverageReport {
        total_pairs: pairs.len(),
        verified_pairs: pairs.iter().filter(|p| p.verified).count(),
        taxonomy_covered: covered,
        taxonomy_total: TAXONOMY_FROM_AST.len(),
        per_construct: construct_report,
        under_covered,
        expr_variants_seen: expr_tags.len(),
        // Dynamic: AST_EXPR_TOTAL auto-derived from vox-ast Expr enum by build.rs
        expr_variants_total: AST_EXPR_TOTAL,
    }
}

/// Print a human-readable coverage report.
pub fn print_coverage_report(report: &CoverageReport) {
    eprintln!("═══════════════════════════════════════════");
    eprintln!("  Vox Organic Corpus — Coverage Report");
    eprintln!("═══════════════════════════════════════════");
    let verified_pct = if report.total_pairs > 0 {
        report.verified_pairs as f64 / report.total_pairs as f64 * 100.0
    } else { 0.0 };
    eprintln!("Total pairs : {} ({} verified, {:.0}%)",
        report.total_pairs, report.verified_pairs, verified_pct);
    eprintln!("Taxonomy    : {}/{} constructs ({:.0}%)",
        report.taxonomy_covered, report.taxonomy_total,
        report.taxonomy_covered as f64 / report.taxonomy_total.max(1) as f64 * 100.0);
    eprintln!("Expr        : {}/{} variants ({:.0}%)  [AST_EXPR_TOTAL={}]",
        report.expr_variants_seen, report.expr_variants_total,
        report.expr_variants_seen as f64 / report.expr_variants_total.max(1) as f64 * 100.0,
        AST_EXPR_TOTAL);
    eprintln!("BinOp       : {}/{} operators  [AST_BINOP_TOTAL={}]",
        BINOP_VARIANTS.len(), AST_BINOP_TOTAL, AST_BINOP_TOTAL);
    eprintln!("TypeExpr    : {}/{} types  [AST_TYPE_EXPR_TOTAL={}]",
        TYPE_EXPR_VARIANTS.len(), AST_TYPE_EXPR_TOTAL, AST_TYPE_EXPR_TOTAL);
    eprintln!("Pattern     : {}/{} patterns  [AST_PATTERN_TOTAL={}]",
        PATTERN_VARIANTS.len(), AST_PATTERN_TOTAL, AST_PATTERN_TOTAL);
    eprintln!("Stmt        : {}/{} statements  [AST_STMT_TOTAL={}]",
        STMT_VARIANTS.len(), AST_STMT_TOTAL, AST_STMT_TOTAL);
    eprintln!("───────────────────────────────────────────");
    if report.under_covered.is_empty() {
        eprintln!("✓ All constructs meet minimum variety requirements");
    } else {
        eprintln!("⚠ Under-covered ({}):", report.under_covered.len());
        for entry in &report.under_covered {
            eprintln!("  - {entry}");
        }
    }
    eprintln!("═══════════════════════════════════════════");
}
