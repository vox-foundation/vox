pub mod github;
/// Syndication lane name in config is `forge`; adapter implementation lives in `github.rs` today.
pub mod forge {
    pub use super::github::post;
}
pub mod researchgate;
pub mod rss;
