//! Gamify (Ludus) wire DTOs for the dev-only Axis ops (plan 07 Task 11). Field names are part
//! of the Axis UI contract — do NOT rename.
use serde::{Deserialize, Serialize};
use vox_gamify::profile::LudusProfile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LudusProfileDto {
    pub user_id: String,
    pub level: u64,
    pub xp: u64,
    pub xp_to_next_level: u64,
    pub xp_progress: f64,
    pub total_xp_earned: u64,
    pub crystals: u64,
    pub lumens: i64,
    pub energy: u64,
    pub max_energy: u64,
    pub current_streak: u64,
    pub prestige_level: u32,
    pub title: String,
    pub full_title: String,
    pub trust_tier: String,
}

impl LudusProfileDto {
    pub fn from_profile(p: &LudusProfile) -> Self {
        Self {
            user_id: p.user_id.clone(),
            level: p.level,
            xp: p.xp,
            xp_to_next_level: p.xp_to_next_level(),
            xp_progress: p.xp_progress(),
            total_xp_earned: p.total_xp_earned,
            crystals: p.crystals,
            lumens: p.lumens,
            energy: p.energy,
            max_energy: p.max_energy,
            current_streak: p.streak.current_streak,
            prestige_level: p.prestige_level,
            title: p.title(),
            full_title: p.full_title(),
            trust_tier: format!("{:?}", p.trust_tier),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DueActionDto {
    pub action_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiEventResultDto {
    pub xp_granted: u32,
    pub lumens_granted: u32,
    pub achievement_title: Option<String>,
}

impl GuiEventResultDto {
    pub fn from_route(result: &vox_gamify::reward_policy::RouteResult) -> Self {
        let xp = result.reward.as_ref().map(|r| r.xp).unwrap_or(0);
        let lumens = result
            .reward
            .as_ref()
            .map(|r| r.lumens.max(0) as u32)
            .unwrap_or(0);
        let achievement_title = if let Some((_, title)) = &result.leveled_up {
            Some(title.clone())
        } else if xp > 0 {
            Some("XP".to_string())
        } else if lumens > 0 {
            Some("Lumens".to_string())
        } else {
            None
        };
        Self {
            xp_granted: xp.min(u32::MAX as u64) as u32,
            lumens_granted: lumens,
            achievement_title,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntryDto {
    pub rank: u32,
    pub user_id: String,
    pub level: u64,
    pub score: i64,
    #[serde(default)]
    pub rank_delta: i32,
    #[serde(default)]
    pub tier_title: String,
    #[serde(default)]
    pub lumens: i64,
    #[serde(default)]
    pub row_chain_hash_hex: String,
    #[serde(default)]
    pub proof_digest_sha3_256: String,
    #[serde(default)]
    pub signature_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanionDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub language: String,
    pub mood: String,
    pub health: i32,
    pub max_health: i32,
    pub energy: i32,
    pub max_energy: i32,
    pub code_quality: u8,
    pub last_active: i64,
    /// Inline SVG markup (no external runtime) rendered from the companion's
    /// current mood via `sprite_svg::generate_svg_from_mood`.
    pub svg: String,
}

impl CompanionDto {
    pub fn from_companion(c: &vox_gamify::companion::Companion) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            language: c.language.clone(),
            mood: format!("{:?}", c.mood),
            health: c.health,
            max_health: c.max_health,
            energy: c.energy,
            max_energy: c.max_energy,
            code_quality: c.code_quality,
            last_active: c.last_active,
            svg: vox_gamify::sprite_svg::generate_svg_from_mood(c.mood, None).svg_body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_gamify::reward_policy::{PolicyReward, RouteResult};

    fn reward(xp: u64) -> Option<PolicyReward> {
        Some(PolicyReward {
            xp,
            crystals: 0,
            lumens: 0,
            grant_shield: false,
            effective_multiplier: 1.0,
            grind_capped: false,
        })
    }

    #[test]
    fn profile_dto_maps_core_fields_and_bounds_progress() {
        let mut p = LudusProfile::new_default("u1");
        p.level = 3;
        p.xp = 200;
        p.crystals = 42;
        let dto = LudusProfileDto::from_profile(&p);
        assert_eq!(dto.user_id, "u1");
        assert_eq!(dto.level, 3);
        assert_eq!(dto.crystals, 42);
        assert!(dto.xp_progress >= 0.0 && dto.xp_progress <= 1.0);
        assert!(!dto.title.is_empty());
    }

    #[test]
    fn event_result_maps_xp_and_level_up_title() {
        let leveled = GuiEventResultDto::from_route(&RouteResult {
            reward: reward(120),
            leveled_up: Some((5, "Initiate".to_string())),
        });
        assert_eq!(leveled.xp_granted, 120);
        assert_eq!(leveled.achievement_title.as_deref(), Some("Initiate"));

        let xp_only = GuiEventResultDto::from_route(&RouteResult {
            reward: reward(5),
            leveled_up: None,
        });
        assert_eq!(xp_only.achievement_title.as_deref(), Some("XP"));

        let wire = serde_json::to_value(&xp_only).expect("json");
        assert_eq!(wire["xpGranted"], 5);
        let back: GuiEventResultDto = serde_json::from_value(wire).expect("round trip");
        assert_eq!(back, xp_only);
    }
}
