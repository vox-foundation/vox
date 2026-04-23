use vox_toestub::detectors::hollow_fn::HollowFnDetector;
use vox_toestub::rules::{DetectionRule, Language, SourceFile};
fn main() {
    let f = SourceFile::new(
        std::path::PathBuf::from("test.rs"),
        "fn get_items() -> Vec<Item> {\n    Vec::new()\n}".to_string(),
    );
    let rust_ctx = if f.language == Language::Rust {
        Some(vox_toestub::analysis::RustFileContext::parse(&f.content))
    } else {
        None
    };
    let d = HollowFnDetector::new();
    let findings = d.detect(&f, rust_ctx.as_ref());
    println!("Findings vec: {:?}", findings.len());
}
