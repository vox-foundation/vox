//! Normalize picker slugs against the serve process's loaded model id.

/// `mens/<run>` / path / file-stem IDs so a picker slug matches
/// the serve process's loaded `file_stem`.
pub fn requested_model_matches_loaded(requested: &str, loaded: &str) -> bool {
    fn stem(id: &str) -> &str {
        let trimmed = id.trim().trim_end_matches('/');
        let without = trimmed.strip_prefix("mens/").unwrap_or(trimmed);
        std::path::Path::new(without)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(without)
    }
    let requested = requested.trim();
    !requested.is_empty() && stem(requested) == stem(loaded)
}

#[cfg(test)]
mod tests {
    use super::requested_model_matches_loaded;

    #[test]
    fn mens_prefix_matches_loaded_file_stem() {
        assert!(requested_model_matches_loaded(
            "mens/e2e-smoke-metal",
            "e2e-smoke-metal"
        ));
        assert!(requested_model_matches_loaded(
            "e2e-smoke-metal",
            "e2e-smoke-metal"
        ));
        assert!(requested_model_matches_loaded(
            "mens/e2e-smoke-metal",
            "mens/e2e-smoke-metal"
        ));
    }

    #[test]
    fn distinct_run_ids_do_not_match() {
        assert!(!requested_model_matches_loaded(
            "mens/e2e-smoke-metal",
            "e2e-smoke"
        ));
        assert!(!requested_model_matches_loaded("mens/run-a", "run-b"));
        assert!(!requested_model_matches_loaded("", "e2e-smoke-metal"));
    }
}
