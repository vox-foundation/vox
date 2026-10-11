//! Graded benchmark cases for `vox model bench`. Every case has a deterministic grader that
//! returns a score in `0.0..=1.0`, so a run is a measurement rather than a judgement call.
//! Suites map to routing task categories so the leaderboard can be read per mode and category.

use serde_json::Value;

/// One graded case.
#[derive(Debug, Clone)]
pub struct BenchCase {
    pub id: String,
    pub suite: Suite,
    pub prompt: String,
    pub max_tokens: u64,
    pub grader: Grader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suite {
    Reasoning,
    Codegen,
    Json,
    ToolCall,
    Needle,
    VoxGen,
    Chat,
}

impl Suite {
    pub const ALL: [Suite; 7] = [
        Suite::Reasoning,
        Suite::Codegen,
        Suite::Json,
        Suite::ToolCall,
        Suite::Needle,
        Suite::VoxGen,
        Suite::Chat,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Suite::Reasoning => "reasoning",
            Suite::Codegen => "codegen",
            Suite::Json => "json",
            Suite::ToolCall => "tool-call",
            Suite::Needle => "needle",
            Suite::VoxGen => "vox-gen",
            Suite::Chat => "chat",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Suite> {
        Suite::ALL.into_iter().find(|x| x.as_str() == s.trim())
    }

    /// The routing task category this suite measures (`contracts/orchestration/model-routing.v1.yaml`).
    #[must_use]
    pub fn task_category(self) -> &'static str {
        match self {
            Suite::Reasoning => "Planning",
            Suite::Codegen | Suite::VoxGen => "CodeGen",
            Suite::Json => "Parsing",
            Suite::ToolCall => "ToolOrchestration",
            Suite::Needle => "Research",
            Suite::Chat => "Chat",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Grader {
    /// The last number in the reply equals this value.
    Number(f64),
    /// Fraction of these substrings present (case-insensitive).
    ContainsAll(Vec<String>),
    /// The first JSON object in the reply has these field values; score = fraction matched.
    JsonFields(Vec<(String, Value)>),
    /// The model must call this tool with `arg == value`.
    ToolCall {
        name: String,
        arg: String,
        value: Value,
    },
    /// The fenced Vox code compiles cleanly (`vox_compiler::pipeline::check_file`) and declares this fn.
    VoxCompiles { fn_name: String },
    /// The reply has at most this many words and contains every listed substring.
    Concise { max_words: usize, must: Vec<String> },
}

/// Score a text reply. Tool-call grading lives in [`grade_tool_call`].
#[must_use]
pub fn grade_text(grader: &Grader, reply: &str) -> f64 {
    match grader {
        Grader::Number(want) => match last_number(reply) {
            Some(got) if (got - want).abs() < 1e-6 => 1.0,
            _ => 0.0,
        },
        Grader::ContainsAll(needles) => fraction(needles, reply),
        Grader::JsonFields(fields) => {
            let Some(obj) = first_json_object(reply) else {
                return 0.0;
            };
            if fields.is_empty() {
                return 1.0;
            }
            let hit = fields
                .iter()
                .filter(|(k, v)| json_eq(obj.get(k), v))
                .count();
            hit as f64 / fields.len() as f64
        }
        Grader::VoxCompiles { fn_name } => {
            let code = fenced_code(reply);
            if !code.contains(&format!("fn {fn_name}")) {
                return 0.0;
            }
            let diags = vox_compiler::pipeline::check_file(&code, "bench.vox");
            let errors = diags
                .iter()
                .filter(|d| {
                    matches!(
                        d.severity,
                        vox_compiler::typeck::diagnostics::TypeckSeverity::Error
                    )
                })
                .count();
            if errors == 0 { 1.0 } else { 0.0 }
        }
        Grader::Concise { max_words, must } => {
            let words = reply.split_whitespace().count();
            if words == 0 || words > *max_words {
                return 0.0;
            }
            fraction(must, reply)
        }
        Grader::ToolCall { .. } => 0.0,
    }
}

/// Score a tool-calling reply: 1.0 for the right tool with the right argument, 0.5 for the
/// right tool with a wrong argument, else 0.
#[must_use]
pub fn grade_tool_call(grader: &Grader, calls: &[(String, Value)]) -> f64 {
    let Grader::ToolCall { name, arg, value } = grader else {
        return 0.0;
    };
    let mut best: f64 = 0.0;
    for (called, args) in calls {
        if called == name {
            best = best.max(if json_eq(args.get(arg), value) {
                1.0
            } else {
                0.5
            });
        }
    }
    best
}

fn fraction(needles: &[String], reply: &str) -> f64 {
    if needles.is_empty() {
        return 1.0;
    }
    let hay = reply.to_ascii_lowercase();
    let hit = needles
        .iter()
        .filter(|n| hay.contains(&n.to_ascii_lowercase()))
        .count();
    hit as f64 / needles.len() as f64
}

fn json_eq(got: Option<&Value>, want: &Value) -> bool {
    match (got, want) {
        (Some(Value::String(a)), Value::String(b)) => a.trim().eq_ignore_ascii_case(b.trim()),
        (Some(Value::Number(a)), Value::Number(b)) => {
            (a.as_f64().unwrap_or(f64::NAN) - b.as_f64().unwrap_or(f64::NAN)).abs() < 1e-9
        }
        (Some(Value::String(a)), Value::Number(b)) => a
            .trim()
            .parse::<f64>()
            .is_ok_and(|x| (x - b.as_f64().unwrap_or(f64::NAN)).abs() < 1e-9),
        (Some(a), b) => a == b,
        (None, _) => false,
    }
}

fn last_number(s: &str) -> Option<f64> {
    let mut last = None;
    let mut cur = String::new();
    let flush = |cur: &mut String, last: &mut Option<f64>| {
        let t = cur.trim_end_matches('.').replace(',', "");
        if let Ok(v) = t.parse::<f64>() {
            *last = Some(v);
        }
        cur.clear();
    };
    for ch in s.chars() {
        if ch.is_ascii_digit()
            || ch == '.'
            || (ch == '-' && cur.is_empty())
            || (ch == ',' && !cur.is_empty())
        {
            cur.push(ch);
        } else if !cur.is_empty() {
            flush(&mut cur, &mut last);
        }
    }
    if !cur.is_empty() {
        flush(&mut cur, &mut last);
    }
    last
}

fn first_json_object(s: &str) -> Option<serde_json::Map<String, Value>> {
    let body = fenced_code(s);
    for (i, _) in body.match_indices('{') {
        let mut de = serde_json::Deserializer::from_str(&body[i..]).into_iter::<Value>();
        if let Some(Ok(Value::Object(m))) = de.next() {
            return Some(m);
        }
    }
    None
}

/// The body of the first fenced block, or the whole reply when there is none.
fn fenced_code(s: &str) -> String {
    let Some(start) = s.find("```") else {
        return s.to_string();
    };
    let after = &s[start + 3..];
    let body_start = after.find('\n').map_or(0, |n| n + 1);
    let body = &after[body_start..];
    body.find("```")
        .map_or(body, |end| &body[..end])
        .to_string()
}

fn case(id: &str, suite: Suite, prompt: &str, max_tokens: u64, grader: Grader) -> BenchCase {
    BenchCase {
        id: id.to_string(),
        suite,
        prompt: prompt.to_string(),
        max_tokens,
        grader,
    }
}

fn strs(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| (*s).to_string()).collect()
}

/// A long, deterministic haystack with one buried fact, for effective-context measurement.
#[must_use]
pub fn needle_prompt(filler_lines: usize, code: &str) -> String {
    let mut s = String::from(
        "Below is a log. Exactly one line contains a vault code. Reply with only the vault code.\n\n",
    );
    let at = filler_lines * 2 / 3;
    for i in 0..filler_lines {
        if i == at {
            s.push_str(&format!(
                "line {i}: maintenance note: the vault code is {code}.\n"
            ));
        } else {
            s.push_str(&format!(
                "line {i}: sensor {} reported nominal readings at checkpoint {}.\n",
                i % 17,
                (i * 7919) % 1000
            ));
        }
    }
    s
}

/// The built-in case set. Small on purpose: enough cases per model for a Wilson verdict
/// (`bench_evidence::MIN_VERDICT_CASES`) at a few cents per cheap model.
#[must_use]
pub fn builtin_cases() -> Vec<BenchCase> {
    use Suite::*;
    vec![
        case(
            "reason-trains",
            Reasoning,
            "A train leaves at 09:40 and travels 210 km at 84 km/h. How many minutes does the trip take? End your reply with the number only.",
            1024,
            Grader::Number(150.0),
        ),
        case(
            "reason-sheep",
            Reasoning,
            "A farmer has 17 sheep. All but 9 run away. How many sheep does the farmer have left? End your reply with the number only.",
            1024,
            Grader::Number(9.0),
        ),
        case(
            "reason-mod",
            Reasoning,
            "What is the remainder when 7^100 is divided by 5? End your reply with the number only.",
            1024,
            Grader::Number(1.0),
        ),
        case(
            "reason-ages",
            Reasoning,
            "Ana is twice as old as Ben. In 6 years the sum of their ages will be 42. How old is Ana now? End your reply with the number only.",
            1024,
            Grader::Number(20.0),
        ),
        case(
            "code-output",
            Codegen,
            "What does this Python print? Reply with only the output.\n\nxs = [3, 1, 4, 1, 5, 9, 2, 6]\nprint(sum(x for x in xs if x % 2 == 0) * len(set(xs)))",
            512,
            Grader::Number(84.0),
        ),
        case(
            "code-rust-bug",
            Codegen,
            "This Rust function should return the largest element but panics on an empty slice. Rewrite it to return Option<i32>. Reply with only the function in a ```rust block.\n\nfn largest(xs: &[i32]) -> i32 { let mut m = xs[0]; for &x in xs { if x > m { m = x; } } m }",
            512,
            Grader::ContainsAll(strs(&["Option<i32>", "None", "Some"])),
        ),
        case(
            "code-sql",
            Codegen,
            "Write one SQL query returning the names of customers with more than 3 orders, given tables customers(id, name) and orders(id, customer_id). Reply with only the SQL.",
            400,
            Grader::ContainsAll(strs(&["join", "group by", "having", "count", "> 3"])),
        ),
        case(
            "json-extract",
            Json,
            "Extract fields from: 'Invoice 4471 for ACME Corp, total 1299.50 EUR, due 2026-11-30.' Reply with only a JSON object with keys invoice (number), customer, total (number), currency, due.",
            300,
            Grader::JsonFields(vec![
                ("invoice".into(), serde_json::json!(4471)),
                ("customer".into(), serde_json::json!("ACME Corp")),
                ("total".into(), serde_json::json!(1299.5)),
                ("currency".into(), serde_json::json!("EUR")),
                ("due".into(), serde_json::json!("2026-11-30")),
            ]),
        ),
        case(
            "json-classify",
            Json,
            "Classify the sentiment of 'The update broke my build twice, but support fixed it within an hour.' Reply with only JSON: {\"sentiment\": \"positive\"|\"negative\"|\"mixed\", \"mentions_support\": true|false}.",
            200,
            Grader::JsonFields(vec![
                ("sentiment".into(), serde_json::json!("mixed")),
                ("mentions_support".into(), serde_json::json!(true)),
            ]),
        ),
        case(
            "tool-weather",
            ToolCall,
            "What's the weather in Lisbon right now? Use the available tool.",
            300,
            Grader::ToolCall {
                name: "get_weather".into(),
                arg: "city".into(),
                value: serde_json::json!("Lisbon"),
            },
        ),
        case(
            "tool-convert",
            ToolCall,
            "Convert 250 US dollars to euros using the available tool.",
            300,
            Grader::ToolCall {
                name: "convert_currency".into(),
                arg: "amount".into(),
                value: serde_json::json!(250),
            },
        ),
        case(
            "needle-8k",
            Needle,
            &needle_prompt(400, "QX-7741-ZETA"),
            64,
            Grader::ContainsAll(strs(&["QX-7741-ZETA"])),
        ),
        case(
            "needle-32k",
            Needle,
            &needle_prompt(1600, "MV-2093-ORCA"),
            64,
            Grader::ContainsAll(strs(&["MV-2093-ORCA"])),
        ),
        case(
            "vox-add",
            VoxGen,
            "Write a Vox function `fn add_all(xs: list[int]) to int` that returns the sum of the list. Vox syntax: `fn name(arg: type) to ReturnType { ... }`, `let mut total = 0`, `for x in xs { total = total + x }`, `return total`. Reply with only the code in a ```vox block.",
            400,
            Grader::VoxCompiles {
                fn_name: "add_all".into(),
            },
        ),
        case(
            "vox-max",
            VoxGen,
            "Write a Vox function `fn bigger(a: int, b: int) to int` returning the larger argument. Vox syntax: `fn name(arg: type) to ReturnType { ... }` with `if cond { return a } else { return b }`. Reply with only the code in a ```vox block.",
            400,
            Grader::VoxCompiles {
                fn_name: "bigger".into(),
            },
        ),
        case(
            "reason-clock",
            Reasoning,
            "How many times in one 24-hour day (00:00 inclusive to 24:00 exclusive) do the hour and minute hands of an analog clock point in exactly the same direction? End your reply with the number only.",
            2048,
            Grader::Number(22.0),
        ),
        case(
            "reason-sieve",
            Reasoning,
            "How many integers from 1 to 1000 inclusive are divisible by none of 3, 5 and 7? End your reply with the number only.",
            2048,
            Grader::Number(457.0),
        ),
        case(
            "code-memo-default",
            Codegen,
            "What does this Python print? Reply with only the output.\n\ndef f(n, memo={}):\n    if n in memo:\n        return memo[n]\n    memo[n] = n if n < 2 else f(n - 1) + f(n - 2)\n    return memo[n]\n\na = [f(i) for i in range(10)]\nprint(a[-1] + len(f.__defaults__[0]))",
            1024,
            Grader::Number(44.0),
        ),
        case(
            "code-alias",
            Codegen,
            "What does this Python print? Reply with only the output.\n\nxs = [[0] * 3] * 3\nxs[0][0] = 5\nprint(sum(map(sum, xs)))",
            512,
            Grader::Number(15.0),
        ),
        case(
            "json-order-math",
            Json,
            "An order has 3 widgets at $4.50 each and 2 gadgets at $10.00 each, with a 20% discount on the subtotal. Reply with only a JSON object with keys items (total item count), subtotal (number), total (number after discount).",
            300,
            Grader::JsonFields(vec![
                ("items".into(), serde_json::json!(5)),
                ("subtotal".into(), serde_json::json!(33.5)),
                ("total".into(), serde_json::json!(26.8)),
            ]),
        ),
        case(
            "chat-brief",
            Chat,
            "In at most 25 words, tell me what a mutex is. Mention the word 'lock'.",
            200,
            Grader::Concise {
                max_words: 25,
                must: strs(&["lock"]),
            },
        ),
        case(
            "chat-format",
            Chat,
            "List exactly three primary colors of light, comma-separated, nothing else.",
            100,
            Grader::Concise {
                max_words: 6,
                must: strs(&["red", "green", "blue"]),
            },
        ),
    ]
}

/// Tool definitions offered to tool-call cases.
#[must_use]
pub fn bench_tools() -> Vec<vox_actor_runtime::llm::LlmToolDef> {
    vec![
        vox_actor_runtime::llm::LlmToolDef {
            name: "get_weather".into(),
            description: Some("Current weather for a city".into()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }),
        },
        vox_actor_runtime::llm::LlmToolDef {
            name: "convert_currency".into(),
            description: Some("Convert an amount between currencies".into()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "number" },
                    "from": { "type": "string" },
                    "to": { "type": "string" }
                },
                "required": ["amount", "from", "to"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_grader_takes_the_final_number_and_ignores_working() {
        let g = Grader::Number(150.0);
        assert_eq!(grade_text(&g, "210 / 84 = 2.5 hours, so 150"), 1.0);
        assert_eq!(grade_text(&g, "It takes 150 minutes.\n**150**"), 1.0);
        assert_eq!(grade_text(&g, "150 minutes... no, 2.5"), 0.0);
        assert_eq!(grade_text(&Grader::Number(1299.5), "total: 1,299.50"), 1.0);
    }

    #[test]
    fn json_grader_scores_partial_credit_and_survives_fences() {
        let g = Grader::JsonFields(vec![
            ("sentiment".into(), serde_json::json!("mixed")),
            ("mentions_support".into(), serde_json::json!(true)),
        ]);
        assert_eq!(
            grade_text(
                &g,
                "```json\n{\"sentiment\": \"Mixed\", \"mentions_support\": true}\n```"
            ),
            1.0
        );
        assert_eq!(
            grade_text(
                &g,
                "{\"sentiment\": \"negative\", \"mentions_support\": true}"
            ),
            0.5
        );
        assert_eq!(grade_text(&g, "no json here"), 0.0);
    }

    #[test]
    fn tool_call_grader_distinguishes_wrong_argument_from_wrong_tool() {
        let g = Grader::ToolCall {
            name: "get_weather".into(),
            arg: "city".into(),
            value: serde_json::json!("Lisbon"),
        };
        assert_eq!(
            grade_tool_call(
                &g,
                &[("get_weather".into(), serde_json::json!({"city": "lisbon"}))]
            ),
            1.0
        );
        assert_eq!(
            grade_tool_call(
                &g,
                &[("get_weather".into(), serde_json::json!({"city": "Porto"}))]
            ),
            0.5
        );
        assert_eq!(
            grade_tool_call(&g, &[("convert_currency".into(), serde_json::json!({}))]),
            0.0
        );
        assert_eq!(grade_tool_call(&g, &[]), 0.0);
    }

    #[test]
    fn concise_grader_enforces_word_budget() {
        let g = Grader::Concise {
            max_words: 6,
            must: strs(&["red", "green", "blue"]),
        };
        assert_eq!(grade_text(&g, "red, green, blue"), 1.0);
        assert_eq!(
            grade_text(
                &g,
                "The three primary colors of light are red, green, and blue."
            ),
            0.0
        );
    }

    #[test]
    fn vox_grader_requires_the_named_fn_and_a_clean_compile() {
        let g = Grader::VoxCompiles {
            fn_name: "bigger".into(),
        };
        let good = "```vox\nfn bigger(a: int, b: int) to int {\n    if a > b {\n        return a\n    } else {\n        return b\n    }\n}\n```";
        assert_eq!(
            grade_text(&g, good),
            1.0,
            "diagnostics: {:?}",
            vox_compiler::pipeline::check_file(&fenced_code(good), "t.vox")
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            grade_text(&g, "```vox\nfn other() to int { return 1 }\n```"),
            0.0
        );
        assert_eq!(
            grade_text(
                &g,
                "```vox\nfn bigger(a: int, b: int) to int { return a + }\n```"
            ),
            0.0
        );
    }

    #[test]
    fn needle_prompt_buries_exactly_one_fact_and_scales() {
        let short = needle_prompt(100, "AB-1");
        let long = needle_prompt(1600, "AB-1");
        assert_eq!(short.matches("AB-1").count(), 1);
        assert!(long.len() > short.len() * 10);
    }

    #[test]
    fn builtin_cases_have_unique_ids_and_cover_every_suite() {
        let cases = builtin_cases();
        let mut ids: Vec<_> = cases.iter().map(|c| c.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), cases.len());
        for s in Suite::ALL {
            assert!(
                cases.iter().any(|c| c.suite == s),
                "{} has no case",
                s.as_str()
            );
            assert_eq!(Suite::parse(s.as_str()), Some(s));
        }
        assert!(
            cases.len() >= 8,
            "a single run must reach the Wilson verdict floor"
        );
    }
}
