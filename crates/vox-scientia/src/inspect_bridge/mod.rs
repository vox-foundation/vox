//! SCIENTIA Phase 5: UK AISI Inspect task adapter, atomic-NEI novelty scoring,
//! ChronoFact timestamp filtering, and EvidenceConflict detection.

pub mod chronofact;
pub mod conflict;
pub mod inspect_task;
pub mod lexical_shingles;
pub mod novelty;

pub use chronofact::ChronoFilter;
pub use conflict::{ClaimPolarity, EvidenceConflict, EvidenceConflictDetector, PolarizedHit};
pub use inspect_task::{InspectSample, InspectTaskDescriptor, vox_probe_to_inspect_sample};
pub use lexical_shingles::{
    MINHASH_PERMUTATIONS, calibrated_lexical_overlap, minhash_signature, minhash_similarity,
    shingle_jaccard, text_overlap_score, tokenize_normalized,
};
pub use novelty::{AtomicNoveltyScorer, NoveltyConfig, NoveltyVerdict};
