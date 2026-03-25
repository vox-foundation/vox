//! Offline deterministic responses.

const FALLBACK_SPRITE: &str = r#"  /\_/\
 ( o.o )
  > ^ <
 /|   |\
(_|   |_)"#;

/// Always-available fallback that returns pattern-based responses.
///
/// This is NOT AI — it's a simple keyword matcher that ensures
/// Vox never fails when AI providers are unavailable.
pub fn deterministic_response(prompt: &str) -> String {
    let lower = prompt.to_lowercase();

    if lower.contains("sprite") || lower.contains("ascii") {
        return FALLBACK_SPRITE.to_string();
    }
    if lower.contains("name") || lower.contains("creative") {
        return "Code Companion".to_string();
    }
    if lower.contains("analyze") || lower.contains("quality") || lower.contains("review") {
        return "CLEAN".to_string();
    }
    if lower.contains("suggest") || lower.contains("fix") {
        return "Consider reviewing this code for potential improvements.".to_string();
    }

    "I'm running in offline mode. AI features will be available when a provider is reachable."
        .to_string()
}
