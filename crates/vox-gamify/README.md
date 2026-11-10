# vox-gamify (`Ludus` & `VRBS ROMA`)

Offline-capable engineering gamification, repository civilization telemetry (`VRBS ROMA`), and cryptographically signed leaderboard engine for Vox (`v6.6` / `v0.6.0`).

## Architecture & Modules

| Module | Description |
|--------|-------------|
| `workspace_town.rs` | Polyglot repository scanner and live git diff (`+LoC / -LoC`) enrichment powering the **`VRBS ROMA`** isometric Roman civilization simulator (`LudusSandbox.tsx`) |
| `quest.rs` | Offline-capable engineering quests, daily objectives, and reward progression |
| `companion.rs` | Code companions with mood, interaction, and energy systems |
| `battle.rs` | Bug battle encounters mapped to compiler and audit diagnostic categories |
| `leaderboard.rs` / `db.rs` | `BLAKE3 + Ed25519` signed `VerifiedLeaderboardSnapshotV1` standings (via `vox-crypto`) persisted in `VoxDb` |
| `ai.rs` | Model-agnostic LLM quest/narrative generation routed through `vox_actor_runtime::llm` with deterministic offline fallbacks |
| `profile.rs` | Player profile, XP, level, and engineering activity telemetry |

## CLI Commands

```bash
vox ludus status               # Show profile, XP, and VRBS ROMA standing
vox ludus quest list           # View active offline-capable engineering quests
vox ludus companion list       # List code companions
vox ludus battle start         # Start a diagnostic bug battle
```

## Design Guarantees

- **Offline-First**: Quests, `VRBS ROMA` workspace scanning (`workspace_town.rs`), and companion systems operate fully offline with deterministic fallbacks.
- **Cryptographic Integrity**: Leaderboard exports emit `VerifiedLeaderboardSnapshotV1` envelopes hashed with `BLAKE3` and signed with `Ed25519` via `vox-crypto`.
- **Model-Agnostic AI**: Dynamic narrative and quest generation routes strictly through `vox_actor_runtime::llm`.
