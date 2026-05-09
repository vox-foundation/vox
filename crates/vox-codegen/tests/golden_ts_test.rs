use std::path::PathBuf;
use vox_codegen::codegen_ts::emitter::generate;
use vox_compiler::{hir::lower_module, lexer::cursor::lex, parser::parse};

#[test]
fn golden_ts_emit() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden-ts");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("vox"))
        .collect();
    entries.sort();
    for p in entries {
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&p).unwrap();
        let m = parse(lex(&src)).expect(&stem);
        let hir = lower_module(&m);
        let out = generate(&hir).unwrap();
        let combined = out
            .files
            .iter()
            .map(|(name, content)| format!("=== {} ===\n{}", name, content))
            .collect::<Vec<_>>()
            .join("\n\n");
        insta::with_settings!({ snapshot_suffix => stem.clone() }, {
            insta::assert_snapshot!(combined);
        });
    }
}
