//! Typed Remote Axis errors. Tauri commands return them as the JSON envelope
//! `{"error":{"code","message",...}}` (the same shape as MCP tool errors); local daemon errors
//! pass through as their original text so local mode is unchanged.

use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisError {
    /// Local daemon RPC failed after the daemon was reached.
    Daemon(String),
    /// Local daemon could not be adopted or spawned.
    DaemonUnavailable(String),
    LocalOnly {
        command: String,
        module: String,
        class: &'static str,
    },
    NotRemoteCapable {
        method: String,
    },
    VersionMismatch {
        server: String,
        gui: String,
    },
    TlsPinRequired {
        fingerprint: String,
    },
    TlsPinMismatch {
        pinned: String,
        fingerprint: String,
    },
    Unauthorized(String),
    Unreachable(String),
    InvalidUrl(String),
    InsecureUrl(String),
    Remote {
        code: String,
        message: String,
    },
}

impl AxisError {
    pub fn code(&self) -> &str {
        match self {
            Self::Daemon(_) => "daemon_error",
            Self::DaemonUnavailable(_) => "daemon_unavailable",
            Self::LocalOnly { .. } => "local_only",
            Self::NotRemoteCapable { .. } => "not_remote_capable",
            Self::VersionMismatch { .. } => "version_mismatch",
            Self::TlsPinRequired { .. } => "tls_pin_required",
            Self::TlsPinMismatch { .. } => "tls_pin_mismatch",
            Self::Unauthorized(_) => "unauthorized",
            Self::Unreachable(_) => "unreachable",
            Self::InvalidUrl(_) => "invalid_url",
            Self::InsecureUrl(_) => "insecure_url",
            Self::Remote { code, .. } => code,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::LocalOnly {
                command,
                module,
                class,
            } => format!(
                "{command} needs this machine's local Vox ({class}-class module `{module}`); disconnect from the server to use it"
            ),
            Self::NotRemoteCapable { method } => {
                format!("daemon method {method} has no remote route yet")
            }
            Self::VersionMismatch { server, gui } => {
                format!("server runs Vox {server}, Axis is {gui}; major.minor must match")
            }
            Self::TlsPinRequired { .. } => {
                "first connection to this server: confirm its certificate fingerprint".to_string()
            }
            Self::TlsPinMismatch { .. } => {
                "the server certificate changed since it was pinned".to_string()
            }
            Self::Daemon(m)
            | Self::DaemonUnavailable(m)
            | Self::Unauthorized(m)
            | Self::Unreachable(m)
            | Self::InvalidUrl(m)
            | Self::InsecureUrl(m) => m.clone(),
            Self::Remote { message, .. } => message.clone(),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut err = json!({ "code": self.code(), "message": self.message() });
        match self {
            Self::LocalOnly {
                command,
                module,
                class,
            } => {
                err["command"] = json!(command);
                err["module"] = json!(module);
                err["class"] = json!(class);
            }
            Self::NotRemoteCapable { method } => err["method"] = json!(method),
            Self::VersionMismatch { server, gui } => {
                err["server"] = json!(server);
                err["gui"] = json!(gui);
            }
            Self::TlsPinRequired { fingerprint } => err["fingerprint"] = json!(fingerprint),
            Self::TlsPinMismatch {
                pinned,
                fingerprint,
            } => {
                err["pinned"] = json!(pinned);
                err["fingerprint"] = json!(fingerprint);
            }
            _ => {}
        }
        json!({ "error": err })
    }
}

impl From<AxisError> for String {
    fn from(err: AxisError) -> String {
        match err {
            AxisError::Daemon(m) | AxisError::DaemonUnavailable(m) => m,
            other => other.to_json().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_daemon_errors_pass_through_unchanged() {
        assert_eq!(
            String::from(AxisError::Daemon("conn refused".into())),
            "conn refused"
        );
        assert_eq!(
            String::from(AxisError::DaemonUnavailable("spawn failed".into())),
            "spawn failed"
        );
    }

    #[test]
    fn remote_errors_serialize_as_the_error_envelope() {
        let s = String::from(AxisError::LocalOnly {
            command: "pty_spawn".into(),
            module: "pty".into(),
            class: "host",
        });
        let v: Value = serde_json::from_str(&s).expect("json");
        assert_eq!(v["error"]["code"], "local_only");
        assert_eq!(v["error"]["command"], "pty_spawn");
        assert_eq!(v["error"]["module"], "pty");
        assert_eq!(v["error"]["class"], "host");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("disconnect")
        );
    }

    #[test]
    fn pin_errors_carry_fingerprints() {
        let v = AxisError::TlsPinMismatch {
            pinned: "aa".into(),
            fingerprint: "bb".into(),
        }
        .to_json();
        assert_eq!(v["error"]["code"], "tls_pin_mismatch");
        assert_eq!(v["error"]["pinned"], "aa");
        assert_eq!(v["error"]["fingerprint"], "bb");
        let v = AxisError::TlsPinRequired {
            fingerprint: "cc".into(),
        }
        .to_json();
        assert_eq!(v["error"]["code"], "tls_pin_required");
        assert_eq!(v["error"]["fingerprint"], "cc");
    }

    #[test]
    fn codes_are_the_contract_codes() {
        assert_eq!(
            AxisError::NotRemoteCapable {
                method: "orch.x".into()
            }
            .code(),
            "not_remote_capable"
        );
        assert_eq!(
            AxisError::VersionMismatch {
                server: "0.7.0".into(),
                gui: "0.6.0".into()
            }
            .code(),
            "version_mismatch"
        );
        assert_eq!(AxisError::Unauthorized("x".into()).code(), "unauthorized");
        assert_eq!(AxisError::Unreachable("x".into()).code(), "unreachable");
        assert_eq!(AxisError::InvalidUrl("x".into()).code(), "invalid_url");
        assert_eq!(AxisError::InsecureUrl("x".into()).code(), "insecure_url");
        assert_eq!(
            AxisError::Remote {
                code: "tool_not_in_profile".into(),
                message: "m".into()
            }
            .code(),
            "tool_not_in_profile"
        );
    }
}
