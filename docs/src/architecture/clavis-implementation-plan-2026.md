---
title: "Clavis V2: Full Implementation Plan (2026)"
description: "Complete, codebase-verified implementation plan for evolving Vox Clavis into a one-stop secrets manager. Covers all data structures, SQL schema, CLI surface, VoxDB integration, and wiring with all consuming crates."
category: "architecture"
status: "implementation-plan"
last_updated: 2026-04-12
training_eligible: true
training_rationale: "Concrete implementation specifications grounded in the actual codebase; directly actionable."

schema_type: "TechArticle"
---

# Clavis V2: Full Implementation Plan (2026)

> **SSOT chain:**
> [clavis-ssot.md](../reference/clavis-ssot.md) → [clavis-cloudless-threat-model-v1.md](clavis-cloudless-threat-model-v1.md) → [clavis-secrets-env-research-2026.md](clavis-secrets-env-research-2026.md) → [clavis-one-stop-secrets-research-2026.md](clavis-one-stop-secrets-research-2026.md) → **this document (implementation plan)**

## 0. Architecture snapshot (what exists today)

Before specifying changes, this section locks in precisely what Clavis does today so the plan can be change-delta driven.

### Codebase anchors

| File | Purpose |
| --- | --- |
| `crates/vox-clavis/src/spec.rs` | `SecretId` enum (581 variants), `SecretSpec`, `SecretMetadata`, `SecretClass`, `SecretMaterialKind`, `RotationPolicy`, `SecretBundle`, `Capability`, `Workflow`, `Profile`, `RequirementMode` — all `const` and `Copy`. |
| `crates/vox-clavis/src/types.rs` | `ResolutionStatus` (9 variants), `SecretSource` (6 variants), `ResolvedSecret` |
| `crates/vox-clavis/src/resolver.rs` | `SecretResolver<B>`, `ResolveProfile` (4 variants: DevLenient/CiStrict/ProdStrict/HardCutStrict), `ResolveOptions` |
| `crates/vox-clavis/src/lib.rs` | `resolve_secret(id)` (public entry), `BackendMode`, `CutoverPhase`, `OPERATOR_TUNING_ENVS` const slice |
| `crates/vox-clavis/src/backend/vox_vault.rs` | `VoxCloudBackend` (AES-256-GCM over libSQL), `CloudlessSecretRecord`, `ensure_schema` |
| `crates/vox-clavis/src/backend/mod.rs` | `SecretBackend` trait, `NoopBackend`, `UnavailableBackend` |
| `crates/vox-clavis/src/sources/` | `env.rs`, `auth_json.rs`, `populi_env.rs` |
| `crates/vox-cli/src/commands/clavis.rs` | `ClavisCmd` (Status/Set/Get/BackendStatus/MigrateAuthStore/ImportEnv), `run_doctor` with human+JSON-V1 output |

### Current DB schema (`clavis_account_secrets`)

```sql
CREATE TABLE clavis_account_secrets (
    account_id          TEXT    NOT NULL,
    secret_id           TEXT    NOT NULL,   -- spec.canonical_env value
    ciphertext          BLOB    NOT NULL,   -- AES-256-GCM of plaintext
    nonce               BLOB    NOT NULL,   -- 12-byte GCM nonce
    cipher_version      INTEGER NOT NULL DEFAULT 1,
    dek_wrapped         BLOB    NOT NULL,   -- AES-256-GCM wrapped DEK
    dek_wrap_alg        TEXT    NOT NULL DEFAULT 'AES-256-GCM',
    kek_ref             TEXT    NOT NULL,
    kek_version         INTEGER NOT NULL,
    aad_hash            TEXT,
    updated_at_ms       INTEGER NOT NULL,
    rotation_epoch      INTEGER NOT NULL DEFAULT 0,
    rotated_at_ms       INTEGER,
    consistency_origin  TEXT    NOT NULL DEFAULT 'canonical',
    consistency_version INTEGER NOT NULL DEFAULT 1,
    last_synced_at_ms   INTEGER,
    checksum_blake3     TEXT    NOT NULL,
    PRIMARY KEY (account_id, secret_id)
);
```

### Gaps confirmed from code review

1. **No version history table.** `write_secret` is UPSERT; previous value is destroyed. History has no row.
2. **No profile-scoped value overrides.** The `account_id + secret_id` primary key has no profile dimension.
3. **No audit log table.** There is no persistent record of who resolved what secret when.
4. **No `vox clavis run` subcommand.** Secrets cannot be injected into subprocess env like Doppler's `doppler run --`.
5. **No `vox clavis rotate` subcommand.** Rotation is ad-hoc write via `ImportEnv` or `Set`; no first-class rotation concept.
6. **No `vox clavis list` subcommand.** No inventory view of what is stored in the vault (metadata only, no values).
7. **No `vox clavis diff` subcommand.** No comparison between `.env` file content and vault state.
8. **`ResolutionStatus` missing lifecycle signals.** No `ProfileOverrideUsed`, `StaleRotation`, `NearingExpiry`.
9. **`SecretSpec` has no lifecycle metadata.** No `rotation_cadence_days`, `expiry_warning_days`, `taxonomy_class`.
10. **`SecretMaterialKind` missing AI-agent types.** No `OAuthClientCredential`, `DelegationRef`, `EndpointUrl` already present but no `JwtHmacSecret`, `Ed25519Key`.
11. **`ClavisCmd` `Set` only writes to `auth.json`.** It does not write to the `VoxCloudBackend` vault.
12. **`ImportEnv` has no conflict detection.** It overwrites silently if the key is already in the vault.
13. **No A2A delegation table.** Described in research but not yet specced in schema.
14. **No `redact_secrets_from_value` runtime API.** Scrubbing is static CI-only today.

---

## 1. Single canonical data structure

The entire implementation is organized around **one** expanded core struct that remains `Copy` + `const`-compatible for the `SPECS` table, plus two new VoxDB tables for runtime-mutable state.

### 1.1 Extended `SecretSpec` (zero allocation, const-compatible)

```rust
// crates/vox-clavis/src/spec.rs — additions to existing struct

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonomyClass {
    /// Class 1: platform identity and bootstrap
    PlatformIdentity,
    /// Class 2: LLM provider API keys (BYOK)
    LlmProviderKey,
    /// Class 3: cloud GPU and training infrastructure
    CloudGpuInfra,
    /// Class 4: publication and scholarly adapter
    ScholarlyPublication,
    /// Class 5: social and syndication
    SocialSyndication,
    /// Class 6: platform service mesh and transport
    MeshTransport,
    /// Class 7: telemetry and search infrastructure
    TelemetrySearch,
    /// Class 8: auxiliary tooling
    AuxTooling,
    /// Class 9: CI and operator tuning (non-secret; config only)
    OperatorTuning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LifecycleMeta {
    /// Suggested rotation cadence in days. None = Manual / no cadence.
    pub rotation_cadence_days: Option<u32>,
    /// Days before expected expiry to start warning. None = never warn.
    pub expiry_warning_days: Option<u32>,
    /// If true, a StaleRotation warning fires when rotation_epoch == 0 and
    /// creation is older than 2x rotation_cadence_days.
    pub track_stale_rotation: bool,
}

impl LifecycleMeta {
    pub const MANUAL: Self = Self {
        rotation_cadence_days: None,
        expiry_warning_days: None,
        track_stale_rotation: false,
    };
    pub const QUARTERLY: Self = Self {
        rotation_cadence_days: Some(90),
        expiry_warning_days: Some(14),
        track_stale_rotation: true,
    };
    pub const MONTHLY: Self = Self {
        rotation_cadence_days: Some(30),
        expiry_warning_days: Some(7),
        track_stale_rotation: true,
    };
    pub const ANNUAL_OAUTH: Self = Self {
        rotation_cadence_days: Some(365),
        expiry_warning_days: Some(30),
        track_stale_rotation: true,
    };
}

// Augment the existing SecretMetadata with two new fields:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SecretMetadata {
    pub class: SecretClass,
    pub material_kind: SecretMaterialKind,
    pub persistable_account_secret: bool,
    pub device_local_only: bool,
    pub allow_env_in_strict: bool,
    pub allow_compat_sources_in_strict: bool,
    pub rotation_policy: RotationPolicy,
    // NEW:
    pub taxonomy_class: TaxonomyClass,
    pub lifecycle: LifecycleMeta,
}

// SecretSpec gets one new const field: scope_description.
// Because SecretSpec is already pub, all call sites using struct literal
// construction (only the SPECS const array) gain the field.
#[derive(Debug, Clone, Copy)]
pub struct SecretSpec {
    pub id: SecretId,
    pub canonical_env: &'static str,
    pub aliases: &'static [&'static str],
    pub deprecated_aliases: &'static [&'static str],
    pub backend_key: Option<&'static str>,
    pub auth_registry: Option<&'static str>,
    pub policy: SecretPolicy,
    pub remediation: &'static str,
    // NEW:
    pub scope_description: &'static str,   // one-line human description for doctor output
}
```

**Key property:** All new fields are `const`-initializable. The ~580 entries in `SPECS` gain these fields without heap allocation. The `SPECS` array size stays constant; it just gets richer metadata.

### 1.2 Extended `ResolutionStatus`

```rust
// crates/vox-clavis/src/types.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStatus {
    Present,
    MissingOptional,
    MissingRequired,
    InvalidEmpty,
    DeprecatedAliasUsed,
    RejectedLegacyAlias,
    RejectedSourcePolicy,
    RejectedClassPolicy,
    BackendUnavailable,
    // NEW:
    ProfileOverrideUsed,   // resolved from profile-scoped override row
    StaleRotation,         // present but rotation_epoch == 0 and past cadence
    NearingExpiry,         // present but expiry_warning fires
}
```

### 1.3 Extended `SecretMaterialKind`

```rust
// crates/vox-clavis/src/spec.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretMaterialKind {
    ApiKey,
    OAuthRefreshToken,
    OAuthClientCredential,   // NEW: (client_id, client_secret) pair reference
    BearerToken,
    HmacSecret,
    JwtHmacSecret,           // NEW: specifically for HS256 JWT signing
    Ed25519Key,              // NEW: Ed25519 raw key material
    EndpointUrl,
    Username,
    Password,
    DelegationRef,           // NEW: opaque A2A delegation token reference
    ConfigValue,             // NEW: non-secret operator tuning values (OperatorTuning class)
}
```

---

## 2. VoxDB schema additions

All three new tables live in the same libSQL database as `clavis_account_secrets`. They are created lazily inside `ensure_schema`. The `account_id` column always matches the operator's `VOX_ACCOUNT_ID` to isolate per-user state.

### 2.1 Secret version history (`clavis_secret_versions`)

Append-only. Every time a value is written (created or rotated), a new row is inserted. Values are never updated in place.

```sql
CREATE TABLE IF NOT EXISTS clavis_secret_versions (
    version_id          INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id          TEXT    NOT NULL,
    secret_id           TEXT    NOT NULL,
    ciphertext          BLOB    NOT NULL,     -- encrypted with per-version DEK
    nonce               BLOB    NOT NULL,     -- 12-byte GCM nonce
    dek_wrapped         BLOB    NOT NULL,     -- DEK wrapped by current KEK
    kek_ref             TEXT    NOT NULL,
    kek_version         INTEGER NOT NULL,
    operation           TEXT    NOT NULL,     -- 'create' | 'rotate' | 'rollback' | 'import'
    source_hint         TEXT,                 -- 'env-import' | 'cli-set' | 'rotation-auto' | null
    created_at_ms       INTEGER NOT NULL,
    created_by          TEXT    NOT NULL,     -- 'cli' | 'mcp' | 'agent:<id>' | 'api'
    checksum_blake3     TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_clavis_sv_acct_secret
    ON clavis_secret_versions(account_id, secret_id, version_id DESC);
```

**Relationship to `clavis_account_secrets`:** `clavis_account_secrets` is the authoritative current-value row (fast path for `resolve`). `clavis_secret_versions` is the historical ledger. Both are updated atomically (within the same libSQL `BEGIN EXCLUSIVE` transaction) on every write.

**Max depth:** Configurable via `OPERATOR_CLAVIS_VERSION_HISTORY_DEPTH` (default 10 per secret per account). Clean up via `vox clavis prune-history --keep 10`.

### 2.2 Audit log (`clavis_audit_log`)

Append-only resolution events. Values never written — only metadata.

```sql
CREATE TABLE IF NOT EXISTS clavis_audit_log (
    row_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id          TEXT    NOT NULL,
    secret_id           TEXT    NOT NULL,
    resolved_at_ms      INTEGER NOT NULL,
    resolution_status   TEXT    NOT NULL,   -- ResolutionStatus Debug name
    resolution_source   TEXT,               -- SecretSource Debug name or NULL
    profile             TEXT    NOT NULL,   -- ResolveProfile Debug name
    caller_context      TEXT,               -- 'cli' | 'mcp' | 'agent:<task_id>' | null
    detail              TEXT
);

CREATE INDEX IF NOT EXISTS idx_clavis_al_acct_time
    ON clavis_audit_log(account_id, resolved_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_clavis_al_secret
    ON clavis_audit_log(account_id, secret_id, resolved_at_ms DESC);
```

**Opt-in:** Audit logging is disabled by default (dev overhead). Enable with `VOX_CLAVIS_AUDIT_LOG=1`. Always enabled in `ProdStrict` and `HardCutStrict` profiles.

**Never logged:** Resolved values, redacted or otherwise. Only metadata.

### 2.3 Profile-scoped value overrides (`clavis_profile_overrides`)

Enables the same `SecretId` to have different values per `ResolveProfile`.

```sql
CREATE TABLE IF NOT EXISTS clavis_profile_overrides (
    account_id          TEXT    NOT NULL,
    secret_id           TEXT    NOT NULL,
    profile             TEXT    NOT NULL,   -- 'dev' | 'ci' | 'prod' | 'hardcut'
    ciphertext          BLOB    NOT NULL,
    nonce               BLOB    NOT NULL,
    dek_wrapped         BLOB    NOT NULL,
    kek_ref             TEXT    NOT NULL,
    kek_version         INTEGER NOT NULL,
    updated_at_ms       INTEGER NOT NULL,
    checksum_blake3     TEXT    NOT NULL,
    PRIMARY KEY (account_id, secret_id, profile)
);
```

**Resolution precedence (updated):**
1. Profile-scoped override row (new — `clavis_profile_overrides`)
2. Canonical override row (`clavis_account_secrets`)
3. Environment variable (`SecretSource::EnvCanonical`)
4. Environment alias (`SecretSource::EnvAlias`)
5. `auth.json` registry token (`SecretSource::AuthJson`)
6. Populi env file (`SecretSource::PopuliEnv`)
7. → Missing

### 2.4 A2A delegation records (`clavis_agent_delegations`)

```sql
CREATE TABLE IF NOT EXISTS clavis_agent_delegations (
    delegation_id       TEXT    PRIMARY KEY,    -- random UUID
    account_id          TEXT    NOT NULL,
    secret_id           TEXT    NOT NULL,
    scope_bits          INTEGER NOT NULL,        -- bitmask: 0x01=read, future extra bits
    parent_context      TEXT    NOT NULL,        -- 'orchestrator' | 'agent:<id>'
    child_context       TEXT    NOT NULL,        -- 'agent:<task_id>'
    issued_at_ms        INTEGER NOT NULL,
    expires_at_ms       INTEGER NOT NULL,        -- hard max: issued + 3600000 (1 hour)
    revoked_at_ms       INTEGER,
    revoke_reason       TEXT
);

CREATE INDEX IF NOT EXISTS idx_clavis_del_acct_secret
    ON clavis_agent_delegations(account_id, secret_id, expires_at_ms DESC);
```

**Resolution via delegation:** `resolve_secret_for_delegation(delegation_id, account_id)` → validates TTL and scope, then delegates to `resolve_secret(secret_id)` for the actual value.

---

## 3. VoxDB wiring (`vox-db` integration)

The Clavis vault currently creates its own independent libSQL connection using `turso` — separate from the main Vox data plane (`VOX_DB_URL`/`VOX_DB_TOKEN`). This is by design (the `AGENTS.md` note: "do not conflate Codex with the Clavis vault plane").

**The wiring model stays dual-plane**, but both planes can point to the same physical Turso database in a single-database deployment:

```
VOX_CLAVIS_VAULT_URL  (+VOX_CLAVIS_VAULT_TOKEN)  →  clavis_account_secrets
                                                      clavis_secret_versions
                                                      clavis_audit_log
                                                      clavis_profile_overrides
                                                      clavis_agent_delegations

VOX_DB_URL            (+VOX_DB_TOKEN)             →  arca_* (Vox main tables)
                                                      clavis tables (if same DB)
```

For the `vox-db` crate to expose a Clavis API surface:

```rust
// crates/vox-db/src/clavis_gate.rs  [NEW FILE]
// Exposes a thin async wrapper for the four new Clavis tables to consumers
// inside the vox-db domain (e.g., MCP tool server, orchestrator tasks).

pub struct ClavisGate {
    conn: Arc<turso::Connection>,
}

impl ClavisGate {
    /// Fetch audit log rows (no secret values) for a given account.
    pub async fn audit_log(
        &self,
        account_id: &str,
        secret_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditLogRow>, DbError>;

    /// Create an A2A delegation record; caller is responsible for TTL enforcement.
    pub async fn create_delegation(
        &self,
        account_id: &str,
        secret_id: &str,
        scope_bits: u32,
        parent_context: &str,
        child_context: &str,
        ttl_secs: u32,      // capped internally at 3600
    ) -> Result<String, DbError>;   // returns delegation_id

    /// Validate and fetch delegation (None if expired/revoked).
    pub async fn get_valid_delegation(
        &self,
        delegation_id: &str,
        account_id: &str,
    ) -> Result<Option<DelegationRow>, DbError>;

    /// Revoke a delegation.
    pub async fn revoke_delegation(
        &self,
        delegation_id: &str,
        account_id: &str,
        reason: &str,
    ) -> Result<bool, DbError>;
}
```

**`vox-db` depends on `vox-clavis`** only for type aliases (`SecretId`, `SecretSpec`) — no circular dep. `vox-clavis` does **not** depend on `vox-db`.

---

## 4. Updated `VoxCloudBackend` write path

The existing `write_secret_for_account` must be extended to:
1. Atomically write to `clavis_account_secrets` (UPSERT) AND append to `clavis_secret_versions`.
2. Accept an optional `profile: Option<ResolveProfile>` — if provided, write to `clavis_profile_overrides` instead of the canonical table.
3. Accept a `caller_context: &str` for audit attribution.

```rust
/// Extended write API — replaces write_secret and write_secret_for_account.
pub fn write_secret_v2(
    &self,
    secret_id: &str,
    plaintext: &str,
    profile: Option<&str>,         // None = canonical, Some("ci") = profile override
    operation: &str,               // 'create' | 'rotate' | 'import' | 'rollback'
    source_hint: Option<&str>,
    caller_context: &str,
) -> Result<(), SecretError>;
```

All existing callers of `write_secret` and `write_secret_for_account` are migrated to `write_secret_v2`.

**Within `write_secret_v2` the transaction sequence:**
```sql
BEGIN EXCLUSIVE;

-- 1. UPSERT current value (unchanged logic)
INSERT INTO clavis_account_secrets ... ON CONFLICT ... DO UPDATE ...;

-- 2. Append version record
INSERT INTO clavis_secret_versions
    (account_id, secret_id, ciphertext, nonce, dek_wrapped, kek_ref, kek_version,
     operation, source_hint, created_at_ms, created_by, checksum_blake3)
VALUES ...;

-- 3. Optionally prune old versions if depth > max
DELETE FROM clavis_secret_versions
WHERE account_id = ?1 AND secret_id = ?2
  AND version_id NOT IN (
    SELECT version_id FROM clavis_secret_versions
    WHERE account_id = ?1 AND secret_id = ?2
    ORDER BY version_id DESC
    LIMIT ?3   -- max_history_depth
  );

COMMIT;
```

---

## 5. Updated `resolve_secret` path (audit logging)

When `VOX_CLAVIS_AUDIT_LOG=1` (or profile is Strict), append an audit row after every resolution. This is done **after** the resolved value is returned to the caller — the audit write is best-effort and never blocks the resolution result.

```rust
// crates/vox-clavis/src/lib.rs

pub fn resolve_secret(id: SecretId) -> ResolvedSecret {
    let result = resolve_secret_inner(id);   // existing logic
    
    if audit_log_enabled() {
        let _ = append_audit_row(&result, caller_context_from_env());
    }
    
    result
}

fn audit_log_enabled() -> bool {
    let profile = resolve_profile_from_env();
    profile.is_strict()
        || std::env::var("VOX_CLAVIS_AUDIT_LOG")
            .ok()
            .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
}
```

`append_audit_row` writes to `clavis_audit_log` via `VoxCloudBackend` if the backend is available, silently no-ops otherwise. Never writes the resolved value.

---

## 6. Updated `resolver.rs` (lifecycle status injection)

After resolution succeeds via any source, lifecycle status checks fire:

```rust
fn resolve_spec(&self, spec: SecretSpec, opts: &ResolveOptions) -> ResolvedSecret {
    // ... existing resolution chain ...

    // After obtaining a Present resolution, inject lifecycle status:
    if resolved.status == ResolutionStatus::Present {
        resolved.status = compute_lifecycle_status(&spec, &resolved, opts);
    }

    resolved
}

fn compute_lifecycle_status(
    spec: &SecretSpec,
    resolved: &ResolvedSecret,
    _opts: &ResolveOptions,
) -> ResolutionStatus {
    let lm = spec.id.metadata().lifecycle;
    
    // ExternalBackend source = vault row; check rotation epoch
    if resolved.source == Some(SecretSource::ExternalBackend) {
        if lm.track_stale_rotation {
            // Query the vault for rotation metadata (lightweight metadata-only read)
            // If rotation_epoch == 0 and update is older than 2x cadence → StaleRotation
            // (deferred to Wave 2 when the metadata-only read path is added)
        }
        if lm.expiry_warning_days.is_some() {
            // If provider expiry metadata is present and within warning window → NearingExpiry
            // (deferred to Wave 3 when provider probe is added)
        }
    }
    
    ResolutionStatus::Present  // baseline, extended per wave
}
```

---

## 7. CLI surface additions

All new subcommands added to `ClavisCmd` in `crates/vox-cli/src/commands/clavis.rs`:

### 7.1 `vox clavis set-secret` (replaces auth-json-only `set`)

```
vox clavis set-secret <SECRET_ID> [--value <val>] [--profile <profile>] [--rotate]
```

Writes directly to `VoxCloudBackend` (not just `auth.json`). `--rotate` flags the operation as a rotation event. Prompts for value interactively if `--value` is omitted. Redacted echo confirmation.

### 7.2 `vox clavis list`

```
vox clavis list [--class <class>] [--format human|json-v1]
```

Shows all secrets present in the vault for the current `VOX_ACCOUNT_ID`. Outputs metadata only: `secret_id`, `canonical_env`, `class`, `material_kind`, `rotation_epoch`, `updated_at_ms`, `kek_ref`. No values.

### 7.3 `vox clavis rotate`

```
vox clavis rotate <SECRET_ID> [--value <val>] [--profile <profile>]
```

Writes to vault with `operation = 'rotate'`, increments `rotation_epoch`, sets `rotated_at_ms`. Prints rotation confirmation with redacted new value and version number.

### 7.4 `vox clavis rollback`

```
vox clavis rollback <SECRET_ID> --to-version <N> [--dry-run]
```

Reads version N from `clavis_secret_versions`, decrypts it, re-encrypts under current KEK, writes to `clavis_account_secrets` with `operation = 'rollback'` in version history. Requires `--reason <text>` flag (logged to audit).

### 7.5 `vox clavis history`

```
vox clavis history <SECRET_ID> [--limit 10]
```

Shows version log for the secret: `version_id`, `created_at_ms`, `operation`, `source_hint`, `created_by`. Never shows ciphertext or decrypted values.

### 7.6 `vox clavis diff`

```
vox clavis diff [--env-file <path>] [--format human|json-v1]
```

Compares all keys in the specified `.env` file against the Clavis managed set:
- **Present in `.env`, managed by Clavis:** shows current source (vault vs env) and whether vault version exists.
- **Present in `.env`, unmanaged:** lists as "unregistered — not managed by Clavis".
- **Managed by Clavis, absent from `.env`:** lists as "vault only" or "missing entirely".
- **Name mismatches:** flags non-canonical names with suggested canonical form.

### 7.7 `vox clavis run`

```
vox clavis run [--bundle <bundle>] [--profile <profile>] -- <command> [args...]
```

Resolves all secrets for the given bundle/profile, sets them as environment variables, then `exec`s the command. Pattern matches Doppler's `doppler run --` and Pulumi ESC's `esc run`. This is the primary developer ergonomics improvement for local development workflows.

**Security:** Environment variables are injected into the child process only; the parent shell environment is not modified. On Unix, `std::os::unix::process::CommandExt::exec` replaces the current process. On Windows, `Command::spawn()` is used.

### 7.8 `vox clavis audit-log`

```
vox clavis audit-log [--secret <id>] [--since <iso8601>] [--limit 50] [--format human|json-v1]
```

Reads from `clavis_audit_log` for the current account. Shows: `resolved_at_ms`, `secret_id`, `resolution_status`, `resolution_source`, `profile`, `caller_context`.

### 7.9 `vox clavis delegate`

```
vox clavis delegate <SECRET_ID> --to <agent-context> --ttl-secs <N>
```

Creates an entry in `clavis_agent_delegations`. Prints the opaque `delegation_id`. Max TTL 3600.

### 7.10 `vox clavis revoke-delegation`

```
vox clavis revoke-delegation <DELEGATION_ID> --reason <text>
```

Sets `revoked_at_ms` on the delegation record.

### 7.11 Updated `import-env` (conflict detection)

The existing `ImportEnv` gains:
- `--overwrite` / `--no-overwrite` (default: `--no-overwrite`; warn and skip if secret already in vault).
- `--classify` flag: pre-analyze each key against the taxonomy table and print human-readable classification before importing.
- Canonical name suggestion: if `GEMINI_KEY` is found instead of `GEMINI_API_KEY`, suggest the canonical form.

### 7.12 Updated `status/doctor`

The `DoctorSecretRow` JSON schema gains:
- `taxonomy_class: String`
- `scope_description: String`
- `lifecycle_cadence_days: Option<u32>`
- `lifecycle_expiry_warning_days: Option<u32>`
- `rotation_epoch: Option<i64>` (from vault row if available)
- `rotated_at_hint: Option<i64>`

Human output gains per-class health grouping.

---

## 8. Consumer wiring — all platforms

### 8.1 `vox-mcp` (`crates/vox-mcp/src/http_gateway.rs`)

**Current:** Resolves `VoxMcpHttpBearerToken`, `VoxMcpHttpReadBearerToken` at startup via `resolve_secret`.

**Change:** No code change needed for secret resolution. New `McpClientCredential` entries added to `SPECS` for future remote MCP OAuth 2.1 support. `VoxMcpAgentFleet` gets `taxonomy_class: TaxonomyClass::MeshTransport`.

**Audit wire-in:** The HTTP gateway should pass `caller_context = "mcp"` to the resolution path for audit attribution when `VOX_CLAVIS_AUDIT_LOG=1`.

### 8.2 `vox-orchestrator` (`crates/vox-orchestrator/src/config/impl_env.rs`)

**Current:** Resolves ~50 `VoxOrchestrator*` secrets at config load.

**Change:** No API change. The `taxonomy_class = TaxonomyClass::OperatorTuning` and `material_kind = SecretMaterialKind::ConfigValue` labels are applied to all `VoxOrchestrator*` entries in `SPECS` annotatively. This enables `vox clavis list --class operator-tuning` to filter them correctly.

**A2A delegation:** When the orchestrator spawns a sub-agent task that requires a specific secret, it will in a future wave call `ClavisGate::create_delegation()` to produce a scoped delegation reference. The sub-agent resolves via `resolve_secret_for_delegation(delegation_id)`. The v1 implementation uses direct `resolve_secret` until the delegation path is wired.

### 8.3 `vox-runtime` / `vox-skills` (LLM call sites)

**Current:** Resolve `OpenRouterApiKey`, `AnthropicApiKey`, etc. at call time via `resolve_secret`.

**Change:** Add `caller_context = "agent:<task_id>"` audit attribution where the task ID is available. Apply `taxonomy_class = TaxonomyClass::LlmProviderKey` in SPECS.

**Secret boundary enforcement:** All LLM call sites already use `SecretString`; the `#[serde(skip_serializing)]` guards confirmed in previous sessions remain effective. No new code needed for the basic boundary — delegation path is Wave 7.

### 8.4 `vox-publisher` (publication adapters)

**Current:** Resolves Zenodo, ORCID, CrossRef, OpenReview, Reddit, YouTube, Discord credentials via `resolve_secret`.

**Change:** Apply `taxonomy_class` annotations in SPECS:
- `VoxZenodoAccessToken`, `VoxOrcidClientId/Secret`, `VoxDataCitePassword` → `ScholarlyPublication`
- `VoxSocialReddit*`, `VoxSocialYoutube*`, `VoxSocialMastodonToken`, `VoxSocialLinkedinAccessToken`, `VoxSocialDiscordWebhook` → `SocialSyndication`

OAuth refresh token entries (`VoxSocialRedditRefreshToken`, `VoxSocialYoutubeRefreshToken`) get `lifecycle: LifecycleMeta::ANNUAL_OAUTH`. Expiry warning threshold applies.

### 8.5 `vox-scientia-ingest` and Tavily search

**Current:** `TavilyApiKey` resolved via `resolve_secret`.

**Change:** `taxonomy_class = TaxonomyClass::AuxTooling`, `lifecycle: LifecycleMeta::QUARTERLY`. `VoxSearchQdrantApiKey` → `TaxonomyClass::TelemetrySearch`.

### 8.6 `vox-db` (new `ClavisGate`)

The `ClavisGate` struct (§3) is new. It uses the same `VOX_CLAVIS_VAULT_URL` connection as `VoxCloudBackend` but is exposed as an async interface for use inside `vox-db` domain operations (MCP tool results audit scrubbing, agent trace writes).

### 8.7 `vox-toestub` and `vox-webhook`

No changes to secret resolution. The existing `#[serde(skip_serializing)]` fields confirmed in previous sessions are sufficient.

### 8.8 `vox-config` (`OPERATOR_TUNING_ENVS`)

Add new operator const: `OPERATOR_CLAVIS_AUDIT_LOG = "VOX_CLAVIS_AUDIT_LOG"` and `OPERATOR_CLAVIS_VERSION_HISTORY_DEPTH = "VOX_CLAVIS_VERSION_HISTORY_DEPTH"` to `lib.rs` and `OPERATOR_TUNING_ENVS`. These are tuning controls, not secrets.

---

## 9. Runtime secret scrubber (`redact_secrets_from_value`)

New public API in `crates/vox-clavis/src/redact.rs`:

```rust
// New file: crates/vox-clavis/src/redact.rs

use serde_json::Value;

/// Scrub all known managed secret values from a JSON `Value` tree.
///
/// Uses an AhoCorasick multi-pattern searcher built from the set of
/// currently resolved secret values. Safe to call on tool results,
/// telemetry payloads, or trace events before serialization/storage.
///
/// # Performance
/// Pattern compilation cost is amortized by caching behind a `OnceLock<Searcher>`.
/// The searcher is invalidated when `invalidate_scrubber_cache()` is called
/// (typically after a rotation or import).
///
/// # Safety contract
/// This function NEVER accesses the vault. It operates on an in-memory
/// snapshot of resolved values provided by the caller. The caller is
/// responsible for not passing the snapshot across process boundaries.
pub fn redact_secrets_from_value(
    value: &Value,
    resolved_values: &[&str],  // caller-obtained from resolve_secret().expose()
) -> Value;

/// Check if a string slice contains any known resolved secret value.
pub fn contains_secret_material(text: &str, resolved_values: &[&str]) -> bool;

/// Invalidate the scrubber cache (call after rotation/import).
pub fn invalidate_scrubber_cache();
```

**Implementation:** Uses the `aho-corasick` crate (already in workspace or easily added; BSD-2-Clause) for O(n) multi-pattern search. The `OnceLock<AhoCorasick>` cache is keyed by the sorted set of patterns; it is rebuilt only when `invalidate_scrubber_cache` is called.

**Wire-in points:**
1. `vox-mcp/src/http_gateway.rs` — before serializing any tool result with external state.
2. `vox-db/src/clavis_gate.rs` — before writing agent events to VoxDB.
3. `crates/vox-telemetry/` — before any telemetry upload batch (when `VOX_TELEMETRY_UPLOAD_URL` is set).

---

## 10. Implementation waves (ordered by dependency)

### Wave 0 — Annotation (zero behaviour change, ~1 day)

**Goal:** Annotate all existing `SPECS` entries with `taxonomy_class`, `lifecycle`, and `scope_description`. No API changes; no DB changes.

Files changed:
- `crates/vox-clavis/src/spec.rs` — add `TaxonomyClass`, `LifecycleMeta`, extend `SecretMetadata` and `SecretSpec` structs. Annotate all ~580 SPECS entries.

Verification:
- `cargo check --workspace` (must be clean).
- `vox ci clavis-parity` (must pass with updated doc output).
- `vox ci secret-env-guard --all` (must remain clean).

### Wave 1 — DB schema + write path (1–2 days)

**Goal:** `clavis_secret_versions`, `clavis_audit_log`, `clavis_profile_overrides`, `clavis_agent_delegations` tables created; `write_secret_v2` replaces `write_secret_for_account`.

Files changed:
- `crates/vox-clavis/src/backend/vox_vault.rs` — `ensure_schema` gains four new `CREATE TABLE IF NOT EXISTS` blocks. Add `write_secret_v2`. Existing `write_secret` and `write_secret_for_account` become thin wrappers around `write_secret_v2` for backward compatibility.

Verification:
- Unit test: `write_secret_v2` creates current row AND appends version row in same transaction.
- Unit test: profile override read/write round-trip.
- `cargo test -p vox-clavis` (must pass).
- `cargo check --workspace` (must be clean).

### Wave 2 — `ResolutionStatus` + resolver lifecycle signals (~1 day)

**Goal:** `ProfileOverrideUsed`, `StaleRotation`, `NearingExpiry` statuses plumbed end-to-end.

Files changed:
- `crates/vox-clavis/src/types.rs` — add three new `ResolutionStatus` variants.
- `crates/vox-clavis/src/resolver.rs` — add `compute_lifecycle_status`, profile override check.
- `crates/vox-clavis/src/backend/vox_vault.rs` — add `get_row_metadata` (returns epoch+timestamp without decrypting, for lifecycle checks).
- `crates/vox-cli/src/commands/clavis.rs` — `DoctorSecretRow` gains new fields.

Verification:
- Unit test: profile override row resolves with `ProfileOverrideUsed` status.
- Unit test: `StaleRotation` fires when `rotation_epoch == 0` and write is old.
- Existing doctor integration test (if present) updated.

### Wave 3 — CLI surface part 1 (~2 days)

**Goal:** Most-impactful new commands: `set-secret`, `rotate`, `list`, `diff`, `run`, updated `import-env`.

Files changed:
- `crates/vox-cli/src/commands/clavis.rs` — add `ClavisCmd` variants, implement handlers.

Verification:
- `vox clavis list` produces clean output against an empty dev vault.
- `vox clavis run --bundle minimal-local-dev -- echo 'started'` resolves and execs.
- `vox clavis diff --env-file .env` produces correct diff.
- `vox clavis import-env --dry-run --classify` classifies keys correctly.
- `cargo check --workspace` clean.

### Wave 4 — Audit log integration (~1 day)

**Goal:** Audit log enabled, writes on every resolution in strict profiles and when `VOX_CLAVIS_AUDIT_LOG=1`.

Files changed:
- `crates/vox-clavis/src/lib.rs` — `resolve_secret` calls `append_audit_row` after resolution.
- `crates/vox-cli/src/commands/clavis.rs` — `audit-log` subcommand.
- `crates/vox-clavis/src/lib.rs` (public API) — expose `OPERATOR_CLAVIS_AUDIT_LOG` const.

Verification:
- Set `VOX_CLAVIS_AUDIT_LOG=1`, resolve a secret, `vox clavis audit-log` shows one row.
- `cargo test -p vox-clavis` clean.

### Wave 5 — Version history CLI (~1 day)

**Goal:** `history`, `rollback`, `prune-history` commands.

Files changed:
- `crates/vox-cli/src/commands/clavis.rs` — add three `ClavisCmd` variants.
- `crates/vox-clavis/src/backend/vox_vault.rs` — add `get_history`, `rollback_to_version`, `prune_history`.

Verification:
- `vox clavis rotate OPENROUTER_API_KEY --value sk-… ; vox clavis history OPENROUTER_API_KEY` shows 2 rows (create + rotate).
- `vox clavis rollback OPENROUTER_API_KEY --to-version 1 --reason 'test'` works.

### Wave 6 — Runtime scrubber (`redact.rs`) (~1 day)

**Goal:** `redact_secrets_from_value` available and wired into MCP tool result path.

Files changed:
- `crates/vox-clavis/src/redact.rs` — new file.
- `crates/vox-clavis/src/lib.rs` — pub re-export.
- `Cargo.toml` for `vox-clavis` — add `aho-corasick` dependency.
- `crates/vox-mcp/src/http_gateway.rs` — wire scrubber before tool result serialization.

Verification:
- Unit test: JSON value containing a real resolved API key value is fully scrubbed.
- `cargo test -p vox-clavis` and `cargo test -p vox-mcp` clean.

### Wave 7 — A2A delegation (~2 days)

**Goal:** `ClavisGate` in `vox-db`, delegation create/validate/revoke, `delegate` and `revoke-delegation` CLI, `resolve_secret_for_delegation` API.

Files changed:
- `crates/vox-db/src/clavis_gate.rs` — new file.
- `crates/vox-db/src/lib.rs` — re-export `ClavisGate`.
- `crates/vox-clavis/src/lib.rs` — add `resolve_secret_for_delegation`.
- `crates/vox-cli/src/commands/clavis.rs` — `delegate`, `revoke-delegation` subcommands.

Verification:
- `vox clavis delegate OPENROUTER_API_KEY --to 'agent:test-task' --ttl-secs 60` returns delegation ID.
- `resolve_secret_for_delegation(id, account_id)` resolves correctly within TTL.
- After 60s (or revoke), resolution fails with `BackendUnavailable` from delegation.

### Wave 8 — `spec.rs` SPECS annotation completion + CI parity update (~1 day)

**Goal:** All SPECS entries have `taxonomy_class`, `lifecycle`, and `scope_description` set correctly. `clavis-parity` CI check validates these.

Files changed:
- `crates/vox-clavis/src/spec.rs` — annotate remaining SPECS entries.
- `crates/vox-cli/src/commands/ci/run_body_helpers/guards.rs` — update `clavis-parity` logic to validate taxonomy metadata.
- `docs/src/reference/clavis-ssot.md` — update SSOT table with taxonomy columns.

Verification:
- `vox ci clavis-parity` passes.
- `vox clavis list --class llm-provider` returns only the 11 LLM provider keys.
- `vox clavis list --class operator-tuning` returns no secrets (they are config, not secrets).

---

## 11. Cargo.toml additions summary

| Crate | New dependency | Reason |
| --- | --- | --- |
| `vox-clavis` | `aho-corasick = "1"` | Scrubber multi-pattern search |
| `vox-clavis` | `uuid = { version = "1", features = ["v4"] }` | Delegation IDs |
| `vox-db` | `vox-clavis` (workspace dep) | ClavisGate types |
| `vox-clavis` | `serde_json` (already likely present via vox-db dep) | Scrubber accepts `Value` |

No changes to `vox-mcp`, `vox-orchestrator`, `vox-runtime`, `vox-publisher`, or `vox-skills` Cargo.toml files. All consumer crates already have `vox-clavis` as a dependency.

---

## 12. CI gates (new/updated)

| Gate | Command | Validates |
| --- | --- | --- |
| Secret env guard (existing) | `vox ci secret-env-guard --all` | No raw `std::env::var` for managed secrets |
| Clavis parity (updated) | `vox ci clavis-parity` | All SPECS entries have `taxonomy_class`, `scope_description`; SSOT doc matches |
| Audit log schema (new) | `vox ci clavis-audit-schema` | `clavis_audit_log` schema matches contract JSON at `contracts/clavis/audit-log.v1.json` |
| Delegation TTL invariant (new) | Part of `cargo test -p vox-clavis` | Delegation records with TTL >3600s are rejected |
| Scrubber coverage (new) | Part of `cargo test -p vox-clavis` | `redact_secrets_from_value` catches known patterns |

---

## 13. SSOT doc update (`clavis-ssot.md`)

The reference doc at `docs/src/reference/clavis-ssot.md` must be updated to add:
- `taxonomy_class` and `lifecycle_cadence_days` columns to the managed secrets table.
- New "Profile-scoped overrides" section documenting `clavis_profile_overrides` usage.
- New "Audit log" section referencing `clavis_audit_log`.
- New "A2A delegation" section with the delegation API usage pattern.
- New "Secret version history" section with rollback instructions.
- Updated CLI reference section listing all new subcommands.

---

## 14. Security invariants delta (additions to V1 threat model)

These extend the 5 invariants in `clavis-cloudless-threat-model-v1.md`:

6. `redact_secrets_from_value` MUST be applied before any content from `resolve_secret` is written to `clavis_audit_log`, MCP tool results, telemetry upload batches, or agent event traces.
7. Delegation records MUST have `expires_at_ms ≤ issued_at_ms + 3_600_000`. Backend enforces this cap regardless of caller-supplied TTL.
8. `clavis_secret_versions` is append-only. Any `UPDATE` or `DELETE` on version rows is a CI blocker if present in production migration files.
9. `clavis_audit_log` rows MUST NOT contain resolved secret values or redacted representations. Only resolution metadata is stored.
10. Profile-scoped override rows (`clavis_profile_overrides`) for the `prod` profile MUST require explicit `--profile prod` confirmation in the CLI; no implicit promotion from `ci` or `dev`.

---

## 15. Open questions (for implementation-time decisions)

1. **`write_secret` backward compat:** The existing `write_secret` is used in `ImportEnv`. Should it call `write_secret_v2` with `operation = 'import'` automatically, or should `ImportEnv` be explicitly updated to use the new API? Recommendation: update `ImportEnv` explicitly so the `source_hint` and `caller_context` are set correctly.

2. **`vox clavis run` on Windows:** `exec` is not available on Windows; must use `Command::spawn()` + `wait()`. The parent process stays alive. This changes the signal propagation behaviour. Acceptable for the target use case (dev workflow), but should be documented.

3. **`aho-corasick` dependency weight:** The `aho-corasick` crate adds ~1 transitive dep. It is already used elsewhere in the workspace (search infrastructure). Verify with `cargo tree` before adding — it may already be a transitive dep of `vox-search` or `vox-tantivy`.

4. **`clavis_profile_overrides` for Mobile profile:** The `Mobile` profile is defined in `ResolveProfile` but currently equivalent to `DevLenient`. If profile overrides land before the Mobile profile is differentiated, `profile = 'mobile'` overrides will be silently ignored. Acceptable; document in release notes.

5. **Historical DEK re-wrapping on KEK rotation:** When `rewrap_secret_for_account` rotates a KEK, it currently only re-wraps the current row's DEK. Historical version rows in `clavis_secret_versions` still have DEKs wrapped with the old KEK. Recommendation: after `rewrap_secret_for_account`, run a background sweep that re-wraps historical version DEKs. Not blocking Wave 1, but needed before v1.0 stable.

---

## 16. Cross-reference map

| Doc | Relationship |
| --- | --- |
| [clavis-ssot.md](../reference/clavis-ssot.md) | SSOT for all managed secrets; updated by Wave 8 |
| [clavis-cloudless-threat-model-v1.md](clavis-cloudless-threat-model-v1.md) | Threat model extended by §14 invariants 6–10 |
| [clavis-secrets-env-research-2026.md](clavis-secrets-env-research-2026.md) | Base research; "Research gates A–D" map to Waves 1–7 |
| [clavis-one-stop-secrets-research-2026.md](clavis-one-stop-secrets-research-2026.md) | Expanded research; feature requirements map to §7 CLI surface |
| [clavis-cloudless-implementation-catalog.md](clavis-cloudless-implementation-catalog.md) | Tactical task checklist; Waves in this plan supersede Wave order there |
| [terminal-exec-policy-research-findings-2026.md](terminal-exec-policy-research-findings-2026.md) | CLI exec policy; `vox clavis run` subprocess launch follows those rules |
