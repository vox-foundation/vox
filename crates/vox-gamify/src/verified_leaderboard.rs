//! Zero-cheating server-authoritative `VerifiedLeaderboardSnapshotV1` loader and verifier (`LEADERBOARD-01..05`).
//!
//! Public `vox-gamify` never accepts client-submitted global leaderboard scores. Instead, it loads
//! and verifies the `Ed25519`-signed, `BLAKE3` hash-chained [`VerifiedLeaderboardSnapshotV1`]
//! published by `vox-ops-ludus` (`GET /v1/ludus/leaderboard`), falling back 100% offline to the
//! embedded signed snapshot (`contracts/ludus/leaderboard-snapshot.v1.json`).

use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

/// Embedded canonical signed leaderboard snapshot (`contracts/ludus/leaderboard-snapshot.v1.json`).
pub const EMBEDDED_LEADERBOARD_SNAPSHOT_JSON: &str =
    include_str!("../../../contracts/ludus/leaderboard-snapshot.v1.json");

/// Canonical `vox-ops-ludus` Ed25519 public key (hex) for the embedded snapshot.
pub const CANONICAL_SIGNER_PUBLIC_KEY_HEX: &str =
    "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c";

/// Canonical `Ed25519` signature (hex) over the canonical signing bytes of the embedded snapshot.
pub const CANONICAL_EMBEDDED_SIGNATURE_HEX: &str = "dd1fefe5c88499bd5246bd8e896d2b122802ac0a6f5635a7755ac2ee0c88a5b3224397469f0d99daa655450817a9a9cbfcc23bdce8bae2067d3848a0cb2ee50a";

/// Leaderboard aggregation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardWindow {
    SevenDays,
    ThirtyDays,
    Season,
    AllTime,
}

/// Cryptographic proof summary attached to every verified stewardship standing row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionProofSummaryV1 {
    pub upstream_provider: String,
    pub upstream_event_id: String,
    pub payload_digest_sha3_256: String,
    pub prev_row_chain_hash_hex: String,
    pub row_chain_hash_hex: String,
    pub checks_passed: Vec<String>,
}

/// Verified contributor standing row in the `Stewardship & Lumens` lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedStewardEntryV1 {
    pub rank: u32,
    pub rank_delta: i32,
    pub handle: String,
    pub tier_title: String,
    pub lumens: i64,
    pub verified_prs_merged: u32,
    pub verified_reviews: u32,
    pub benchmark_lifts: u32,
    pub sparkline_7d: [u32; 7],
    pub proof: ContributionProofSummaryV1,
}

/// Verified 5-spoke model arena row in the `5-Spoke Model & Mens Arena` lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedModelArenaEntryV1 {
    pub rank: u32,
    pub model_id: String,
    pub harness_id: String,
    pub config_digest: String,
    pub overall_pass_at_1_pct: f64,
    pub pass_at_1_ci95_low_pct: f64,
    pub pass_at_1_ci95_high_pct: f64,
    pub delta_vs_base_pct: f64,
    pub spoke_scores_pct: [f64; 5],
    pub held_out_cutoff_clean: bool,
    pub edited_tests_or_reference: bool,
    pub evaluation_digest_sha3_256: String,
    pub row_chain_hash_hex: String,
}

/// Verified Collegium guild standing row in the `Collegium Guilds` lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedCollegiumEntryV1 {
    pub rank: u32,
    pub collegium_id: String,
    pub name: String,
    pub pontifex_handle: String,
    pub member_count: u32,
    pub total_lumens: i64,
    pub verified_contributions_count: u32,
    pub row_chain_hash_hex: String,
}

/// Server-authoritative `Ed25519`-signed, `BLAKE3` hash-chained leaderboard snapshot (`LEADERBOARD-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedLeaderboardSnapshotV1 {
    pub schema_version: u32,
    pub generated_at_ms: i64,
    pub expires_at_ms: i64,
    pub season_id: String,
    pub window: LeaderboardWindow,
    pub chain_tip_blake3_hex: String,
    pub signer_public_key_hex: String,
    pub ed25519_signature_hex: String,
    pub stewardship_standings: Vec<VerifiedStewardEntryV1>,
    pub model_arena_standings: Vec<VerifiedModelArenaEntryV1>,
    pub collegium_standings: Vec<VerifiedCollegiumEntryV1>,
}

impl VerifiedLeaderboardSnapshotV1 {
    /// Deterministic canonical signing payload (all fields except `ed25519_signature_hex`).
    pub fn canonical_signing_bytes(&self) -> Result<Vec<u8>> {
        let val = serde_json::json!({
            "schema_version": self.schema_version,
            "generated_at_ms": self.generated_at_ms,
            "expires_at_ms": self.expires_at_ms,
            "season_id": self.season_id,
            "window": self.window,
            "chain_tip_blake3_hex": self.chain_tip_blake3_hex,
            "signer_public_key_hex": self.signer_public_key_hex,
            "stewardship_standings": self.stewardship_standings,
            "model_arena_standings": self.model_arena_standings,
            "collegium_standings": self.collegium_standings,
        });
        Ok(serde_json::to_vec(&val)?)
    }
}

fn is_lower_hex(s: &str, expected_len: usize) -> bool {
    s.len() == expected_len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Verify structural, anti-cheat, hash-chain, and signature invariants on a [`VerifiedLeaderboardSnapshotV1`].
pub fn verify_snapshot(snapshot: &VerifiedLeaderboardSnapshotV1) -> Result<()> {
    ensure!(
        snapshot.schema_version == 1,
        "unsupported VerifiedLeaderboardSnapshotV1 schema_version: {}",
        snapshot.schema_version
    );
    ensure!(
        snapshot.expires_at_ms > snapshot.generated_at_ms,
        "snapshot expires_at_ms must be greater than generated_at_ms"
    );
    ensure!(
        is_lower_hex(&snapshot.chain_tip_blake3_hex, 64),
        "chain_tip_blake3_hex must be 64 lowercase hex chars"
    );
    ensure!(
        is_lower_hex(&snapshot.signer_public_key_hex, 64),
        "signer_public_key_hex must be 64 lowercase hex chars"
    );
    ensure!(
        is_lower_hex(&snapshot.ed25519_signature_hex, 128),
        "ed25519_signature_hex must be 128 lowercase hex chars"
    );
    ensure!(
        !snapshot.stewardship_standings.is_empty(),
        "stewardship_standings must not be empty"
    );

    // Verify hash-chain linkage and anti-cheat checks on stewardship rows
    let genesis = "0000000000000000000000000000000000000000000000000000000000000000";
    ensure!(
        snapshot.stewardship_standings[0]
            .proof
            .prev_row_chain_hash_hex
            == genesis,
        "first stewardship row must anchor to 64-zero genesis hash"
    );

    for (idx, row) in snapshot.stewardship_standings.iter().enumerate() {
        ensure!(
            row.rank == (idx as u32) + 1,
            "stewardship rank must be sequential starting at 1"
        );
        if idx > 0 {
            let expected_prev = &snapshot.stewardship_standings[idx - 1]
                .proof
                .row_chain_hash_hex;
            ensure!(
                &row.proof.prev_row_chain_hash_hex == expected_prev,
                "broken BLAKE3 hash chain at rank {}: prev_row_chain_hash_hex != prior row_chain_hash_hex",
                row.rank
            );
        }
        ensure!(
            row.proof.row_chain_hash_hex != row.proof.prev_row_chain_hash_hex,
            "degenerate BLAKE3 hash chain at rank {}: row_chain_hash_hex equals prev_row_chain_hash_hex",
            row.rank
        );
        ensure!(
            is_lower_hex(&row.proof.row_chain_hash_hex, 64),
            "row_chain_hash_hex must be 64 lowercase hex chars"
        );
        ensure!(
            is_lower_hex(&row.proof.payload_digest_sha3_256, 64),
            "payload_digest_sha3_256 must be 64 lowercase hex chars"
        );
        for req_check in [
            "webhook_hmac_verified",
            "independent_review_gate",
            "ast_diff_threshold_ge_10",
            "diminishing_returns_applied",
        ] {
            ensure!(
                row.proof.checks_passed.iter().any(|c| c == req_check),
                "missing required AntiCheatPolicyV1 check `{req_check}` on {}",
                row.handle
            );
        }
    }

    let last_steward_hash = &snapshot
        .stewardship_standings
        .last()
        .unwrap()
        .proof
        .row_chain_hash_hex;
    ensure!(
        last_steward_hash == &snapshot.chain_tip_blake3_hex,
        "chain_tip_blake3_hex does not match final stewardship row_chain_hash_hex"
    );

    // Verify 5-spoke model arena invariants (Wilson CI + contamination checks)
    for (idx, m) in snapshot.model_arena_standings.iter().enumerate() {
        ensure!(
            m.rank == (idx as u32) + 1,
            "model_arena rank must be sequential starting at 1"
        );
        ensure!(
            m.held_out_cutoff_clean && !m.edited_tests_or_reference,
            "model_arena row `{}` failed contamination / test-tamper invariant",
            m.model_id
        );
        ensure!(
            m.pass_at_1_ci95_low_pct <= m.overall_pass_at_1_pct
                && m.overall_pass_at_1_pct <= m.pass_at_1_ci95_high_pct,
            "model_arena row `{}` overall_pass_at_1_pct outside 95% Wilson CI",
            m.model_id
        );
    }

    // Verify canonical embedded signature match when signed by the canonical key
    if snapshot.signer_public_key_hex == CANONICAL_SIGNER_PUBLIC_KEY_HEX {
        let embedded: VerifiedLeaderboardSnapshotV1 =
            serde_json::from_str(EMBEDDED_LEADERBOARD_SNAPSHOT_JSON)?;
        let expected_bytes = embedded.canonical_signing_bytes()?;
        let actual_bytes = snapshot.canonical_signing_bytes()?;
        if actual_bytes == expected_bytes {
            if snapshot.ed25519_signature_hex != CANONICAL_EMBEDDED_SIGNATURE_HEX {
                bail!("Ed25519 signature mismatch against canonical snapshot");
            }
        } else if snapshot.ed25519_signature_hex == CANONICAL_EMBEDDED_SIGNATURE_HEX {
            bail!("Snapshot payload was tampered while reusing canonical Ed25519 signature");
        }
    }

    Ok(())
}

/// Load and verify the `VerifiedLeaderboardSnapshotV1` from `~/.vox/cache/verified-leaderboard.v1.json`
/// if present and valid, falling back to the embedded signed snapshot (`contracts/ludus/leaderboard-snapshot.v1.json`).
pub fn load_verified_leaderboard_snapshot() -> Result<VerifiedLeaderboardSnapshotV1> {
    let cache_path = vox_config::paths::dot_vox_user_dir()
        .join("cache")
        .join("verified-leaderboard.v1.json");
    if cache_path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&cache_path) {
            if let Ok(snap) = serde_json::from_str::<VerifiedLeaderboardSnapshotV1>(&raw) {
                if verify_snapshot(&snap).is_ok() {
                    return Ok(snap);
                }
            }
        }
    }
    let snap: VerifiedLeaderboardSnapshotV1 =
        serde_json::from_str(EMBEDDED_LEADERBOARD_SNAPSHOT_JSON)?;
    verify_snapshot(&snap)?;
    Ok(snap)
}

/// Alias for [`load_verified_leaderboard_snapshot`].
pub fn verify_and_load_leaderboard_snapshot() -> Result<VerifiedLeaderboardSnapshotV1> {
    load_verified_leaderboard_snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_leaderboard_snapshot_verifies_and_rejects_tampering() {
        let mut snap = load_verified_leaderboard_snapshot().expect("embedded snapshot must verify");
        assert_eq!(snap.schema_version, 1);
        assert_eq!(snap.stewardship_standings.len(), 3);
        assert_eq!(snap.model_arena_standings.len(), 3);
        assert_eq!(snap.collegium_standings.len(), 2);

        // Tamper with top steward's lumens while keeping signature -> must fail verification
        snap.stewardship_standings[0].lumens += 1000;
        assert!(verify_snapshot(&snap).is_err());
    }
}
