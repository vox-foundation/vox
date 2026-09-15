//! Specialized domain engines for deep research.

pub mod codegen;
pub mod micro_benchmark;
pub mod polyglot_sandbox;
pub mod shopping;

pub use codegen::{
    CodeSandboxResult, CodeSelfCorrectionResult, attempt_code_self_correction,
    codegen_synthesis_instructions, extract_code_snippets_from_markdown,
    generate_codegen_subqueries, verify_rust_code_in_sandbox, wrap_code_snippet_if_needed,
};
pub use micro_benchmark::{
    MicroBenchmarkReport, run_rust_micro_benchmark, scaffold_rust_benchmark_source,
};
pub use polyglot_sandbox::{
    PolyglotLanguage, SqlDialect, verify_in_polyglot_sandbox, verify_python_in_sandbox,
    verify_sql_in_sandbox, verify_typescript_in_sandbox, verify_vox_in_sandbox,
};
pub use shopping::{
    deboost_affiliate_spam, generate_shopping_subqueries, is_affiliate_seo_spam,
    shopping_synthesis_instructions,
};
