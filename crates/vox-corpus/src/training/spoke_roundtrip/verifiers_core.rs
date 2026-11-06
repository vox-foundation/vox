use super::*;

/// Strip optional markdown code fences (` ```vox ... ``` `, ` ```rust ... ``` `, ` ```json ... ``` `)
/// or extract one or more embedded fenced code blocks when preceded/followed by explanatory prose
/// (`SOTA-SPOKE-HARDEN-01`, `SOTA-SPOKE-UNSEEN-01`), while preserving `SKILL.md` documents whose
/// YAML frontmatter starts with `---`.
#[must_use]
pub fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n") {
        return trimmed.to_string();
    }
    let mut blocks = Vec::new();
    let mut cursor = trimmed;
    while let Some(open_idx) = cursor.find("```") {
        let after_ticks = &cursor[open_idx + 3..];
        let Some(nl_idx) = after_ticks.find('\n') else {
            break;
        };
        let lang_tag = after_ticks[..nl_idx].trim().to_ascii_lowercase();
        let body_start = &after_ticks[nl_idx + 1..];
        if matches!(lang_tag.as_str(), "markdown" | "md")
            && (body_start.trim_start().starts_with("---\n")
                || body_start.trim_start().starts_with("---\r\n"))
            && let Some(last_close) = body_start.rfind("```")
        {
            return body_start[..last_close].trim().to_string();
        }
        let Some(close_idx) = body_start.find("```") else {
            break;
        };
        let block_body = body_start[..close_idx].trim();
        if !block_body.is_empty() {
            blocks.push(block_body);
        }
        cursor = &body_start[close_idx + 3..];
    }
    if !blocks.is_empty() {
        return blocks.join("\n\n");
    }
    trimmed.to_string()
}

/// Spoke 1 (`SPOKE-01`): `.vox` compiler (`lex -> parse -> HIR -> typeck`) + `Interpreter` round-trip.
#[must_use]
pub fn verify_voxlang_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    let code = strip_code_fences(response);
    let receipt = crate::corpus::corpus_readiness::verify_vox_admission(&code);

    let mut constructs = Vec::new();
    for kw in [
        "fn ",
        "type ",
        "actor ",
        "workflow ",
        "activity ",
        "table ",
        "index ",
        "query ",
        "mutation ",
        "server ",
        "component ",
        "tool ",
        "resource ",
        "form ",
        "state_machine ",
        "routes ",
    ] {
        if code.contains(kw) {
            constructs.push(kw.trim().to_string());
        }
    }

    let syntax_valid = receipt.stage != "empty_check"
        && receipt.stage != "parse"
        && receipt.stage != "grammar_unification_gate";
    let semantic_valid = syntax_valid && receipt.stage != "typeck";
    let execution_passed = receipt.passed;

    let admitted_record = if execution_passed {
        crate::corpus::corpus_readiness::admit_vox_record(
            prompt,
            &code,
            "spoke_roundtrip",
            case_id,
            "vox_roundtrip",
        )
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::VoxLang,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "vox-compiler frontend + Interpreter::run_module ok".to_string()
        } else {
            receipt.diagnostics.join("; ")
        },
        oracle_receipt: receipt,
        admitted_record,
    }
}

/// Verify a VoxLang-First project automation script (`vox run --interp scripts/*.vox`),
/// rejecting legacy `.ps1`/`.sh`/`.py` glue, `shell_exec` sandbox bypasses, and raw secret env reads (`AGENTS.md §VoxLang-First Glue Code`).
#[must_use]
pub fn verify_voxlang_automation_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    let trimmed = response.trim();
    for forbidden_fence in [
        "```powershell",
        "```pwsh",
        "```bash",
        "```sh",
        "```python",
        "```py",
        "#!/bin/",
        "@echo off",
    ] {
        if trimmed.to_ascii_lowercase().contains(forbidden_fence) {
            let receipt = OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: false,
                stage: "voxlang_first_gate".to_string(),
                diagnostics: vec![format!(
                    "VoxLang-first violation: automation must be written in `.vox` (`vox run --interp`), not `{forbidden_fence}`"
                )],
                exit_code: Some(1),
            };
            return SpokeRoundtripReceipt {
                case_id: case_id.to_string(),
                spoke: SpokeKind::VoxLang,
                syntax_valid: false,
                semantic_valid: false,
                execution_passed: false,
                pass_at_1: false,
                pass_at_k: false,
                constructs_covered: Vec::new(),
                stdout_or_result: receipt.diagnostics.join("; "),
                oracle_receipt: receipt,
                admitted_record: None,
            };
        }
    }

    let mut base = verify_voxlang_roundtrip(case_id, prompt, response);
    if let Some(ref mut rec) = base.admitted_record {
        rec.provenance.task_family = "vox_script_interp".to_string();
    }
    base
}

/// Backwards compatibility alias for `verify_voxlang_automation_roundtrip`.
#[must_use]
pub fn verify_voxscript_automation_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    verify_voxlang_automation_roundtrip(case_id, prompt, response)
}

/// Spoke 2 (`SPOKE-02`): Rust Workspace (`syn::parse_file` + stub filter + source-policy audit) round-trip.
#[must_use]
pub fn verify_rust_roundtrip(case_id: &str, prompt: &str, response: &str) -> SpokeRoundtripReceipt {
    let code = strip_code_fences(response);
    let base_receipt = crate::corpus::extract_rs::verify_rust_item_admission(&code);

    let mut policy_violations = Vec::new();
    for banned_crypto in [
        "use blake3",
        "use sha2",
        "use sha3",
        "use md5",
        "use sha1",
        "use chacha20poly1305",
        "use ed25519_dalek",
    ] {
        if code.contains(banned_crypto) {
            policy_violations.push(format!(
                "vox/crypto/banned-crate-import: `{banned_crypto}` must go through vox-crypto"
            ));
        }
    }
    for vendor_url in [
        "api.anthropic.com",
        "api.openai.com",
        "generativelanguage.googleapis.com",
        "api.cohere.ai",
        "api.mistral.ai",
    ] {
        if code.contains(vendor_url) {
            policy_violations.push(format!(
                "llm_provider_call: hardcoded vendor hostname `{vendor_url}` is forbidden; use vox_actor_runtime::llm"
            ));
        }
    }
    for raw_secret in [
        "env::var(\"OPENAI_API_KEY\"",
        "env::var(\"ANTHROPIC_API_KEY\"",
        "env::var(\"GEMINI_API_KEY\"",
        "env::var(\"VOX_SECRET",
    ] {
        if code.contains(raw_secret) {
            policy_violations.push(format!(
                "secret-env-guard: direct secret env read `{raw_secret}` is forbidden; use vox_secrets::resolve_secret"
            ));
        }
    }

    let syntax_valid = syn::parse_file(&code).is_ok() && !code.trim().is_empty();
    let semantic_valid = base_receipt.passed && policy_violations.is_empty();
    let execution_passed = semantic_valid;

    let mut constructs = Vec::new();
    for kw in ["pub fn ", "fn ", "struct ", "enum ", "trait ", "impl "] {
        if code.contains(kw) {
            constructs.push(kw.trim().to_string());
        }
    }

    let mut diagnostics = base_receipt.diagnostics.clone();
    diagnostics.extend(policy_violations);

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::RustWorkspace,
        passed: execution_passed,
        stage: if execution_passed {
            "syn_ast+quality+policy_audit".to_string()
        } else if !syntax_valid {
            "syn_ast".to_string()
        } else {
            "policy_or_quality_audit".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::RustWorkspace,
            prompt,
            "You are a Rust systems engineer working in the Vox workspace.",
            &code,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "rust_roundtrip".to_string(),
                difficulty: 5,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::RustWorkspace,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "syn::parse_file + stub-free + policy-clean ok".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

fn item_has_cfg_attr(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
}

fn item_has_path_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("path"))
}

fn verify_rust_items_structural_integrity(
    resolved_path: &std::path::Path,
    items: &[syn::Item],
    scope_label: &str,
) -> Result<(), String> {
    let mut seen_values: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_types: std::collections::HashSet<String> = std::collections::HashSet::new();

    for item in items {
        match item {
            syn::Item::Fn(item_fn) => {
                if !item_has_cfg_attr(&item_fn.attrs) {
                    let ident = item_fn.sig.ident.to_string();
                    if !seen_values.insert(ident.clone()) {
                        return Err(format!(
                            "duplicate top-level `fn {ident}` definition in {scope_label}"
                        ));
                    }
                }
            }
            syn::Item::Struct(item_struct) => {
                if !item_has_cfg_attr(&item_struct.attrs) {
                    let ident = item_struct.ident.to_string();
                    if !seen_types.insert(ident.clone()) {
                        return Err(format!(
                            "duplicate top-level `struct {ident}` definition in {scope_label}"
                        ));
                    }
                }
            }
            syn::Item::Enum(item_enum) => {
                if !item_has_cfg_attr(&item_enum.attrs) {
                    let ident = item_enum.ident.to_string();
                    if !seen_types.insert(ident.clone()) {
                        return Err(format!(
                            "duplicate top-level `enum {ident}` definition in {scope_label}"
                        ));
                    }
                }
            }
            syn::Item::Trait(item_trait) => {
                if !item_has_cfg_attr(&item_trait.attrs) {
                    let ident = item_trait.ident.to_string();
                    if !seen_types.insert(ident.clone()) {
                        return Err(format!(
                            "duplicate top-level `trait {ident}` definition in {scope_label}"
                        ));
                    }
                }
            }
            syn::Item::Type(item_type) => {
                if !item_has_cfg_attr(&item_type.attrs) {
                    let ident = item_type.ident.to_string();
                    if !seen_types.insert(ident.clone()) {
                        return Err(format!(
                            "duplicate top-level `type {ident}` definition in {scope_label}"
                        ));
                    }
                }
            }
            syn::Item::Mod(item_mod) => {
                let ident = item_mod.ident.to_string();
                let has_cfg = item_has_cfg_attr(&item_mod.attrs);
                let has_path = item_has_path_attr(&item_mod.attrs);
                if let Some((_, sub_items)) = &item_mod.content {
                    let sub_scope = format!("inline `mod {ident}`");
                    verify_rust_items_structural_integrity(resolved_path, sub_items, &sub_scope)?;
                } else if !has_cfg
                    && !has_path
                    && let Some(dir) = resolved_path.parent()
                {
                    let direct_rs = dir.join(format!("{ident}.rs"));
                    let nested_mod = dir.join(&ident).join("mod.rs");
                    let stem_subdir_ok = resolved_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .filter(|s| !matches!(*s, "mod" | "lib" | "main"))
                        .is_some_and(|stem| {
                            dir.join(stem).join(format!("{ident}.rs")).is_file()
                                || dir.join(stem).join(&ident).join("mod.rs").is_file()
                        });
                    if !direct_rs.is_file() && !nested_mod.is_file() && !stem_subdir_ok {
                        return Err(format!(
                            "unresolved out-of-line `mod {ident};` declaration (neither `{ident}.rs` nor `{ident}/mod.rs` exists)"
                        ));
                    }
                }
            }
            // Item::Impl, Item::Use, Item::Const, Item::Static, Item::Macro, etc.
            // Note: Item::Impl is deliberately ignored so multiple `impl` blocks defining
            // `pub fn new()` in the same file never false-positive even if strings/chars
            // contain unmatched braces.
            _ => {}
        }
    }
    Ok(())
}

/// Phase 166 (`SOTA-SYN-STRUCT-01..03`): `syn::File` AST-based structural integrity verifier for
/// Rust modules. Replaces naive line-by-line `brace_depth` scanning so unmatched braces inside
/// string/char/raw-string literals or multi-line `#[cfg(...)]` attributes never corrupt module
/// symbol uniqueness or out-of-line `mod <name>;` resolution checks.
pub fn verify_rust_ast_structural_integrity(
    resolved_path: &std::path::Path,
    text: &str,
) -> Result<(), String> {
    let parsed = syn::parse_file(text).map_err(|e| format!("syn::parse_file failed: {e}"))?;
    verify_rust_items_structural_integrity(resolved_path, &parsed.items, "the same module")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fences_unwraps_blocks_and_preserves_skill_frontmatter() {
        assert_eq!(strip_code_fences("  plain text  "), "plain text");
        assert_eq!(strip_code_fences("```vox\nfn a() {}\n```"), "fn a() {}");
        assert_eq!(
            strip_code_fences("intro\n```rust\nA\n```\nmid\n```rust\nB\n```\noutro"),
            "A\n\nB"
        );
        let skill = "---\nname: x\n---\n# Body";
        assert_eq!(strip_code_fences(skill), skill);
    }

    #[test]
    fn automation_roundtrip_rejects_shell_glue_and_tags_vox_scripts() {
        let bad = verify_voxlang_automation_roundtrip(
            "auto_bad",
            "write automation",
            "```bash\necho hi\n```",
        );
        assert!(!bad.execution_passed);
        assert_eq!(bad.oracle_receipt.stage, "voxlang_first_gate");
        assert!(bad.admitted_record.is_none());

        let ok = verify_voxscript_automation_roundtrip(
            "auto_ok",
            "write automation",
            "```vox\nfn double(x: int) to int { return x * 2 }\n```",
        );
        assert!(ok.execution_passed);
        let rec = ok.admitted_record.expect("admitted");
        assert_eq!(rec.provenance.task_family, "vox_script_interp");
    }

    #[test]
    fn rust_ast_structural_integrity_accepts_valid_and_rejects_unparsable_source() {
        let path = std::path::Path::new("/nonexistent/lib.rs");
        assert!(verify_rust_ast_structural_integrity(path, "pub fn a() {}\npub fn b() {}").is_ok());
        let err = verify_rust_ast_structural_integrity(path, "pub fn a( {").unwrap_err();
        assert!(err.contains("syn::parse_file failed"));
    }
}
