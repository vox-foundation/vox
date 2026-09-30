//! Default / fallback model ids, generated at build time from the single source
//! `contracts/orchestration/model-defaults.v1.yaml` (Task 14). Every role is a
//! `pub const` (SCREAMING_SNAKE of its `role`), usable in const contexts; list
//! roles are `&[&str]`. Never write a vendor model id literal elsewhere —
//! `tests/model_defaults_test.rs` scans the workspace for them.
//!
//! These are fallbacks only: registry selection, operator pins
//! (`VOX_MODEL_FORCE*`) and user config win.

/// One contract entry: its role and its model id(s), in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelDefault {
    pub role: &'static str,
    pub models: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/model_defaults_generated.rs"));

/// Every contract entry, in contract order.
#[must_use]
pub fn all() -> &'static [ModelDefault] {
    ALL_DEFAULTS
}

/// The (first) model id for `role`, or `None` for an unknown role.
#[must_use]
pub fn default_for(role: &str) -> Option<&'static str> {
    ALL_DEFAULTS
        .iter()
        .find(|d| d.role == role)
        .and_then(|d| d.models.first().copied())
}

/// `premium_alias` entries: `(alias, model)` for every `premium_<alias>` role.
pub fn premium_aliases() -> impl Iterator<Item = (&'static str, &'static str)> {
    ALL_DEFAULTS
        .iter()
        .filter_map(|d| Some((d.role.strip_prefix("premium_")?, *d.models.first()?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_for_returns_the_first_model_and_none_for_unknown() {
        assert_eq!(default_for("chat"), Some(CHAT));
        assert_eq!(default_for("free_floor"), FREE_FLOOR.first().copied());
        assert_eq!(default_for("nope"), None);
        assert!(premium_aliases().any(|(a, m)| a == "logic" && m == PREMIUM_LOGIC));
    }
}
