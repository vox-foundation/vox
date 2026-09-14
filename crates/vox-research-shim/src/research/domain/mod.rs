//! Specialized domain engines for deep research.

pub mod codegen;
pub mod shopping;

pub use codegen::{
    CodeSandboxResult, CodeSelfCorrectionResult, attempt_code_self_correction,
    codegen_synthesis_instructions, extract_code_snippets_from_markdown,
    generate_codegen_subqueries, verify_rust_code_in_sandbox, wrap_code_snippet_if_needed,
};
pub use shopping::{
    deboost_affiliate_spam, generate_shopping_subqueries, is_affiliate_seo_spam,
    shopping_synthesis_instructions,
};
