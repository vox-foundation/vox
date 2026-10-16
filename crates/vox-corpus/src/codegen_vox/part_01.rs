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
            format!("fn({p1}) to {ret}")
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
        0 => {
            tags.push("expr:int_lit".into());
            format!("{}", rng.next() % 1000)
        }
        1 => {
            tags.push("expr:float_lit".into());
            format!("{}.{}", rng.next() % 100, rng.next() % 99 + 1)
        }
        2 => {
            tags.push("expr:string_lit".into());
            format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())])
        }
        3 => {
            tags.push("expr:bool_lit".into());
            if rng.coin() { "true" } else { "false" }.into()
        }
        4 => {
            tags.push("expr:ident".into());
            gen_ident(rng)
        }
        5 => {
            tags.push("expr:object_lit".into());
            gen_object(rng, depth, tags)
        }
        6 => {
            tags.push("expr:list_lit".into());
            gen_list(rng, depth, tags)
        }
        7 => {
            tags.push("expr:tuple_lit".into());
            gen_tuple(rng, depth, tags)
        }
        8 => {
            tags.push("expr:binary".into());
            gen_binary(rng, depth)
        }
        9 => {
            tags.push("expr:unary".into());
            gen_unary(rng, depth, tags)
        }
        10 => {
            tags.push("expr:call".into());
            gen_call_expr(rng, depth, tags)
        }
        11 => {
            tags.push("expr:method_call".into());
            gen_method_call(rng)
        }
        12 => {
            tags.push("expr:field_access".into());
            gen_field_access(rng)
        }
        13 => {
            tags.push("expr:match".into());
            gen_match(rng, depth, tags)
        }
        14 => {
            tags.push("expr:if".into());
            gen_if(rng, depth, tags)
        }
        15 => {
            tags.push("expr:for".into());
            gen_for_expr(rng, depth, tags)
        }
        16 => {
            tags.push("expr:lambda".into());
            gen_lambda(rng, depth, tags)
        }
        17 => {
            tags.push("expr:pipe".into());
            gen_pipe(rng, depth, tags)
        }
        18 => {
            tags.push("expr:spawn".into());
            gen_spawn_expr(rng)
        }
        19 => {
            tags.push("expr:with".into());
            gen_with_expr(rng, depth, tags)
        }
        20 => {
            tags.push("expr:jsx".into());
            gen_jsx_element(rng)
        }
        21 => {
            tags.push("expr:jsx_self_closing".into());
            gen_jsx_self_closing(rng)
        }
        22 => {
            tags.push("expr:string_interp".into());
            gen_string_interp(rng)
        }
        _ => {
            tags.push("expr:block".into());
            gen_block(rng, depth, tags)
        }
    }
}

fn gen_literal(rng: &mut Rng, tags: &mut Vec<String>) -> String {
    match rng.usize(6) {
        0 => {
            tags.push("expr:int_lit".into());
            format!("{}", rng.next() % 1000)
        }
        1 => {
            tags.push("expr:float_lit".into());
            format!("{}.{}", rng.next() % 100, rng.next() % 99 + 1)
        }
        2 => {
            tags.push("expr:string_lit".into());
            format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())])
        }
        3 => {
            tags.push("expr:bool_lit".into());
            if rng.coin() { "true" } else { "false" }.into()
        }
        4 => {
            tags.push("expr:string_interp".into());
            gen_string_interp(rng)
        }
        _ => {
            tags.push("expr:ident".into());
            gen_ident(rng)
        }
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
    let prim = ["column", "row", "card", "box"][rng.usize(4)];
    let child = NOUNS[rng.usize(NOUNS.len())];
    format!("{prim}() {{ text() {{ \"{child}\" }} }}")
}

fn gen_jsx_self_closing(rng: &mut Rng) -> String {
    let val = NOUNS[rng.usize(NOUNS.len())];
    format!("text() {{ \"{val}\" }}")
}

fn gen_literal_for_type(rng: &mut Rng, ty: &str) -> String {
    match ty {
        "int" => format!("{}", rng.next() % 1000),
        "float" => format!("{}.{}", rng.next() % 100, rng.next() % 99 + 1),
        "bool" => if rng.coin() { "true" } else { "false" }.to_string(),
        "Unit" => "()".to_string(),
        _ => format!("\"{}\"", NOUNS[rng.usize(NOUNS.len())]),
    }
}

fn gen_binary(rng: &mut Rng, depth: u8) -> String {
    // Canonical BinOp variants: +, -, *, /, <, >, <=, >=, and, or, is, is not
    let ops = [
        "+", "-", "*", "/", ">", "<", ">=", "<=", "and", "or", "is", "is not",
    ];
    let op = ops[rng.usize(ops.len())];
    let left = NOUNS[rng.usize(NOUNS.len())];
    let right = if depth > 1 {
        format!(
            "{} {} {}",
            NOUNS[rng.usize(NOUNS.len())],
            ops[rng.usize(4)],
            rng.next() % 50 + 1
        )
    } else {
        format!("{}", rng.next() % 100 + 1)
    };
    format!("{left} {op} {right}")
}

fn gen_unary(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    if rng.coin() {
        format!("not {}", NOUNS[rng.usize(NOUNS.len())])
    } else {
        format!("-{}", rng.next() % 100 + 1)
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
    let then_val = gen_literal(rng, tags);
    let else_val = gen_literal(rng, tags);
    let _ = depth;
    format!("if {cond} {{ {then_val} }} else {{ {else_val} }}")
}

/// Match with all Pattern variants exercised across calls using canonical `=>` arms.
fn gen_match(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let subject = NOUNS[rng.usize(NOUNS.len())];
    let arm1_body = gen_literal(rng, tags);
    let arm2_body = gen_literal(rng, tags);
    let arm3_body = gen_literal(rng, tags);

    // Rotate through all Pattern variants based on RNG
    let arms = match rng.usize(5) {
        // Ident pattern
        0 => format!("match {subject} {{ {subject} => {arm1_body}, _ => {arm2_body} }}"),
        // Literal pattern
        1 => format!("match {subject} {{ 0 => {arm1_body}, 1 => {arm2_body}, _ => {arm3_body} }}"),
        // Constructor pattern (ADT)
        2 => {
            tags.push("pattern:constructor".into());
            format!("match {subject} {{ Ok(value) => {arm1_body}, Error(msg) => {arm2_body} }}")
        }
        // Tuple destructuring pattern
        3 => {
            tags.push("pattern:tuple".into());
            let (f1, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            let (f2, _) = FIELD_POOL[(rng.usize(FIELD_POOL.len()) + 1) % FIELD_POOL.len()];
            format!("match {subject} {{ ({f1}, {f2}) => {arm1_body}, _ => {arm2_body} }}")
        }
        // Wildcard + constructor
        _ => format!("match {subject} {{ Some(x) => {arm1_body}, None => {arm2_body} }}"),
    };
    tags.push("expr:match".into());
    arms
}

fn gen_lambda(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let param = NOUNS[rng.usize(NOUNS.len())];
    let body = gen_literal(rng, tags);
    format!("fn({param}: int) to int {{ return {body} }}")
}

fn gen_pipe(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    let source = NOUNS[rng.usize(NOUNS.len())];
    let f1 = VERBS[rng.usize(VERBS.len())];
    let f2 = VERBS[rng.usize(VERBS.len())];
    format!("{source} |> {f1} |> {f2}")
}

fn gen_list(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(3);
    let items: Vec<String> = (0..n).map(|_| gen_literal(rng, tags)).collect();
    format!("[{}]", items.join(", "))
}

fn gen_tuple(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(2);
    let items: Vec<String> = (0..n).map(|_| gen_literal(rng, tags)).collect();
    format!("({})", items.join(", "))
}

fn gen_object(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let n = 2 + rng.usize(2);
    let mut used = std::collections::HashSet::new();
    let mut fields = Vec::new();
    for _ in 0..n {
        let (name, _) = loop {
            let e = FIELD_POOL[rng.usize(FIELD_POOL.len())];
            if used.insert(e.0) {
                break e;
            }
        };
        let val = gen_literal(rng, tags);
        fields.push(format!("{name}: {val}"));
    }
    format!("{{{}}}", fields.join(", "))
}

fn gen_for_expr(rng: &mut Rng, _depth: u8, _tags: &mut Vec<String>) -> String {
    let binding = NOUNS[rng.usize(NOUNS.len())];
    let iterable = NOUNS[rng.usize(NOUNS.len())];
    format!("for {binding} in {iterable} {{ {binding} }}")
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

fn gen_block(rng: &mut Rng, _depth: u8, tags: &mut Vec<String>) -> String {
    let val = gen_literal(rng, tags);
    let (name, _) = FIELD_POOL[rng.usize(FIELD_POOL.len())];
    format!("{{\n        let {name} = {val}\n        {name}\n    }}")
}
