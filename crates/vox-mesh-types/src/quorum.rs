use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonMode {
    Exact,
    HashMatch,
    SemanticSample,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumPolicy {
    pub min_agreeing_workers: u8,
    pub comparison_mode: ComparisonMode,
}
