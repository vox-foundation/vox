//! `generate` / script-mode entrypoints (OP-0209).

use std::collections::HashMap;
use std::path::Path;

use vox_compiler::ast::span::Span;
use vox_compiler::hir::{DurabilityKind, HirFn, HirModule, HirStmt};
use vox_compiler::rust_interop_support::{
    classify_rust_crate, is_template_managed_script_native_dependency,
    is_template_managed_script_wasi_dependency, is_wasi_unsupported_rust_import,
};

use super::GENERATED_CARGO_EDITION;
use super::emit;
use super::emit::script_db;
use super::manifest::{CodegenOutput, manifest_dependency_path};

fn module_needs_workflow_runtime(module: &HirModule) -> bool {
    let scan = |f: &HirFn| {
        matches!(
            f.durability,
            Some(DurabilityKind::Workflow | DurabilityKind::Activity)
        )
    };
    module.functions.iter().any(scan)
        || module.tests.iter().any(scan)
        || module.mcp_tools.iter().any(|t| scan(&t.func))
        || module.mcp_resources.iter().any(|r| scan(&r.func))
        || module.foralls.iter().any(|forall| scan(&forall.func))
        || module
            .functions
            .iter()
            .any(|f| f.schedule_interval.is_some())
}

/// Generate a full Rust project from a HIR module.
pub fn generate(
    module: &HirModule,
    package_name: &str,
    shell: super::RustAppShell,
) -> Result<CodegenOutput, miette::Error> {
    let out = emit::generate(module, package_name, shell)?;
    Ok(CodegenOutput {
        files: out.files,
        api_client_ts: out.api_client_ts,
    })
}

/// Target for script-mode execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTarget {
    /// Native Rust binary (tokio, vox-actor-runtime).
    Native,
    /// WASI binary (wasm32-wasip1, vox-script-wasi, no tokio).
    Wasi,
}

/// Generate a minimal Rust binary project for script-mode execution.
///
/// Unlike [generate], this skips all web server boilerplate (warp, axum,
/// rust-embed, metrics) and emits only the code needed to compile and run
/// a `fn main()`.
///
/// `runtime_path` should point to the vox-actor-runtime crate directory (e.g. workspace
/// `crates/vox-actor-runtime`). If `None`, uses `VOX_RUNTIME_PATH` env or a fallback.
pub fn generate_script(
    module: &HirModule,
    package_name: &str,
    runtime_path: Option<&Path>,
) -> Result<CodegenOutput, miette::Error> {
    generate_script_with_target(module, package_name, runtime_path, ScriptTarget::Native)
}

/// Generate a script project for the given target (Native or Wasi).
pub fn generate_script_with_target(
    module: &HirModule,
    package_name: &str,
    runtime_path: Option<&Path>,
    target: ScriptTarget,
) -> Result<CodegenOutput, miette::Error> {
    let mut rust_import_dep_lines = std::collections::BTreeMap::<String, String>::new();
    for dep in &module.rust_imports {
        let crate_name = dep.crate_name.trim();
        let is_template_dep = match target {
            ScriptTarget::Native => is_template_managed_script_native_dependency(crate_name),
            ScriptTarget::Wasi => is_template_managed_script_wasi_dependency(crate_name),
        };
        if crate_name.is_empty() || is_template_dep {
            continue;
        }
        let support = classify_rust_crate(crate_name).as_label();
        let dep_spec = if let Some(path) = &dep.path {
            format!("{crate_name} = {{ path = \"{path}\" }}")
        } else if let Some(git) = &dep.git {
            if let Some(rev) = &dep.rev {
                format!("{crate_name} = {{ git = \"{git}\", rev = \"{rev}\" }}")
            } else {
                format!("{crate_name} = {{ git = \"{git}\" }}")
            }
        } else if let Some(version) = &dep.version {
            format!("{crate_name} = \"{version}\"")
        } else {
            format!("{crate_name} = \"*\"")
        };
        let line = format!("# vox_rust_import support_class={support}\n{dep_spec}");
        rust_import_dep_lines
            .entry(crate_name.to_string())
            .or_insert(line);
    }
    let rust_import_deps = if rust_import_dep_lines.is_empty() {
        String::new()
    } else {
        format!(
            "{}\n",
            rust_import_dep_lines
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let mut files = HashMap::new();

    let crate_name = package_name.replace('-', "_");

    let runtime_path_str = runtime_path
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .or_else(|| std::env::var("VOX_RUNTIME_PATH").ok())
        .unwrap_or_else(|| "../vox-actor-runtime".to_string());

    // Native codegen for `crypto.hash_fast` calls `vox_crypto::hash_fast_hex`
    // directly (Task 2, interpreter-first execution PR 2 — SSOT with the
    // interpreter's `eval/builtins.rs` `Some("crypto")` arm, both routing
    // through `vox-crypto` rather than duplicating the XXH3 hex logic).
    // Unconditional like the other core deps below (tokio/serde/etc.) since
    // `builtin_registry.rs`'s emit for `crypto.hash_fast` always references
    // this path regardless of whether this particular script calls it.
    let vox_crypto_path_str = runtime_path
        .and_then(|p| p.parent())
        .map(|p| manifest_dependency_path(&p.join("vox-crypto")))
        .unwrap_or_else(|| "../vox-crypto".to_string());

    let has_tables = !module.tables.is_empty();
    let vox_db_dep = if has_tables {
        let vox_db_path = runtime_path
            .and_then(|p| p.parent())
            .map(|p| manifest_dependency_path(&p.join("vox-db")))
            .unwrap_or_else(|| "../vox-db".to_string());
        format!("vox-db = {{ path = \"{vox_db_path}\" }}\n")
    } else {
        String::new()
    };
    let turso_dep = if has_tables {
        // Must match workspace `[workspace.dependencies].turso` (vox-db uses 0.6).
        "turso = { version = \"0.6\", default-features = false, features = [\"sync\"] }\n"
            .to_string()
    } else {
        String::new()
    };

    // `@traced` functions emit `vox_telemetry::current_trace_context()` directly;
    // vox-actor-runtime re-exports the span helpers but NOT the `vox_telemetry` crate
    // itself (E0433 without this dep).
    let has_traced =
        module.functions.iter().any(|f| f.is_traced) || module.tests.iter().any(|f| f.is_traced);
    let vox_telemetry_dep = if has_traced {
        let vox_telemetry_path = runtime_path
            .and_then(|p| p.parent())
            .map(|p| manifest_dependency_path(&p.join("vox-telemetry")))
            .unwrap_or_else(|| "../vox-telemetry".to_string());
        format!("vox-telemetry = {{ path = \"{vox_telemetry_path}\" }}\n")
    } else {
        String::new()
    };

    let needs_workflow = module_needs_workflow_runtime(module);
    let vox_workflow_runtime_dep = if needs_workflow {
        let vox_workflow_runtime_path = runtime_path
            .and_then(|p| p.parent())
            .map(|p| manifest_dependency_path(&p.join("vox-workflow-runtime")))
            .unwrap_or_else(|| "../vox-workflow-runtime".to_string());
        format!(
            "vox-workflow-runtime = {{ path = \"{vox_workflow_runtime_path}\", default-features = false }}\nanyhow = \"1\"\n"
        )
    } else {
        String::new()
    };

    // ── WASI feature guardrail ──────────────────────────────────────────────
    // Jai-inspired: fail loudly and immediately with a clear diagnostic
    // rather than emitting broken code that produces confusing linker errors
    // or silent runtime panics inside the Wasmtime sandbox.
    if target == ScriptTarget::Wasi {
        let mut unsupported: Vec<String> = Vec::new();

        if !module.endpoint_fns.is_empty() {
            unsupported.push(format!(
                "endpoint functions are not supported in WASI mode: {}",
                module
                    .endpoint_fns
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !module.mcp_tools.is_empty() || !module.mcp_resources.is_empty() {
            let mut names: Vec<&str> = module
                .mcp_tools
                .iter()
                .map(|t| t.func.name.as_str())
                .collect();
            names.extend(module.mcp_resources.iter().map(|r| r.func.name.as_str()));
            unsupported.push(format!(
                "MCP tools/resources are not supported in WASI mode: {}",
                names.join(", ")
            ));
        }
        let mut wasi_blocked = Vec::new();
        for dep in &module.rust_imports {
            if is_wasi_unsupported_rust_import(dep.crate_name.as_str()) {
                wasi_blocked.push(dep.crate_name.clone());
            }
        }
        if !wasi_blocked.is_empty() {
            unsupported.push(format!(
                "some rust imports are not supported in WASI mode: {}",
                wasi_blocked.join(", ")
            ));
        }

        if !unsupported.is_empty() {
            return Err(miette::miette!(
                help = "Remove these features from the script, or run without --isolation wasm to use the full native runtime.",
                "WASI mode does not support the following features used in this script:\n  • {}",
                unsupported.join("\n  • ")
            ));
        }
    }

    let aegis_patch_path = runtime_path
        .and_then(|p| p.parent())
        .map(|crates| manifest_dependency_path(&crates.join("../patches/aegis-0.9.8")));
    let aegis_patch_section = aegis_patch_path
        .map(|p| format!("\n[patch.crates-io]\naegis = {{ path = \"{p}\" }}\n"))
        .unwrap_or_default();

    let cargo_toml = match target {
        ScriptTarget::Native => format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "{edition}"

[workspace]

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = {{ version = "1", features = ["preserve_order"] }}
tracing = "0.1"
rust_decimal = "1.36"
regex = "1"
vox-actor-runtime = {{ path = "{runtime_path_str}" }}
vox-crypto = {{ path = "{vox_crypto_path_str}" }}
{vox_telemetry_dep}{vox_workflow_runtime_dep}{vox_db_dep}{turso_dep}{rust_import_deps}{aegis_patch_section}"#,
            package_name = package_name,
            runtime_path_str = runtime_path_str,
            vox_crypto_path_str = vox_crypto_path_str,
            vox_telemetry_dep = vox_telemetry_dep,
            vox_workflow_runtime_dep = vox_workflow_runtime_dep,
            vox_db_dep = vox_db_dep,
            turso_dep = turso_dep,
            rust_import_deps = rust_import_deps,
            aegis_patch_section = aegis_patch_section,
            edition = GENERATED_CARGO_EDITION,
        ),
        ScriptTarget::Wasi => {
            let wasi_path = runtime_path
                .and_then(|p| p.parent())
                .map(|p| manifest_dependency_path(&p.join("vox-script-wasi")))
                .unwrap_or_else(|| "../vox-script-wasi".to_string());
            format!(
                r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "{edition}"

[workspace]

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tracing = "0.1"
{rust_import_deps}

[target.'cfg(target_arch = "wasm32")'.dependencies]
vox-script-wasi = {{ path = "{wasi_path}" }}
"#,
                package_name = package_name,
                wasi_path = wasi_path,
                rust_import_deps = rust_import_deps,
                edition = GENERATED_CARGO_EDITION,
            )
        }
    };
    files.insert("Cargo.toml".to_string(), cargo_toml);

    // Emit lib.rs with all non-main declarations (no warp/SSE for script mode).
    // Script mode splits the program into a `vox-script` lib + a thin bin whose
    // `main.rs` does `use vox-script::*` and calls the user's helper functions.
    // Glob imports only see `pub` items, so script-defined functions must be
    // `pub` in the lib or the bin fails with E0425 (e.g. `check_command` in
    // scripts/setup.vox). Force `pub` here — local to script codegen so app-mode
    // lib emission (and its golden snapshots) is untouched.
    let mut script_module = module.clone();
    for f in &mut script_module.functions {
        f.is_pub = true;
    }
    script_db::prepare_script_module(&mut script_module);
    // Script mode runs the bin (`cargo run`), not `cargo test`. `@test` fns are
    // emitted with `#[test]`, which a normal build strips (`cfg(test)`-only) —
    // but a user's `main` may call them directly (the interpreter registers
    // tests as ordinary callables, so it works under `--mode interp`). Re-home
    // them as plain `pub fn`s here so they exist in the lib and are reachable
    // from main.rs. App-mode `emit_lib` still emits them as `#[test]`.
    let mut tests_as_fns = std::mem::take(&mut script_module.tests);
    for t in &mut tests_as_fns {
        t.is_pub = true;
    }
    script_module.functions.append(&mut tests_as_fns);
    script_db::mark_transitive_async(&mut script_module.functions);
    files.insert(
        "src/lib.rs".to_string(),
        emit::emit_script_lib(&script_module),
    );

    // Emit a script-mode main.rs: just `use crate::*;` and the user's main fn body
    let mut main_rs = String::new();
    main_rs.push_str("// Generated by Vox Compiler (script mode)\n");
    // `unconditional_panic`/`arithmetic_overflow`: see the matching comment on
    // `emit::emit_script_lib` (`emit/workflow.rs`) — a Vox script's `main` body
    // is duplicated into both `lib.rs` and here, and a golden may deliberately
    // contain a compile-time-evident fault (Task 2 Step 10 corollary).
    main_rs.push_str("#![allow(unused, unconditional_panic, arithmetic_overflow)]\n\n");
    main_rs.push_str(&format!("use {}::*;\n\n", crate_name));

    let mut found_main = false;
    for func in &module.functions {
        if func.name == "main" {
            found_main = true;
            match target {
                ScriptTarget::Native => {
                    let is_async = func.is_async;
                    // A non-Unit-returning `main` (`fn main() to int`/`to str`)
                    // cannot map to a Rust `fn main()` (which returns `()`):
                    // the emitted `return <value>` would be E0308. Mirror the
                    // interpreter (`vox run --mode interp`, which prints main's
                    // display value): run the body in a closure and print the
                    // result. Unit / unannotated mains keep the direct form.
                    let non_unit_ret = if is_async {
                        None
                    } else {
                        func.return_type
                            .as_ref()
                            .map(emit::emit_type)
                            .filter(|t| t != "()")
                    };
                    // Task 2 Step 10: every variant below wraps the Vox-script
                    // body so a fault (panic — e.g. an `overflow-checks = true`
                    // overflow, or a division by zero) exits the *process*
                    // with code 1 and a clean one-line stderr message, instead
                    // of Rust's default unwind exit code (101) plus a
                    // backtrace-shaped panic message — matching the
                    // interpreter tier, which already turns faults into
                    // `eprintln!` + `process::exit(1)` rather than an
                    // unwinding Rust panic
                    // (`int_overflow_produces_clean_error_not_panic`,
                    // `crates/vox-compiler/tests/eval_typeck_parity_test.rs`).
                    // The sync variants use `std::panic::catch_unwind`
                    // directly; the async variant spawns the body as a task
                    // and reads `JoinError::into_panic()` instead, because
                    // `catch_unwind` around a closure that internally
                    // `.await`s is not well-defined (the unwind can cross a
                    // suspend point). Both funnel into the shared
                    // `vox_actor_runtime::builtins::vox_report_panic_and_exit`
                    // helper so the message-extraction logic has one owner.
                    // Suppress Rust's default panic hook (the
                    // "thread 'main' panicked at ..." + backtrace-hint lines)
                    // so the *only* stderr output on a fault is the clean
                    // message `vox_report_panic_and_exit` prints — matching
                    // the interpreter's single-line fault output.
                    // `catch_unwind`/`JoinError::into_panic` still capture the
                    // payload regardless of the hook; the hook only controls
                    // what gets printed as a side effect of unwinding.
                    let set_quiet_panic_hook =
                        "    std::panic::set_hook(std::boxed::Box::new(|_| {}));\n";
                    if is_async {
                        main_rs.push_str("#[tokio::main]\nasync fn main() {\n");
                        main_rs.push_str(set_quiet_panic_hook);
                        main_rs.push_str("    let __vox_join = tokio::spawn(async move {\n");
                        if has_tables {
                            main_rs.push_str("        vox_script_boot_db().await;\n");
                        }
                        append_script_main_body(
                            &mut main_rs,
                            &func.body,
                            2,
                            &module.inferred_types,
                            &script_module,
                        );
                        main_rs.push_str("    });\n");
                        main_rs.push_str("    match __vox_join.await {\n");
                        main_rs.push_str("        Ok(()) => {}\n");
                        main_rs.push_str("        Err(__vox_join_err) => {\n");
                        main_rs.push_str("            if __vox_join_err.is_panic() {\n");
                        main_rs.push_str(
                            "                vox_actor_runtime::builtins::vox_report_panic_and_exit(__vox_join_err.into_panic());\n",
                        );
                        main_rs.push_str("            } else {\n");
                        main_rs.push_str("                eprintln!(\"{__vox_join_err}\");\n");
                        main_rs.push_str("                std::process::exit(1);\n");
                        main_rs.push_str("            }\n");
                        main_rs.push_str("        }\n");
                        main_rs.push_str("    }\n");
                        main_rs.push_str("}\n");
                    } else if let Some(ret_ty) = non_unit_ret {
                        if has_tables {
                            main_rs.push_str("fn main() {\n");
                            main_rs.push_str(set_quiet_panic_hook);
                            main_rs.push_str(
                                "    let __vox_panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n",
                            );
                            main_rs
                                .push_str("        tokio::runtime::Runtime::new().expect(\"tokio runtime\").block_on(async {\n");
                            main_rs.push_str("            vox_script_boot_db().await;\n");
                            main_rs.push_str(&format!(
                                "            let __vox_main_ret: {ret_ty} = {{\n"
                            ));
                            append_script_main_body(
                                &mut main_rs,
                                &func.body,
                                4,
                                &module.inferred_types,
                                &script_module,
                            );
                            main_rs.push_str("            };\n");
                            main_rs.push_str("            __vox_main_ret\n");
                            main_rs.push_str("        })\n");
                            main_rs.push_str("    }));\n");
                            main_rs.push_str("    match __vox_panic_result {\n");
                            main_rs.push_str(
                                "        Ok(__vox_main_ret) => println!(\"{}\", __vox_main_ret),\n",
                            );
                            main_rs.push_str(
                                "        Err(__vox_panic) => vox_actor_runtime::builtins::vox_report_panic_and_exit(__vox_panic),\n",
                            );
                            main_rs.push_str("    }\n");
                            main_rs.push_str("}\n");
                        } else {
                            main_rs.push_str("fn main() {\n");
                            main_rs.push_str(set_quiet_panic_hook);
                            main_rs.push_str(&format!(
                                "    let __vox_main_ret: {ret_ty} = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{\n"
                            ));
                            append_script_main_body(
                                &mut main_rs,
                                &func.body,
                                3,
                                &module.inferred_types,
                                &script_module,
                            );
                            main_rs.push_str("    })) {\n");
                            main_rs.push_str("        Ok(__vox_v) => __vox_v,\n");
                            main_rs.push_str(
                                "        Err(__vox_panic) => vox_actor_runtime::builtins::vox_report_panic_and_exit(__vox_panic),\n",
                            );
                            main_rs.push_str("    };\n");
                            main_rs.push_str("    println!(\"{}\", __vox_main_ret);\n");
                            main_rs.push_str("}\n");
                        }
                    } else if has_tables {
                        main_rs.push_str("fn main() {\n");
                        main_rs.push_str(set_quiet_panic_hook);
                        main_rs.push_str(
                            "    if let Err(__vox_panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n",
                        );
                        main_rs.push_str(
                            "        tokio::runtime::Runtime::new().expect(\"tokio runtime\").block_on(async {\n",
                        );
                        main_rs.push_str("            vox_script_boot_db().await;\n");
                        append_script_main_body(
                            &mut main_rs,
                            &func.body,
                            3,
                            &module.inferred_types,
                            &script_module,
                        );
                        main_rs.push_str("        });\n");
                        main_rs.push_str("    })) {\n");
                        main_rs.push_str(
                            "        vox_actor_runtime::builtins::vox_report_panic_and_exit(__vox_panic);\n",
                        );
                        main_rs.push_str("    }\n");
                        main_rs.push_str("}\n");
                    } else {
                        main_rs.push_str("fn main() {\n");
                        main_rs.push_str(set_quiet_panic_hook);
                        main_rs.push_str(
                            "    if let Err(__vox_panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n",
                        );
                        append_script_main_body(
                            &mut main_rs,
                            &func.body,
                            2,
                            &module.inferred_types,
                            &script_module,
                        );
                        main_rs.push_str("    })) {\n");
                        main_rs.push_str(
                            "        vox_actor_runtime::builtins::vox_report_panic_and_exit(__vox_panic);\n",
                        );
                        main_rs.push_str("    }\n");
                        main_rs.push_str("}\n");
                    }
                }
                ScriptTarget::Wasi => {
                    if func.is_async {
                        // Jai-inspired: compile-time error, not a runtime surprise.
                        // async fn main() is not supported in WASI mode because Wasmtime P1
                        // does not expose an async executor — use native mode for async scripts.
                        main_rs.push_str("fn main() {\n");
                        main_rs.push_str("    compile_error!(\"async fn main() is not supported in --isolation wasm mode. \\nRemove async or use vox run without --isolation wasm.\");\n");
                        main_rs.push_str("}\n");
                    } else {
                        main_rs.push_str("fn main() {\n");
                        append_script_main_body(
                            &mut main_rs,
                            &func.body,
                            1,
                            &module.inferred_types,
                            &script_module,
                        );
                        main_rs.push_str("}\n");
                    }
                }
            }
            break;
        }
    }

    if !found_main {
        main_rs.push_str("fn main() {\n    eprintln!(\"No main function found.\");\n}\n");
    }

    files.insert("src/main.rs".to_string(), main_rs);

    Ok(CodegenOutput {
        files,
        api_client_ts: String::new(),
    })
}

fn append_script_main_body(
    out: &mut String,
    body: &[HirStmt],
    indent: usize,
    inferred_types: &HashMap<Span, vox_compiler::hir::HirType>,
    script_module: &HirModule,
) {
    script_db::refresh_script_async_metadata(script_module);
    script_db::with_script_db_emit_mode(|| {
        for stmt in body {
            out.push_str(&emit::emit_main_stmt(stmt, indent, Some(inferred_types)));
        }
    });
}
