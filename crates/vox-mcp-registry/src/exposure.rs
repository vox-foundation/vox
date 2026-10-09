//! Exposure class and side-effect class of an MCP tool (capability profiles).

use serde::{Deserialize, Serialize};

/// Who may be served a tool. Ordered `Public < Operator < Internal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exposure {
    /// Read-only, or writes only the owning subsystem's own state.
    Public,
    /// Mutates the workspace, repo, remote services, or other agents.
    Operator,
    /// Never exposed through a named profile; loopback `dev` only.
    Internal,
}

impl Exposure {
    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Exposure::Public => "public",
            Exposure::Operator => "operator",
            Exposure::Internal => "internal",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Exposure::Public),
            "operator" => Some(Exposure::Operator),
            "internal" => Some(Exposure::Internal),
            _ => None,
        }
    }
}

/// Strongest effect any code path of a tool handler can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    /// Pure computation over compiled-in data.
    None,
    /// Reads files, rows, or process state.
    ReadOnly,
    /// Local persistent or in-process state mutation (files, DB rows, queues, sessions).
    WritesFiles,
    /// Starts an OS process or agent worker.
    SpawnsProcess,
    /// Outbound network or inference without remote mutation.
    Network,
    /// Mutates state in a remote service or another host.
    RemoteMutation,
    /// Irreversibly deletes or overwrites user data.
    Destructive,
}

impl SideEffectClass {
    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            SideEffectClass::None => "none",
            SideEffectClass::ReadOnly => "read_only",
            SideEffectClass::WritesFiles => "writes_files",
            SideEffectClass::SpawnsProcess => "spawns_process",
            SideEffectClass::Network => "network",
            SideEffectClass::RemoteMutation => "remote_mutation",
            SideEffectClass::Destructive => "destructive",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(SideEffectClass::None),
            "read_only" => Some(SideEffectClass::ReadOnly),
            "writes_files" => Some(SideEffectClass::WritesFiles),
            "spawns_process" => Some(SideEffectClass::SpawnsProcess),
            "network" => Some(SideEffectClass::Network),
            "remote_mutation" => Some(SideEffectClass::RemoteMutation),
            "destructive" => Some(SideEffectClass::Destructive),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPOSURES: [Exposure; 3] = [Exposure::Public, Exposure::Operator, Exposure::Internal];
    const SIDE_EFFECTS: [SideEffectClass; 7] = [
        SideEffectClass::None,
        SideEffectClass::ReadOnly,
        SideEffectClass::WritesFiles,
        SideEffectClass::SpawnsProcess,
        SideEffectClass::Network,
        SideEffectClass::RemoteMutation,
        SideEffectClass::Destructive,
    ];

    #[test]
    fn exposure_round_trips_through_str_and_serde() {
        for e in EXPOSURES {
            assert_eq!(Exposure::parse(e.as_str()), Some(e));
            assert_eq!(
                serde_json::to_string(&e).unwrap(),
                format!("\"{}\"", e.as_str())
            );
        }
    }

    #[test]
    fn exposure_orders_public_operator_internal() {
        assert!(Exposure::Public < Exposure::Operator);
        assert!(Exposure::Operator < Exposure::Internal);
    }

    #[test]
    fn exposure_parse_rejects_unknown_and_non_canonical() {
        for s in ["", "Public", "core", "admin", "dev"] {
            assert_eq!(Exposure::parse(s), None, "{s}");
        }
    }

    #[test]
    fn side_effect_round_trips_through_str_and_serde() {
        for c in SIDE_EFFECTS {
            assert_eq!(SideEffectClass::parse(c.as_str()), Some(c));
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.as_str())
            );
        }
    }

    #[test]
    fn side_effect_strings_are_the_catalog_vocabulary() {
        let got: Vec<&str> = SIDE_EFFECTS.iter().map(|c| c.as_str()).collect();
        assert_eq!(
            got,
            [
                "none",
                "read_only",
                "writes_files",
                "spawns_process",
                "network",
                "remote_mutation",
                "destructive"
            ]
        );
        for s in ["mixed", "unknown", "ReadOnly", "read-only"] {
            assert_eq!(SideEffectClass::parse(s), None, "{s}");
        }
    }
}
