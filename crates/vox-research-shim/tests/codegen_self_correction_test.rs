use vox_research_shim::research::domain::codegen::attempt_code_self_correction;

#[tokio::test]
async fn test_attempt_code_self_correction_repairs_syntax_error() {
    let broken_code = "pub fn add(a: i32, b: i32) -> i32 { a + b";

    let result = attempt_code_self_correction(broken_code, &[], 2, |code, stderr| async move {
        assert!(stderr.contains("unclosed") || stderr.contains("expected"));
        let fixed = format!("{code}\n}}");
        let explanation = "Added missing closing brace to function add.".to_string();
        Ok((fixed, explanation))
    })
    .await
    .expect("self-correction loop should execute");

    assert!(!result.initial_passed);
    assert!(result.final_passed);
    assert_eq!(result.iterations, 1);
    assert!(result.corrected_code.is_some());
    assert_eq!(
        result.repair_explanation.as_deref(),
        Some("Added missing closing brace to function add.")
    );
}

#[tokio::test]
async fn test_attempt_code_self_correction_passes_clean_code() {
    let valid_code = "pub fn mul(a: i32, b: i32) -> i32 { a * b }";
    let result = attempt_code_self_correction(valid_code, &[], 2, |_c, _e| async move {
        panic!("Repair function should not be called on valid code");
    })
    .await
    .unwrap();

    assert!(result.initial_passed);
    assert!(result.final_passed);
    assert_eq!(result.iterations, 0);
}

#[tokio::test]
async fn test_attempt_code_self_correction_exhausts_attempts() {
    let broken_code = "pub fn bad() { nonsense123 }";
    let result = attempt_code_self_correction(broken_code, &[], 2, |code, _err| async move {
        Ok((code, "Still broken attempt".to_string()))
    })
    .await
    .unwrap();

    assert!(!result.initial_passed);
    assert!(!result.final_passed);
    assert_eq!(result.iterations, 2);
    assert!(result.final_error.is_some());
    assert_eq!(
        result.repair_explanation.as_deref(),
        Some("Still broken attempt")
    );
}

#[tokio::test]
async fn test_attempt_code_self_correction_repair_fn_error() {
    let broken_code = "pub fn bad() { nonsense123 }";
    let result = attempt_code_self_correction(broken_code, &[], 3, |_code, _err| async move {
        anyhow::bail!("LLM inference failed");
    })
    .await
    .unwrap();

    assert!(!result.initial_passed);
    assert!(!result.final_passed);
    assert_eq!(result.iterations, 1);
    assert!(
        result
            .final_error
            .as_deref()
            .unwrap_or("")
            .contains("Repair generation failed: LLM inference failed")
    );
}

#[test]
fn test_wrap_code_snippet_doc_comments_and_attributes() {
    use vox_research_shim::research::domain::codegen::wrap_code_snippet_if_needed;

    let doc_and_attr = "/// Doc comment\n#[inline]\npub fn foo() {}";
    assert_eq!(wrap_code_snippet_if_needed(doc_and_attr), doc_and_attr);

    let struct_with_derive = "#[derive(Debug)]\npub struct S;";
    assert_eq!(
        wrap_code_snippet_if_needed(struct_with_derive),
        struct_with_derive
    );

    let async_fn = "pub async fn fetch() {}";
    assert_eq!(wrap_code_snippet_if_needed(async_fn), async_fn);

    let bare_async_fn = "async fn helper() {}";
    assert_eq!(wrap_code_snippet_if_needed(bare_async_fn), bare_async_fn);

    let trait_def = "pub trait Greeter { fn greet(&self); }";
    assert_eq!(wrap_code_snippet_if_needed(trait_def), trait_def);

    let bare_trait = "trait InternalGreeter {}";
    assert_eq!(wrap_code_snippet_if_needed(bare_trait), bare_trait);

    let type_alias = "pub type UserId = u64;";
    assert_eq!(wrap_code_snippet_if_needed(type_alias), type_alias);

    let bare_type = "type InternalId = u32;";
    assert_eq!(wrap_code_snippet_if_needed(bare_type), bare_type);

    let const_item = "pub const MAX: usize = 100;";
    assert_eq!(wrap_code_snippet_if_needed(const_item), const_item);

    let bare_const = "const MIN: usize = 1;";
    assert_eq!(wrap_code_snippet_if_needed(bare_const), bare_const);

    let statement = "let a = 10;\nlet b = 20;";
    let wrapped = wrap_code_snippet_if_needed(statement);
    assert!(wrapped.contains("__vox_sandbox_probe"));
}
