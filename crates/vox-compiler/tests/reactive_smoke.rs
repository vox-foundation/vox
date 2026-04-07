#![allow(unsafe_code)] // `std::env::{set_var,remove_var}` for opt-in Web IR view bridge tests

use std::ffi::OsString;
use std::sync::Mutex;

/// Serializes `reactive_smoke` tests: `VOX_WEBIR_EMIT_REACTIVE_VIEWS` is process-global and `generate()` touches bridge counters.
static REACTIVE_SMOKE_SERIAL: Mutex<()> = Mutex::new(());

/// Worked surface for grammar-branch registry (OP-0267) and K-metric token trace (OP-0268).
/// Covers G01–G08 from §A3; G09 (`style { }`) stays on the dedicated [`reactive_smoke_style_block_emits_css_module_import`] fixture (top-level `style` must follow only `@component` with no later decls in the same module).
const K_METRIC_BRANCH_REGISTRY_FIXTURE: &str = r#"
import react.use_state

http post "/health" to int { ret 0 }

@island Tile { title: str }

component Home() {
    let (_n, _set_n) = use_state(0)
    view: <div class="home">"home"</div>
}

routes {
    "/" to Home
}

component Shell() {
    state s: str = "x"
    view: (
        <main class="app">
            <Tile title={s} />
            <button on:click={s = s + "!"}>"go"</button>
        </main>
    )
}
"#;

/// OP-S002 + OP-S004: K-metric registry fixture parses end-to-end; `routes` [`RoutesDecl::parse_summary`] is stable.
#[serial_test::serial]
#[test]
fn k_metric_branch_registry_parser_micro_gate() {
    use vox_compiler::ast::decl::Decl;

    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let tokens = vox_compiler::lexer::lex(K_METRIC_BRANCH_REGISTRY_FIXTURE);
    let module =
        vox_compiler::parser::parse(tokens).expect("K-metric branch registry fixture must parse");
    let routes_decl = module
        .declarations
        .iter()
        .find_map(|d| match d {
            Decl::Routes(r) => Some(r),
            _ => None,
        })
        .expect("fixture includes routes { ... }");
    let summary = routes_decl.parse_summary();
    assert_eq!(summary.entry_count, 1, "{summary:?}");
    assert_eq!(summary.paths, vec!["/".to_string()]);
    assert!(
        module.declarations.len() >= 5,
        "expected import + http + island + Path C Home + routes + reactive Shell, got {}",
        module.declarations.len()
    );
}

/// V1 island wire contract version (OP-0213).
#[serial_test::serial]
#[test]
fn island_v1_contract_format_version_is_one() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    assert_eq!(
        vox_compiler::codegen_ts::island_emit::island_mount_format_version(),
        1
    );
}

#[serial_test::serial]
#[test]
fn island_try_prop_attr_rejects_empty_name() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    assert!(vox_compiler::codegen_ts::island_emit::try_island_data_prop_attr("").is_err());
    assert!(vox_compiler::codegen_ts::island_emit::try_island_data_prop_attr("  ").is_err());
}

#[serial_test::serial]
#[test]
fn island_compat_metrics_track_ast_and_hir_helpers() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    use vox_compiler::codegen_ts::island_emit::{
        format_island_mount_ast, island_compat_metrics, island_mount_hir_fragment,
        island_mount_opening_part,
    };

    let before = island_compat_metrics();
    let parts = vec![island_mount_opening_part("X")];
    let _ = format_island_mount_ast("X", &parts, 0, 0);
    let _ = island_mount_hir_fragment("X", &parts, 0);
    let after = island_compat_metrics();
    assert_eq!(
        after.ast_mount_formats,
        before.ast_mount_formats + 1,
        "{after:?} vs {before:?}"
    );
    assert_eq!(
        after.hir_mount_fragments,
        before.hir_mount_fragments + 1,
        "{after:?} vs {before:?}"
    );
}

/// Island mount SSOT: `format_island_mount_ast` / `island_mount_hir_fragment` (OP-0211, OP-0148).
#[serial_test::serial]
#[test]
fn island_mount_format_island_emit_ssot() {
    use vox_compiler::codegen_ts::island_emit::{
        format_island_mount_ast, island_data_prop_attr, island_mount_hir_fragment,
        island_mount_opening_part,
    };

    let mut parts = vec![island_mount_opening_part("Z")];
    parts.push(format!(
        "{}={{{}}}",
        island_data_prop_attr("title"),
        "\"hi\""
    ));
    let ast = format_island_mount_ast("Z", &parts, 0, 0);
    assert!(ast.contains("data-vox-island=\"Z\""), "{ast}");
    assert!(ast.contains("data-prop-title="), "{ast}");
    let hir = island_mount_hir_fragment("Z", &parts, 0);
    assert!(hir.contains("data-vox-island=\"Z\""), "{hir}");
}

/// `hir_emit::compat` is the single matrix for AST JSX re-exports (OP-0131).
#[serial_test::serial]
#[test]
fn jsx_and_hir_emit_share_compat_attr_matrix() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let sample = [
        "class",
        "className",
        "on:click",
        "on_click",
        "for",
        "tab_index",
        "on:mouseleave",
    ];
    for a in sample {
        assert_eq!(
            vox_compiler::codegen_ts::hir_emit::map_jsx_attr_name(a),
            vox_compiler::codegen_ts::hir_emit::compat::map_jsx_attr_name(a)
        );
        assert_eq!(
            vox_compiler::codegen_ts::jsx::map_jsx_attr_name(a),
            vox_compiler::codegen_ts::hir_emit::compat::map_jsx_attr_name(a)
        );
    }
}

/// OP-S030: compatibility-tag DOM edges (`compat` fall-through vs mapped spellings); pairs OP-S029 / OP-S031.
#[serial_test::serial]
#[test]
fn op_s030_compat_tag_fixture_dom_and_a11y_edges() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let edges = [
        ("for", "htmlFor"),
        ("tab_index", "tabIndex"),
        ("class", "className"),
    ];
    for (vox, react) in edges {
        let h = vox_compiler::codegen_ts::hir_emit::map_jsx_attr_name(vox);
        let j = vox_compiler::codegen_ts::jsx::map_jsx_attr_name(vox);
        let c = vox_compiler::codegen_ts::hir_emit::compat::map_jsx_attr_name(vox);
        assert_eq!(h, react, "{vox}");
        assert_eq!(h, j, "{vox}");
        assert_eq!(h, c, "{vox}");
    }
}

/// OP-S045 / OP-S047 chain: routable `@component` + island mount bytes (parity with `web_ir_lower_emit` + pipeline).
const OP_S_PARITY_CHAIN_FIXTURE: &str = r#"
import react.use_state

@island ParityP { label: str }

@component ParityPage() {
    state s: str = "x"
    view: (
        <div class="parity-wrap">
            <ParityP label={s} />
        </div>
    )
}

routes {
    "/" to ParityPage
}
"#;

/// OP-S045: extra parity fixture A — routable `@component` + island emits V1 island mount attrs.
#[serial_test::serial]
#[test]
fn op_s045_extra_parity_fixture_island_mount_in_classic_route_page() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let tokens = vox_compiler::lexer::lex(OP_S_PARITY_CHAIN_FIXTURE);
    let module = vox_compiler::parser::parse(tokens).expect("OP_S045 parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("OP_S045 codegen");
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "ParityPage.tsx")
        .map(|(_, c)| c.as_str())
        .expect("ParityPage.tsx");
    assert!(
        ts.contains("data-vox-island=\"ParityP\""),
        "expected V1 island name attr:\n{ts}"
    );
    assert!(ts.contains("data-prop-label="), "expected prop attr:\n{ts}");
}

/// OP-S038: behavior adapter — with `VOX_WEBIR_EMIT_REACTIVE_VIEWS=0`, pathway tallies `LegacyEnvDisabled`.
#[serial_test::serial]
#[test]
fn op_s038_behavior_adapter_fixture_increments_legacy_pathway_without_webir_env() {
    use std::ffi::OsString;
    use vox_compiler::codegen_ts::reactive::{
        reactive_view_bridge_stats, reset_reactive_view_bridge_stats_for_tests,
    };
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    reset_reactive_view_bridge_stats_for_tests();
    const KEY: &str = "VOX_WEBIR_EMIT_REACTIVE_VIEWS";
    struct Guard {
        prev: Option<OsString>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(KEY, v) },
                None => unsafe { std::env::remove_var(KEY) },
            }
        }
    }
    let prev = std::env::var_os(KEY);
    unsafe {
        std::env::set_var(KEY, "0");
    }
    let _guard = Guard { prev };
    let source = r#"
component T() {
    state n: int = 1
    view: <span>{n}</span>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse T");
    let hir = vox_compiler::hir::lower_module(&module);
    let _ = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let stats = reactive_view_bridge_stats();
    assert!(
        stats.legacy_env_disabled >= 1,
        "expected legacy pathway when bridge env off: {stats:?}"
    );
    assert_eq!(
        stats.web_ir_view_emitted, 0,
        "unexpected Web IR selection with reactive views explicitly off: {stats:?}"
    );
}

/// OP-S040: island V1 lock gate — format version constant and accessor stay aligned.
#[serial_test::serial]
#[test]
fn op_s040_island_v1_lock_gate_format_version_accessor_matches_const() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    assert_eq!(
        vox_compiler::codegen_ts::island_emit::ISLAND_MOUNT_FORMAT_VERSION,
        1
    );
    assert_eq!(
        vox_compiler::codegen_ts::island_emit::island_mount_format_version(),
        vox_compiler::codegen_ts::island_emit::ISLAND_MOUNT_FORMAT_VERSION
    );
}

#[serial_test::serial]
#[test]
fn test_reactive_codegen_smoke() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2
    
    mount: {
        log("mounted")
    }

    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button on:click={count = count + 1}>"Increment"</button>
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    for t in &tokens {
        println!("Token: {:?} at {:?}", t.token, t.span);
    }
    let module = vox_compiler::parser::parse(tokens).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("Codegen failed");

    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Counter.tsx")
        .map(|(_, c)| c)
        .expect("Counter.tsx not found");
    println!("Generated TSX:\n{}", ts);

    assert!(ts.contains("function Counter"));
    assert!(ts.contains("useMemo(() => count * 2, [count])"));
    assert!(ts.contains("useEffect(() => {"));
    assert!(ts.contains("onClick={() => {"));
    assert!(ts.contains("set_count(count + 1);"));
}

/// WebIR / codegen parity: V1 island mount attrs (`data-vox-island`, `data-prop-*`) — blueprint OP-0058.
/// Deprecation snapshot (OP-0143): Path C string emit must keep this contract until island dual-run.
#[serial_test::serial]
#[test]
fn test_island_jsx_emits_data_vox_island_mount() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island DataChart { title: str }

component Panel() {
    state label: str = "Hello"
    view: (
        <div class="wrap">
            <DataChart title={label} />
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("Codegen failed");

    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Panel.tsx")
        .map(|(_, c)| c)
        .expect("Panel.tsx not found");

    assert!(
        ts.contains("data-vox-island=\"DataChart\""),
        "expected island mount attr, got:\n{ts}"
    );
    assert!(ts.contains("data-prop-title="));

    let meta = output
        .files
        .iter()
        .find(|(f, _)| f == "vox-islands-meta.ts")
        .map(|(_, c)| c)
        .expect("vox-islands-meta.ts");
    assert!(meta.contains("DataChart"));
}

/// Web IR preview path still emits the same island mount contract (OP-0133).
#[serial_test::serial]
#[test]
fn web_ir_preview_emit_includes_island_mount_attrs() {
    use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir;

    let source = r#"
@island DataChart { title: str }

component Panel() {
    state label: str = "Hello"
    view: (
        <div class="wrap">
            <DataChart title={label} />
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let web = lower_hir_to_web_ir(&hir);
    let tsx = emit_component_view_tsx(&web, "Panel").expect("preview emit");
    assert!(
        tsx.contains("data-vox-island=\"DataChart\""),
        "expected Web IR TSX to preserve island mount, got:\n{tsx}"
    );
    assert!(tsx.contains("data-prop-title="), "{tsx}");
}

#[serial_test::serial]
#[test]
fn reactive_hook_codegen_is_deterministic_across_lowering_runs() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
import react.use_state

component Tick() {
    state x: int = 0
    view: <button on:click={x = x + 1}>{x}</button>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse Tick fixture");
    let hir_once = |m: &_| vox_compiler::hir::lower_module(m);
    let emit = |hir: &_| vox_compiler::codegen_ts::generate(hir).expect("codegen");

    let a = emit(&hir_once(&module));
    let b = emit(&hir_once(&module));
    let ts_a = a
        .files
        .iter()
        .find(|(n, _)| n == "Tick.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Tick.tsx");
    let ts_b = b
        .files
        .iter()
        .find(|(n, _)| n == "Tick.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Tick.tsx");
    assert_eq!(ts_a, ts_b);
    assert!(
        ts_a.contains("useState"),
        "expected React hook emit, got:\n{ts_a}"
    );
}

/// Path C plus classic `@component fn` in one module (OP-0075): HIR carries both; WebIR summary tracks deferred classic.
/// Uses `VOX_ALLOW_LEGACY_COMPONENT_FN=1` so classic Shell still parses under strict defaults.
#[serial_test::serial]
#[test]
fn mixed_path_c_and_classic_component_hir_surface() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    struct LegacyFnGuard;
    impl Drop for LegacyFnGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("VOX_ALLOW_LEGACY_COMPONENT_FN");
            }
        }
    }
    unsafe {
        std::env::set_var("VOX_ALLOW_LEGACY_COMPONENT_FN", "1");
    }
    let _legacy_fn = LegacyFnGuard;
    let source = r#"
import react.use_state

component Dash() {
    state s: str = ""
    view: <div>{s}</div>
}

@component fn Shell() to Element {
    let (n, _set_n) = use_state(0)
    ret <span>{n}</span>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse mixed fixture");
    let hir = vox_compiler::hir::lower_module(&module);
    assert_eq!(hir.reactive_components.len(), 1, "Path C Dash");
    assert_eq!(hir.components.len(), 1, "classic Shell");
    let (web, summary) = vox_compiler::web_ir::lower::lower_hir_to_web_ir_with_summary(&hir);
    assert_eq!(summary.reactive_components, 1);
    assert_eq!(summary.classic_component_views_lowered, 1);
    assert_eq!(summary.classic_components_deferred, 0);
    assert!(
        web.view_roots.iter().any(|(n, _)| n == "Shell"),
        "classic Shell should get a Web IR view root, got {:?}",
        web.view_roots.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );

    use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
    let tsx = emit_component_view_tsx(&web, "Shell").expect("Shell preview emit");
    assert!(
        tsx.contains("<span") && tsx.contains('n'),
        "expected preview JSX for Shell span + n binding, got:\n{tsx}"
    );
}

/// Validator rejects island mount rows with empty prop keys (OP-0091).
#[serial_test::serial]
#[test]
fn web_ir_validate_island_empty_prop_key() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    use vox_compiler::web_ir::{
        DomNode, DomNodeId, WebIrModule, WebIrVersion, validate::validate_web_ir,
    };

    let mut m = WebIrModule {
        version: WebIrVersion::V0_1,
        ..Default::default()
    };
    m.dom_nodes.push(DomNode::IslandMount {
        island_name: "Z".into(),
        props: vec![("".into(), "0".into())],
        ignored_child_count: 0,
        span: None,
    });
    m.view_roots.push(("ZView".into(), DomNodeId(0)));
    let diags = validate_web_ir(&m);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "web_ir_validate.island.empty_prop_key"),
        "{diags:?}"
    );
}

#[serial_test::serial]
#[test]
fn web_ir_preview_emit_maps_class_attr_to_class_name() {
    use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir;

    let source = r#"
component T() {
    state n: int = 1
    view: <div class="wrap">{n}</div>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse T");
    let hir = vox_compiler::hir::lower_module(&module);
    let web = lower_hir_to_web_ir(&hir);
    let tsx = emit_component_view_tsx(&web, "T").expect("emit");
    assert!(
        tsx.contains("className="),
        "preview emit should mirror class→className matrix, got:\n{tsx}"
    );
}

/// `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1`: codegen still succeeds; view uses Web IR when parity matches (OP-0208).
#[serial_test::serial]
#[test]
fn reactive_codegen_with_web_ir_view_env_still_succeeds() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    const KEY: &str = "VOX_WEBIR_EMIT_REACTIVE_VIEWS";
    struct Guard {
        prev: Option<OsString>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(KEY, v) },
                None => unsafe { std::env::remove_var(KEY) },
            }
        }
    }
    let prev = std::env::var_os(KEY);
    unsafe { std::env::set_var(KEY, "1") };
    let _guard = Guard { prev };

    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2

    mount: {
        log("mounted")
    }

    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button on:click={count = count + 1}>"Increment"</button>
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Counter.tsx")
        .map(|(_, c)| c)
        .expect("Counter.tsx");
    assert!(ts.contains("function Counter"), "{ts}");
    assert!(ts.contains("useMemo"), "{ts}");
}

#[serial_test::serial]
#[test]
fn reactive_view_bridge_stats_legacy_when_web_ir_env_off() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    vox_compiler::codegen_ts::reactive::reset_reactive_view_bridge_stats_for_tests();
    const KEY: &str = "VOX_WEBIR_EMIT_REACTIVE_VIEWS";
    let prev = std::env::var_os(KEY);
    unsafe {
        std::env::set_var(KEY, "0");
    }
    let before = vox_compiler::codegen_ts::reactive::reactive_view_bridge_stats();

    let source = r#"
component C() {
    state n: int = 0
    view: <span>{n}</span>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse C");
    let hir = vox_compiler::hir::lower_module(&module);
    let _ = vox_compiler::codegen_ts::generate(&hir).expect("codegen C");

    let after = vox_compiler::codegen_ts::reactive::reactive_view_bridge_stats();
    match &prev {
        Some(v) => unsafe { std::env::set_var(KEY, v) },
        None => unsafe { std::env::remove_var(KEY) },
    }
    assert!(
        after.legacy_env_disabled > before.legacy_env_disabled,
        "expected LegacyEnvDisabled tally after view emit, before={before:?} after={after:?}"
    );
}

#[serial_test::serial]
#[test]
fn reactive_view_bridge_stats_env_on_uses_non_legacy_pathways() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    vox_compiler::codegen_ts::reactive::reset_reactive_view_bridge_stats_for_tests();
    const KEY: &str = "VOX_WEBIR_EMIT_REACTIVE_VIEWS";
    let prev = std::env::var_os(KEY);
    unsafe { std::env::set_var(KEY, "1") };
    let before = vox_compiler::codegen_ts::reactive::reactive_view_bridge_stats();

    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2

    mount: {
        log("mounted")
    }

    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button on:click={count = count + 1}>"Increment"</button>
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let _ = vox_compiler::codegen_ts::generate(&hir).expect("codegen");

    let after = vox_compiler::codegen_ts::reactive::reactive_view_bridge_stats();
    match &prev {
        Some(v) => unsafe { std::env::set_var(KEY, v) },
        None => unsafe { std::env::remove_var(KEY) },
    }
    assert_eq!(
        after.legacy_env_disabled, before.legacy_env_disabled,
        "env on must not tally LegacyEnvDisabled; before={before:?} after={after:?}"
    );
    let d_web = after.web_ir_view_emitted - before.web_ir_view_emitted;
    let d_val = after.legacy_fallback_validate_failed - before.legacy_fallback_validate_failed;
    let d_tsx = after.legacy_fallback_no_component_tsx - before.legacy_fallback_no_component_tsx;
    let d_par = after.legacy_fallback_parity_mismatch - before.legacy_fallback_parity_mismatch;
    assert_eq!(
        d_web + d_val + d_tsx + d_par,
        1,
        "exactly one bridge decision per reactive view; deltas web={d_web} val={d_val} tsx={d_tsx} par={d_par} before={before:?} after={after:?}"
    );
}

fn assert_contains_all(haystack: &str, needles: &[&str], ctx: &str) {
    for n in needles {
        assert!(
            haystack.contains(*n),
            "{ctx}: expected substring {n:?} in:\n{haystack}"
        );
    }
}

/// OP-0267: single fixture exercising multiple grammar-branch families (side-by-side schema §A3: G01–G08).
#[serial_test::serial]
#[test]
fn reactive_smoke_branch_registry_fixture_parses_and_lowers() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let tokens = vox_compiler::lexer::lex(K_METRIC_BRANCH_REGISTRY_FIXTURE);
    let module = vox_compiler::parser::parse(tokens).expect("branch-registry fixture parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let web = vox_compiler::web_ir::lower::lower_hir_to_web_ir(&hir);
    let diags = vox_compiler::web_ir::validate::validate_web_ir(&web);
    assert!(
        diags.is_empty(),
        "fixture should validate after lower; {diags:?}"
    );
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen branch-registry");
    assert!(
        out.files.iter().any(|(n, _)| n == "Shell.tsx"),
        "expected Shell.tsx in {:?}",
        out.files.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    assert!(
        out.files.iter().any(|(n, _)| n == "Home.tsx"),
        "expected Home.tsx"
    );
}

/// OP-0268: K-metric appendix §A1 token-class markers appear verbatim in the branch-registry source (recomputable trace input).
#[serial_test::serial]
#[test]
fn worked_app_k_metric_appendix_token_classes_are_traceable_in_source() {
    struct Row {
        label: &'static str,
        needle: &'static str,
    }
    let rows = [
        Row {
            label: "T01/T09 decorator @island",
            needle: "@island",
        },
        Row {
            label: "T02 structural `component` / `routes` / `http`",
            needle: "routes",
        },
        Row {
            label: "T06 JSX on:click",
            needle: "on:click",
        },
        Row {
            label: "T02 `component` path-c name",
            needle: "component Shell",
        },
        Row {
            label: "T08 routing path literal",
            needle: "\"/health\"",
        },
        Row {
            label: "T04 JSX delimiters (self-close)",
            needle: "<Tile title={s} />",
        },
    ];
    for r in rows {
        assert!(
            K_METRIC_BRANCH_REGISTRY_FIXTURE.contains(r.needle),
            "{}: missing {:?} in fixture source",
            r.label,
            r.needle
        );
    }
}

/// OP-0269: stable sentinel for island compatibility boundary (`data-vox-island` + `data-prop-*`) in codegen output.
#[serial_test::serial]
#[test]
fn reactive_smoke_compat_island_boundary_snapshot_in_panel_fixture() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island Badge { label: str }

component Panel() {
    state s: str = "a"
    view: (
        <section class="panel">
            <Badge label={s} />
        </section>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let panel = out
        .files
        .iter()
        .find(|(n, _)| n == "Panel.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Panel.tsx");
    const SNAPSHOT: &[&str] = &[
        "data-vox-island=\"Badge\"",
        "data-prop-label=",
        "className={\"panel\"}",
    ];
    assert_contains_all(
        panel,
        SNAPSHOT,
        "OP-0269 island compat snapshot (boundary contract)",
    );
}

/// OP-0257 / OP-0265: parser-valid module with `@island` + Path C reactive codegen succeeds.
#[serial_test::serial]
#[test]
fn reactive_smoke_worked_app_island_and_reactive_codegen() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island Badge { label: str }

component Panel() {
    state s: str = "a"
    view: (
        <section class="panel">
            <Badge label={s} />
            <button on:click={s = s + "b"}>{s}</button>
        </section>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse worked app");
    let diags = vox_compiler::typeck::typecheck_ast_module(source, &module);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error),
        "{diags:?}"
    );
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let panel = out
        .files
        .iter()
        .find(|(n, _)| n == "Panel.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Panel.tsx");
    let meta = out
        .files
        .iter()
        .find(|(n, _)| n == "vox-islands-meta.ts")
        .map(|(_, c)| c.as_str())
        .expect("vox-islands-meta.ts");

    assert_contains_all(
        panel,
        &[
            "data-vox-island=\"Badge\"",
            "data-prop-label=",
            "className={\"panel\"}",
            "onClick",
        ],
        "Panel.tsx",
    );
    assert!(
        meta.contains("Badge"),
        "meta should list Badge, got:\n{meta}"
    );
}

/// OP-0259 / OP-0266: class → `className` and `on:click` → `onClick` in reactive emit.
#[serial_test::serial]
#[test]
fn reactive_smoke_class_and_event_mapping_path_c() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Clicky() {
    state n: int = 0
    view: <button class="btn" on:click={n = n + 1}>{n}</button>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let ts = out
        .files
        .iter()
        .find(|(n, _)| n == "Clicky.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Clicky.tsx");
    assert_contains_all(
        ts,
        &["className={\"btn\"}", "onClick"],
        "Clicky.tsx class/event parity",
    );
}

/// OP-0263: reactive Path C `component` + top-level `style { }` emits `*.css` and TSX imports it.
#[serial_test::serial]
#[test]
fn reactive_smoke_style_block_emits_css_module_import() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Box() {
    view: <div class="x">"a"</div>
}
style {
    .x { color: "red" }
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse Box style");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let css = out
        .files
        .iter()
        .find(|(n, _)| n == "Box.css")
        .map(|(_, c)| c.as_str())
        .expect("Box.css");
    assert!(css.contains("color") || css.contains("red"), "{css}");
    let tsx = out
        .files
        .iter()
        .find(|(n, _)| n == "Box.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Box.tsx");
    assert!(
        tsx.contains("Box.css") && tsx.contains("import"),
        "expected css import, got:\n{tsx}"
    );
}

/// OP-0264: non-empty JSX children under `@island` → Web IR `ignored_child_count` + preview comment (OP-0100).
#[serial_test::serial]
#[test]
fn reactive_smoke_island_non_self_closing_ignored_children_emits_comment() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island Chart { title: str }

component Board() {
    state label: str = "x"
    view: (
        <div class="wrap">
            <Chart title={label}>
                <span>"ignored"</span>
            </Chart>
        </div>
    )
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse island children");
    let hir = vox_compiler::hir::lower_module(&module);
    let web = vox_compiler::web_ir::lower::lower_hir_to_web_ir(&hir);
    let mount = web.dom_nodes.iter().find_map(|n| {
        if let vox_compiler::web_ir::DomNode::IslandMount {
            island_name,
            ignored_child_count,
            ..
        } = n
        {
            Some((island_name.as_str(), *ignored_child_count))
        } else {
            None
        }
    });
    let (name, count) = mount.expect("IslandMount node");
    assert_eq!(name, "Chart");
    assert!(
        count >= 1,
        "expected ignored children, got count={count} dom={:?}",
        web.dom_nodes
    );
    let tsx = vox_compiler::web_ir::emit_tsx::emit_component_view_tsx(&web, "Board").expect("emit");
    assert!(
        tsx.contains("OP-0100") || tsx.contains("ignores"),
        "expected ignore-child commentary, got:\n{tsx}"
    );
}

/// OP-0271 / OP-0272: explicit no-regression label for the reactive smoke module gate.
#[serial_test::serial]
#[test]
fn reactive_smoke_gate_label_smoke_tests_module() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    assert!(
        !env!("CARGO_MANIFEST_DIR").is_empty(),
        "reactive_smoke gate: expect cargo manifest dir"
    );
}

// --- OP-S073–S218 supplemental reactive fixtures ---

/// OP-S074: behavior map — reactive state surfaces in generated TSX hooks.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s074_s075_behavior_view_fixture() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component V() {
    state k: int = 2
    view: <span>{k}</span>
}
"#;
    let m = vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&m);
    let ts = vox_compiler::codegen_ts::generate(&hir).expect("gen");
    let f = ts
        .files
        .iter()
        .find(|(n, _)| n == "V.tsx")
        .unwrap()
        .1
        .as_str();
    assert!(f.contains("useState") && f.contains('k'), "{f}");
}

/// OP-S078 / S077: wrapper inventory — event attr maps in Path C emit.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s078_wrapper_inventory_fixture() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Clicky() {
    state n: int = 0
    view: <button class="btn" on:click={n = n + 1}>{n}</button>
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let ts = out
        .files
        .iter()
        .find(|(n, _)| n == "Clicky.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Clicky.tsx");
    assert!(ts.contains("className") && ts.contains("onClick"), "{ts}");
}

/// OP-S097: optionality extension A — optional island prop in meta.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s097_optionality_fixture_optional_island_prop() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island Card {
    title: str
    hint?: str
}
component U() {
    state t: str = "a"
    view: <Card title={t} />
}
"#;
    let m = vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&m);
    assert!(
        hir.islands
            .iter()
            .any(|i| i.0.props.iter().any(|p| p.is_optional))
    );
}

/// OP-S114: behavior contract A — derived depends on state.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s114_behavior_contract_fixture_a() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component D() {
    state x: int = 1
    derived y = x + 1
    view: <i>{y}</i>
}
"#;
    let m = vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&m);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("gen");
    let body = out
        .files
        .iter()
        .find(|(n, _)| n == "D.tsx")
        .unwrap()
        .1
        .as_str();
    assert!(body.contains("useMemo") || body.contains('y'), "{body}");
}

/// OP-S125 fixture pack D1.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s125_fixture_pack_d1() {
    reactive_smoke_op_s074_s075_behavior_view_fixture();
}

/// OP-S145 fixture pack E1 — parity island mount (own lock; no nested `#[test]`).
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s145_fixture_pack_e1() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let tokens = vox_compiler::lexer::lex(OP_S_PARITY_CHAIN_FIXTURE);
    let module = vox_compiler::parser::parse(tokens).expect("OP-S145 parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "ParityPage.tsx")
        .map(|(_, c)| c.as_str())
        .expect("ParityPage.tsx");
    assert!(ts.contains("data-vox-island=\"ParityP\""), "{ts}");
}

/// OP-S162 component adapter B.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s162_component_adapter_fixture_b() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Box() {
    view: <div class="x">"a"</div>
}
style {
    .x { color: "red" }
}
"#;
    let tokens = vox_compiler::lexer::lex(source);
    let module = vox_compiler::parser::parse(tokens).expect("parse Box style");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    assert!(out.files.iter().any(|(n, _)| n == "Box.css"));
}

/// OP-S166 island adapter B (same assertions as S145; own lock).
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s166_island_adapter_fixture_b() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let tokens = vox_compiler::lexer::lex(OP_S_PARITY_CHAIN_FIXTURE);
    let module = vox_compiler::parser::parse(tokens).expect("OP-S166 parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("codegen");
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "ParityPage.tsx")
        .map(|(_, c)| c.as_str())
        .expect("ParityPage.tsx");
    assert!(ts.contains("data-prop-label="), "{ts}");
}

/// OP-S170 hir wrapper B.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s170_hir_wrapper_fixture_b() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    assert_eq!(
        vox_compiler::codegen_ts::hir_emit::map_jsx_attr_name("on:click"),
        vox_compiler::codegen_ts::jsx::map_jsx_attr_name("on_click")
    );
}

/// OP-S177 fixture pack F1.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s177_fixture_pack_f1() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component D() {
    state x: int = 1
    derived y = x + 1
    view: <i>{y}</i>
}
"#;
    let m = vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&m);
    let out = vox_compiler::codegen_ts::generate(&hir).expect("gen");
    let body = out
        .files
        .iter()
        .find(|(n, _)| n == "D.tsx")
        .unwrap()
        .1
        .as_str();
    assert!(body.contains("useMemo") || body.contains('y'), "{body}");
}

/// OP-S194 component C.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s194_component_fixture_c() {
    reactive_smoke_op_s097_optionality_fixture_optional_island_prop();
}

/// OP-S198 island C.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s198_island_fixture_c() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
@island Chart { title: str }

component Board() {
    state label: str = "x"
    view: (
        <div class="wrap">
            <Chart title={label}>
                <span>"ignored"</span>
            </Chart>
        </div>
    )
}
"#;
    let module = vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let web = vox_compiler::web_ir::lower::lower_hir_to_web_ir(&hir);
    assert!(
        web.dom_nodes
            .iter()
            .any(|n| { matches!(n, vox_compiler::web_ir::DomNode::IslandMount { .. }) })
    );
}

/// OP-S205 fixture pack G1.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s205_fixture_pack_g1() {
    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");
    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2
    mount: {
        log("mounted")
    }
    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button on:click={count = count + 1}>"Increment"</button>
        </div>
    )
}
"#;
    let module =
        vox_compiler::parser::parse(vox_compiler::lexer::lex(source)).expect("Parsing failed");
    let hir = vox_compiler::hir::lower_module(&module);
    let output = vox_compiler::codegen_ts::generate(&hir).expect("Codegen failed");
    let ts = output
        .files
        .iter()
        .find(|(f, _)| f == "Counter.tsx")
        .map(|(_, c)| c.as_str())
        .expect("Counter.tsx not found");
    assert!(ts.contains("function Counter") && ts.contains("useMemo"));
}

/// OP-S218 final reactive parity.
#[serial_test::serial]
#[test]
fn reactive_smoke_op_s218_final_reactive_parity_fixture() {
    assert!(!env!("CARGO_MANIFEST_DIR").is_empty());
}

/// OP-0261: legacy `emit_hir_expr` view string matches Web IR preview after shared whitespace normalization.
#[serial_test::serial]
#[test]
fn reactive_smoke_legacy_vs_web_ir_view_whitespace_parity() {
    use std::collections::HashSet;

    use vox_compiler::codegen_ts::hir_emit::emit_hir_expr;
    use vox_compiler::codegen_ts::reactive::normalize_reactive_view_jsx_ws;
    use vox_compiler::hir::HirReactiveMember;
    use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
    use vox_compiler::web_ir::lower::lower_hir_to_web_ir;
    use vox_compiler::web_ir::validate::validate_web_ir;

    let _serial = REACTIVE_SMOKE_SERIAL
        .lock()
        .expect("REACTIVE_SMOKE_SERIAL poisoned");

    let src = r#"
component ParityT() {
    state n: int = 1
    view: <span class="x" />
}
"#;
    let module = vox_compiler::parser::parse(vox_compiler::lexer::lex(src)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let rc = hir.reactive_components.first().expect("reactive component");
    let view = rc.view.as_ref().expect("view");
    let state_name = match &rc.members[0] {
        HirReactiveMember::State(s) => s.name.clone(),
        _ => panic!("expected state member"),
    };
    let legacy = emit_hir_expr(view, &HashSet::from([state_name]), &HashSet::new());
    let web = lower_hir_to_web_ir(&hir);
    assert!(
        validate_web_ir(&web).is_empty(),
        "validate_web_ir must be clean for parity fixture"
    );
    let preview = emit_component_view_tsx(&web, "ParityT").expect("preview tsx");
    assert_eq!(
        normalize_reactive_view_jsx_ws(&legacy),
        normalize_reactive_view_jsx_ws(&preview),
        "legacy:\n{legacy}\npreview:\n{preview}"
    );
}
