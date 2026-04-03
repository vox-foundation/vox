pub mod route_telemetry;

pub mod research_policy {
    pub use vox_socrates_policy::ConfidencePolicy;

    #[must_use]
    pub const fn persist_min_confidence() -> f64 {
        ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE
    }

    #[must_use]
    pub const fn training_pair_min_confidence() -> f64 {
        ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE
    }
}
