//! Scientia social distribution adapters and syndication outcomes.
//!
//! Reddit and YouTube integration builds on [`vox_publisher`]; enable `scientia-reddit` /
//! `scientia-youtube` when layering crate-specific wiring.

#![forbid(unsafe_code)]

pub use vox_publisher;
pub use vox_publisher::types::{
    ChannelPolicyConfig, DistributionPolicyConfig, SyndicationConfig, TopicFiltersConfig,
    UnifiedNewsItem,
};
pub use vox_publisher::{ChannelOutcome, Publisher, PublisherConfig, SyndicationResult};
