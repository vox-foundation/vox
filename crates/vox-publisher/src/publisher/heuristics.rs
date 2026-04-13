use std::collections::BTreeMap;
use crate::types::UnifiedNewsItem;
use super::config::PublisherConfig;

pub fn summarize_for_social(raw: &str, max_chars: usize) -> String {
    crate::contract::clamp_text(raw, max_chars)
}

#[must_use]
pub fn syndication_template_profile_enabled() -> bool {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSyndicationTemplateProfile)
        .expose()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn channel_template_profile_label(item: &UnifiedNewsItem, channel: &str) -> Option<String> {
    let map = &item.syndication.distribution_policy.channel_policy;
    let p = map.get(channel).or_else(|| {
        map.iter()
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(channel))
            .map(|(_, v)| v)
    })?;
    p.template_profile
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

pub fn note_template_profile_inert(
    item: &UnifiedNewsItem,
    channel: &str,
    decision_reasons: &mut BTreeMap<String, String>,
) {
    if channel_template_profile_label(item, channel).is_some()
        && !syndication_template_profile_enabled()
        && !decision_reasons.contains_key("template_profile_inert")
    {
        decision_reasons.insert(
            "template_profile_inert".to_string(),
            "VOX_SYNDICATION_TEMPLATE_PROFILE is not enabled; channel template_profile keys are ignored"
                .to_string(),
        );
    }
}

pub fn twitter_effective_summary_max_chars(
    item: &UnifiedNewsItem,
    cfg: &PublisherConfig,
    decision_reasons: &mut BTreeMap<String, String>,
) -> usize {
    let margin_base = cfg
        .twitter_summary_margin_chars
        .unwrap_or(crate::contract::TWITTER_SUMMARY_MARGIN_CHARS);
    note_template_profile_inert(item, "twitter", decision_reasons);
    if !syndication_template_profile_enabled() {
        return crate::contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_base);
    }
    let Some(ref p) = channel_template_profile_label(item, "twitter") else {
        return crate::contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_base);
    };
    let p_low = p.to_ascii_lowercase();
    let margin_adj = match p_low.as_str() {
        "brief" | "tight" | "compact" => margin_base.saturating_sub(16).max(4),
        "roomy" | "spacious" | "narrative" => {
            (margin_base.saturating_add(24)).min(crate::contract::TWITTER_TEXT_CHUNK_MAX.saturating_div(3))
        }
        _ => {
            decision_reasons.insert(
                "template_profile_fallback_twitter".to_string(),
                format!("unknown template_profile {p:?}; using default twitter margin"),
            );
            margin_base
        }
    };
    decision_reasons.insert(
        "template_profile_resolved_twitter".to_string(),
        format!("{p}:margin_chars={margin_adj}"),
    );
    crate::contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_adj)
}

pub fn social_text_cap_with_template_profile(
    item: &UnifiedNewsItem,
    channel: &str,
    base_cap: usize,
    decision_reasons: &mut BTreeMap<String, String>,
) -> usize {
    note_template_profile_inert(item, channel, decision_reasons);
    if !syndication_template_profile_enabled() {
        return base_cap;
    }
    let Some(ref p) = channel_template_profile_label(item, channel) else {
        return base_cap;
    };
    let p_low = p.to_ascii_lowercase();
    let scaled = match p_low.as_str() {
        "brief" | "tight" | "compact" => base_cap.saturating_mul(88).saturating_div(100).max(120),
        "roomy" | "spacious" | "narrative" => base_cap
            .saturating_mul(114)
            .saturating_div(100)
            .min(base_cap.saturating_add(900)),
        _ => {
            decision_reasons.insert(
                format!("template_profile_fallback_{channel}"),
                format!("unknown template_profile {p:?}; using default cap"),
            );
            base_cap
        }
    };
    decision_reasons.insert(
        format!("template_profile_resolved_{channel}"),
        format!("{p}:cap={scaled}"),
    );
    scaled
}

pub fn normalized_tags(item: &UnifiedNewsItem) -> Vec<String> {
    item.tags.iter().map(|t| t.trim().to_lowercase()).collect()
}

pub fn topic_score(item: &UnifiedNewsItem, include_tags: &[String]) -> f64 {
    if include_tags.is_empty() {
        return 1.0;
    }
    let tags = normalized_tags(item);
    let include_norm: Vec<String> = include_tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    if include_norm.is_empty() {
        return 1.0;
    }
    let matched = include_norm
        .iter()
        .filter(|needle| tags.iter().any(|t| t == *needle))
        .count();
    matched as f64 / include_norm.len() as f64
}

pub fn policy_block_reason(
    item: &UnifiedNewsItem,
    channel: &str,
    cfg: &PublisherConfig,
) -> Option<String> {
    let map = &item.syndication.distribution_policy.channel_policy;
    let p = map.get(channel).or_else(|| {
        map.iter()
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(channel))
            .map(|(_, v)| v)
    })?;
    if p.enabled == Some(false) {
        return Some("policy_disabled".to_string());
    }
    if let Some(filters) = p.topic_filters.as_ref() {
        let tags = normalized_tags(item);
        let include_norm: Vec<String> = filters
            .include_tags
            .iter()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if !include_norm.is_empty()
            && !include_norm
                .iter()
                .any(|needle| tags.iter().any(|t| t == needle))
        {
            return Some("topic_filtered_out".to_string());
        }
        let exclude_norm: Vec<String> = filters
            .exclude_tags
            .iter()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if exclude_norm
            .iter()
            .any(|needle| tags.iter().any(|t| t == needle))
        {
            return Some("topic_excluded".to_string());
        }
        if let Some(min) = filters.min_topic_score {
            let score = topic_score(item, &filters.include_tags);
            if score < min {
                return Some(format!("topic_score_below_min:{score:.3}<{min:.3}"));
            }
        }
    }
    if let Some(min_floor) = p.worthiness_floor {
        if let Some(actual) = cfg.worthiness_score {
            if actual < min_floor {
                return Some(format!("worthiness_below_floor:{actual:.3}<{min_floor:.3}"));
            }
        } else {
            return Some("worthiness_unavailable".to_string());
        }
    }
    None
}
