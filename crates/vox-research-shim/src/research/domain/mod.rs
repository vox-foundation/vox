//! Specialized domain engines for deep research.

pub mod codegen;
pub mod shopping;

pub use codegen::{
    CodeSandboxResult, codegen_synthesis_instructions, generate_codegen_subqueries,
    verify_rust_code_in_sandbox,
};
pub use shopping::{
    deboost_affiliate_spam, generate_shopping_subqueries, is_affiliate_seo_spam,
    shopping_synthesis_instructions,
};
