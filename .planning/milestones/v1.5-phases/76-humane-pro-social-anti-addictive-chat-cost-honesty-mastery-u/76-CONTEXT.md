# Phase 76: Humane, Pro-Social & Anti-Addictive Chat, Cost Honesty & Mastery UX (HUMANE-01..04) - Context

**Gathered:** 2026-10-14
**Status:** Ready for execution
**Mode:** Grounded in `docs/src/architecture/humane-prosocial-anti-addictive-ai-tooling-research-2026.md`

<domain>
## Phase Boundary

Make Vox's chat (`vox chat` & `Loquela`), model routing, budget ledger (`vox-orchestrator::budget`), and developer progression (`vox-gamify`) explicitly anti-addictive, zero-markup, and pro-social—centered on functionality, compiler-verified accuracy, single-shot efficiency, architectural genius, and craft rather than extractive token churn, silent auto-reload billing, or FOMO streak anxiety.

</domain>

<decisions>
## Implementation Decisions

1. **HUMANE-01 (Zero-Markup Cost Honesty & Hard No-Auto-Reload Cap):**
   - Add `TurnCostReceipt` (`model_id`, `tier`, `prompt_tokens`, `cached_tokens`, `completion_tokens`, `upstream_cost_usd`, `vox_markup_usd: 0.0`, `saved_vs_frontier_usd`) in `crates/vox-orchestrator/src/budget/mod.rs`.
   - Enforce a hard stop + `DowngradeToFreeOrLocal` recommendation on `BudgetSignal::CostExceeded` (never auto-reload or bill silent overages).
   - Wire `TurnCostReceipt` footer and `--budget-usd` cap into `crates/vox-cli/src/commands/chat.rs`.
2. **HUMANE-02 (Anti-FOMO Progression & Mastery-Only Rewards):**
   - Set `message_sent` XP and crystals to `0` in `crates/vox-gamify/src/reward_policy.rs` and `contracts/gamify/economy.v1.yaml`.
   - Add `single_shot_verified` (`base_xp: 35, base_crystals: 8`) and `local_mens_verified` (`base_xp: 25, base_crystals: 6`).
   - Remove purchasable `ShopItem::StreakShield` from `crates/vox-gamify/src/shop.rs` (`default_catalog`).
   - Add `RestDayPolicy` (`WeekendsExempt` default + `rest_mode_until_ts`) to `StreakTracker` in `crates/vox-gamify/src/streak.rs`.
3. **HUMANE-03 (Anti-Sycophancy & Context Compaction UX):**
   - Inject the Anti-Sycophancy & Craft Directness Preamble into `vox chat` and `mens/config/system_prompt.txt`.
   - Populate pre-send cost/savings/cache transparency badges and a `> 24k`-token **Session Compact / Fresh Context** nudge in `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`.
4. **HUMANE-04 (Calm Tech & Cognitive Rest):**
   - Add `SessionClosureSummary` to `crates/vox-orchestrator/src/fatigue_monitor.rs` and render a calm, dismissible **Session Wrap-Up & Step Away** card in `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`.

</decisions>
