#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingBehavior {
    Fail,
    SkipWithReason,
    WarnOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SecretPolicy {
    pub required: bool,
    pub behavior: MissingBehavior,
}

impl SecretPolicy {
    #[must_use]
    pub const fn required_fail() -> Self {
        Self {
            required: true,
            behavior: MissingBehavior::Fail,
        }
    }

    #[must_use]
    pub const fn optional_skip() -> Self {
        Self {
            required: false,
            behavior: MissingBehavior::SkipWithReason,
        }
    }
}
