use crate::{earley, pda};

/// Opaque state carried between generation steps.
///
/// Each backend stores its own internal representation; callers treat this as a
/// cookie returned by `mask_logits` and fed back on the next step.
#[derive(Debug, Clone, Default)]
pub enum SamplerState {
    /// No state (unconstrained mode).
    #[default]
    Empty,
    /// Earley chart state.
    Earley(earley::EarleyState),
    /// PDA stack state.
    Pda(pda::PdaState),
}
