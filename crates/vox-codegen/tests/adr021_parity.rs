#![allow(missing_docs)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use vox_codegen::codegen_rust::emit::emit_fn;
use vox_codegen::codegen_rust::generate_script;
use vox_compiler::hir::{DurabilityKind, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::lexer::lex as lex_script;
use vox_compiler::parser::parse;
use vox_compiler::parser::parse_script;

const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Ok("tx") }
workflow checkout(amount: int) to Result[str] {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;

#[test]
fn emit_contains_dispatcher_and_sql_tracker() {
    let module = parse(lex(SRC)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow))
        .expect("workflow");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);
    assert!(
        rust.contains("VoxDbTracker"),
        "generated workflow must construct VoxDbTracker; got:\n{rust}"
    );
    assert!(
        !rust.contains("journal::execute"),
        "activity/workflow emit must not wrap journal::execute; got:\n{rust}"
    );
}

#[test]
fn generated_crate_defines_vox_run_workflow_and_sql_feature() {
    let module = parse_script(lex_script(SRC)).expect("parse");
    let hir = lower_module(&module);
    let runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-actor-runtime");
    let output = generate_script(&hir, "vox-adr021-emit", Some(&runtime)).expect("emit");
    let joined = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("fn __vox_run_workflow"),
        "emit_main already calls __vox_run_workflow; define it. got {} chars",
        joined.len()
    );
    let manifest = output
        .files
        .get("Cargo.toml")
        .or_else(|| {
            output
                .files
                .iter()
                .find(|(k, _)| k.ends_with("Cargo.toml"))
                .map(|(_, v)| v)
        })
        .expect("manifest");
    assert!(
        manifest.contains("features") && manifest.contains("sql"),
        "generated Cargo.toml must enable vox-workflow-runtime sql; got:\n{manifest}"
    );
    assert!(
        manifest.contains("vox-db"),
        "generated Cargo.toml must depend on vox-db for VoxDbTracker"
    );
}

#[tokio::test]
async fn generated_binary_journal_matches_interpreter() {
    // Copy compile_vox_script from emit_compile_harness.rs (same lock + target dir).
    // After `cargo build` succeeds, `cargo run` with:
    //   VOX_RUN_WORKFLOW=checkout
    //   VOX_WORKFLOW_RUN_ID=adr021-gen
    //   argv or env for amount=5
    // Generated main must print the journal JSON to stdout.
    //
    // Interp side (same process, different run_id so the two trackers do not
    // share rows — we compare *shape*, not the same SQL rows):
    let module = parse(lex(SRC)).expect("parse");
    let hir = lower_module(&module);
    let mut tracker = vox_workflow_runtime::workflow::DefaultTracker;
    let interp_journal = vox_workflow_runtime::workflow::interpret_workflow_durable(
        &hir,
        "checkout",
        vec![serde_json::json!(5)],
        &mut tracker,
    )
    .await
    .expect("interp");

    let gen_stdout = run_generated_workflow(SRC, "checkout", "adr021-gen", &[("amount", "5")])
        .expect("generated binary ran");
    let gen_journal: Vec<Value> = serde_json::from_str(&gen_stdout).expect("journal json");

    let ids = |j: &[Value]| -> Vec<String> {
        j.iter()
            .filter_map(|e| {
                e.get("activity_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    };
    assert_eq!(
        ids(&interp_journal),
        ids(&gen_journal),
        "ADR-021: same activity_id set\ninterp={interp_journal:#?}\ngen={gen_journal:#?}"
    );
    let ret = |j: &[Value]| {
        j.iter()
            .rev()
            .find(|e| e["event"] == "WorkflowCompleted")
            .and_then(|e| e.get("return_value"))
            .cloned()
    };
    assert_eq!(ret(&interp_journal), ret(&gen_journal));
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Standalone script crates built outside the workspace need the same
/// `[patch.crates-io]` entries as the repo root (pure-rust `aegis` on Windows).
/// Also normalize the generated `turso` dep to match workspace 0.6 + sync.
fn inject_workspace_patches(project_dir: &Path) {
    let cargo_path = project_dir.join("Cargo.toml");
    let Ok(mut toml) = std::fs::read_to_string(&cargo_path) else {
        return;
    };
    const CANON_TURSO: &str =
        "turso = { version = \"0.6\", default-features = false, features = [\"sync\"] }";
    if toml.contains("vox-db") {
        if let Some(start) = toml.find("turso = {") {
            if let Some(line_end) = toml[start..].find('\n') {
                let end = start + line_end;
                toml.replace_range(start..end, CANON_TURSO);
            }
        } else if let Some(idx) = toml.find("[dependencies]") {
            let insert_at = toml[idx..]
                .find('\n')
                .map(|off| idx + off + 1)
                .unwrap_or(toml.len());
            toml.insert_str(insert_at, &format!("{CANON_TURSO}\n"));
        }
    }
    let aegis_path = repo_root()
        .join("patches/aegis-0.9.8")
        .to_string_lossy()
        .replace('\\', "/");
    if !toml.contains("[patch.crates-io]") {
        toml.push_str(&format!(
            "\n[patch.crates-io]\naegis = {{ path = \"{aegis_path}\" }}\n"
        ));
    } else if !toml.contains("aegis = ") {
        toml = toml.replace(
            "[patch.crates-io]\n",
            &format!("[patch.crates-io]\naegis = {{ path = \"{aegis_path}\" }}\n"),
        );
    }
    let _ = std::fs::write(cargo_path, toml);
}

static COMPILE_LOCK: Mutex<()> = Mutex::new(());
static PACKAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn run_generated_workflow(
    src: &str,
    workflow: &str,
    run_id: &str,
    args: &[(&str, &str)],
) -> Result<String, String> {
    let module = parse_script(lex_script(src)).map_err(|e| format!("{e:?}"))?;
    let hir = lower_module(&module);
    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vox-actor-runtime");
    let pkg_name = format!(
        "vox-adr021-run-{}-{}",
        std::process::id(),
        PACKAGE_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let output =
        generate_script(&hir, &pkg_name, Some(&runtime)).map_err(|e| format!("emit: {e}"))?;
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    output
        .write_to_dir(dir.path())
        .map_err(|e| format!("write: {e}"))?;
    inject_workspace_patches(dir.path());
    let _guard = COMPILE_LOCK.lock().unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target = std::env::temp_dir().join("vox-emit-harness-target");
    let db_path = dir.path().join("adr021.db");
    let args_json = if args.len() == 1 && args[0].0 == "amount" {
        format!("[{}]", args[0].1)
    } else {
        let values: Vec<Value> = args
            .iter()
            .map(|(_, v)| {
                v.parse::<i64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| Value::String((*v).to_string()))
            })
            .collect();
        serde_json::to_string(&values).unwrap_or_else(|_| "[]".into())
    };
    let out = Command::new(&cargo)
        .current_dir(dir.path())
        .args(["run", "--quiet", "--config", "build.rustc-wrapper=\"\""])
        .env("CARGO_TARGET_DIR", &target)
        .env("VOX_RUN_WORKFLOW", workflow)
        .env("VOX_WORKFLOW_RUN_ID", run_id)
        .env("VOX_WORKFLOW_ARGS", args_json)
        .env("VOX_DB_PATH", &db_path)
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(|e| format!("spawn: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!("cargo run failed: {stderr}\nstdout: {stdout}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
