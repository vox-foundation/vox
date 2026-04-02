pub mod github;
/// Syndication lane name in config is `forge`; adapter implementation lives in `github.rs` today.
pub mod forge {
    pub use super::github::post;
}
pub mod hacker_news;
pub mod opencollective;
#[cfg(feature = "scientia-reddit")]
pub mod reddit;
pub mod rss;
pub mod twitter;
#[cfg(feature = "scientia-youtube")]
pub mod youtube;
