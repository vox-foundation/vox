#![allow(clippy::module_inception)]
//! Structural pipeline-gap regression tests — guard the *named headline bug* of the
//! semantic-coverage initiative, not a leaf utility.
//!
//! Pattern #1 (silent-drop catch-all) from the pipeline-gap audit: the parser emits
//! [`crate::ast::decl::Decl::Const`] for BOTH `const Name = …` and top-level `let name = …`
//! (see `parser/descent/mod.rs`). HIR lowering MUST route that node into `hir.consts`
//! (`hir/lower/mod.rs`), never into the `legacy_ast_nodes` `_ =>` catch-all that emits a
//! `LOWER_UNLOWERED_DECL` "silently dropped" warning. These tests pin that wiring so the
//! binding (and its initializer value) cannot vanish between stages again.
//!
//! Every test carries a `// Catches:` comment naming the concrete regression it guards.

#[cfg(test)]
mod semcov_struct_pipeline_tests {
    #![allow(clippy::module_inception)]
    use crate::ast::decl::Decl;
    use crate::hir::lower::lower_module;
    use crate::hir::nodes::{HirExpr, HirStmt};
    use crate::lexer::cursor::lex;
    use crate::parser::parse;

    fn lower(src: &str) -> crate::hir::nodes::HirModule {
        let m = parse(lex(src)).unwrap_or_else(|e| panic!("expected parse ok, got: {e:?}"));
        lower_module(&m)
    }

    #[test]
    fn top_level_let_lowers_to_const_not_swallowed_by_catch_all() {
        // Catches: top-level `let` (parsed as Decl::Const) falling into the
        // `legacy_ast_nodes` `_ =>` catch-all in hir/lower/mod.rs and silently vanishing —
        // the named headline pipeline-gap bug.
        // NB: no trailing `;` — Vox has no semicolon statement-separator token; since
        // Task 4 (Token::Unknown) a stray `;` now lexes to a real, parser-visible
        // token instead of being silently dropped.
        let hir = lower("let answer = 42");
        assert_eq!(
            hir.consts.len(),
            1,
            "top-level let must lower to exactly one HirConst, got {:?}",
            hir.consts
        );
        assert_eq!(hir.consts[0].name, "answer");
        assert!(
            !hir.legacy_ast_nodes
                .iter()
                .any(|d| matches!(d, Decl::Const(_))),
            "the binding must NOT land in the legacy_ast_nodes catch-all"
        );
        assert!(
            !hir.lower_warnings
                .iter()
                .any(|w| w.contains("silently dropped")),
            "no unlowered-decl warning should fire for a top-level let: {:?}",
            hir.lower_warnings
        );
    }

    #[test]
    fn top_level_let_initializer_value_survives_lowering() {
        // Catches: the binding being lowered by NAME while the initializer is dropped or
        // zeroed — a "half-lowered" variant of the silent-drop bug that a name-only
        // assertion would miss.
        let hir = lower("let answer = 42");
        assert_eq!(hir.consts.len(), 1);
        assert!(
            matches!(hir.consts[0].value, HirExpr::IntLit(42, _)),
            "initializer literal 42 must survive lowering, got {:?}",
            hir.consts[0].value
        );
    }

    #[test]
    fn let_type_annotation_survives_lowering() {
        // Catches: the optional `: Type` annotation on a top-level `let` being dropped
        // during lowering (lower_const maps c.type_ann through lower_type) — a binding can
        // survive by name+value while its declared type silently vanishes.
        let hir = lower("let titled: Int = 7");
        assert_eq!(
            hir.consts.len(),
            1,
            "annotated let must lower to one HirConst"
        );
        assert_eq!(hir.consts[0].name, "titled");
        assert!(
            hir.consts[0].type_ann.is_some(),
            "declared type annotation must survive lowering, got None"
        );
        assert!(
            matches!(hir.consts[0].value, HirExpr::IntLit(7, _)),
            "initializer must survive, got {:?}",
            hir.consts[0].value
        );
    }

    #[test]
    fn multiple_top_level_lets_none_swallowed() {
        // Catches: the lowering loop dropping a binding when several siblings are present
        // (e.g. an early `continue`/overwrite that loses all but one) — each must survive.
        let hir = lower("let keep = 1\nlet also = 2\nlet third = 3");
        let names: Vec<&str> = hir.consts.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            hir.consts.len(),
            3,
            "all three top-level lets must lower; got {names:?}"
        );
        assert!(
            names.contains(&"keep") && names.contains(&"also") && names.contains(&"third"),
            "every binding name must be present; got {names:?}"
        );
    }

    #[test]
    fn let_string_initializer_survives_lowering() {
        // Catches: a non-numeric initializer (string literal) being dropped or mistyped
        // while numeric ones survive — the value must round-trip through lowering intact,
        // not just for IntLit.
        let hir = lower("let greeting = \"hi\"");
        assert_eq!(hir.consts.len(), 1);
        assert_eq!(hir.consts[0].name, "greeting");
        assert!(
            matches!(hir.consts[0].value, HirExpr::StringLit(ref s, _) if s == "hi"),
            "string initializer must survive lowering, got {:?}",
            hir.consts[0].value
        );
    }

    // ── Pattern #5: half-wired `when {}` blocks ─────────────────────────────
    // A Vox `when src { fetching => … empty => … error e => … ok x => … }`
    // parses to `Expr::AsyncView` (4 distinct Option arm slots) and lowers via
    // independent `.map(...)` lines in hir/lower/expr.rs. Dropping or mis-wiring
    // any single arm during a refactor would silently lose that branch with NO
    // parse error — exactly the "half-wired" gap. This pins all four arms.

    fn lower_when_view() -> crate::hir::nodes::HirAsyncView {
        let hir = lower(
            "fn render(data: Async[Int]) -> Int {\n\
             when data {\n\
             fetching => 0\n\
             empty => 1\n\
             error e => 2\n\
             ok x => 3\n\
             }\n\
             }",
        );
        let f = hir
            .functions
            .iter()
            .find(|f| f.name == "render")
            .expect("render fn must lower");
        f.body
            .iter()
            .find_map(|s| match s {
                HirStmt::Expr {
                    expr: HirExpr::AsyncView(v),
                    ..
                } => Some((**v).clone()),
                _ => None,
            })
            .expect("when{} must lower to HirExpr::AsyncView in the fn body")
    }

    #[test]
    fn when_block_all_four_arms_survive_lowering() {
        // Catches: a refactor dropping or mis-routing one of the four `when` arms'
        // `.map(self.lower_expr)` lines in hir/lower/expr.rs — the branch vanishes
        // with no parse error (half-wired when{}).
        let view = lower_when_view();
        assert!(
            view.missing_arms().is_empty(),
            "no when-arm may be dropped in lowering; missing: {:?}",
            view.missing_arms()
        );
    }

    #[test]
    fn when_block_error_and_ok_bindings_survive_lowering() {
        // Catches: the `error e` / `ok x` arm BODY surviving while its binding name
        // is dropped — the arm would lower but reference an unbound variable.
        let view = lower_when_view();
        assert_eq!(
            view.error_binding.as_deref(),
            Some("e"),
            "error-arm binding must survive lowering"
        );
        assert_eq!(
            view.ok_binding.as_deref(),
            Some("x"),
            "ok-arm binding must survive lowering"
        );
    }

    // ── Pattern #3: context-dependent silent drop ──────────────────────────
    // The `@pure` marker must survive IDENTICALLY whether it decorates a free
    // `fn` (context A) or an `@example`-wrapped fn (context B). The two routes
    // through lowering are different (free fn -> hir.functions; example ->
    // hir.examples via lower_fn), so a decorator-threading refactor could
    // preserve purity in one context and silently drop it in the other.
    // (Note: `to int`, not `-> int`; and the wrapper decorator must precede
    // `@pure` — `@pure` first is a parse error.)

    #[test]
    fn pure_marker_survives_in_both_fn_and_example_contexts() {
        // Catches: @pure honored on a free fn but silently dropped when the same
        // fn is @example-wrapped (context-dependent drop) — asserted as a parity
        // relationship so it fires on divergence in either direction.
        let a = lower("@pure\nfn f() to int { 1 }");
        let fa = a
            .functions
            .iter()
            .find(|f| f.name == "f")
            .expect("free fn f must lower");
        assert!(fa.is_pure, "context A: @pure dropped on free fn");

        let b = lower("@example\n@pure\nfn ef() to int { 1 }");
        assert_eq!(b.examples.len(), 1, "context B: @example fn must lower");
        let eb = &b.examples[0];
        assert_eq!(eb.name, "ef");
        assert_eq!(
            fa.is_pure, eb.is_pure,
            "pattern #3: @pure survives on free fn (A) but is dropped in @example context (B)"
        );
        assert!(
            eb.is_pure,
            "context B: @pure silently dropped on @example fn"
        );
    }

    // ── R5: decorator-order asymmetry (known limitation, executable TODO) ────
    // `@pure` AFTER `@example`/`@test` parses (see the test above); `@pure` BEFORE
    // them is a hard parse error because the top-level dispatch in
    // parser/descent/mod.rs routes a leading `@pure` straight to `parse_fn_decl`,
    // whose decorator loop has no arm for `@example`/`@test`. The real fix is to
    // collect ALL leading decorators first, then dispatch on the decl-kind keyword.
    // This test pins the DESIRED behavior; remove `#[ignore]` once the refactor lands.
    #[test]
    #[ignore = "owner:compiler sunset:2026-12-31 R5: needs collect-all-leading-decorators-then-dispatch refactor in parser/descent/mod.rs"]
    fn pure_before_example_should_parse_and_stay_pure() {
        // Catches (post-fix): decorator order changing acceptance or dropping @pure.
        let m = parse(lex("@pure\n@example\nfn ef() to int { 1 }")).expect(
            "@pure before @example should parse once decorator collection is order-independent",
        );
        assert_eq!(m.declarations.len(), 1, "must lower to exactly one decl");
    }

    #[test]
    fn traced_marker_survives_lowering() {
        // Catches: @traced parsed but dropped during lowering (no HirFn.is_traced).
        let hir = lower("@traced\nfn f() to int { return 1 }");
        let f = hir.functions.iter().find(|f| f.name == "f").expect("fn f");
        assert!(
            f.is_traced,
            "@traced must survive lowering into HirFn.is_traced"
        );
    }

    #[test]
    fn at_traced_sets_fndecl_is_traced() {
        // Catches: @traced parsed but FnDecl.is_traced not set (decorator-loop gap).
        let m = parse(lex("@traced\nfn f() to int { return 1 }")).expect("parse");
        let f = m
            .declarations
            .iter()
            .find_map(|d| match d {
                Decl::Function(f) => Some(f),
                _ => None,
            })
            .expect("fn decl");
        assert!(f.is_traced, "@traced must set FnDecl.is_traced");
    }

    // ── Phase 81 (VOX-01..05): Language, Compiler & Interpreter Parity ──────

    #[test]
    fn vox_01_first_party_scripts_check_and_execute_without_panics() {
        use crate::eval::{Interpreter, value::VoxValue};
        use crate::pipeline::{PipelineOptions, run_frontend_str_with_options};
        use std::path::PathBuf;

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let scripts_dir = repo_root.join("scripts");
        let mut checked_count = 0usize;

        for entry in std::fs::read_dir(&scripts_dir).expect("read scripts/") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("vox") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("read script");
            let path_str = path.to_string_lossy().to_string();
            let res = std::panic::catch_unwind(|| {
                run_frontend_str_with_options(
                    &src,
                    &path_str,
                    &PipelineOptions {
                        script_mode: true,
                        ..Default::default()
                    },
                )
            });
            assert!(
                res.is_ok(),
                "VOX-01: compiler panicked on first-party script {}",
                path.display()
            );
            let _frontend = res
                .unwrap()
                .unwrap_or_else(|e| panic!("VOX-01: frontend error on {}: {e}", path.display()));
            checked_count += 1;
        }
        assert!(
            checked_count >= 20,
            "expected at least 20 top-level scripts/*.vox, checked {checked_count}"
        );

        // Verify deterministic execution of a canonical script in the interpreter
        let script_src = r#"fn compute_checksum(items: List[int]) to int {
    let mut acc = 0
    for x in items {
        acc = acc + x
    }
    return acc
}

fn main() to int {
    return compute_checksum([10, 20, 12])
}
"#;
        let frontend = run_frontend_str_with_options(
            script_src,
            "scripts/canonical_check.vox",
            &PipelineOptions {
                script_mode: true,
                ..Default::default()
            },
        )
        .expect("canonical script frontend");
        assert!(!frontend.has_errors(), "{:?}", frontend.diagnostics);
        let mut interp = Interpreter::new(100_000);
        interp
            .run_module(&frontend.hir)
            .expect("run canonical script module");
        let out = interp.call("main", vec![]).expect("call main");
        assert_eq!(out, VoxValue::Int(42));
    }

    #[test]
    fn vox_02_canonical_constructs_compile_and_interpret_cleanly() {
        use crate::eval::{Interpreter, value::VoxValue};
        use crate::hir::nodes::DurabilityKind;
        use crate::pipeline::run_frontend_str;

        let src = r#"table Task {
    title: str
    done: bool
}

@pure
fn score_task(done: bool) to int {
    if done {
        return 10
    }
    return 1
}

query list_task_count() to int {
    return score_task(true)
}

mutation mark_task_done(id: int) to bool {
    return id > 0
}

tool "Summarize task status" summarize_tasks(prefix: str) to str {
    return prefix
}

activity fetch_status(name: str) to Result[str] {
    return Ok(name)
}

workflow process_task(name: str) to Result[str] {
    let status = fetch_status(name)
    return status
}

actor Worker {
    on increment(count: int) to int {
        return count + 1
    }
}

component TaskBadge() {
    state label: str = "Ready"
    view: text() { label }
}
"#;
        let frontend = run_frontend_str(src, "parity_app.vox").expect("frontend ok");
        assert!(
            !frontend.has_errors(),
            "VOX-02 canonical module must have zero diagnostics: {:?}",
            frontend.diagnostics
        );

        let hir = &frontend.hir;
        assert_eq!(hir.tables.len(), 1, "table lowered");
        assert_eq!(
            hir.endpoint_fns.len(),
            2,
            "query + mutation endpoints lowered"
        );
        assert_eq!(hir.mcp_tools.len(), 1, "mcp tool lowered");
        assert_eq!(hir.components.len(), 1, "reactive component lowered");
        assert!(
            hir.functions
                .iter()
                .any(|f| f.durability == Some(DurabilityKind::Activity)),
            "activity lowered with DurabilityKind::Activity"
        );
        assert!(
            hir.functions
                .iter()
                .any(|f| f.durability == Some(DurabilityKind::Workflow)),
            "workflow lowered with DurabilityKind::Workflow"
        );
        assert!(
            hir.functions
                .iter()
                .any(|f| f.durability == Some(DurabilityKind::Actor)),
            "actor lowered with DurabilityKind::Actor"
        );

        let mut interp = Interpreter::new(100_000);
        interp.run_module(hir).expect("run canonical module");
        let pure_res = interp
            .call("score_task", vec![VoxValue::Bool(true)])
            .expect("call score_task");
        assert_eq!(pure_res, VoxValue::Int(10));
        let query_res = interp
            .call("list_task_count", vec![])
            .expect("call query endpoint in interpreter");
        assert_eq!(query_res, VoxValue::Int(10));
    }

    #[test]
    fn vox_03_scaffold_templates_pass_frontend_and_interpreter() {
        use crate::eval::{Interpreter, value::VoxValue};
        use crate::pipeline::{PipelineOptions, run_frontend_str_with_options};

        let simulation_rpg = r#"table Character {
    name: str
    hp: int
    gold: int
}

actor GameMaster {
    on advance_world(hero: str) to str {
        return hero + " explores the realm"
    }
}

@pure
fn resolve_encounter(hp: int, damage: int) to int {
    if hp > damage {
        return hp - damage
    }
    return 0
}

query world_status() to str {
    let remaining = resolve_encounter(100, 15)
    return "Hero HP: " + str(remaining)
}
"#;
        let frontend = run_frontend_str_with_options(
            simulation_rpg,
            "simulation_rpg.vox",
            &PipelineOptions::default(),
        )
        .expect("simulation-rpg frontend");
        assert!(!frontend.has_errors(), "{:?}", frontend.diagnostics);

        let mut interp = Interpreter::new(100_000);
        interp
            .run_module(&frontend.hir)
            .expect("run simulation-rpg module");
        let hp = interp
            .call(
                "resolve_encounter",
                vec![VoxValue::Int(100), VoxValue::Int(15)],
            )
            .expect("call resolve_encounter");
        assert_eq!(hp, VoxValue::Int(85));
    }

    #[test]
    fn vox_04_for_llm_diagnostics_span_accurate_and_never_panic() {
        use crate::pipeline::check_file;

        let malformed_cases = [
            (
                "unclosed_fn.vox",
                "fn broken(x: int to int {\n    return x\n",
            ),
            (
                "type_mismatch.vox",
                "fn bad_ret() to int {\n    return \"not an int\"\n}\n",
            ),
            (
                "retired_syntax.vox",
                "@table type OldUser {\n    name: str\n}\n",
            ),
            ("stray_tokens.vox", "fn f() to int { ;;; return 1 }"),
        ];

        for (file_name, src) in malformed_cases {
            let res = std::panic::catch_unwind(|| check_file(src, file_name));
            assert!(
                res.is_ok(),
                "VOX-04: check_file panicked on malformed input {file_name}"
            );
            let diags = res.unwrap();
            assert!(
                !diags.is_empty(),
                "VOX-04: expected diagnostics for malformed input {file_name}"
            );
            for d in &diags {
                assert_eq!(d.file_path, file_name);
                assert!(d.span.start_line >= 1, "span start_line must be 1-indexed");
                assert!(d.span.start_col >= 1, "span start_col must be 1-indexed");
                assert!(
                    d.span.end_line >= d.span.start_line,
                    "span end_line must be >= start_line"
                );
                assert!(
                    !d.message.is_empty(),
                    "diagnostic message must not be empty"
                );
                let json = serde_json::to_string(d).expect("serialize diagnostic for --for-llm");
                assert!(json.contains(&d.file_path));
            }
        }
    }

    #[test]
    fn vox_05_test_declarations_and_assertions_execute_deterministically() {
        use crate::eval::{EvalError, Interpreter};
        use crate::pipeline::run_frontend_str;

        let src = r#"@pure
fn add_scores(a: int, b: int) to int {
    return a + b
}

@test
fn test_addition_passes() to Unit {
    assert(add_scores(20, 22) is 42)
}

@test
fn test_addition_fails() to Unit {
    assert(add_scores(1, 1) is 3)
}
"#;
        let frontend = run_frontend_str(src, "tests_suite.vox").expect("frontend");
        assert!(!frontend.has_errors(), "{:?}", frontend.diagnostics);
        assert_eq!(
            frontend.hir.tests.len(),
            2,
            "VOX-05: both @test declarations must lower into hir.tests"
        );

        let mut interp = Interpreter::new(100_000);
        interp
            .run_module(&frontend.hir)
            .expect("run module with @test blocks");

        let pass_res = interp.call("test_addition_passes", vec![]);
        assert!(
            pass_res.is_ok(),
            "VOX-05: passing @test block must succeed: {pass_res:?}"
        );

        let fail_res = interp.call("test_addition_fails", vec![]);
        assert!(
            matches!(fail_res, Err(EvalError::AssertionFailed(_))),
            "VOX-05: failing @test block must return EvalError::AssertionFailed, got {fail_res:?}"
        );
    }
}
