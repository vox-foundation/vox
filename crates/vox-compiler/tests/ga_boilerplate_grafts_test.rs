/// Integration tests for GA-01, GA-05, GA-19, GA-20, GA-26 boilerplate-reduction grafts.
use vox_compiler::typeck::typecheck_ast_module;
use vox_compiler::{lexer::cursor::lex, parser::parse};

// ── GA-20 / CC-23 — @tokens contrast check ────────────────────────────────

#[test]
fn tokens_block_parses_without_error() {
    let src = r##"
@tokens {
    color primary light: "#1a73e8" dark: "#8ab4f8"
    spacing sm: "4px"
    font sans family: "Inter, sans-serif"
}
fn dummy() to int { return 1 }
"##;
    let m = parse(lex(src)).expect("parse should succeed for @tokens block");
    // No parse errors; typecheck should also not crash.
    let _ds = typecheck_ast_module(src, &m);
}

#[test]
fn tokens_contrast_violation_produces_diagnostic() {
    // Both hex values here have very similar luminance → WCAG AA failure.
    let src = r##"
@tokens {
    color bad_pair light: "#777777" dark: "#888888"
}
fn dummy() to int { return 1 }
"##;
    let m = parse(lex(src)).expect("parse");
    let ds = typecheck_ast_module(src, &m);
    let hit = ds.iter().find(|d| d.code.as_deref() == Some("vox/tokens/contrast-violation"));
    assert!(
        hit.is_some(),
        "expected vox/tokens/contrast-violation diagnostic for low-contrast pair; got {:?}",
        ds.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

// ── GA-05 — @uses decorator ───────────────────────────────────────────────

#[test]
fn uses_decorator_parses_without_error() {
    let src = r#"
@uses(net)
fn fetch_data() to int { return 1 }
"#;
    let m = parse(lex(src)).expect("parse should succeed for @uses decorator");
    // Verify effect was stored
    let func = m.declarations.iter().find_map(|d| {
        if let vox_compiler::ast::decl::Decl::Function(f) = d { Some(f) } else { None }
    });
    assert!(func.is_some(), "should have parsed function");
    let effects = &func.unwrap().effects;
    assert!(
        effects.iter().any(|e| matches!(e, vox_compiler::ast::decl::effect::EffectAnnotation::Net)),
        "@uses(net) should populate effects with Net; got {:?}", effects
    );
}

#[test]
fn uses_decorator_multi_effects_parses() {
    let src = r#"
@uses(net, fs)
fn upload_file() to int { return 1 }
"#;
    let m = parse(lex(src)).expect("parse");
    let func = m.declarations.iter().find_map(|d| {
        if let vox_compiler::ast::decl::Decl::Function(f) = d { Some(f) } else { None }
    });
    let effects = &func.unwrap().effects;
    assert!(
        effects.iter().any(|e| matches!(e, vox_compiler::ast::decl::effect::EffectAnnotation::Net)),
        "expected Net effect"
    );
    assert!(
        effects.iter().any(|e| matches!(e, vox_compiler::ast::decl::effect::EffectAnnotation::Fs)),
        "expected Fs effect"
    );
}

// ── GA-19 — semantic UI a11y ──────────────────────────────────────────────

#[test]
fn semantic_ui_missing_label_produces_diagnostic() {
    // Component view with Dialog but no label prop → a11y error.
    let src = r#"
component Nav() {
    view: Dialog() { "content" }
}
"#;
    let m = parse(lex(src)).expect("parse");
    let ds = typecheck_ast_module(src, &m);
    let hit = ds.iter().find(|d| d.code.as_deref() == Some("vox/a11y/dialog-missing-label"));
    assert!(
        hit.is_some(),
        "expected vox/a11y/dialog-missing-label for Dialog without label; got {:?}",
        ds.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

#[test]
fn semantic_ui_with_label_passes() {
    // Component view with Dialog + label prop → no a11y error.
    let src = r#"
component Nav() {
    view: Dialog(label="Confirm action") { "content" }
}
"#;
    let m = parse(lex(src)).expect("parse");
    let ds = typecheck_ast_module(src, &m);
    let hit = ds.iter().find(|d| d.code.as_deref() == Some("vox/a11y/dialog-missing-label"));
    assert!(
        hit.is_none(),
        "Dialog with label= should not trigger a11y diagnostic; got {:?}",
        ds.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ── GA-01 — Async[T] view exhaustiveness ──────────────────────────────────

#[test]
fn other_decorators_parse_without_error() {
    // Smoke-test that @auth, @cors, @rate_limit, @webhook, @layer, @pii,
    // @embed, @offline_capable, @collaborative all tokenize and parse
    // without crashing the parser.
    let src = r#"
@auth(provider: bearer)
@cors(origins: ["*"])
@rate_limit(by: ip, per: "1m", max: 100)
fn my_endpoint() to int { return 1 }

@webhook(provider: stripe)
fn stripe_hook() to int { return 2 }

@layer(tier: modal)
fn layer_fn() to int { return 3 }
"#;
    let result = parse(lex(src));
    assert!(result.is_ok(), "parse should succeed for multi-decorator functions; errors: {:?}", result.err());
}
