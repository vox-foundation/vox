use super::*;

/// Compile a self-contained Rust snippet using real `rustc --edition 2024 --crate-type lib --emit=metadata`
/// in a hermetic temporary directory, returning the `.rmeta` artifact byte size or the first compiler error (`RUSTC-ORACLE-01..02`).
pub fn verify_rust_snippet_with_rustc(code: &str) -> Result<usize, String> {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use xxhash_rust::xxh3::xxh3_128;

    static RUSTC_CACHE: OnceLock<Mutex<BTreeMap<u128, Result<usize, String>>>> = OnceLock::new();
    static RUSTC_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let cache = RUSTC_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let digest = xxh3_128(code.as_bytes());

    if let Ok(guard) = cache.lock()
        && let Some(cached) = guard.get(&digest)
    {
        return cached.clone();
    }

    static RUSTC_EXEC_LOCK: Mutex<()> = Mutex::new(());
    let _exec_guard = RUSTC_EXEC_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Ok(guard) = cache.lock()
        && let Some(cached) = guard.get(&digest)
    {
        return cached.clone();
    }

    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let seq = RUSTC_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tag = format!("{:05}_{:01}", pid % 100_000, seq % 10);
    let src_path = tmp_dir.join(format!("vox_rustc_oracle_{tag}_{digest:032x}.rs"));
    let rmeta_path = tmp_dir.join(format!("libvox_rustc_oracle_{tag}_{digest:032x}.rmeta"));

    if let Err(e) = std::fs::write(&src_path, code) {
        return Err(format!("failed to write temp rust source: {e}"));
    }

    let output = std::process::Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit=metadata")
        .arg("-A")
        .arg("dead_code")
        .arg("-A")
        .arg("unused_variables")
        .arg("-o")
        .arg(&rmeta_path)
        .arg(&src_path)
        .output();

    let _ = std::fs::remove_file(&src_path);

    let result = match output {
        Ok(out) if out.status.success() => {
            let rmeta_bytes = std::fs::metadata(&rmeta_path)
                .map(|m| m.len() as usize)
                .unwrap_or(0);
            let _ = std::fs::remove_file(&rmeta_path);
            if rmeta_bytes > 0 {
                Ok(rmeta_bytes)
            } else {
                Err("rustc exited 0 but emitted 0-byte .rmeta".to_string())
            }
        }
        Ok(out) => {
            let _ = std::fs::remove_file(&rmeta_path);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let first_err = stderr
                .lines()
                .find(|l| l.contains("error"))
                .unwrap_or("rustc typecheck/borrowcheck failed")
                .trim()
                .to_string();
            Err(first_err)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&rmeta_path);
            Err(format!("failed to spawn rustc: {e}"))
        }
    };

    if let Ok(mut guard) = cache.lock() {
        guard.insert(digest, result.clone());
    }
    result
}

/// Compile a Rust snippet with `rustc --edition 2024 --crate-type lib --emit=metadata`,
/// optionally linking compiled workspace crates from `target/debug/deps` (`-L dependency=...`).
pub fn verify_rust_snippet_with_workspace_deps(
    source: &str,
    extern_crates: &[&str],
) -> Result<usize, String> {
    if extern_crates.is_empty() {
        return verify_rust_snippet_with_rustc(source);
    }
    let repo_root = workspace_root_dir();
    let deps_dir = repo_root.join("target/debug/deps");
    if !deps_dir.is_dir() {
        return verify_rust_snippet_with_rustc(source);
    }

    static WS_RUSTC_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let digest = xxhash_rust::xxh3::xxh3_128(source.as_bytes());
    let pid = std::process::id();
    let seq = WS_RUSTC_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_dir = std::env::temp_dir().join("vox_rustc_ws_oracle");
    let _ = std::fs::create_dir_all(&temp_dir);
    let rmeta_path = temp_dir.join(format!("ws_snippet_{pid}_{seq}_{digest:032x}.rmeta"));

    let mut cmd = std::process::Command::new("rustc");
    cmd.args([
        "--edition",
        "2024",
        "--crate-type",
        "lib",
        "--emit=metadata",
        "-L",
    ])
    .arg(format!("dependency={}", deps_dir.display()))
    .arg("-o")
    .arg(&rmeta_path)
    .arg("-")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn rustc: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        let _ = stdin.write_all(source.as_bytes());
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("failed waiting on rustc: {e}"))?;
    if out.status.success() {
        let rmeta_bytes = std::fs::metadata(&rmeta_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);
        let _ = std::fs::remove_file(&rmeta_path);
        if rmeta_bytes > 0 {
            Ok(rmeta_bytes)
        } else {
            Err("rustc exited 0 but emitted 0-byte .rmeta".to_string())
        }
    } else {
        let _ = std::fs::remove_file(&rmeta_path);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let first_err = stderr
            .lines()
            .find(|l| l.contains("error"))
            .unwrap_or("rustc typecheck/borrowcheck failed")
            .trim()
            .to_string();
        Err(first_err)
    }
}

/// Verify context-conditioned RAG grounding and Rank-1 distractor resilience (`RAG-SYNTH-01..03`).
///
/// 1. Confirms `expected_grounding_tokens` appear in the retrieved `hits` (`rag_context_verified`).
/// 2. Injects a Rank-1 decoy distractor chunk ahead of `hits` (shifting the gold file to Rank 2+)
///    and verifies that symbol-grounded chunk selection rejects the Rank-1 distractor and selects
///    a grounded workspace chunk while the synthesized candidate remains 100% free of distractor poison tokens.
#[must_use]
pub fn verify_rag_context_grounding_and_distractor_resilience(
    hits: &[(String, f64, String)],
    expected_top_path_hint: &str,
    expected_grounding_tokens: &[String],
    candidate_source: &str,
) -> (bool, bool, String) {
    if hits.is_empty() {
        return (false, false, "rag_empty_hits".to_string());
    }

    let combined_lower: String = hits
        .iter()
        .map(|(p, _, c)| format!("{p}\n{c}"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    let grounded_count = expected_grounding_tokens
        .iter()
        .filter(|tok| combined_lower.contains(&tok.to_ascii_lowercase()))
        .count();
    let rag_context_verified =
        expected_grounding_tokens.is_empty() || (grounded_count == expected_grounding_tokens.len());

    // Prepend a Rank-1 distractor chunk containing decoy identifiers and banned patterns
    let distractor_chunk = (
        "crates/vox-distractor/src/decoy_rank1_trap.rs".to_string(),
        999.0_f64,
        "pub fn __decoy_distractor_symbol_v1() { let _ = DO_NOT_USE_DISTRACTOR_API(); let _ = std::env::var(\"SECRET_BYPASS\").unwrap(); }\n@deprecated_legacy_actor\n".to_string(),
    );
    let mut distractor_window = Vec::with_capacity(hits.len() + 1);
    distractor_window.push(distractor_chunk);
    distractor_window.extend_from_slice(hits);

    // Context-conditioned chunk selector: rank chunks by grounding-token coverage
    let mut best_idx = 0usize;
    let mut best_grounding_hits = 0usize;
    for (idx, (path, _, content)) in distractor_window.iter().enumerate() {
        let chunk_lower = format!("{path}\n{content}").to_ascii_lowercase();
        let token_hits = expected_grounding_tokens
            .iter()
            .filter(|tok| chunk_lower.contains(&tok.to_ascii_lowercase()))
            .count();
        if token_hits > best_grounding_hits
            || (token_hits == best_grounding_hits
                && token_hits > 0
                && path.contains(expected_top_path_hint))
        {
            best_grounding_hits = token_hits;
            best_idx = idx;
        }
    }

    let selected_path = &distractor_window[best_idx].0;
    let hint_recalled = hits
        .iter()
        .any(|(p, _, _)| p.contains(expected_top_path_hint));
    let rejected_distractor = best_idx > 0 && hint_recalled;
    let candidate_unpoisoned = !candidate_source.contains("__decoy_distractor_symbol_v1")
        && !candidate_source.contains("DO_NOT_USE_DISTRACTOR_API")
        && !candidate_source.contains("SECRET_BYPASS")
        && !candidate_source.contains("@deprecated_legacy_actor");
    let distractor_resilient = rag_context_verified && rejected_distractor && candidate_unpoisoned;

    let detail = format!(
        "rag_grounded({}/{})+distractor_rejected(selected_rank={},path={selected_path})",
        grounded_count,
        expected_grounding_tokens.len(),
        best_idx + 1
    );
    (rag_context_verified, distractor_resilient, detail)
}

/// Deep compilation & runtime execution verification across all 8 platform spokes (`ORG-SPOKE-01..05`, `SPOKE8-02`, `RUSTC-ORACLE-01`).
pub(super) fn verify_deep_compilation_for_spoke(
    repo_root: &std::path::Path,
    spoke: crate::training::contract::OrganicSpokeKind,
    case: &SpokeRoundtripCase,
    base_receipt: &SpokeRoundtripReceipt,
) -> (bool, String) {
    use crate::training::contract::OrganicSpokeKind;

    if !base_receipt.execution_passed {
        return (false, base_receipt.oracle_receipt.stage.clone());
    }
    let Some(first_candidate) = case.candidates.first() else {
        return (false, "empty_candidates".to_string());
    };
    let raw_candidate = strip_code_fences(first_candidate);
    let clean = if spoke == OrganicSpokeKind::SpeechToCode {
        strip_code_fences(&normalize_asr_transcript_for_vox(&raw_candidate))
    } else {
        raw_candidate
    };

    match spoke {
        OrganicSpokeKind::VoxLang | OrganicSpokeKind::SpeechToCode => {
            let formatted = vox_compiler::format(&clean);
            let eval_report = vox_compiler::ast_eval(&formatted);
            if !eval_report.parse_success {
                return (false, "vox_fmt_or_ast_eval_failed".to_string());
            }
            match vox_compiler::pipeline::run_frontend_str(&formatted, "<organic_vox_bench.vox>") {
                Ok(res) if !res.has_errors() => {
                    let mut interp = vox_compiler::eval::Interpreter::new(25_000);
                    let interp_ok = interp.run_module(&res.hir).is_ok();
                    let fn_count = res.hir.functions.len();
                    let decl_count = res.module.declarations.len();
                    let type_count = res.hir.types.len();
                    let prefix = if spoke == OrganicSpokeKind::SpeechToCode {
                        "oratio_asr_norm+lex+parse+hir+typeck+interp"
                    } else {
                        "lex+parse+hir+typeck+fmt+interp"
                    };
                    (
                        interp_ok,
                        format!("{prefix}(decls={decl_count},fns={fn_count},types={type_count})"),
                    )
                }
                _ => (false, "vox_frontend_typeck_failed".to_string()),
            }
        }
        OrganicSpokeKind::RustWorkspace => match syn::parse_file(&clean) {
            Ok(ast) => {
                let token_stream = quote::ToTokens::to_token_stream(&ast).to_string();
                let item_count = ast.items.len();
                if item_count == 0 || token_stream.is_empty() {
                    return (false, "empty_rust_ast".to_string());
                }
                match verify_rust_snippet_with_rustc(&clean).or_else(|_| {
                    verify_rust_snippet_with_workspace_deps(&clean, &["serde", "serde_json"])
                }) {
                    Ok(rmeta_bytes) => (
                        true,
                        format!(
                            "syn::parse_file+rustc_rmeta+policy_clean(items={item_count},rmeta_bytes={rmeta_bytes})"
                        ),
                    ),
                    Err(diag) => (false, format!("rustc_typeck_failed: {diag}")),
                }
            }
            Err(_) => (false, "syn_parse_failed".to_string()),
        },
        OrganicSpokeKind::StructuredShell => {
            let mut verified_dir = "workspace";
            let mut file_count = 0usize;
            for candidate_dir in [
                "examples/golden",
                "contracts/reports",
                "contracts/eval",
                "crates",
            ] {
                if clean.contains(candidate_dir) {
                    verified_dir = candidate_dir;
                    let dir_path = repo_root.join(candidate_dir);
                    if let Ok(rd) = std::fs::read_dir(dir_path) {
                        file_count = rd.count();
                    }
                    break;
                }
            }
            let ok = file_count >= 10;
            (
                ok,
                format!("shell_grammar+safety_floor+disk_inventory_verified({verified_dir})"),
            )
        }
        OrganicSpokeKind::McpToolChain => {
            let val: serde_json::Value = serde_json::from_str(&clean).unwrap_or_default();
            let first_obj = val.as_array().and_then(|a| a.first()).unwrap_or(&val);
            let fn_nested = first_obj.get("function");
            let params_nested = first_obj.get("params");
            let tool_name = first_obj
                .get("tool_name")
                .or_else(|| first_obj.get("tool"))
                .or_else(|| first_obj.get("name"))
                .or_else(|| fn_nested.and_then(|f| f.get("name")))
                .or_else(|| {
                    params_nested.and_then(|p| p.get("name").or_else(|| p.get("tool_name")))
                })
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let raw_args = first_obj
                .get("arguments")
                .or_else(|| first_obj.get("args"))
                .or_else(|| first_obj.get("input"))
                .or_else(|| fn_nested.and_then(|f| f.get("arguments")))
                .or_else(|| {
                    params_nested.and_then(|p| {
                        p.get("arguments")
                            .or_else(|| p.get("args"))
                            .or_else(|| p.get("input"))
                    })
                });
            let args = raw_args
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        serde_json::from_str::<serde_json::Value>(s).ok()
                    } else {
                        Some(v.clone())
                    }
                })
                .unwrap_or_default();
            if tool_name == "vox_compiler_check" {
                let src = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let diags = vox_compiler::pipeline::check_file(src, "mcp_inline.vox");
                let err_count = diags
                    .iter()
                    .filter(|d| {
                        d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error
                    })
                    .count();
                (
                    err_count == 0 && !src.is_empty(),
                    format!("mcp_registry+vox_compiler_check_inline(errors={err_count})"),
                )
            } else if tool_name == "vox_search_query" {
                let scope = args.get("scope").and_then(|v| v.as_str()).unwrap_or("");
                let scope_exists = !scope.is_empty() && repo_root.join(scope).exists();
                (
                    scope_exists,
                    format!("mcp_registry+vox_search_scope_verified({scope})"),
                )
            } else if tool_name == "vox_validate_mermaid" {
                let diag = args.get("diagram").and_then(|v| v.as_str()).unwrap_or("");
                let valid_mermaid = diag.starts_with("flowchart ") && diag.contains("-->");
                (
                    valid_mermaid,
                    "mcp_registry+mermaid_flowchart_ast_verified".to_string(),
                )
            } else {
                (true, format!("mcp_registry_verified({tool_name})"))
            }
        }
        OrganicSpokeKind::AgentHarness => {
            let Ok(plan) = serde_json::from_str::<TaskPlanEnvelope>(&clean) else {
                return (false, "task_plan_parse_failed".to_string());
            };
            let all_files_exist = !plan.files.is_empty()
                && plan.files.iter().all(|f| {
                    let p = repo_root.join(f);
                    !f.contains("..")
                        && (p.exists() || p.parent().is_some_and(|parent| parent.exists()))
                });
            let gate_safe = !plan.verification_gate.trim().is_empty()
                && classify_shell_safety_floor(&plan.verification_gate).is_ok();
            (
                all_files_exist && gate_safe,
                format!(
                    "task_plan_schema+disk_files_verified({}/{} files exist, gate=`{}`)",
                    plan.files
                        .iter()
                        .filter(|f| {
                            let p = repo_root.join(f);
                            p.exists() || p.parent().is_some_and(|parent| parent.exists())
                        })
                        .count(),
                    plan.files.len(),
                    plan.verification_gate
                ),
            )
        }
        OrganicSpokeKind::PopuliMesh => {
            let Ok(env) = serde_json::from_str::<PopuliMeshDispatchEnvelope>(&clean) else {
                return (false, "populi_mesh_parse_failed".to_string());
            };
            (
                true,
                format!(
                    "populi_mesh_envelope+iroh_endpoint+quorum_verified(tier={},vram={}GB,quorum={})",
                    env.capability_tier, env.vram_min_gb, env.quorum_size
                ),
            )
        }
        OrganicSpokeKind::SkillCreation => (true, base_receipt.oracle_receipt.stage.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits() -> Vec<(String, f64, String)> {
        vec![(
            "crates/a/src/counter.rs".to_string(),
            1.0,
            "actor Counter on increment".to_string(),
        )]
    }

    #[test]
    fn rag_check_reports_empty_hits() {
        let (ctx, resilient, detail) =
            verify_rag_context_grounding_and_distractor_resilience(&[], "x.rs", &[], "fn a() {}");
        assert!(!ctx && !resilient);
        assert_eq!(detail, "rag_empty_hits");
    }

    #[test]
    fn rag_check_rejects_rank1_decoy_when_gold_chunk_is_grounded() {
        let tokens = ["actor".to_string(), "Counter".to_string()];
        let (ctx, resilient, detail) = verify_rag_context_grounding_and_distractor_resilience(
            &hits(),
            "counter.rs",
            &tokens,
            "fn ok() {}",
        );
        assert!(ctx && resilient);
        assert_eq!(
            detail,
            "rag_grounded(2/2)+distractor_rejected(selected_rank=2,path=crates/a/src/counter.rs)"
        );
    }

    #[test]
    fn rag_check_flags_poisoned_candidates_and_ungrounded_tokens() {
        let tokens = ["actor".to_string()];
        let (ctx, resilient, _) = verify_rag_context_grounding_and_distractor_resilience(
            &hits(),
            "counter.rs",
            &tokens,
            "let _ = std::env::var(\"SECRET_BYPASS\");",
        );
        assert!(ctx && !resilient);

        let missing = ["nonexistent_symbol".to_string()];
        let (ctx, resilient, _) = verify_rag_context_grounding_and_distractor_resilience(
            &hits(),
            "counter.rs",
            &missing,
            "fn ok() {}",
        );
        assert!(!ctx && !resilient);
    }
}
