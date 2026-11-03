// A raw env read of a Clavis-managed secret. `secret-env-guard` must fail this file.
fn openrouter_key() -> Option<String> {
    std::env::var("OPENROUTER_API_KEY").ok()
}
