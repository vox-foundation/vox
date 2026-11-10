# vox-db

Unified SQLite / Turso persistence engine (`VoxDb`) for the Vox ecosystem, paired with `vox-db-types` (`L0`) at **`BASELINE_VERSION = 97`** ([`contracts/db/baseline-version-policy.yaml`](../../contracts/db/baseline-version-policy.yaml)).

## Core Schema Domains (`BASELINE_VERSION = 97`)

`VoxDb` consolidates runtime, knowledge base, workflow, and orchestrator state into a single versioned schema:

- **`search_documents`**: Hybrid lexical/vector document store with explicit epistemic `trust_tier` classification (`SystemVerified`, `Trusted`, `Advisory`, `Untrusted`).
- **`kb_findings`**: Quarantine inbox for unverified agent research and external findings awaiting promotion or pruning.
- **`workflow_patch_log`**: Durable workflow replay and live code-patch journal (per [ADR-019 / ADR-041](../../docs/src/adr/index.md)).
- **`hopper_inbox`**: SQLite-backed multi-agent task queue and `resource_id` lock table (per [ADR-025](../../docs/src/adr/index.md)).
- **`model_benchmark_results`**: Empirical latency, cost, and quality benchmark records feeding `vox-orchestrator-models` Pareto/McNemar selection.

## Connection Modes

| Mode | Feature Flag | Use Case |
|------|-------------|----------|
| Local Turso / SQLite | `local` (default) | Local development and single-node agent execution |
| In-Memory | `local` | Ephemeral unit and integration testing |
| Remote Turso | default | Cloud-hosted libSQL / Turso deployment |
| Embedded Replica | `replication` | Local embedded database with remote cloud sync |

## Usage

```rust
use vox_db::{DbConfig, VoxDb};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = VoxDb::connect(DbConfig::Memory).await?;
    assert_eq!(db.schema_version().await?, 97);
    Ok(())
}
```

## Canonical References

- Baseline Version Policy: [`contracts/db/baseline-version-policy.yaml`](../../contracts/db/baseline-version-policy.yaml)
- Architecture Lookup: [`docs/src/architecture/where-things-live.md`](../../docs/src/architecture/where-things-live.md)
