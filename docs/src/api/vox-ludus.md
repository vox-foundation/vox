---
title: "vox-ludus"
description: "Official documentation for vox-ludus for the Vox language. Detailed technical reference, architecture guides, and implementation patterns"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# vox-ludus

Gamification layer for the Vox programming language. Code companions, daily quests, bug battles, and ASCII sprites — all working fully offline.

## Features

| Module | Description |
|--------|-------------|
| `companion.rs` | Code companions with mood, interaction, and energy systems |
| `quest.rs` | Daily quests with randomized objectives |
| `battle.rs` | Bug battle encounters with typed bug categories |
| `sprite.rs` | ASCII art sprites for companions and bugs |
| `ai.rs` | Free multi-provider AI client (Pollinations / Ollama / Gemini) |
| `profile.rs` | Player profile with XP, level, and statistics |
| `db.rs` | Turso persistence for game state |
| `schema.rs` | Database schema (V18) for all gamification data |

## Wiring (router vs CLI-only vs unwired)

| Area | Routed via `event_router` | CLI / MCP only | Unwired / experimental |
|------|---------------------------|----------------|-------------------------|
| Policy XP/crystals | All producers using `route_event` | — | — |
| Companions & quests | Subset of `process_event_rewards` match arms | `vox ludus companion|quest|battle` | — |
| Shop | — | `vox ludus shop`; MCP `vox_ludus_shop_*` | — |
| Collegium / arena | `collegium_joined` and related when emitted | `vox ludus collegium|arena`; MCP `vox_ludus_collegium_join` | Full arena loop is thin |
| **`ability` / `combat`** | Not driven by `route_event` today | N/A | **Product-complete in-crate; treat as legacy/experimental** until wired to the router or explicitly removed from public API. Prefer **not** exporting new surface that implies automatic rewards from these modules. |
| LSP / IDE | `diagnostics_clean` from `vox-lsp` + `vox check` (extras-ludus) | — | `completion_accepted` has no portable LSP server callback yet |

## CLI Commands

Requires building `vox-cli` with `--features extras-ludus`.

```bash
vox ludus status
vox ludus enable | vox ludus disable
vox ludus mode --effective
vox ludus mode --set balanced|serious|learning|off
vox ludus metrics
vox ludus digest
vox ludus profile-merge
vox ludus companion list
vox ludus quest list
vox ludus battle start
```

See also: [`ludus-integration-contract.md`](../architecture/ludus-integration-contract.md) and [`ludus-non-goals.md`](../architecture/ludus-non-goals.md) (optional UX, kill-switch, legacy `gamify_*` tables).

### MCP (Codex)

Canonical tools include **`vox_ludus_notifications_list`**, **`vox_ludus_progress_snapshot`**, **`vox_ludus_notification_ack`**, **`vox_ludus_notifications_ack_all`**, plus optional **`vox_ludus_quest_list`**, **`vox_ludus_shop_catalog`**, **`vox_ludus_shop_buy`**, **`vox_ludus_collegium_join`**, **`vox_ludus_battle_start`**, **`vox_ludus_battle_submit`** (see [`tool-registry.canonical.yaml`](../../../contracts/mcp/tool-registry.canonical.yaml)).

## Design

All features work offline with deterministic fallbacks. The AI client (`FreeAiClient`) attempts multiple providers in order and falls back to template-based responses if all providers are unavailable.
