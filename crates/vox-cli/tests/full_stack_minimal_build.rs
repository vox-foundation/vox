//! Golden `vox build` for `examples/full_stack_minimal.vox` (no Node).
#![allow(missing_docs)]
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use vox_cli::commands::build;
use vox_cli::frontend;
use vox_cli::templates;

struct EnvGuard {
    key: &'static str,
    prev: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var_os(key);
        unsafe {
            unsafe { std::env::set_var(key, value) };
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => unsafe {
                unsafe { std::env::set_var(self.key, v) };
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

#[tokio::test]
async fn full_stack_minimal_build_writes_app_tsx_and_api() {
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo.pop();
    repo.pop();
    let vox_file = repo.join("crates/vox-integration-tests/tests/fixtures/full_stack_minimal.vox");

    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    build::run(&vox_file, &out).await.expect("build");

    assert!(out.join("App.tsx").is_file());
    assert!(out.join("Home.tsx").is_file());
    assert!(out.join("api.ts").is_file());
}

/// OP-0227 / OP-0229: island-mount template embeds V1 prop-attr decode and unknown-island warn path (no Node).
#[test]
fn full_stack_golden_island_mount_template_hydration_contract() {
    let mount = templates::islands_island_mount_tsx();
    assert!(
        mount.contains(r#"startsWith("data-prop-")"#),
        "expected data-prop- prefix decode:\n{mount}"
    );
    assert!(
        mount.contains("replace(/-([a-z])/g"),
        "expected kebab→camel decode:\n{mount}"
    );
    assert!(
        mount.contains("[vox-islands] unknown island:") && mount.contains("console.warn"),
        "expected unknown-island warning path:\n{mount}"
    );
    assert!(
        templates::islands_island_mount_tsx().contains(templates::islands_props_from_element_ts()),
        "mount bundle should embed islands_props_from_element_ts verbatim"
    );
}

/// OP-0231 / OP-0239: embedded V1 trace markers + metrics hook string stable for offline gates.
#[test]
fn full_stack_golden_island_template_v1_trace_markers() {
    let m = templates::islands_island_mount_tsx();
    assert!(
        m.contains("vox:island-mount contract=V1") && m.contains("vox:island-metrics contract=V1"),
        "expected V1 trace markers:\n{m}"
    );
    assert!(
        m.contains("__VOX_ISLANDS_V1_METRICS") && m.contains("unknownIslandWarnCount"),
        "expected runtime metrics export:\n{m}"
    );
}

/// OP-S042: decode-helper fixture — [`templates::islands_props_from_element_ts`] bytes are embedded in the mount bundle.
#[test]
fn op_s042_decode_helper_fixture_props_from_element_embedded_in_mount_tsx() {
    let props = templates::islands_props_from_element_ts();
    let full = templates::islands_island_mount_tsx();
    assert!(
        full.contains(props),
        "island-mount.tsx must embed propsFromElement SSOT verbatim"
    );
    assert!(
        props.contains("function propsFromElement")
            && props.contains(r#"startsWith("data-prop-")"#),
        "decode helper shape:\n{props}"
    );
}

/// OP-S044: runtime injection helper gate — pure roundtrip + single `island-mount.js` ref after inject ([`frontend::apply_island_mount_script_to_index_html`], OP-S043).
#[test]
fn op_s044_runtime_injection_helper_gate_idempotent_and_single_mount_ref() {
    let shell = "<!doctype html><html><body><div id=\"root\"></div></body></html>";
    let (once, r1) = frontend::apply_island_mount_script_to_index_html(shell).unwrap();
    assert!(r1.injected);
    assert_eq!(once.matches("island-mount.js").count(), 1);
    let (twice, r2) = frontend::apply_island_mount_script_to_index_html(&once).unwrap();
    assert!(r2.skipped_already_present);
    assert_eq!(twice, once);
}

/// OP-0243 / OP-0255: `index.html` island-mount injection stays explicit and idempotent (no Node).
#[test]
fn frontend_island_mount_index_injection_pure_roundtrip() {
    let shell = "<!doctype html><html><body><div id=\"root\"></div></body></html>";
    let (once, r1) = frontend::apply_island_mount_script_to_index_html(shell).unwrap();
    assert!(r1.injected);
    assert!(once.contains(frontend::ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET));
    let (twice, r2) = frontend::apply_island_mount_script_to_index_html(&once).unwrap();
    assert!(r2.skipped_already_present);
    assert_eq!(twice, once);
}

/// OP-0240 / OP-0256: default build summary when no **`islands/package.json`** at repo root (clone of this test’s cwd is not a full app — assert API types compile and empty summary shape).
#[test]
fn islands_build_summary_default_is_empty() {
    let s = frontend::IslandsBuildSummary::default();
    assert!(!s.islands_package_present);
    assert!(!s.vite_dist_copied);
    assert!(!s.index_injection.evaluated);
}

/// OP-0312: invalid Web IR (duplicate client route ids) fails `vox build` when validate is on.
#[tokio::test]
async fn full_stack_build_fails_web_ir_validate_on_duplicate_client_routes() {
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo.pop();
    repo.pop();
    let vox_file =
        repo.join("crates/vox-integration-tests/tests/fixtures/web_ir_validate_dup_routes.vox");
    assert!(
        vox_file.is_file(),
        "missing fixture: {}",
        vox_file.display()
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    let err = match build::run(&vox_file, &out).await {
        Ok(()) => panic!("expected WebIR validate gate failure for duplicate route contracts"),
        Err(e) => e,
    };
    let display = format!("{err:#}");
    assert!(
        display.contains("VOX_WEBIR_VALIDATE") && display.contains("web_ir_validate."),
        "expected VOX_WEBIR_VALIDATE + taxonomy codes in error, got:\n{display}"
    );
}

/// OP-S048: parity extra gate — temp `vox build` with Web IR validate emits V1 island mount on classic page.
#[tokio::test]
async fn op_s048_parity_extra_gate_build_emits_island_mount_attrs() {
    const SRC: &str = r#"
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
    let tmp = tempfile::tempdir().expect("tempdir");
    let vox_path = tmp.path().join("parity.vox");
    std::fs::write(&vox_path, SRC).expect("write parity.vox");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    build::run(&vox_path, &out)
        .await
        .expect("OP-S048 build must succeed");
    let ts_path = out.join("ParityPage.tsx");
    assert!(ts_path.is_file(), "missing {}", ts_path.display());
    let ts = std::fs::read_to_string(&ts_path).expect("read ParityPage.tsx");
    assert!(
        ts.contains("data-vox-island=\"ParityP\""),
        "expected V1 island attr:\n{ts}"
    );
    assert!(ts.contains("data-prop-label="), "expected prop attr:\n{ts}");
}

/// OP-0314: default island index wiring remains V1 snippet (V2 env is opt-in stub only).
#[test]
fn full_stack_island_mount_snippet_is_v1_by_default() {
    assert!(
        frontend::ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET.contains("/islands/island-mount.js"),
        "{}",
        frontend::ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET
    );
}

/// OP-S070 / S071 / S072: compatibility telemetry strings stable in island-mount bundle.
#[test]
fn op_s070_s071_s072_telemetry_fixture_and_gate_markers() {
    let m = templates::islands_island_mount_tsx();
    assert!(m.contains("__VOX_ISLANDS_V1_METRICS") && m.contains("contract=V1"));
}

/// OP-S094 / S095 / S096: full-stack artifact expectations for golden minimal build.
#[tokio::test]
async fn op_s094_s095_s096_artifact_gate_minimal_build_outputs() {
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo.pop();
    repo.pop();
    let vox_file = repo.join("crates/vox-integration-tests/tests/fixtures/full_stack_minimal.vox");
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    build::run(&vox_file, &out).await.expect("build");
    assert!(out.join("api.ts").is_file());
    assert!(templates::islands_island_mount_tsx().contains("propsFromElement"));
}

/// OP-S122 / S123 / S124: V1 parity + script/injection docs exercised by snippet + pure inject.
#[test]
fn op_s122_s123_s124_v1_runtime_parity_gate() {
    op_s044_runtime_injection_helper_gate_idempotent_and_single_mount_ref();
    full_stack_island_mount_snippet_is_v1_by_default();
}

/// OP-S142 / S143 / S144: build telemetry hook strings (hydration path in template).
#[test]
fn op_s142_s143_s144_build_telemetry_fixture_gate() {
    op_s070_s071_s072_telemetry_fixture_and_gate_markers();
}

/// OP-S202 / S203 / S204: runtime + build notes gate — minimal build with validate.
#[tokio::test]
async fn op_s202_s203_s204_runtime_build_gate_c() {
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo.pop();
    repo.pop();
    let vox_file = repo.join("crates/vox-integration-tests/tests/fixtures/full_stack_minimal.vox");
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    build::run(&vox_file, &out).await.expect("build");
    assert!(out.join("api.ts").is_file());
}

/// OP-S217: final full-stack parity — same as S048 island attrs on temp build.
#[tokio::test]
async fn op_s217_final_full_stack_parity_fixture() {
    const SRC: &str = r#"
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
    let tmp = tempfile::tempdir().expect("tempdir");
    let vox_path = tmp.path().join("parity.vox");
    std::fs::write(&vox_path, SRC).expect("write parity.vox");
    let out = tmp.path().join("out");
    let _validate = EnvGuard::set("VOX_WEBIR_VALIDATE", "1");
    build::run(&vox_path, &out)
        .await
        .expect("OP-S217 build must succeed");
    let ts = std::fs::read_to_string(out.join("ParityPage.tsx")).expect("read");
    assert!(ts.contains("data-vox-island=\"ParityP\""));
}

// --- Deferred OP-0310 / OP-0315..OP-0319: explicit `#[ignore]` anchors (no Node/telemetry contract in CI yet) ---

/// OP-0310: would assert `islands/dist` is copied into the app output when a repo ships `islands/package.json` + Vite build.
#[ignore = "requires Node+Vite on host to produce islands/dist"]
#[tokio::test]
async fn deferred_op_0310_islands_dist_copy_integration() {
    panic!("enable when CI provides Node+Vite for islands workspace builds");
}

/// OP-0315: would assert stable build telemetry fields on stdout/stderr from `vox build`.
#[ignore = "build telemetry stdout contract not implemented"]
#[tokio::test]
async fn deferred_op_0315_build_telemetry_stdout_contract() {
    panic!("define telemetry schema then assert parser-friendly lines");
}

/// OP-0316: SPA vs Start mode matrix (router shell differences).
#[ignore = "start/spa mode matrix not a single golden yet"]
#[tokio::test]
async fn deferred_op_0316_spa_start_mode_matrix() {
    panic!("add matrix when `vox build` exposes mode flags in tests");
}

/// OP-0317: deterministic snapshot ordering for generated files.
#[ignore = "snapshot ordering audit not automated"]
#[tokio::test]
async fn deferred_op_0317_generated_file_ordering_audit() {
    panic!("enumerate outputs + sort key policy");
}

/// OP-0318: explicit CRLF/LF expectations — repo policy is `vox ci line-endings`.
#[ignore = "use vox ci line-endings on golden outputs instead of duplicating here"]
#[tokio::test]
async fn deferred_op_0318_line_ending_golden_assertions() {
    panic!("prefer workspace SSOT: docs/src/ci/runner-contract.md + vox ci line-endings");
}

/// OP-0319: single-line gate summary protocol for tooling parsers.
#[ignore = "gate summary line protocol not specified"]
#[tokio::test]
async fn deferred_op_0319_gate_summary_line_protocol() {
    panic!("add SUMMARY=ok|fail style contract to build then assert here");
}
