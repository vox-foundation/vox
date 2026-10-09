//! Embedded `contracts/gui/command-backends.v1.yaml` + `surface-registry.v1.yaml`, and the
//! per-surface availability rule for Remote Axis.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const COMMAND_BACKENDS_YAML: &str =
    include_str!("../../../../contracts/gui/command-backends.v1.yaml");
const SURFACE_REGISTRY_YAML: &str =
    include_str!("../../../../contracts/gui/surface-registry.v1.yaml");

/// Daemon-class modules whose every daemon method has a `RemoteRoute` (see `gateway_client::remote_route`).
pub const REMOTE_ROUTED_DAEMON_MODULES: &[&str] =
    &["daemon", "event_annotate", "mcp", "vcs_isolation"];
/// Daemon-class modules with at least one method the gateway does not expose yet.
pub const REMOTE_UNROUTED_DAEMON_MODULES: &[&str] = &["control_plane", "mission_control"];
/// Surfaces that configure this Axis install itself; available in both modes.
pub const AXIS_LOCAL_SURFACES: &[&str] = &["settings", "coverage"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleClass {
    Daemon,
    Db,
    Host,
    Shell,
}

impl ModuleClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Db => "db",
            Self::Host => "host",
            Self::Shell => "shell",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandBackendRow {
    pub module: String,
    pub class: ModuleClass,
    #[serde(default)]
    pub surfaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandBackends {
    pub commands: Vec<CommandBackendRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotInProfile,
    NeedsLocalMachine,
    NotRemoteCapable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfaceAvailability {
    pub surface: String,
    pub available: bool,
    pub reason: Option<UnavailableReason>,
}

#[derive(Deserialize)]
struct SurfaceRegistry {
    surfaces: Vec<SurfaceRegistryRow>,
}

#[derive(Deserialize)]
struct SurfaceRegistryRow {
    view_key: Option<String>,
}

pub fn command_backends() -> &'static CommandBackends {
    static PARSED: OnceLock<CommandBackends> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_yaml::from_str(COMMAND_BACKENDS_YAML).expect(
            "contracts/gui/command-backends.v1.yaml parses (enforced by vox ci gui-catalog-parity)",
        )
    })
}

pub fn module_class(module: &str) -> Option<ModuleClass> {
    command_backends()
        .commands
        .iter()
        .find(|r| r.module == module)
        .map(|r| r.class)
}

pub fn registry_surfaces() -> &'static [String] {
    static PARSED: OnceLock<Vec<String>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let registry: SurfaceRegistry = serde_yaml::from_str(SURFACE_REGISTRY_YAML)
            .expect("contracts/gui/surface-registry.v1.yaml parses (enforced by vox ci gui-surface-registry)");
        registry
            .surfaces
            .into_iter()
            .filter_map(|r| r.view_key)
            .filter(|k| !k.is_empty())
            .collect()
    })
}

pub fn surface_availability(
    backends: &CommandBackends,
    surfaces: &[String],
    remote_axis_surfaces: Option<&[String]>,
) -> Vec<SurfaceAvailability> {
    surfaces
        .iter()
        .map(|surface| {
            let reason =
                remote_axis_surfaces.and_then(|served| remote_reason(backends, surface, served));
            SurfaceAvailability {
                surface: surface.clone(),
                available: reason.is_none(),
                reason,
            }
        })
        .collect()
}

fn remote_reason(
    backends: &CommandBackends,
    surface: &str,
    served: &[String],
) -> Option<UnavailableReason> {
    if AXIS_LOCAL_SURFACES.contains(&surface) {
        return None;
    }
    if !served.iter().any(|s| s == surface || s == "*") {
        return Some(UnavailableReason::NotInProfile);
    }
    let rows = || {
        backends
            .commands
            .iter()
            .filter(|r| r.surfaces.iter().any(|s| s == surface))
    };
    if rows().any(|r| r.class == ModuleClass::Host) {
        return Some(UnavailableReason::NeedsLocalMachine);
    }
    let not_remote_capable = rows().any(|r| match r.class {
        ModuleClass::Db => true,
        ModuleClass::Daemon => !REMOTE_ROUTED_DAEMON_MODULES.contains(&r.module.as_str()),
        ModuleClass::Host | ModuleClass::Shell => false,
    });
    not_remote_capable.then_some(UnavailableReason::NotRemoteCapable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(module: &str, class: ModuleClass, surfaces: &[&str]) -> CommandBackendRow {
        CommandBackendRow {
            module: module.into(),
            class,
            surfaces: surfaces.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn reason_of(out: &[SurfaceAvailability], surface: &str) -> Option<UnavailableReason> {
        out.iter()
            .find(|a| a.surface == surface)
            .expect("surface listed")
            .reason
    }

    #[test]
    fn local_mode_makes_every_surface_available() {
        let b = CommandBackends {
            commands: vec![row("pty", ModuleClass::Host, &["console"])],
        };
        let out = surface_availability(&b, &strings(&["console", "mesh"]), None);
        assert!(out.iter().all(|a| a.available && a.reason.is_none()));
    }

    #[test]
    fn remote_mode_reasons_follow_the_most_restrictive_module() {
        let b = CommandBackends {
            commands: vec![
                row("mcp", ModuleClass::Daemon, &["mesh", "tasks"]),
                row("control_plane", ModuleClass::Daemon, &["tasks"]),
                row("models", ModuleClass::Db, &["models"]),
                row("pty", ModuleClass::Host, &["console"]),
                row("preferences", ModuleClass::Shell, &["memory"]),
            ],
        };
        let surfaces = strings(&[
            "console", "mesh", "models", "tasks", "memory", "runs", "settings",
        ]);
        let profile = strings(&["console", "mesh", "models", "tasks", "memory"]);
        let out = surface_availability(&b, &surfaces, Some(&profile));
        assert_eq!(reason_of(&out, "mesh"), None);
        assert_eq!(reason_of(&out, "memory"), None);
        assert_eq!(
            reason_of(&out, "console"),
            Some(UnavailableReason::NeedsLocalMachine)
        );
        assert_eq!(
            reason_of(&out, "models"),
            Some(UnavailableReason::NotRemoteCapable)
        );
        assert_eq!(
            reason_of(&out, "tasks"),
            Some(UnavailableReason::NotRemoteCapable)
        );
        assert_eq!(
            reason_of(&out, "runs"),
            Some(UnavailableReason::NotInProfile)
        );
        assert_eq!(
            reason_of(&out, "settings"),
            None,
            "Axis-local surfaces are always available"
        );
    }

    /// `dev` serves `axis_surfaces: ["*"]` (00-contracts Amendments "From plan 02"): every surface is in profile.
    #[test]
    fn wildcard_axis_surfaces_put_every_surface_in_profile() {
        let out = surface_availability(
            command_backends(),
            registry_surfaces(),
            Some(&strings(&["*"])),
        );
        assert!(
            out.iter()
                .all(|a| a.reason != Some(UnavailableReason::NotInProfile)),
            "{out:?}"
        );
        assert_eq!(reason_of(&out, "mesh"), None);
    }

    #[test]
    fn embedded_manifest_parses_and_covers_axis_remote() {
        assert_eq!(module_class("axis_remote"), Some(ModuleClass::Shell));
        assert_eq!(module_class("pty"), Some(ModuleClass::Host));
        assert_eq!(module_class("mcp"), Some(ModuleClass::Daemon));
        assert_eq!(module_class("no_such_module"), None);
        assert!(registry_surfaces().iter().any(|s| s == "mesh"));
        assert!(!registry_surfaces().iter().any(|s| s.is_empty()));
    }

    #[test]
    fn every_daemon_module_has_a_routing_decision() {
        for row in &command_backends().commands {
            if row.class == ModuleClass::Daemon {
                let decided = REMOTE_ROUTED_DAEMON_MODULES.contains(&row.module.as_str())
                    || REMOTE_UNROUTED_DAEMON_MODULES.contains(&row.module.as_str());
                assert!(
                    decided,
                    "daemon module `{}` needs a remote routing decision",
                    row.module
                );
            }
        }
    }

    #[test]
    fn real_inventory_gates_known_surfaces() {
        let profile = strings(&["mesh", "repository", "models", "console", "vox-search"]);
        let out = surface_availability(command_backends(), registry_surfaces(), Some(&profile));
        assert_eq!(reason_of(&out, "mesh"), None);
        assert_eq!(reason_of(&out, "repository"), None);
        assert_eq!(reason_of(&out, "vox-search"), None);
        assert_eq!(
            reason_of(&out, "models"),
            Some(UnavailableReason::NotRemoteCapable)
        );
        assert_eq!(
            reason_of(&out, "console"),
            Some(UnavailableReason::NeedsLocalMachine)
        );
        assert_eq!(
            reason_of(&out, "chat"),
            Some(UnavailableReason::NotInProfile)
        );
    }
}
