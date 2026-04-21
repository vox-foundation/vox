//! SSOT for task category to model routing semantics

use super::generated::{ModelTier, StrengthTag, TaskCategory};

/// A routing rule defining how a task category maps to model capabilities.
#[derive(Debug, Clone)]
pub struct RoutingTableEntry {
    pub task_category: TaskCategory,
    pub strength_tag: StrengthTag,
    pub premium_alias_key: &'static str,
    pub preferred_tier: ModelTier,
    /// Minimum context window in thousands of tokens.
    pub context_min_k: u32,
}

pub const ROUTING_TABLE: &[RoutingTableEntry] = &[
    RoutingTableEntry {
        task_category: TaskCategory::CodeGen,
        strength_tag: StrengthTag::Codegen,
        premium_alias_key: "codegen",
        preferred_tier: ModelTier::Elite,
        context_min_k: 100,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Testing,
        strength_tag: StrengthTag::Codegen,
        premium_alias_key: "testing",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Debugging,
        strength_tag: StrengthTag::Debugging,
        premium_alias_key: "debugging",
        preferred_tier: ModelTier::Elite,
        context_min_k: 100,
    },
    RoutingTableEntry {
        task_category: TaskCategory::TypeChecking,
        strength_tag: StrengthTag::Logic,
        premium_alias_key: "logic",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Research,
        strength_tag: StrengthTag::Research,
        premium_alias_key: "research",
        preferred_tier: ModelTier::Pro,
        context_min_k: 128,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Parsing,
        strength_tag: StrengthTag::Parsing,
        premium_alias_key: "parsing",
        preferred_tier: ModelTier::Light,
        context_min_k: 8,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Review,
        strength_tag: StrengthTag::Review,
        premium_alias_key: "review",
        preferred_tier: ModelTier::Pro,
        context_min_k: 100,
    },
    RoutingTableEntry {
        task_category: TaskCategory::General,
        strength_tag: StrengthTag::Logic,
        premium_alias_key: "logic",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Ars,
        strength_tag: StrengthTag::Logic,
        premium_alias_key: "logic",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Planning,
        strength_tag: StrengthTag::Planning,
        premium_alias_key: "planning",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::InterAgent,
        strength_tag: StrengthTag::InterAgent,
        premium_alias_key: "inter_agent",
        preferred_tier: ModelTier::Light,
        context_min_k: 8,
    },
    RoutingTableEntry {
        task_category: TaskCategory::ToolOrchestration,
        strength_tag: StrengthTag::Logic,
        premium_alias_key: "logic",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
    RoutingTableEntry {
        task_category: TaskCategory::Visus,
        strength_tag: StrengthTag::Visus,
        premium_alias_key: "visus",
        preferred_tier: ModelTier::Pro,
        context_min_k: 32,
    },
];

/// Get the routing config for a category.
#[must_use]
pub fn route_for_category(cat: TaskCategory) -> &'static RoutingTableEntry {
    ROUTING_TABLE
        .iter()
        .find(|r| r.task_category == cat)
        .unwrap_or(&ROUTING_TABLE[7]) // Default General
}
