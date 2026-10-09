//! Models-surface wire DTOs (moved from `vox-gui/src/commands/models.rs`, plan 07).

use serde::{Deserialize, Serialize};
use vox_config::AutoRoutingPriority;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPriorityDto {
    pub efficiency: u8,
    pub precision: u8,
    pub latency: u8,
    pub availability: u8,
    pub balance: u8,
    pub mobile: u8,
}

impl From<AutoRoutingPriority> for RoutingPriorityDto {
    fn from(p: AutoRoutingPriority) -> Self {
        Self {
            efficiency: p.efficiency,
            precision: p.precision,
            latency: p.latency,
            availability: p.availability,
            balance: p.balance,
            mobile: p.mobile,
        }
    }
}

/// Parse a `VOX_AUTO_ROUTING_PRIORITY`-style CSV (`efficiency=25,precision=30,...`)
/// onto a base priority, leaving any axes not present in the CSV untouched.
pub fn apply_routing_csv(mut base: RoutingPriorityDto, csv: &str) -> RoutingPriorityDto {
    for part in csv.split(',') {
        let mut it = part.splitn(2, '=');
        let key = it.next().map(str::trim).unwrap_or("").to_ascii_lowercase();
        let Ok(parsed) = it.next().map(str::trim).unwrap_or("").parse::<u8>() else {
            continue;
        };
        match key.as_str() {
            "efficiency" | "cost" => base.efficiency = parsed,
            "precision" | "quality" => base.precision = parsed,
            "latency" | "speed" => base.latency = parsed,
            "availability" => base.availability = parsed,
            "balance" => base.balance = parsed,
            "mobile" => base.mobile = parsed,
            _ => {}
        }
    }
    base
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsPreferencesDto {
    pub active_model: Option<String>,
    /// Effective value: server env (`AutoRoutingPriority::from_env`) overlaid with the persisted CSV.
    pub routing_priority: RoutingPriorityDto,
    /// Persisted `SelectionPolicy` JSON, or `{"steps":[]}` when absent.
    pub selection_policy: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> RoutingPriorityDto {
        RoutingPriorityDto {
            efficiency: 1,
            precision: 2,
            latency: 3,
            availability: 4,
            balance: 5,
            mobile: 6,
        }
    }

    #[test]
    fn apply_routing_csv_overlays_known_axes_and_aliases_only() {
        let p = apply_routing_csv(base(), "cost=40, quality=30,bogus=9,latency=x,mobile=7");
        assert_eq!(p.efficiency, 40);
        assert_eq!(p.precision, 30);
        assert_eq!(p.latency, 3);
        assert_eq!(p.mobile, 7);
        assert_eq!(p.balance, 5);
    }

    #[test]
    fn preferences_dto_round_trips() {
        let dto = ModelsPreferencesDto {
            active_model: Some("m".into()),
            routing_priority: base(),
            selection_policy: "{}".into(),
        };
        let back: ModelsPreferencesDto =
            serde_json::from_value(serde_json::to_value(&dto).unwrap()).unwrap();
        assert_eq!(back.routing_priority, base());
        assert_eq!(back.active_model.as_deref(), Some("m"));
    }
}
