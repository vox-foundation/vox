#![allow(missing_docs)]

use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::typecheck_module;

#[test]
fn multimodal_image_gen_pipeline() {
    let source = r#"
fn generate_image(prompt: str, size: Option[str]) to Result[str] {
    ret Ok("https://example.com/banner.png")
}

activity generate_banner(prompt: str) to Result[str] {
    let result = generate_image(prompt, Some("1024x1024"))
    ret result
}

workflow handle_branding(description: str) to Unit {
    let banner_url = generate_banner(description)
    match banner_url {
        Ok(_) -> print("Banner generated")
        Error(e) -> print("Failed: " + e)
    }
}
"#;

    let tokens = lex(source);
    let module = parse(tokens).expect("Parse failed");
    let hir = lower_module(&module);

    use vox_compiler::hir::nodes::DurabilityKind;
    let activities: Vec<_> = hir.functions.iter().filter(|f| f.durability == Some(DurabilityKind::Activity)).collect();
    let workflows: Vec<_> = hir.functions.iter().filter(|f| f.durability == Some(DurabilityKind::Workflow)).collect();
    assert_eq!(activities.len(), 1);
    assert_eq!(workflows.len(), 1);
    assert_eq!(activities[0].name, "generate_banner");

    let diagnostics = typecheck_module(&module, "");
    assert!(
        diagnostics.is_empty(),
        "Typecheck failed: {:?}",
        diagnostics
    );
}
