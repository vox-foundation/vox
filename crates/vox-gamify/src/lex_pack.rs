//! TOML-backed extensibility for custom gamification packs (Lex Packs).
//!
//! Custom packs can define new glyphs (collectible achievements) and
//! Lumen weights for specific event types.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A custom gamification pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexPack {
    /// Unique identifier for the pack.
    pub id: String,
    /// Human-readable name of the pack.
    pub name: String,
    /// Semantic version of the pack.
    pub version: String,
    /// Optional long description of the pack.
    pub description: Option<String>,
    /// Custom collectible glyphs included in this pack.
    #[serde(default)]
    pub glyphs: Vec<LexGlyph>,
    /// Custom Lumen reward adjustments.
    #[serde(default)]
    pub lumens_weights: Vec<LumensWeight>,
}

/// A collectible badge or glyph defined in a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexGlyph {
    /// Unique identifier for the glyph.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of how to unlock.
    pub description: String,
    /// Unicode or SVG icon.
    pub icon: String,
    /// Event type that triggers progress for this glyph.
    pub trigger_event: String,
    /// Number of times the event must trigger to unlock.
    pub trigger_count: u32,
    /// XP bonus awarded on unlock.
    pub xp_reward: u64,
    /// Crystal bonus awarded on unlock.
    pub crystal_reward: u64,
}

/// Custom Lumen reward adjustment for an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LumensWeight {
    /// The event type slug to adjust.
    pub event_type: String,
    /// The fixed Lumen change when this event fires.
    pub lumens_delta: i64,
}

/// Load a Lex Pack from a TOML file.
pub fn load_lex_pack<P: AsRef<Path>>(path: P) -> Result<LexPack> {
    LexPack::load_from_file(path)
}

/// Save a Lex Pack to a TOML file.
pub fn save_lex_pack<P: AsRef<Path>>(pack: &LexPack, path: P) -> Result<()> {
    pack.save_to_file(path)
}

impl LexPack {
    /// Load a Lex Pack from a TOML file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let pack: LexPack = toml::from_str(&content)?;
        Ok(pack)
    }

    /// Save a Lex Pack to a TOML file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_pack_serde() {
        let pack = LexPack {
            id: "test-pack".to_string(),
            name: "Test Pack".to_string(),
            version: "0.1.0".to_string(),
            description: Some("A test pack".to_string()),
            glyphs: vec![LexGlyph {
                id: "g1".to_string(),
                name: "G1".to_string(),
                description: "D1".to_string(),
                icon: "✦".to_string(),
                trigger_event: "test".to_string(),
                trigger_count: 5,
                xp_reward: 100,
                crystal_reward: 10,
            }],
            lumens_weights: vec![LumensWeight {
                event_type: "test".to_string(),
                lumens_delta: 5,
            }],
        };

        let toml_str = toml::to_string_pretty(&pack).unwrap();
        let decoded: LexPack = toml::from_str(&toml_str).unwrap();
        assert_eq!(decoded.id, "test-pack");
        assert_eq!(decoded.lumens_weights[0].lumens_delta, 5);
    }
}
