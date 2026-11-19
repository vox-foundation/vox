---
phase: 76
phase_name: "humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 76 Learnings: humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u

## Decisions

### Implement zero-markup BYOK/local cost honesty (TurnCostReceipt) and hard...
Implement zero-markup BYOK/local cost honesty (`TurnCostReceipt`) and hard No-Auto-Reload enforcement on `BudgetSignal::CostExceeded` in `crates/vox-orchestrator/src/budget/mod.rs` and `crates/vox-cli/src/commands/chat.rs` (`HUMANE-01`).

**Rationale:** Primary phase objective established in plan.
**Source:** 76-01-PLAN.md

---

### Eliminate dark-pattern engagement hooks in vox-gamify (HUMANE-02)
Eliminate dark-pattern engagement hooks in `vox-gamify` (`HUMANE-02`): zero out `message_sent` XP in `reward_policy.rs` and `contracts/gamify/economy.v1.yaml`, reward single-shot verified closure (`single_shot_verified`, `local_mens_verified`), remove purchasable `ShopItem::StreakShield` from `shop.rs`, and add `RestDayPolicy` (`WeekendsExempt` + `rest_mode_until_ts`) in `streak.rs`.

**Rationale:** Primary phase objective established in plan.
**Source:** 76-02-PLAN.md

## Lessons

## Patterns

## Surprises

