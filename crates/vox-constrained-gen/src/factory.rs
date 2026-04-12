use crate::constrained_sampler::ConstrainedSampler;
use crate::deadlock;
use crate::earley;
use crate::grammar_mode::GrammarMode;
use crate::pda;
use crate::revision;

/// Build the appropriate sampler for the given [`GrammarMode`].
///
/// Returns `None` for `GrammarMode::None` and `GrammarMode::Json` (the latter
/// still uses the legacy `JsonGrammarAutomaton` in vox-populi).
pub fn build_sampler(mode: &GrammarMode) -> Option<Box<dyn ConstrainedSampler>> {
    match mode {
        GrammarMode::None | GrammarMode::Json => None,
        GrammarMode::Vox => {
            let sampler = earley::EarleySampler::from_vox_grammar()
                .expect("failed to build Earley sampler from Vox EBNF");
            Some(Box::new(deadlock::DeadlockWatchdog::new(
                revision::RevisionSampler::new(sampler),
                deadlock::WatchdogConfig::default(),
            )))
        }
        GrammarMode::VoxPda => {
            let sampler = pda::PdaSampler::from_vox_grammar()
                .expect("failed to build PDA sampler from Vox EBNF");
            Some(Box::new(deadlock::DeadlockWatchdog::new(
                revision::RevisionSampler::new(sampler),
                deadlock::WatchdogConfig::default(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_mode_default_is_none() {
        assert_eq!(GrammarMode::default(), GrammarMode::None);
    }

    #[test]
    fn build_sampler_none_returns_none() {
        assert!(build_sampler(&GrammarMode::None).is_none());
        assert!(build_sampler(&GrammarMode::Json).is_none());
    }

    #[test]
    fn build_sampler_vox_returns_some() {
        let s = build_sampler(&GrammarMode::Vox);
        assert!(s.is_some());
        assert_eq!(s.unwrap().name(), "deadlock-watchdog");
    }

    #[test]
    fn build_sampler_vox_pda_returns_some() {
        let s = build_sampler(&GrammarMode::VoxPda);
        assert!(s.is_some());
        assert_eq!(s.unwrap().name(), "deadlock-watchdog");
    }
}
