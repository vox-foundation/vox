//! Per-declaration training pairs with compiler-verified answers and error-correction mutants.
//!
//! One `.vox` file becomes one pair per top-level declaration (answer = that declaration
//! with its own doc comments / decorators), plus a whole-file pair only when the author
//! wrote a `// @training_prompt:` for it. Every answer is re-compiled together with the
//! declarations it references; failures are dropped and counted. Error-correction pairs
//! mutate a single declaration, keep the mutant only if the compiler rejects it, and put
//! the real diagnostic in the prompt.

use std::collections::BTreeSet;

use vox_compiler::ast::decl::Decl;
use vox_compiler::lexer::Token;

use super::instruction::{
    extract_name_from_source, instruction_templates, split_training_metadata,
};

/// Counters for the `vox corpus pairs` report.
#[derive(Debug, Default, Clone, Copy)]
pub struct PairStats {
    pub files: u32,
    /// Files whose metadata-stripped source did not compile (skipped entirely).
    pub files_failed: u32,
    /// Negative fixtures (`// expect-error:`, e.g. `examples/forbidden/`) skipped entirely.
    pub files_expect_error: u32,
    pub decls: u32,
    pub decl_pairs: u32,
    pub whole_file_pairs: u32,
    /// Declaration answers that did not compile in context (dropped).
    pub verify_failed: u32,
    /// Declarations with no name and no name-free template (dropped).
    pub unnamed_skipped: u32,
    pub mutants_tried: u32,
    /// Mutants the compiler accepted (dropped: not a real error).
    pub mutants_accepted: u32,
    pub error_pairs: u32,
    /// Declarations skipped for mutation by `ERROR_CORRECTION_SAMPLE_PCT` (not
    /// attempted at all — distinct from `mutants_accepted`, which is a mutation
    /// that WAS attempted but didn't produce a compile error).
    pub error_correction_sampled_out: u32,
}

impl PairStats {
    pub fn add(&mut self, o: &PairStats) {
        self.files += o.files;
        self.files_failed += o.files_failed;
        self.files_expect_error += o.files_expect_error;
        self.decls += o.decls;
        self.decl_pairs += o.decl_pairs;
        self.whole_file_pairs += o.whole_file_pairs;
        self.verify_failed += o.verify_failed;
        self.unnamed_skipped += o.unnamed_skipped;
        self.mutants_tried += o.mutants_tried;
        self.mutants_accepted += o.mutants_accepted;
        self.error_pairs += o.error_pairs;
        self.error_correction_sampled_out += o.error_correction_sampled_out;
    }
}

/// One top-level declaration cut out of a file.
#[derive(Debug, Clone)]
struct DeclUnit {
    /// Construct category (`function`, `query`, `table`, `import`, …).
    category: String,
    name: Option<String>,
    /// Leading doc comments + decorators + the declaration itself.
    text: String,
    /// Leading `//` comment block, comment markers stripped.
    doc: Option<String>,
}

impl DeclUnit {
    /// The declaration without its leading comment lines.
    fn code(&self) -> &str {
        let mut rest = self.text.as_str();
        while rest.trim_start().starts_with("//") {
            rest = rest.split_once('\n').map_or("", |(_, r)| r);
        }
        rest
    }

    /// What the prompt shows for a referenced declaration: the header of callables,
    /// the whole definition of data declarations (types, tables, consts).
    fn signature(&self) -> String {
        let code = self.code();
        let callable = matches!(
            self.category.as_str(),
            "function"
                | "query"
                | "mutation"
                | "server"
                | "workflow"
                | "activity"
                | "mcp_tool"
                | "mcp_resource"
                | "component"
                | "actor"
                | "test"
                | "scheduled"
        );
        match code.find('{').filter(|_| callable) {
            Some(i) => code[..i].trim_end().to_string(),
            None => code.to_string(),
        }
    }
}

fn category_of(decl: &Decl, code: &str) -> String {
    match decl {
        Decl::Function(_) => "function".into(),
        Decl::ReactiveComponent(_) => "component".into(),
        Decl::Endpoint(_) => code
            .lines()
            .map(str::trim_start)
            .find(|l| !l.starts_with("//") && !l.starts_with('@'))
            .and_then(|l| l.split_whitespace().next())
            .filter(|kw| matches!(*kw, "query" | "mutation" | "server"))
            .unwrap_or("endpoint")
            .into(),
        d => d.kind_name().into(),
    }
}

/// Cut `code` (already metadata-stripped) into declarations using the parsed module's spans.
///
/// Parser spans start at (or just after) the declaration keyword and may overshoot into the
/// next token, so each region starts at its line, walks back over directly-preceding `//` and
/// `@decorator` lines, and ends before the next region.
fn split_decls(code: &str, decls: &[Decl]) -> Vec<DeclUnit> {
    let line_start = |i: usize| code[..i.min(code.len())].rfind('\n').map_or(0, |p| p + 1);
    let mut starts = Vec::with_capacity(decls.len());
    let mut floor = 0usize;
    for d in decls {
        let mut s = line_start(d.span().start).max(floor);
        while s > floor {
            let prev = line_start(s - 1);
            let t = code[prev..s].trim();
            if t.starts_with("//") || t.starts_with('@') {
                s = prev;
            } else {
                break;
            }
        }
        starts.push(s);
        floor = s;
    }
    decls
        .iter()
        .enumerate()
        .filter_map(|(i, d)| {
            let limit = starts.get(i + 1).copied().unwrap_or(code.len());
            let end = d.span().end.clamp(starts[i], limit);
            let mut lines: Vec<&str> = code[starts[i]..end].lines().collect();
            while lines
                .last()
                .is_some_and(|l| l.trim().is_empty() || l.trim_start().starts_with("//"))
            {
                lines.pop();
            }
            let text = lines.join("\n");
            if text.trim().is_empty() {
                return None;
            }
            let doc: Vec<&str> = lines
                .iter()
                .map(|l| l.trim())
                .take_while(|l| l.starts_with("//"))
                .map(|l| l.trim_start_matches('/').trim())
                .collect();
            let doc = Some(doc.join(" ").trim().to_string()).filter(|d| !d.is_empty());
            let unit = DeclUnit {
                category: category_of(d, &text),
                name: extract_name_from_source(&text),
                doc,
                text,
            };
            Some(unit)
        })
        .collect()
}

/// Identifiers a declaration mentions (other than its own name).
fn referenced_names(unit: &DeclUnit) -> BTreeSet<String> {
    vox_compiler::lexer::lex(&unit.text)
        .into_iter()
        .filter_map(|t| match t.token {
            Token::Ident(s) | Token::TypeIdent(s) => Some(s),
            _ => None,
        })
        .filter(|s| unit.name.as_ref() != Some(s))
        .collect()
}

/// Indices of the declarations `idx` needs, transitively (excluding `idx`, imports always included).
fn context_for(units: &[DeclUnit], idx: usize) -> Vec<usize> {
    let mut need: BTreeSet<usize> = units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.category == "import")
        .map(|(i, _)| i)
        .collect();
    let mut stack = vec![idx];
    let mut seen = BTreeSet::from([idx]);
    while let Some(i) = stack.pop() {
        let refs = referenced_names(&units[i]);
        for (j, u) in units.iter().enumerate() {
            if u.name.as_ref().is_some_and(|n| refs.contains(n)) && seen.insert(j) {
                need.insert(j);
                stack.push(j);
            }
        }
    }
    need.remove(&idx);
    need.into_iter().collect()
}

fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The task sentence for one declaration: author prompt when it describes this declaration,
/// else its doc comment, else a construct template with the real name.
fn task_for(unit: &DeclUnit, training_prompt: Option<&str>, only_decl: bool) -> Option<String> {
    let label = unit.category.replace('_', " ");
    if let Some(tp) = training_prompt {
        let mentions = unit.name.as_ref().is_some_and(|n| {
            tp.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == n)
        });
        if only_decl {
            return Some(tp.to_string());
        }
        if mentions {
            let name = unit.name.as_deref().unwrap_or_default();
            return Some(format!("{tp}\n\nWrite only the {label} `{name}`."));
        }
    }
    if let Some(doc) = &unit.doc {
        return Some(match &unit.name {
            Some(n) => format!("Write a Vox {label} `{n}`: {doc}"),
            None => format!("Write a Vox {label}: {doc}"),
        });
    }
    // Named declarations only get templates that mention the name (a nameless task
    // like "Create an MCP tool" is not answerable with one specific tool).
    let named = unit.name.is_some();
    let templates: Vec<&str> = instruction_templates(&unit.category)
        .iter()
        .copied()
        .filter(|t| t.contains("{name}") == named)
        .collect();
    let templates = if templates.is_empty() && named {
        vec!["Write a Vox {kind} called {name}"]
    } else {
        templates
    };
    let key = format!("{}{}", unit.category, unit.name.as_deref().unwrap_or(""));
    let t = templates.get((fnv1a(&key) % templates.len().max(1) as u64) as usize)?;
    Some(
        t.replace("{kind}", &label)
            .replace("{name}", unit.name.as_deref().unwrap_or_default()),
    )
}

/// Compiler errors for `src`, each rendered with its structured details. Lines are
/// reported relative to `answer_offset` (the byte where the declaration under test starts).
fn compile_errors(src: &str, path: &str, answer_offset: usize) -> Vec<String> {
    use vox_compiler::parser::error::ParseSeverity;
    use vox_compiler::typeck::diagnostics::TypeckSeverity;
    let line_of = |at: usize| {
        (at >= answer_offset && at <= src.len())
            .then(|| src[answer_offset..at].matches('\n').count() + 1)
    };
    let with_line = |msg: String, at: usize| match line_of(at) {
        Some(l) => format!("line {l}: {msg}"),
        None => msg,
    };
    let tokens = vox_compiler::lexer::lex(src);
    if let Err(errs) = vox_compiler::parser::parse_and_warnings(tokens) {
        return errs
            .into_iter()
            .filter(|e| e.severity == ParseSeverity::Error)
            .map(|e| {
                let mut m = e.message.clone();
                if !e.expected.is_empty() {
                    m.push_str(&format!(" (expected: {})", e.expected.join(", ")));
                }
                if let Some(f) = &e.found {
                    m.push_str(&format!(" (found: {f})"));
                }
                if let Some(r) = &e.replacement {
                    m.push_str(&format!(" (replace `{}` with `{}`)", r.from, r.to));
                }
                with_line(m, e.span.start)
            })
            .collect();
    }
    match vox_compiler::pipeline::run_frontend_str(src, path) {
        Err(e) => vec![e.to_string()],
        Ok(r) => r
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .map(|d| {
                let mut m = d.message.clone();
                if let (Some(x), Some(f)) = (&d.expected_type, &d.found_type) {
                    m.push_str(&format!(" (expected `{x}`, found `{f}`)"));
                }
                if let Some(s) = d.suggestions.first() {
                    m.push_str(&format!(" (hint: {s})"));
                }
                with_line(m, d.span.start)
            })
            .collect(),
    }
}

/// Assemble `context decls + body` and return it with the byte offset of `body`.
fn in_context(units: &[DeclUnit], ctx: &[usize], body: &str) -> (String, usize) {
    let mut src: String = ctx
        .iter()
        .map(|&j| format!("{}\n\n", units[j].text))
        .collect();
    let off = src.len();
    src.push_str(body);
    src.push('\n');
    (src, off)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutKind {
    WrongKeyword,
    MissingBrace,
    TypeMismatch,
    UndefinedName,
    WrongReturnType,
    RetiredDecorator,
}

const MUT_KINDS: [MutKind; 6] = [
    MutKind::WrongKeyword,
    MutKind::MissingBrace,
    MutKind::TypeMismatch,
    MutKind::UndefinedName,
    MutKind::WrongReturnType,
    MutKind::RetiredDecorator,
];

/// One seeded mutant of `unit.text` of the given kind, or `None` if the kind does not apply.
fn mutate(unit: &DeclUnit, kind: MutKind, seed: u64) -> Option<String> {
    let text = &unit.text;
    let toks = vox_compiler::lexer::lex(text);
    let pick = |cands: Vec<(std::ops::Range<usize>, String)>| -> Option<String> {
        let (r, rep) = cands
            .get((seed % cands.len().max(1) as u64) as usize)?
            .clone();
        Some(format!("{}{}{}", &text[..r.start], rep, &text[r.end..]))
    };
    match kind {
        MutKind::WrongKeyword => pick(
            toks.iter()
                .filter_map(|t| {
                    let rep = match t.token {
                        Token::Let => "lett",
                        Token::Return => "retrun",
                        Token::Fn => "fun",
                        Token::Match => "mach",
                        _ => return None,
                    };
                    Some((t.span.clone(), rep.to_string()))
                })
                .collect(),
        ),
        MutKind::MissingBrace => {
            let t = toks.iter().rev().find(|t| t.token == Token::RBrace)?;
            Some(format!("{}{}", &text[..t.span.start], &text[t.span.end..]))
        }
        MutKind::TypeMismatch => pick(
            toks.iter()
                .filter_map(|t| match &t.token {
                    Token::IntLit(v) => Some((t.span.clone(), format!("\"{v}\""))),
                    Token::StringLit(_) => Some((t.span.clone(), "0".to_string())),
                    _ => None,
                })
                .collect(),
        ),
        MutKind::UndefinedName => pick(
            toks.iter()
                .enumerate()
                .filter_map(|(i, t)| match &t.token {
                    Token::Ident(s)
                        if s.len() > 3
                            && unit.name.as_ref() != Some(s)
                            && (i == 0 || toks[i - 1].token != Token::Dot) =>
                    {
                        Some((t.span.clone(), s[..s.len() - 1].to_string()))
                    }
                    _ => None,
                })
                .collect(),
        ),
        MutKind::WrongReturnType => pick(
            toks.windows(2)
                .filter(|w| w[0].token == Token::To)
                .filter_map(|w| match &w[1].token {
                    Token::Ident(s) => {
                        let rep = match s.as_str() {
                            "int" => "str",
                            "str" => "int",
                            "bool" => "int",
                            "float" => "str",
                            _ => return None,
                        };
                        Some((w[1].span.clone(), rep.to_string()))
                    }
                    _ => None,
                })
                .collect(),
        ),
        MutKind::RetiredDecorator => {
            let kw = unit.category.as_str();
            if !matches!(
                kw,
                "table" | "query" | "mutation" | "server" | "form" | "index"
            ) {
                return None;
            }
            let at = text
                .match_indices(&format!("{kw} "))
                .map(|(i, _)| i)
                .find(|&i| i == 0 || text[..i].ends_with('\n') || text[..i].ends_with(' '))?;
            let rep = if matches!(kw, "query" | "mutation" | "server") {
                format!("@{kw} fn ")
            } else {
                format!("@{kw} ")
            };
            Some(format!(
                "{}{}{}",
                &text[..at],
                rep,
                &text[at + kw.len() + 1..]
            ))
        }
    }
    .filter(|m| m != text)
}

fn pair_json(
    prompt: &str,
    response: &str,
    category: &str,
    task_family: &str,
    source: &str,
    rating: u8,
) -> serde_json::Value {
    serde_json::json!({
        "prompt": prompt,
        "response": response,
        "messages": [
            { "role": "user", "content": prompt },
            { "role": "assistant", "content": response }
        ],
        "instruction": prompt,
        "output": response,
        "category": category,
        "difficulty": super::construct_difficulty(category),
        "source": source,
        "rating": rating,
        "schema_version": super::SCHEMA_VERSION,
        "lane": "vox_codegen",
        "response_mode": "code_only",
        "task_family": task_family,
    })
}

/// Max error-correction mutants kept per declaration.
const MUTANTS_PER_DECL: usize = 1;

/// Percent (0-100) of eligible declarations that get an error-correction mutation
/// attempt at all. At 100% (the old behavior), error_correction pairs came out
/// roughly 1:1 with positive declaration pairs — 655 of 1,510 pairs (43%) in a
/// full-repo run, the single largest category. Sampled deterministically (from
/// the same per-declaration seed used to pick mutation kinds), so a given file's
/// output is stable across reruns.
const ERROR_CORRECTION_SAMPLE_PCT: u64 = 40;

/// Build every training pair for one extracted file (`raw_code` as written by `vox corpus
/// extract`, frontmatter included). `source` is the file path; it is also handed to the
/// frontend so local-file imports resolve.
pub fn pairs_for_file(raw_code: &str, source: &str) -> (Vec<serde_json::Value>, PairStats) {
    let mut st = PairStats {
        files: 1,
        ..PairStats::default()
    };
    let mut out = Vec::new();
    let (code, training_prompt) = split_training_metadata(raw_code);
    // Fixtures that compile at the frontend but are rejected by a later layer are
    // negative examples; never teach them as answers.
    if code
        .lines()
        .any(|l| l.trim_start().starts_with("// expect-error:"))
    {
        st.files_expect_error = 1;
        return (out, st);
    }
    let path = if std::path::Path::new(source).is_file() {
        source
    } else {
        ""
    };
    let module = match vox_compiler::pipeline::run_frontend_str(&code, path) {
        Ok(r) if r.error_count() == 0 => r.module,
        _ => {
            st.files_failed = 1;
            return (out, st);
        }
    };
    let units = split_decls(&code, &module.declarations);
    let pairable: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].category != "import")
        .collect();
    let only_decl = pairable.len() == 1;

    if let Some(tp) = training_prompt.as_deref().filter(|_| !only_decl) {
        out.push(pair_json(tp, &code, "program", "vox_codegen", source, 5));
        st.whole_file_pairs += 1;
    }

    for &i in &pairable {
        st.decls += 1;
        let unit = &units[i];
        let Some(task) = task_for(unit, training_prompt.as_deref(), only_decl) else {
            st.unnamed_skipped += 1;
            continue;
        };
        let ctx = context_for(&units, i);
        let (src, off) = in_context(&units, &ctx, &unit.text);
        if !compile_errors(&src, path, off).is_empty() {
            st.verify_failed += 1;
            continue;
        }
        let ctx_block = |p: String| {
            if ctx.is_empty() {
                return p;
            }
            let sigs: Vec<String> = ctx.iter().map(|&j| units[j].signature()).collect();
            format!(
                "{p}\n\nThe file already defines:\n```vox\n{}\n```",
                sigs.join("\n\n")
            )
        };
        out.push(pair_json(
            &ctx_block(task),
            &unit.text,
            &unit.category,
            "vox_codegen",
            source,
            5,
        ));
        st.decl_pairs += 1;

        let seed = fnv1a(&format!(
            "{source}\0{}\0{i}",
            unit.name.as_deref().unwrap_or("")
        ));
        if seed % 100 >= ERROR_CORRECTION_SAMPLE_PCT {
            st.error_correction_sampled_out += 1;
            continue;
        }
        let mut kept = 0;
        for k in 0..MUT_KINDS.len() {
            if kept == MUTANTS_PER_DECL {
                break;
            }
            let kind = MUT_KINDS[(seed as usize + k) % MUT_KINDS.len()];
            let Some(mutant) = mutate(unit, kind, seed >> 8) else {
                continue;
            };
            st.mutants_tried += 1;
            let (msrc, moff) = in_context(&units, &ctx, &mutant);
            let errs = compile_errors(&msrc, path, moff);
            if errs.is_empty() {
                st.mutants_accepted += 1;
                continue;
            }
            let diag: String = errs.iter().take(3).map(|e| format!("- {e}\n")).collect();
            let prompt = ctx_block(format!(
                "This Vox declaration does not compile. The compiler reports:\n{diag}\nFix it and return only the corrected declaration.\n\n```vox\n{mutant}\n```"
            ));
            out.push(pair_json(
                &prompt,
                &unit.text,
                "error_correction",
                "error_correction",
                source,
                4,
            ));
            st.error_pairs += 1;
            kept += 1;
        }
    }
    (out, st)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units_of(code: &str) -> Vec<DeclUnit> {
        let r = vox_compiler::pipeline::run_frontend_str(code, "").expect("parse");
        assert_eq!(r.error_count(), 0, "{:?}", r.diagnostics);
        split_decls(code, &r.module.declarations)
    }

    const SRC: &str = "// A user row.\ntable User {\n    name: str\n    active: bool\n}\n\n// Count users.\nquery user_count() to int {\n    return len(db.User.all())\n}\n\nfn add(a: int, b: int) to int {\n    return a + b\n}\n\n@test\nfn adds() {\n    assert(add(1, 2) is 3)\n}\n";

    #[test]
    fn splits_decls_with_docs_and_decorators() {
        let u = units_of(SRC);
        let got: Vec<(&str, Option<&str>)> = u
            .iter()
            .map(|u| (u.category.as_str(), u.name.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("table", Some("User")),
                ("query", Some("user_count")),
                ("function", Some("add")),
                ("test", Some("adds")),
            ]
        );
        assert_eq!(
            u[0].text,
            "// A user row.\ntable User {\n    name: str\n    active: bool\n}"
        );
        assert_eq!(u[0].doc.as_deref(), Some("A user row."));
        assert!(u[1].text.starts_with("// Count users.\nquery user_count()"));
        assert!(u[1].text.ends_with("}"));
        assert_eq!(
            u[3].text,
            "@test\nfn adds() {\n    assert(add(1, 2) is 3)\n}"
        );
        assert_eq!(u[2].doc, None);
    }

    #[test]
    fn context_lists_referenced_decls_only() {
        let u = units_of(SRC);
        assert_eq!(context_for(&u, 1), vec![0]); // query -> table
        assert_eq!(context_for(&u, 3), vec![2]); // test -> add
        assert!(context_for(&u, 2).is_empty());
        assert_eq!(u[1].signature(), "query user_count() to int");
        assert!(u[0].signature().starts_with("table User {"));
    }

    #[test]
    fn prompt_prefers_author_prompt_then_doc_then_template() {
        let u = units_of(SRC);
        assert_eq!(
            task_for(&u[0], Some("Build a user store."), true).as_deref(),
            Some("Build a user store.")
        );
        let t = task_for(&u[2], Some("Implement add and its test."), false).unwrap();
        assert!(
            t.starts_with("Implement add and its test.") && t.contains("`add`"),
            "{t}"
        );
        assert_eq!(
            task_for(&u[1], Some("Unrelated file task."), false).as_deref(),
            Some("Write a Vox query `user_count`: Count users.")
        );
        let t = task_for(&u[2], None, false).unwrap();
        assert!(t.contains("add") && !t.contains("example"), "{t}");
        let unit = |category: &str, name: Option<&str>| DeclUnit {
            category: category.into(),
            name: name.map(String::from),
            text: String::new(),
            doc: None,
        };
        for cat in ["mcp_tool", "test", "http_route", "state_machine"] {
            let t = task_for(&unit(cat, Some("read_file")), None, false).unwrap();
            assert!(t.contains("read_file"), "{cat}: {t}");
        }
        assert_eq!(
            task_for(&unit("routes", None), None, false).as_deref(),
            Some("Define client-side routes in Vox")
        );
        assert_eq!(task_for(&unit("function", None), None, false), None);
    }

    #[test]
    fn mutants_are_rejected_by_compiler_or_dropped() {
        let u = units_of(SRC);
        // Retired decorator spelling carries the compiler's replacement.
        let m = mutate(&u[0], MutKind::RetiredDecorator, 0).unwrap();
        assert!(m.contains("@table User"), "{m}");
        let errs = compile_errors(&format!("{m}\n"), "", 0);
        assert!(errs.iter().any(|e| e.contains("table")), "{errs:?}");
        // Missing brace is a parse error.
        let m = mutate(&u[2], MutKind::MissingBrace, 0).unwrap();
        assert!(!compile_errors(&m, "", 0).is_empty());
        // A no-op-ish mutant the compiler accepts yields no error pair.
        let accepted = "fn add(a: int, b: int) to int {\n    return a + b\n}\n";
        assert!(compile_errors(accepted, "", 0).is_empty());
        // Retired decorator does not apply to plain functions.
        assert!(mutate(&u[2], MutKind::RetiredDecorator, 0).is_none());
    }

    #[test]
    fn pairs_for_file_emits_verified_decl_pairs_and_error_pairs() {
        let raw = format!(
            "// ---\n// title: \"X\"\n// ---\n// @training_prompt: Build a user store with an add helper.\n// ANCHOR: a\n{SRC}// ANCHOR_END: a\n"
        );
        let (pairs, st) = pairs_for_file(&raw, "mem.vox");
        assert_eq!(st.whole_file_pairs, 1);
        assert_eq!(st.decl_pairs, 4, "{st:?}");
        assert_eq!(st.verify_failed, 0);
        // ERROR_CORRECTION_SAMPLE_PCT gates which of the 4 decls get a mutation
        // attempt at all; deterministic per (source, name, index), so this fixture
        // always comes out the same way. All 4 decl_pairs above exist regardless.
        assert_eq!(st.error_pairs, 1, "{st:?}");
        assert_eq!(st.error_correction_sampled_out, 3, "{st:?}");
        assert_eq!(st.mutants_tried, st.error_pairs + st.mutants_accepted);
        for p in &pairs {
            let r = p["response"].as_str().unwrap();
            assert!(
                !r.contains("ANCHOR") && !r.contains("// ---") && !r.contains("@training_prompt")
            );
        }
        // Every error pair's prompt carries a real diagnostic and answers with one decl.
        let decl_texts: Vec<String> = units_of(SRC).into_iter().map(|u| u.text).collect();
        for p in pairs.iter().filter(|p| p["category"] == "error_correction") {
            assert!(
                p["prompt"]
                    .as_str()
                    .unwrap()
                    .contains("The compiler reports:\n- ")
            );
            assert!(decl_texts.iter().any(|t| p["response"] == t.as_str()));
        }
        // Deterministic.
        let (again, _) = pairs_for_file(&raw, "mem.vox");
        assert_eq!(pairs, again);
    }

    /// The sampling gate must actually reduce error_correction's share (this
    /// change exists because it was 655/1510 = 43% of a full-repo run, the
    /// single largest category) rather than being a no-op, and must land close
    /// to the configured rate rather than some other fraction.
    #[test]
    fn error_correction_sampling_hits_the_configured_rate() {
        let mut total_decls = 0u32;
        let mut sampled_in = 0u32;
        for i in 0..500 {
            let raw = format!("fn distinct_name_{i}(x: int) to int {{\n    return x + 1\n}}\n");
            let (_, st) = pairs_for_file(&raw, &format!("f{i}.vox"));
            assert_eq!(
                st.decl_pairs, 1,
                "fixture {i} must still produce its positive pair"
            );
            total_decls += 1;
            sampled_in += u32::from(st.error_correction_sampled_out == 0);
        }
        let pct = 100 * sampled_in / total_decls;
        // Hardcoded, not `ERROR_CORRECTION_SAMPLE_PCT`: comparing the measured rate
        // to the same constant that produced it is a tautology that can never catch
        // someone reverting the sampling rate back toward 100%.
        assert!(
            pct.abs_diff(40) <= 5,
            "sampled-in rate {pct}% too far from the intended ~40% over {total_decls} decls"
        );
    }

    #[test]
    fn expect_error_fixtures_are_skipped() {
        let raw = "// expect-error: vox/layer/leaf-surface\nfn a() to int {\n    return 1\n}\n";
        let (pairs, st) = pairs_for_file(raw, "forbidden.vox");
        assert!(pairs.is_empty());
        assert_eq!((st.files_expect_error, st.decls), (1, 0));
    }

    /// Real golden files: every answer compiles in its context and carries no file metadata.
    #[test]
    fn golden_files_answers_compile() {
        for f in ["crud_api", "nested_types", "checkout_workflow"] {
            let p = format!(
                "{}/../../examples/golden/{f}.vox",
                env!("CARGO_MANIFEST_DIR")
            );
            let raw = std::fs::read_to_string(&p).unwrap();
            let (pairs, st) = pairs_for_file(&raw, &p);
            assert_eq!(st.files_failed, 0, "{f}");
            assert!(st.decl_pairs >= 2, "{f}: {st:?}");
            for pair in &pairs {
                let r = pair["response"].as_str().unwrap();
                assert!(
                    !r.contains("ANCHOR")
                        && !r.contains("training_eligible")
                        && !r.contains("@training_prompt"),
                    "{f}: {r}"
                );
            }
            // Re-verify decl answers independently: answer plus the prompt's context
            // cannot be reassembled from signatures, so re-run the file-level splitter.
            let (code, _) = split_training_metadata(&raw);
            let module = vox_compiler::pipeline::run_frontend_str(&code, &p)
                .unwrap()
                .module;
            let units = split_decls(&code, &module.declarations);
            for (i, u) in units.iter().enumerate() {
                if u.category == "import" {
                    continue;
                }
                let (src, off) = in_context(&units, &context_for(&units, i), &u.text);
                assert!(compile_errors(&src, &p, off).is_empty(), "{f}: {}", u.text);
                assert!(
                    pairs.iter().any(|x| x["response"] == u.text.as_str()),
                    "{f}: missing {}",
                    u.text
                );
            }
        }
    }
}
