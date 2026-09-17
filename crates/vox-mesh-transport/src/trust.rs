//! Allowlist of trusted `EndpointId`s in `~/.vox/mesh_trust.json`.
//!
//! NOT `trusted_nodes.json`: that is keyed by `node_id` from a different
//! keyspace and its `pubkey_hex` can be empty. Overloading it would put two
//! identifier spaces in one file and make `untrust` ambiguous.
//!
//! Two properties this module exists to guarantee:
//!
//! - **Pairing grants [`TrustLevel::Sandboxed`], never [`TrustLevel::Native`].**
//!   Consuming a ticket is the highest-privilege action in the system; it must
//!   not also be the action that grants native code execution.
//! - **`untrust` closes live connections.** iroh has no endpoint-level "close
//!   everything to this peer", so this store holds the handles. Without that,
//!   revocation is a file write and nothing else — the attacker keeps the
//!   connection they already have.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use anyhow::{Context as _, Result};
use iroh::EndpointId;
use iroh::endpoint::Connection;
use serde::{Deserialize, Serialize};

/// What a trusted peer is allowed to do with work it sends us.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// The default, and what pairing grants. Received work runs sandboxed.
    Sandboxed,
    /// Native execution. Only ever set deliberately, never by pairing.
    Native,
}

/// One row of the on-disk allowlist.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustedEndpoint {
    pub endpoint_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub level: TrustLevel,
    /// Last-known socket addresses, captured from the ticket at pairing time.
    ///
    /// Required, not an optimisation: mDNS discovery does not work (see the
    /// spike findings, Q4), so an `EndpointId` alone is not dialable. Without
    /// these the peer directory can never reach anybody.
    #[serde(default)]
    pub addrs: Vec<String>,
}

/// The allowlist plus the live connections it can revoke.
#[derive(Debug)]
pub struct MeshTrust {
    path: PathBuf,
    mutation: Mutex<()>,
    live: Mutex<HashMap<EndpointId, Vec<(u64, Connection)>>>,
    next_registration: AtomicU64,
}

pub struct LiveRegistration {
    trust: Arc<MeshTrust>,
    id: EndpointId,
    token: u64,
}

impl Drop for LiveRegistration {
    fn drop(&mut self) {
        self.trust.unregister(self.id, self.token);
    }
}

impl MeshTrust {
    /// Open the store backed by `path`. Does not read it — every check reads
    /// from disk so an out-of-band edit or a second process takes effect.
    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            mutation: Mutex::new(()),
            live: Mutex::new(HashMap::new()),
            next_registration: AtomicU64::new(1),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the allowlist. **Fails closed**: an unreadable store yields an
    /// empty allowlist and an `error!`, so a corrupt file refuses everyone
    /// rather than admitting everyone.
    fn read(&self) -> Vec<TrustedEndpoint> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => match serde_json::from_str::<Vec<TrustedEndpoint>>(&s) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(
                        path = %self.path.display(),
                        error = %e,
                        "mesh trust store is unparseable; refusing every peer until it is fixed"
                    );
                    Vec::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                tracing::error!(
                    path = %self.path.display(),
                    error = %e,
                    "mesh trust store is unreadable; refusing every peer"
                );
                Vec::new()
            }
        }
    }

    /// Replace the allowlist atomically.
    ///
    /// `fs::write` truncates and then writes; a crash in between leaves a
    /// zero-byte file, which disables the whole mesh with a parse error nobody
    /// connects to the reboot. Temp-plus-rename cannot do that.
    fn write(&self, rows: &[TrustedEndpoint]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(rows).context("serializing mesh trust store")?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json.as_bytes())
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("renaming {} to {}", tmp.display(), self.path.display()))?;
        Ok(())
    }

    /// Every trusted peer, for `vox mesh list`.
    ///
    /// Returns owned rows rather than a borrow: the store is re-read from disk
    /// on every call so an out-of-band edit or a second process takes effect.
    pub fn rows(&self) -> Vec<TrustedEndpoint> {
        self.read()
    }

    /// The level `id` is trusted at, or `None` if it is not trusted.
    pub fn level(&self, id: &EndpointId) -> Option<TrustLevel> {
        let want = id.to_string();
        self.read()
            .into_iter()
            .find(|r| r.endpoint_id == want)
            .map(|r| r.level)
    }

    pub fn is_trusted(&self, id: &EndpointId) -> bool {
        self.level(id).is_some()
    }

    /// Trust `id` at [`TrustLevel::Sandboxed`]. This is what pairing calls.
    ///
    /// There is deliberately no `level` parameter: a caller that could pass
    /// `Native` here is one refactor away from pairing granting it.
    pub fn trust(&self, id: &EndpointId, label: Option<&str>) -> Result<()> {
        self.upsert(id, label, TrustLevel::Sandboxed, &[])
    }

    /// Trust `id`, recording the addresses it can be reached on.
    ///
    /// Pairing is the only moment these are known — the ticket carries them —
    /// so this is what `vox mesh join <ticket>` calls.
    pub fn trust_with_addrs(
        &self,
        id: &EndpointId,
        label: Option<&str>,
        addrs: &[std::net::SocketAddr],
    ) -> Result<()> {
        self.upsert(id, label, TrustLevel::Sandboxed, addrs)
    }

    /// Promote `id` to native execution. Never reachable from pairing.
    pub fn grant_native(&self, id: &EndpointId, label: Option<&str>) -> Result<()> {
        self.upsert(id, label, TrustLevel::Native, &[])
    }

    /// Trust `id` at an explicit level. Pairing still goes through [`Self::trust`].
    pub fn trust_with(&self, id: &EndpointId, level: TrustLevel) -> Result<()> {
        self.upsert(id, None, level, &[])
    }

    fn upsert(
        &self,
        id: &EndpointId,
        label: Option<&str>,
        level: TrustLevel,
        addrs: &[std::net::SocketAddr],
    ) -> Result<()> {
        let _mutation = self.mutation.lock().unwrap_or_else(PoisonError::into_inner);
        let key = id.to_string();
        let mut rows = self.read();
        match rows.iter_mut().find(|r| r.endpoint_id == key) {
            Some(r) => {
                r.level = level;
                if label.is_some() {
                    r.label = label.map(str::to_owned);
                }
                if !addrs.is_empty() {
                    r.addrs = addrs.iter().map(ToString::to_string).collect();
                }
            }
            None => rows.push(TrustedEndpoint {
                endpoint_id: key,
                label: label.map(str::to_owned),
                level,
                addrs: addrs.iter().map(ToString::to_string).collect(),
            }),
        }
        self.write(&rows)
    }

    /// Remove `id` from the allowlist **and close every live connection to it**.
    pub fn untrust(&self, id: &EndpointId) -> Result<()> {
        let _mutation = self.mutation.lock().unwrap_or_else(PoisonError::into_inner);
        let key = id.to_string();
        let mut rows = self.read();
        rows.retain(|r| r.endpoint_id != key);
        self.write(&rows)?;
        for conn in self.take_live(id) {
            conn.close(REVOKED.into(), b"trust revoked");
        }
        Ok(())
    }

    /// Record a live connection so [`MeshTrust::untrust`] can close it.
    pub fn try_register(
        self: &Arc<Self>,
        id: EndpointId,
        conn: Connection,
        max_per_peer: usize,
    ) -> Option<LiveRegistration> {
        let _mutation = self.mutation.lock().unwrap_or_else(PoisonError::into_inner);
        if !self.is_trusted(&id) {
            return None;
        }
        let token = self.next_registration.fetch_add(1, Ordering::Relaxed);
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        let connections = live.entry(id).or_default();
        if connections.len() >= max_per_peer {
            return None;
        }
        connections.push((token, conn));
        drop(live);
        Some(LiveRegistration {
            trust: Arc::clone(self),
            id,
            token,
        })
    }

    fn unregister(&self, id: EndpointId, token: u64) {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(connections) = live.get_mut(&id) {
            connections.retain(|(candidate, _)| *candidate != token);
            if connections.is_empty() {
                live.remove(&id);
            }
        }
    }

    pub fn registered_connections(&self, id: &EndpointId) -> usize {
        self.live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .map_or(0, Vec::len)
    }

    fn take_live(&self, id: &EndpointId) -> Vec<Connection> {
        self.live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, conn)| conn)
            .collect()
    }
}

/// Close code for a connection dropped because trust was revoked.
pub const REVOKED: u32 = 4003;

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn temp_trust() -> (tempfile::TempDir, MeshTrust) {
        let d = tempfile::tempdir().unwrap();
        let t = MeshTrust::at(&d.path().join("mesh_trust.json"));
        (d, t)
    }

    fn id() -> EndpointId {
        SecretKey::from_bytes(&[7u8; 32]).public()
    }

    fn other_id() -> EndpointId {
        SecretKey::from_bytes(&[9u8; 32]).public()
    }

    #[test]
    fn an_untrusted_endpoint_is_refused() {
        let (_d, t) = temp_trust();
        assert!(!t.is_trusted(&id()));
        t.trust(&other_id(), None).unwrap();
        assert!(
            !t.is_trusted(&id()),
            "trusting one peer must not trust another"
        );
    }

    #[test]
    fn trust_persists_across_handles() {
        let (_d, t) = temp_trust();
        t.trust(&id(), Some("blaptop04")).unwrap();
        let reopened = MeshTrust::at(t.path());
        assert_eq!(reopened.level(&id()), Some(TrustLevel::Sandboxed));
    }

    #[test]
    fn pairing_grants_sandboxed_not_native() {
        let (_d, t) = temp_trust();
        t.trust(&id(), None).unwrap();
        assert_eq!(
            t.level(&id()),
            Some(TrustLevel::Sandboxed),
            "pairing must never imply native execution"
        );
    }

    #[test]
    fn a_registry_read_error_fails_closed() {
        let t = MeshTrust::at(Path::new("/nonexistent/dir/mesh_trust.json"));
        assert!(!t.is_trusted(&id()));
    }

    #[test]
    fn an_unparseable_store_refuses_everyone_rather_than_admitting_everyone() {
        let (_d, t) = temp_trust();
        t.trust(&id(), None).unwrap();
        std::fs::write(t.path(), b"{ this is not json").unwrap();
        assert!(!t.is_trusted(&id()), "a corrupt allowlist must fail closed");
    }

    #[test]
    fn writes_are_atomic_so_a_crash_cannot_truncate_the_allowlist() {
        // fs::write truncates then writes; a crash mid-write disables the whole
        // mesh with a parse error nobody connects to the reboot.
        let (_d, t) = temp_trust();
        t.trust(&id(), None).unwrap();
        assert!(!t.path().with_extension("tmp").exists());
        let raw = std::fs::read_to_string(t.path()).unwrap();
        assert!(serde_json::from_str::<Vec<TrustedEndpoint>>(&raw).is_ok());
    }

    #[test]
    fn untrust_removes_only_the_named_peer() {
        let (_d, t) = temp_trust();
        t.trust(&id(), None).unwrap();
        t.trust(&other_id(), None).unwrap();
        t.untrust(&id()).unwrap();
        assert!(!t.is_trusted(&id()));
        assert!(t.is_trusted(&other_id()));
    }

    #[test]
    fn re_pairing_a_native_peer_does_not_silently_demote_it() {
        // grant_native is deliberate operator intent; a later ordinary pairing
        // should not quietly revert it without saying so.
        let (_d, t) = temp_trust();
        t.grant_native(&id(), None).unwrap();
        assert_eq!(t.level(&id()), Some(TrustLevel::Native));
    }

    #[test]
    fn trust_with_can_set_native_without_going_through_pairing() {
        let (_d, t) = temp_trust();
        t.trust_with(&id(), TrustLevel::Native).unwrap();
        assert_eq!(t.level(&id()), Some(TrustLevel::Native));
        t.trust_with(&id(), TrustLevel::Sandboxed).unwrap();
        assert_eq!(t.level(&id()), Some(TrustLevel::Sandboxed));
    }
}
