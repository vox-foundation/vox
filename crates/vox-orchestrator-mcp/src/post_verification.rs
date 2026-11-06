//! Automatic post-mutation verification feedback — the verification-driven agent loop.
//!
//! After a file-mutating MCP tool succeeds, if it touched a `.vox` file we run the
//! existing [`crate::code_validator::vox_check`] validator and surface any
//! error-severity diagnostics back inside that tool's result. The agent's normal
//! loop then self-corrects on its next turn, without having to remember to call the
//! validator. This makes "verification the agent can run itself" automatic — the
//! single most-loved Claude Code property.
//!
//! Gated by `VOX_VERIFY_ON_WRITE` (default ON; set to `0`/`false`/`off`/`no` to disable).

use crate::code_validator;
use crate::params::VoxCheckParams;
use crate::server_state::ServerState;

/// Returns the workspace-relative `.vox` path to verify, iff `name` is a file-mutating
/// tool (the write-tool set gated in [`crate::scope_guard`]) that touched a `.vox` file.
/// Pure; performs no I/O.
pub fn verifiable_vox_path(name: &str, args: &serde_json::Value) -> Option<String> {
    if !crate::scope_guard::WRITE_TOOLS.contains(&name) {
        return None;
    }
    let path = crate::scope_guard::PATH_ARG_KEYS
        .iter()
        .find_map(|key| args.get(*key).and_then(|v| v.as_str()))?;
    let is_vox = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("vox"));
    is_vox.then(|| path.to_string())
}

/// Whether auto-verification is enabled. Env-gated, default ON.
pub fn enabled() -> bool {
    match std::env::var("VOX_VERIFY_ON_WRITE") {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        Err(_) => true,
    }
}

/// Given the JSON returned by [`crate::code_validator::vox_check`], produce a concise,
/// model-facing advisory iff the check ran successfully **and** found error-severity
/// diagnostics. Returns `None` when the check is clean or when the check itself failed
/// to run (so we never fabricate a verification verdict). Pure.
pub fn advisory_from_check_json(check_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(check_json).ok()?;
    if v.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return None;
    }
    let data = v.get("data")?;
    if !data
        .get("has_errors")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
    {
        return None;
    }
    let count = data.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    let first = data
        .get("diagnostics")
        .and_then(|d| d.as_array())
        .and_then(|arr| {
            arr.iter()
                .find_map(|d| d.get("message").and_then(|m| m.as_str()))
        })
        .unwrap_or("see diagnostics");
    Some(format!(
        "AUTO_VERIFICATION_FAILED: your edit was written, but `vox check` reports {count} \
         diagnostic(s) (first error: {first}). Fix the file and re-verify before continuing."
    ))
}

/// Attach an advisory to a tool-result payload. If the payload is a JSON object we set
/// a top-level `auto_verification_failed: true` flag and a `meta.auto_verification`
/// note (preserving any existing `meta`); otherwise we append the advisory as text.
/// Pure.
pub fn attach_advisory(payload: String, advisory: String) -> String {
    match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(mut v) if v.is_object() => {
            let obj = v.as_object_mut().expect("checked is_object");
            obj.insert(
                "auto_verification_failed".to_string(),
                serde_json::Value::Bool(true),
            );
            let meta = obj
                .entry("meta")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if !meta.is_object() {
                *meta = serde_json::Value::Object(serde_json::Map::new());
            }
            meta.as_object_mut()
                .expect("meta normalized to object")
                .insert(
                    "auto_verification".to_string(),
                    serde_json::Value::String(advisory),
                );
            serde_json::to_string(&v).unwrap_or_else(|_| payload.clone())
        }
        _ => format!("{payload}\n\n{advisory}"),
    }
}

/// Returns the workspace-relative `.md`/`.mdx` path to verify, iff `name` is a
/// file-mutating tool that touched a Markdown file. Pure; performs no I/O.
pub fn verifiable_markdown_path(name: &str, args: &serde_json::Value) -> Option<String> {
    if !crate::scope_guard::WRITE_TOOLS.contains(&name) {
        return None;
    }
    let path = crate::scope_guard::PATH_ARG_KEYS
        .iter()
        .find_map(|key| args.get(*key).and_then(|v| v.as_str()))?;
    let is_md = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("mdx"));
    is_md.then(|| path.to_string())
}

/// Returns the workspace-relative `.rs` path to verify, iff `name` is a
/// file-mutating tool that touched a Rust source file (`SOTA-VERIFY-02`). Pure; performs no I/O.
pub fn verifiable_rust_path(name: &str, args: &serde_json::Value) -> Option<String> {
    if !crate::scope_guard::WRITE_TOOLS.contains(&name) {
        return None;
    }
    let path = crate::scope_guard::PATH_ARG_KEYS
        .iter()
        .find_map(|key| args.get(*key).and_then(|v| v.as_str()))?;
    let is_rs = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"));
    is_rs.then(|| path.to_string())
}

/// Returns the workspace-relative `SKILL.md` path to verify, iff `name` is a
/// file-mutating tool that touched a `SKILL.md` file (`SOTA-VERIFY-03`). Pure; performs no I/O.
pub fn verifiable_skill_md_path(name: &str, args: &serde_json::Value) -> Option<String> {
    if !crate::scope_guard::WRITE_TOOLS.contains(&name) {
        return None;
    }
    let path = crate::scope_guard::PATH_ARG_KEYS
        .iter()
        .find_map(|key| args.get(*key).and_then(|v| v.as_str()))?;
    let is_skill_md = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("SKILL.md"));
    is_skill_md.then(|| path.to_string())
}

/// Phase 156 (`SOTA-RS-STRUCT-01..03`): Structural AST/module integrity check for `.rs` edits
/// (including non-standalone files that import `crate::`/`super::`):
/// 1. Flags duplicate un-gated top-level `fn`, `struct`, `enum`, `trait`, or `type` definitions
///    in the same file (a common LLM partial-edit failure mode).
/// 2. Flags out-of-line `mod <name>;` declarations whose target file (`<name>.rs` or
///    `<name>/mod.rs`) does not exist on disk when neither `#[cfg(...)]` nor `#[path = ...]`
///    is present.
pub fn verify_rust_module_structural_integrity(
    resolved_path: &std::path::Path,
    text: &str,
) -> Result<(), String> {
    vox_corpus::training::spoke_roundtrip::verify_rust_ast_structural_integrity(resolved_path, text)
}

/// Run auto-verification for a just-succeeded mutating tool, returning the (already
/// attached) payload. No-op (returns `payload` unchanged) when disabled, when the tool
/// didn't touch a `.vox`, `.rs`, `SKILL.md`, or `.md`/`.mdx` file, or when the file checks clean.
pub async fn verify_and_attach(
    state: &ServerState,
    name: &str,
    args: &serde_json::Value,
    payload: String,
) -> String {
    if !enabled() {
        return payload;
    }
    if let Some(path) = verifiable_vox_path(name, args) {
        let check_json =
            code_validator::vox_check(state, VoxCheckParams { path: path.clone() }).await;
        if let Some(advisory) = advisory_from_check_json(&check_json) {
            return attach_advisory(payload, advisory);
        }
        // SOTA-VERIFY-01: Also enforce Vox Grammar Unification & security rules (retired @island, @table, ret, isnt, shell_exec)
        if let Ok(resolved) =
            crate::workspace_path::resolve_existing_path_in_repository(state, &path)
            && let Ok(text) = tokio::fs::read_to_string(&resolved).await
        {
            let rule_violations =
                vox_corpus::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(
                    &text,
                );
            if let Some(first_violation) = rule_violations.first() {
                let advisory = format!(
                    "AUTO_VERIFICATION_FAILED: your `.vox` edit was written, but Vox Grammar Unification / Policy gate reported {} violation(s) (first: {}). Replace retired decorators/keywords and re-verify.",
                    rule_violations.len(),
                    first_violation
                );
                return attach_advisory(payload, advisory);
            }
        }
        return payload;
    }
    // SOTA-VERIFY-02: Inline Rust (`syn` + policy detectors + standalone `rustc --emit=metadata` borrow/type check)
    if let Some(rs_rel) = verifiable_rust_path(name, args)
        && let Ok(resolved) =
            crate::workspace_path::resolve_existing_path_in_repository(state, &rs_rel)
        && let Ok(text) = tokio::fs::read_to_string(&resolved).await
    {
        let rt = vox_corpus::training::spoke_roundtrip::verify_rust_workspace_roundtrip(
            "post_verify_rs",
            "inline rust post-write check",
            &text,
        );
        if !rt.oracle_receipt.passed {
            let first = rt
                .oracle_receipt
                .diagnostics
                .first()
                .map(String::as_str)
                .unwrap_or("Rust syntax or policy violation");
            let advisory = format!(
                "AUTO_VERIFICATION_FAILED: your Rust edit to `{rs_rel}` failed syntax/policy verification ({first}). Fix the file before continuing."
            );
            return attach_advisory(payload, advisory);
        }
        if let Err(struct_err) = verify_rust_module_structural_integrity(&resolved, &text) {
            let advisory = format!(
                "AUTO_VERIFICATION_FAILED: your Rust edit to `{rs_rel}` failed module/symbol structural verification ({struct_err}). Fix the error before continuing."
            );
            return attach_advisory(payload, advisory);
        }
        let is_standalone = !text.contains("use crate::")
            && !text.contains("use super::")
            && !text.contains("crate::")
            && !text.contains("super::");
        if is_standalone
            && let Err(rustc_err) =
                vox_corpus::training::spoke_roundtrip::verify_rust_snippet_with_rustc(&text)
        {
            let advisory = format!(
                "AUTO_VERIFICATION_FAILED: your Rust edit to `{rs_rel}` failed `rustc` borrow/type checking ({rustc_err}). Fix the error before continuing."
            );
            return attach_advisory(payload, advisory);
        }
        return payload;
    }
    // SOTA-VERIFY-03: Inline SKILL.md agentskills.io frontmatter + embedded .vox doctest verification
    if let Some(skill_rel) = verifiable_skill_md_path(name, args)
        && let Ok(resolved) =
            crate::workspace_path::resolve_existing_path_in_repository(state, &skill_rel)
        && let Ok(text) = tokio::fs::read_to_string(&resolved).await
    {
        let rt = vox_corpus::training::spoke_roundtrip::verify_skill_creation_roundtrip(
            "post_verify_skill_md",
            "inline SKILL.md post-write check",
            &text,
        );
        if !rt.oracle_receipt.passed {
            let first = rt
                .oracle_receipt
                .diagnostics
                .first()
                .map(String::as_str)
                .unwrap_or("SKILL.md agentskills compliance or embedded .vox doctest failed");
            let advisory = format!(
                "AUTO_VERIFICATION_FAILED: your `SKILL.md` edit to `{skill_rel}` failed agentskills/doctest verification ({first}). Fix frontmatter or `.vox` code blocks before continuing."
            );
            return attach_advisory(payload, advisory);
        }
    }
    if let Some(md_rel) = verifiable_markdown_path(name, args)
        && let Ok(resolved) =
            crate::workspace_path::resolve_existing_path_in_repository(state, &md_rel)
        && let Ok(text) = tokio::fs::read_to_string(&resolved).await
        && text.contains("```mermaid")
    {
        let (_, diags) =
            crate::chat_tools::chat::mermaid_repair::validate_and_repair_markdown_mermaid(&text);
        if let Some(first) = diags.first() {
            let advisory = format!(
                "AUTO_VERIFICATION_FAILED: your Markdown edit was written, but Mermaid validation reports {} \
                 error(s) (first at line {}: {} [{}]). Call `vox_validate_mermaid` or quote special characters in node/edge labels.",
                diags.len(),
                first.line,
                first.message,
                first.code
            );
            return attach_advisory(payload, advisory);
        }
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn verifiable_path_is_some_for_vox_write_of_vox_file() {
        // The real tool's key is `file_path`, not `path`.
        let args = json!({ "file_path": "src/foo.vox", "replacement_code": "x" });
        assert_eq!(
            verifiable_vox_path("vox_apply_structured_edit", &args),
            Some("src/foo.vox".to_string())
        );
    }

    #[test]
    fn verifiable_path_is_none_for_non_mutating_tool() {
        let args = json!({ "path": "src/foo.vox" });
        assert_eq!(verifiable_vox_path("vox_git_status", &args), None);
    }

    #[test]
    fn verifiable_path_is_none_for_non_vox_extension() {
        let args = json!({ "path": "src/foo.rs" });
        assert_eq!(
            verifiable_vox_path("vox_apply_structured_edit", &args),
            None
        );
    }

    #[test]
    fn verifiable_path_is_none_when_path_missing() {
        let args = json!({ "content": "x" });
        assert_eq!(
            verifiable_vox_path("vox_apply_structured_edit", &args),
            None
        );
    }

    #[test]
    fn advisory_present_when_check_reports_errors() {
        let check_json = json!({
            "success": true,
            "data": { "has_errors": true, "count": 2,
                      "diagnostics": [{ "message": "cannot find value `x`" }] }
        })
        .to_string();
        let advisory = advisory_from_check_json(&check_json).expect("advisory expected");
        assert!(advisory.contains("AUTO_VERIFICATION_FAILED"));
        assert!(advisory.contains("cannot find value `x`"));
        assert!(advisory.contains('2'));
    }

    #[test]
    fn advisory_absent_when_check_is_clean() {
        let check_json = json!({
            "success": true,
            "data": { "has_errors": false, "count": 0, "diagnostics": [] }
        })
        .to_string();
        assert_eq!(advisory_from_check_json(&check_json), None);
    }

    #[test]
    fn advisory_absent_when_check_itself_failed() {
        // e.g. file unreadable / path rejected — don't fabricate a verification verdict.
        let check_json = json!({ "success": false, "error": "failed to read file" }).to_string();
        assert_eq!(advisory_from_check_json(&check_json), None);
    }

    #[test]
    fn attach_advisory_injects_meta_and_flag_into_json_object() {
        let payload = json!({ "success": true, "data": { "written": true } }).to_string();
        let out = attach_advisory(payload, "ADV".to_string());
        let v: serde_json::Value = serde_json::from_str(&out).expect("still valid json");
        assert_eq!(v["auto_verification_failed"], json!(true));
        assert_eq!(v["meta"]["auto_verification"], json!("ADV"));
        // original fields preserved
        assert_eq!(v["data"]["written"], json!(true));
    }

    #[test]
    fn attach_advisory_preserves_existing_meta_object() {
        let payload = json!({ "success": true, "meta": { "k": 1 } }).to_string();
        let out = attach_advisory(payload, "ADV".to_string());
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(v["meta"]["k"], json!(1));
        assert_eq!(v["meta"]["auto_verification"], json!("ADV"));
    }

    #[test]
    fn attach_advisory_appends_text_for_non_json_payload() {
        let out = attach_advisory("not json".to_string(), "ADV".to_string());
        assert!(out.contains("not json"));
        assert!(out.contains("ADV"));
    }

    /// End-to-end against the real compiler (`code_validator::vox_check` →
    /// `vox_compiler::pipeline::check_file`): a syntactically-broken `.vox` write
    /// gets an advisory attached; a clean `.vox` write is passed through untouched.
    /// Writes uniquely-named probe files under the repo root (then removes them) since
    /// path resolution rejects files outside the repository.
    #[tokio::test]
    async fn verify_and_attach_flags_broken_vox_and_passes_clean_vox() {
        let state = crate::server_state::ServerState::new_test().await;
        let root = state.repository.root.clone();
        let ok_payload =
            serde_json::json!({ "success": true, "data": { "written": true } }).to_string();

        // --- broken file -> advisory attached ---
        let bad_name = "__post_verify_probe_bad__.vox";
        let bad_path = root.join(bad_name);
        std::fs::write(&bad_path, "fn ( {").expect("write bad probe");
        let bad_args = serde_json::json!({ "path": bad_name });
        let bad_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &bad_args,
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&bad_path);
        let bad_v: serde_json::Value =
            serde_json::from_str(&bad_out).expect("broken-file output is still JSON");
        assert_eq!(
            bad_v["auto_verification_failed"],
            serde_json::json!(true),
            "broken .vox should attach a verification advisory; got: {bad_out}"
        );

        // --- clean file -> unchanged passthrough ---
        let ok_name = "__post_verify_probe_ok__.vox";
        let ok_path = root.join(ok_name);
        std::fs::write(&ok_path, "let answer = 42\n").expect("write ok probe");
        let ok_args = serde_json::json!({ "path": ok_name });
        let ok_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &ok_args,
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&ok_path);
        assert_eq!(
            ok_out, ok_payload,
            "clean .vox must pass the payload through untouched; got: {ok_out}"
        );
    }

    #[tokio::test]
    async fn verify_and_attach_catches_vox_retired_syntax_rust_borrow_errors_and_skill_md() {
        let state = crate::server_state::ServerState::new_test().await;
        let root = state.repository.root.clone();
        let ok_payload =
            serde_json::json!({ "success": true, "data": { "written": true } }).to_string();

        // 1. Retired @table / ret in .vox (`SOTA-VERIFY-01`)
        let retired_vox_name = "__post_verify_retired__.vox";
        let retired_vox_path = root.join(retired_vox_name);
        std::fs::write(
            &retired_vox_path,
            "@table type LegacyRow {\n    id: int\n}\nfn bad(x: int) to int {\n    ret x + 1\n}\n",
        )
        .expect("write retired vox");
        let vox_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": retired_vox_name }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&retired_vox_path);
        let vox_v: serde_json::Value = serde_json::from_str(&vox_out).expect("json");
        assert_eq!(vox_v["auto_verification_failed"], serde_json::json!(true));

        // 2. Rust E0502 borrow-checker error vs clean Rust (`SOTA-VERIFY-02`)
        let bad_rs_name = "__post_verify_borrow_bad__.rs";
        let bad_rs_path = root.join(bad_rs_name);
        std::fs::write(
            &bad_rs_path,
            "pub fn append_first(items: &mut Vec<i64>) -> i64 {\n    let first = &items[0];\n    items.push(1);\n    *first\n}\n",
        )
        .expect("write bad rs");
        let bad_rs_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": bad_rs_name }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&bad_rs_path);
        let bad_rs_v: serde_json::Value = serde_json::from_str(&bad_rs_out).expect("json");
        assert_eq!(
            bad_rs_v["auto_verification_failed"],
            serde_json::json!(true),
            "Rust E0502 borrow conflict must attach AUTO_VERIFICATION_FAILED: {bad_rs_out}"
        );
        assert!(
            bad_rs_v["meta"]["auto_verification"]
                .as_str()
                .unwrap_or("")
                .contains("E0502")
                || bad_rs_v["meta"]["auto_verification"]
                    .as_str()
                    .unwrap_or("")
                    .contains("cannot borrow"),
            "Rust advisory must report the rustc borrow diagnostic: {bad_rs_out}"
        );

        let ok_rs_name = "__post_verify_borrow_ok__.rs";
        let ok_rs_path = root.join(ok_rs_name);
        std::fs::write(
            &ok_rs_path,
            "pub fn append_first(items: &mut Vec<i64>) -> i64 {\n    let first = items[0];\n    items.push(1);\n    first\n}\n",
        )
        .expect("write ok rs");
        let ok_rs_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": ok_rs_name }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&ok_rs_path);
        assert_eq!(ok_rs_out, ok_payload);

        // 3. SKILL.md agentskills compliance + embedded .vox doctest (`SOTA-VERIFY-03`)
        let skill_dir = root.join("__post_verify_skill_probe__");
        let _ = std::fs::create_dir_all(&skill_dir);
        let skill_rel = "__post_verify_skill_probe__/SKILL.md";
        let skill_path = root.join(skill_rel);
        std::fs::write(&skill_path, "# Missing frontmatter\n").expect("write bad skill");
        let bad_skill_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": skill_rel }),
            ok_payload.clone(),
        )
        .await;
        let bad_skill_v: serde_json::Value = serde_json::from_str(&bad_skill_out).expect("json");
        assert_eq!(
            bad_skill_v["auto_verification_failed"],
            serde_json::json!(true)
        );

        std::fs::write(
            &skill_path,
            "---\nname: probe-skill\ndescription: \"Valid probe skill with compiled vox doctest.\"\nmetadata:\n  vox-id: vox.skills.probe-skill\n  vox-category: observability\n  vox-tools: vox_validate_file\n---\n# Probe Skill\n\nValid skill body for post-verification testing.\n\n```vox\nfn health(x: int) to int {\n    return x + 1\n}\n```\n",
        )
        .expect("write ok skill");
        let ok_skill_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": skill_rel }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_dir_all(&skill_dir);
        assert_eq!(ok_skill_out, ok_payload);

        // 4. Non-standalone Rust file (`use crate::...`) with duplicate top-level fn or missing `mod` (`SOTA-RS-STRUCT-01..03`)
        let dup_rs_name = "__post_verify_dup_fn__.rs";
        let dup_rs_path = root.join(dup_rs_name);
        std::fs::write(
            &dup_rs_path,
            "use crate::server_state::ServerState;\npub fn compute(x: i32) -> i32 { x + 1 }\npub fn compute(x: i32) -> i32 { x + 2 }\n",
        )
        .expect("write dup rs");
        let dup_rs_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": dup_rs_name }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&dup_rs_path);
        let dup_rs_v: serde_json::Value = serde_json::from_str(&dup_rs_out).expect("json");
        assert_eq!(
            dup_rs_v["auto_verification_failed"],
            serde_json::json!(true),
            "duplicate top-level fn in non-standalone .rs must fail structural verification: {dup_rs_out}"
        );
        assert!(
            dup_rs_v["meta"]["auto_verification"]
                .as_str()
                .unwrap_or("")
                .contains("duplicate top-level `fn compute`"),
            "advisory must name duplicate symbol: {dup_rs_out}"
        );

        let missing_mod_name = "__post_verify_missing_mod__.rs";
        let missing_mod_path = root.join(missing_mod_name);
        std::fs::write(
            &missing_mod_path,
            "use crate::server_state::ServerState;\nmod __definitely_nonexistent_submodule__;\n",
        )
        .expect("write missing mod rs");
        let missing_mod_out = verify_and_attach(
            &state,
            "vox_apply_structured_edit",
            &serde_json::json!({ "path": missing_mod_name }),
            ok_payload.clone(),
        )
        .await;
        let _ = std::fs::remove_file(&missing_mod_path);
        let missing_mod_v: serde_json::Value =
            serde_json::from_str(&missing_mod_out).expect("json");
        assert_eq!(
            missing_mod_v["auto_verification_failed"],
            serde_json::json!(true),
            "missing out-of-line mod must fail structural verification: {missing_mod_out}"
        );
    }

    #[test]
    fn syn_ast_structural_integrity_handles_braces_in_literals_multi_impl_new_and_multiline_cfg() {
        let dummy_path = std::path::Path::new("/tmp/probe_mod.rs");

        // 1. Unmatched `}` in char/string literals + multiple `impl` blocks with `pub fn new()` must pass cleanly
        let ok_code = r#"
pub struct Alpha {
    pub tag: char,
}

impl Alpha {
    pub fn new() -> Self {
        let closing_brace = '}';
        let raw_json_tail = "}}}}}";
        let _ = raw_json_tail;
        Self { tag: closing_brace }
    }
}

pub struct Beta {
    pub count: usize,
}

impl Beta {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[cfg(
    target_os = "linux"
)]
#[allow(dead_code)]
pub fn platform_hook() -> i32 { 1 }

#[cfg(
    not(target_os = "linux")
)]
#[allow(dead_code)]
pub fn platform_hook() -> i32 { 2 }

#[cfg(test)]
#[allow(dead_code)]
mod __missing_cfg_gated_mod__;
"#;
        assert!(
            verify_rust_module_structural_integrity(dummy_path, ok_code).is_ok(),
            "syn AST walker must ignore braces in literals, allow multiple `impl::new()`, and honor multi-line `#[cfg(...)]`"
        );

        // 2. Duplicate un-gated top-level `fn` or `struct` (including inside inline `mod sub { ... }`) is rejected
        let bad_inline_mod = r#"
pub mod sub {
    pub struct Widget { pub id: u32 }
    pub struct Widget { pub id: u64 }
}
"#;
        let err = verify_rust_module_structural_integrity(dummy_path, bad_inline_mod)
            .expect_err("duplicate struct inside inline mod must fail");
        assert!(err.contains("duplicate top-level `struct Widget`"));
        assert!(err.contains("inline `mod sub`"));

        // 3. Un-gated `mod missing_sub;` without backing file is rejected
        let bad_mod = "mod __definitely_missing_unconditional_sub__;\n";
        let mod_err = verify_rust_module_structural_integrity(dummy_path, bad_mod)
            .expect_err("un-gated missing mod must fail");
        assert!(
            mod_err
                .contains("unresolved out-of-line `mod __definitely_missing_unconditional_sub__;`")
        );
    }
}
