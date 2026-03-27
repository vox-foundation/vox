// --- New golden example integration tests (T031–T043) ---

// generics_option.vox
const GENERICS_OPTION_SRC: &str = r#"type Option =
    | Some(value: str)
    | None

type Result =
    | Ok(value: str)
    | Err(message: str)

fn unwrap_or(opt: Option, default: str) to str {
    match opt {
        Some(value) -> value
        None -> default
    }
}

@test fn test_unwrap_some() to Unit {
    let opt = Some("hello")
    let result = unwrap_or(opt, "default")
    assert(result is "hello")
}

@test fn test_ok_result() to Unit {
    let r = Ok("success")
    match r {
        Ok(value) -> assert(value is "success")
        Err(msg) -> assert(false)
    }
}
"#;

#[test]
fn pipeline_generics_option_parse() {
    let tokens = lex(GENERICS_OPTION_SRC);
    let module = parse(tokens).expect("generics_option should parse");
    // type Option + type Result + fn unwrap_or + @test x2
    assert_eq!(module.declarations.len(), 5, "2 types + 1 fn + 2 tests");
}

#[test]
fn pipeline_generics_option_codegen() {
    let tokens = lex(GENERICS_OPTION_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let types = output.files.iter().find(|(n, _)| n == "types.ts").unwrap();
    assert!(
        types.1.contains("export type Option"),
        "Should export Option"
    );
    assert!(types.1.contains("_tag: \"Some\""), "Should have Some tag");
    assert!(types.1.contains("_tag: \"None\""), "Should have None tag");
    assert!(
        types.1.contains("export type Result"),
        "Should export Result"
    );
    assert!(types.1.contains("_tag: \"Ok\""), "Should have Ok tag");
    assert!(types.1.contains("_tag: \"Err\""), "Should have Err tag");
}

// hooks_demo.vox
const HOOKS_DEMO_SRC: &str = r#"import react.use_state
import react.use_effect
import react.use_memo
import react.use_ref
import react.use_callback

@component fn HooksDemo() to Element {
    let (count, set_count) = use_state(0)
    let doubled = use_memo(fn(_x) count * 2)
    let input_ref = use_ref(0)
    let increment = use_callback(fn(_e) set_count(count + 1))
    use_effect(fn(_x) count)
    <div class="hooks_demo">
        <p>"Count: " {count}</p>
        <p>"Doubled: " {doubled}</p>
        <button on_click={increment}>"+"</button>
    </div>
}

routes {
    "/" to HooksDemo
}
"#;

#[test]
fn pipeline_hooks_demo_parse() {
    let tokens = lex(HOOKS_DEMO_SRC);
    let module = parse(tokens).expect("hooks_demo should parse");
    // 5 imports + 1 component + 1 routes
    assert_eq!(module.declarations.len(), 7);
}

#[test]
fn pipeline_hooks_demo_codegen() {
    let tokens = lex(HOOKS_DEMO_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let tsx = output
        .files
        .iter()
        .find(|(n, _)| n == "HooksDemo.tsx")
        .unwrap();
    assert!(tsx.1.contains("useState"), "Should use useState");
    assert!(tsx.1.contains("useEffect"), "Should use useEffect");
    assert!(tsx.1.contains("useMemo"), "Should use useMemo");
    assert!(tsx.1.contains("useRef"), "Should use useRef");
    assert!(tsx.1.contains("useCallback"), "Should use useCallback");
    assert!(tsx.1.contains("import React,"), "Should have React import");
}

#[test]
fn pipeline_web_ir_preview_emit_hooks_reactive_fixture() {
    use vox_compiler::codegen_ts::reactive::{
        reactive_view_bridge_stats, reset_reactive_view_bridge_stats_for_tests,
    };
    use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir;
    use vox_compiler::web_ir::validate::validate_web_ir;

    let tokens = lex(HOOKS_DEMO_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let web = lower_hir_to_web_ir(&hir);
    assert!(
        validate_web_ir(&web).is_empty(),
        "hooks demo should validate clean as Web IR"
    );
    let preview = emit_component_view_tsx(&web, "HooksDemo").expect("preview HooksDemo view");
    assert!(
        preview.contains("hooks_demo") || preview.contains("\"hooks_demo\""),
        "expected hooks demo class/root in preview:\n{preview}"
    );

    // Mixed surface: classic `Shell` (hooks) + Path C `Dash` — `VOX_WEBIR_EMIT_REACTIVE_VIEWS` runs the bridge
    // for `Dash` (Web IR preview when normalized JSX matches legacy, else parity fallback).
    let mix_tokens = lex(MIXED_SURFACE_SRC);
    let mix_mod = parse(mix_tokens).expect("parse MIXED_SURFACE");
    let mix_hir = vox_compiler::hir::lower_module(&mix_mod);
    // Do not nest `with_web_ir_validate_cleared` here: it also takes `ENV_MUTEX` and would deadlock.
    with_reactive_emit_views_enabled(|| {
        reset_reactive_view_bridge_stats_for_tests();
        let out = generate(&mix_hir).expect("MIXED_SURFACE codegen");
        let dash = out
            .files
            .iter()
            .find(|(n, _)| n == "Dash.tsx")
            .map(|(_, c)| c.as_str())
            .expect("Dash.tsx");
        assert!(
            dash.contains("Chart") || dash.contains("chart"),
            "Dash should reference Chart:\n{dash}"
        );
        let shell = out
            .files
            .iter()
            .find(|(n, _)| n == "Shell.tsx")
            .map(|(_, c)| c.as_str())
            .expect("Shell.tsx");
        assert!(
            shell.contains("useState"),
            "classic Shell retains hooks:\n{shell}"
        );
        let stats = reactive_view_bridge_stats();
        assert!(
            stats.web_ir_view_emitted >= 1,
            "expected Web IR preview for reactive Dash (island prop order matches legacy); stats={stats:?}"
        );
    });
}

// island_demo.vox
const ISLAND_DEMO_SRC: &str = r#"@island InteractiveChart {
    data: str
    title: str
    width?: int
}

@island SearchWidget {
    placeholder: str
    endpoint: str
}

@component fn IslandHost() to Element {
    <div class="island_host">
        <h1>"Islands Demo"</h1>
    </div>
}

routes {
    "/" to IslandHost
}
"#;

#[test]
fn pipeline_island_parse() {
    let tokens = lex(ISLAND_DEMO_SRC);
    let module = parse(tokens).expect("island_demo should parse");
    // 2 islands + 1 component + 1 routes
    assert_eq!(module.declarations.len(), 4);
}

#[test]
fn pipeline_island_codegen() {
    let tokens = lex(ISLAND_DEMO_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"vox-islands-meta.ts"),
        "Should produce vox-islands-meta.ts, got: {:?}",
        filenames
    );
    let meta = output
        .files
        .iter()
        .find(|(n, _)| n == "vox-islands-meta.ts")
        .unwrap();
    assert!(
        meta.1.contains("InteractiveChart"),
        "Island meta should list InteractiveChart"
    );
    assert!(
        meta.1.contains("SearchWidget"),
        "Island meta should list SearchWidget"
    );
}

// v0_component.vox
const V0_COMPONENT_SRC: &str = r#"@v0 "A modern analytics dashboard with KPI cards" fn Analytics() to Element

@v0 from "design/landing-mockup.png" fn LandingPage() to Element

routes {
    "/" to Analytics
    "/landing" to LandingPage
}
"#;

#[test]
fn pipeline_v0_parse() {
    let tokens = lex(V0_COMPONENT_SRC);
    let module = parse(tokens).expect("v0_component should parse");
    // 2 v0 + 1 routes
    assert_eq!(module.declarations.len(), 3);
}

#[test]
fn pipeline_v0_codegen() {
    let tokens = lex(V0_COMPONENT_SRC);
    let module = parse(tokens).unwrap();
    let hir = vox_compiler::hir::lower_module(&module);
    let output = generate(&hir).unwrap();
    let filenames: Vec<&str> = output.files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        filenames.contains(&"Analytics.tsx"),
        "Should produce Analytics.tsx"
    );
    assert!(
        filenames.contains(&"LandingPage.tsx"),
        "Should produce LandingPage.tsx"
    );
    let analytics = output
        .files
        .iter()
        .find(|(n, _)| n == "Analytics.tsx")
        .unwrap();
    assert!(
        analytics.1.contains("@v0 generated"),
        "Analytics should be v0 placeholder"
    );
    let landing = output
        .files
        .iter()
        .find(|(n, _)| n == "LandingPage.tsx")
        .unwrap();
    assert!(
        landing.1.contains("landing-mockup.png"),
        "LandingPage should reference the image"
    );
}
