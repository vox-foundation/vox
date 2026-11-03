---
title: "Sustainability, Open Collective Operations, and Grant Funding Playbook (2026)"
description: "Canonical runbook for Vox project funding: Open Collective Stripe activation, honest tier and goal definitions, standalone crate adoption funnel, and ready-to-submit R&D grant proposals (NLnet Open Internet Stack, Rust Foundation, Mozilla, and Compute Grants)."
category: "Contributors"
status: "current"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "gemini-4-argon"
---

# Sustainability, Open Collective Operations, and Grant Funding Playbook (2026)

<!-- kb:summary -->
This document defines the canonical financial sustainability strategy and operational runbook for Vox and the in-formation Vox Foundation. It diagnoses the live Open Collective configuration (`opencollective.com/vox-foundation`), prescribes immediate zero-fee payment activation via GitHub Sponsors and Open Collective's Discover Organization tier, aligns sponsor tiers with repository reality, and provides ready-to-submit non-dilutive R&D grant proposals for NLnet (November 3, 2026 deadline), the Rust Foundation, Mozilla, and cloud compute credit programs.
<!-- /kb:summary -->

## 1. Executive Summary & Audit Baseline (2026-10-14)

Vox is a pre-1.0 open-source research language and AI development suite licensed under Apache-2.0. On 2026-10-14, an end-to-end audit of both the repository and the live Open Collective profile (`https://opencollective.com/vox-foundation`) identified five concrete blockers preventing external funding and early adopter traction:

1. **Disabled Payment Rails (`RECEIVE_FINANCIAL_CONTRIBUTIONS: "DISABLED"`):** On Open Collective, `vox-foundation` had `host: null`, `isActive: false`, and `isApproved: false`. Without either an approved Fiscal Host or a self-hosted Stripe connection under an Open Collective Organization, the public "Contribute" button is disabled.
2. **Open Source Collective (OSC) Fiscal Host Gate vs. Direct Stripe:** OSC (501(c)(6), 10% host fee) requires at least two active administrators/representatives and established community contributor activity (or 100+ GitHub stars for instant GitHub verification). Furthermore, following the December 2024 dissolution of the Open Collective Foundation (OCF 501(c)(3)), OSC contributions are not 501(c)(3) tax-deductible charitable donations in the United States.
3. **Profile Copy & Tier Mismatch:**
   - The Open Collective *About* blurb claimed enterprise production readiness, contradicting `README.md`'s candid pre-1.0 research framing.
   - The budget goal `"The First Aqueduct"` had `amount: 0` and an empty description.
   - The `Centurion Sponsor` tier promised logo placement in a `Centurion Sponsors` section of `README.md` that did not exist.
   - The profile omitted `website: https://voxlang.org` and used an anonymous `The Vox Foundation` user card instead of the maintainer's identity (**Bertrand Reyna-Brainerd**).
4. **Checkout Lock-In on `vox build`:** Generated `Cargo.toml` manifests required a local source clone of the Vox monorepo (`VOX_REPO_ROOT`) on disk, blocking external evaluators who install prebuilt binaries via `voxup`.
5. **Unpublished Standalone Crates:** Only 1 of 103 workspace crates (`vox-crypto`) passed `cargo publish --dry-run`, keeping reusable Rust infrastructure (`vox-arch-check`, `vox-secrets`, `vox-bounded-fs`) invisible to the wider `crates.io` ecosystem.

---

## 2. Payment Rail Math & Recommended Configuration

| Funding Rail | Platform / Host Fee | Stripe Processing (~2.9% + \$0.30) | Net Retained (per \$10,000) | Approval Gate | When to Use |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **GitHub Sponsors** (`brbrainerd` & `vox-foundation`) | **0%** | ~\$290 | **~\$9,710 (97.1%)** | Instant (solo maintainer friendly) | Primary developer & corporate sponsor rail integrated into GitHub's Sponsor button (`.github/FUNDING.yml`). |
| **Open Collective Organization — Discover Tier (Self-Hosted Stripe)** | **\$0/mo** (1 active collective, ≤10 expenses/mo) + **0% fee** (with donor optional platform tips) | ~\$290 | **~\$9,710 (97.1%)** | Instant (connect your own Stripe account; works with 1 admin) | Unlocks the `"Contribute"` button and transparent public ledger on `opencollective.com/vox-foundation` immediately without waiting for OSC. |
| **Open Source Collective (OSC 501(c)(6) Fiscal Host)** | **10% flat fee** (\$1,000) | ~\$290 | **~\$8,710 (87.1%)** | Requires ≥ 2 admins + multi-contributor history or 100+ GitHub stars | Apply only after adding a second co-admin/trustee or crossing 100 stars, and only when a corporate sponsor requires OSC vendor invoicing. |

---

## 3. Exact Step-by-Step Checklist to Activate `opencollective.com/vox-foundation`

Perform these steps in the Open Collective dashboard (`https://opencollective.com/dashboard/vox-foundation`):

### Step 1: Turn On Financial Contributions (0% Fee via Stripe)
1. Go to **Dashboard → Settings → Fiscal Host** (or **Receiving Money**).
2. Select **Manage our own funds (Independent / Organization — Discover \$0/mo plan)** and connect your Stripe account directly.
3. Verify that `RECEIVE_FINANCIAL_CONTRIBUTIONS` flips to **Active** and the public **Contribute** button renders on `https://opencollective.com/vox-foundation`.

### Step 2: Fix Identity & Links
1. **Admin Profile:** Log in with or invite your personal account (**Bertrand Reyna-Brainerd**, linking `https://github.com/brbrainerd` and `https://x.com/voxlang`) as Core Admin so grant reviewers see the real human researcher behind the project.
2. **Website URL:** Under **Settings → Info**, set **Website** to `https://voxlang.org`.

### Step 3: Replace the "About" Description (Copy-Paste Ready)
Replace the current generic enterprise blurb with this candid, reviewer-trusted description that matches `README.md`:

> **About Vox & The Vox Foundation**
>
> Vox ([voxlang.org](https://voxlang.org)) is an experimental, AI-native programming language and open-source development suite written in Rust (Apache-2.0).
>
> Mainstream web stacks force humans and AI coding agents to restate the same domain model four times across SQL migrations, backend structs, JSON wire schemas, and frontend TypeScript state. Vox collapses that fragmentation into a single `.vox` grammar (`table`, `query`, `mutation`, `server`, `component`, `workflow`, `tool`) with zero `null` values, mandatory compile-time error handling (`Result[T]` / `Option[T]`), capability-sandboxed script execution (`vox run --caps`), cryptographic HMAC receipts for MCP agent tool calls, and a four-tier retrieval quarantine (`SystemVerified` → `Untrusted`) in `vox-db`.
>
> **How Funds Are Used:**
> Every dollar in this Collective goes toward:
> 1. **Public Infrastructure (`$150/mo`):** Hosting the documentation book (`voxlang.org`), the deterministic browser playground (`eval.voxlang.org`), and cross-platform CI runners.
> 2. **Open Empirical Benchmarks (`$500/mo`):** API and GPU compute budgets to run and publish reproducible, pre-registered multi-model trials (`vox model eval-compare`) measuring whether Vox's grammar and compiler diagnostics measurably improve LLM `pass@k` and multi-turn repair over TypeScript and Python.
> 3. **Core Toolchain & Standalone Rust Crates:** Maintaining `vox-compiler`, `vox-arch-check`, `vox-secrets` (`Clavis`), and `vox-terminal-core`.

### Step 4: Replace the Broken `$0` Goal with Two Concrete Goals
Delete `"The First Aqueduct ($0)"` under **Settings → Goals** and add:

1. **Goal 1 — Public Playground & CI Infrastructure (`$150 / month`, recurring):**
   - *Description:* "Covers hosting for the public `/v1/eval` deterministic browser sandbox (`eval.voxlang.org`), `voxlang.org` documentation infrastructure, and cross-platform release artifact storage."
2. **Goal 2 — Open Cross-Model Compiler Benchmark Trials (`$500 / month`, recurring):**
   - *Description:* "Funds seeded, multi-turn compiler-feedback evaluation runs (`vox model eval-compare` with McNemar, Holm-Bonferroni, and paired bootstrap tests) comparing `.vox` against TypeScript and Python across frontier and open-weights models, published openly with cryptographic provenance bundles."

### Step 5: Align Contribution Tiers with `README.md`
Under **Settings → Tiers**:
1. **Enable the Default Flexible Tier ("Custom Contribution"):** Allow one-time or monthly donations of any amount starting at `$5`.
2. **Plebeian Supporter (`$5 / month`):** "Supports public playground hosting and CI infrastructure. Listed in the `Sponsors & Backing` section of `README.md`."
3. **Centurion Sponsor (`$50 / month`):** "For indie teams and AI engineers backing verifiable agent tooling and standalone Rust crates (`vox-arch-check`, `vox-secrets`). Includes logo/link placement under `Centurion Sponsors` in `README.md`."
4. **Senator of the Republic (`$500 / month`):** "For organizations funding open AI-compiler research, benchmark campaigns, and security infrastructure. Includes prominent logo placement in `README.md` and `voxlang.org`, plus quarterly architecture syncs."

---

## 4. Non-Dilutive R&D Grant Proposals (Ready to Submit)

Pre-1.0 compilers and verification toolchains raise 10x more funding from R&D grants and compute credits than from tip jars. Crucially, **grant applications must pitch bounded, verifiable technical subsystems** rather than the entire 100+ crate monorepo.

### 4.1 NLnet Foundation — Open Internet Stack Call (Next Deadline: November 3, 2026)

- **Target Grant Size:** €25,000 – €50,000 (paid directly to individuals upon milestone completion; no 501(c)(3) or fiscal host required).
- **Critical Framing Rule:** NLnet's *Open Internet Stack* call (which succeeded the NGI Zero Commons Fund after June 2026 on an odd-month 3rd deadline cadence: Nov 3, Jan 3, Mar 3) **explicitly excludes generic generative AI model training or LLM wrappers**, but **strongly funds open-source compilers, capability-based sandboxes, formal/mechanical verification, and cryptographic provenance**.
- **Recommended Proposal Title:** *Vox Safe-Exec & Provenance: Capability-Sandboxed Script Execution, Cryptographic Tool Receipts, and Retrieval Quarantine for Local-First Software*

#### Copy-Paste NLnet Abstract (Proposal Form §Abstract)
> Software developers increasingly rely on automated scripts, Model Context Protocol (MCP) tools, and external documentation indices, yet current toolchains execute glue scripts with ambient OS authority and ingest unverified web content directly into local databases without provenance boundaries.
>
> This project delivers three standalone, Apache-2.0 open-source components written in Rust from the Vox toolchain ecosystem:
> 1. **Capability-Sandboxed Deterministic Interpreter (`vox run --interp --caps`) & Shell Grammar Verifier (`vox-container-types::exec_grammar`):** A zero-ambient-authority execution tier for `.vox` automation scripts and shell pipelines that enforces fine-grained filesystem, network, process, and determinism capabilities at the language and AST boundary without requiring heavy container or microVM daemons.
> 2. **Cryptographic Execution Receipts (`vox-crypto` / ADR-029):** A tamper-evident BLAKE3 + Ed25519/HMAC receipt ledger around local tool and terminal execution (`vox-terminal-core` & `vox-orchestrator-mcp`), ensuring every automated side effect produces an offline-verifiable cryptographic claim bundle.
> 3. **Four-Tier Epistemic Quarantine (`vox-db` & `vox-kb-facets`):** A local-first SQLite/Turso (`libSQL`) storage and Tantivy search layer that enforces strict trust-tier separation (`SystemVerified`, `Trusted`, `Advisory`, `Untrusted`), quarantining unattested external inputs in a review inbox (`kb_findings`) with deterministic SKOS taxonomy validation (`vox-doc-pipeline`).
>
> All components will be published as standalone, documented Rust crates on `crates.io` (`vox-crypto`, `vox-bounded-fs`, `vox-secrets`, `vox-kb-facets`, `vox-arch-check`) alongside cross-platform binary releases (`voxup`) and reproducible verification suites.

#### Proposed NLnet Milestone Breakdown (€35,000 Total)
| Milestone | Deliverable | Verification Metric | Budget |
| :--- | :--- | :--- | :--- |
| **M1: Standalone Crate Extraction & Supply-Chain Provenance** | Publish `vox-crypto`, `vox-bounded-fs`, `vox-scaling-policy`, `vox-secrets` (`Clavis`), `vox-kb-facets`, and `vox-arch-check` to `crates.io` with CycloneDX SBOMs and SHA-256 release manifests (`CR-K1..K7`). | `cargo publish --dry-run` + `voxup install` smoke tests pass across Linux/macOS/Windows. | **€8,000** |
| **M2: Capability-Sandboxed Interpreter & Pure-Rust Shell Policy Gate** | Complete behavioral golden verification (`CR-F1`) for `--caps deterministic` / `--caps readonly` in `vox-compiler` and pure-Rust AST command policy verification (`vox shell check --backend grammar`). | 100% behavioral golden coverage (`golden_behavioral_gate`) + zero ambient FS/net escapes under property tests. | **€10,000** |
| **M3: Cryptographic Execution Receipts & Offline Verifier** | Ship offline bundle verification for HMAC/Ed25519 tool and terminal execution receipts (`vox_verify_task_claims` & `vox-terminal-core` `BlockOrigin` provenance). | Tamper-injection mutation suite (`Phase 11` / `Phase 12` safety floor closures) passes 100%. | **€9,000** |
| **M4: Epistemic Quarantine & Deterministic Documentation Verification** | Complete `vox-db` four-tier retrieval quarantine (`kb_findings`), SKOS facet validator, and symbol-hash staleness detector (`vox-graph-reader` v5). | Full external security/reproducibility audit + public benchmark report under CC-BY-4.0 / Apache-2.0. | **€8,000** |

---

### 4.2 Rust Foundation — Project Grant / Fellowship Pitch

- **Target Grant Size:** \$5,000 – \$20,000.
- **Core Angle:** Pitch **`vox-arch-check`** and **`vox-secrets` (`Clavis`)** as standalone tools for large Rust workspaces (solving Cargo workspace layer violations, fan-in explosions, god-object thresholds, and environment-variable secret sprawl).
- **Prerequisite (Phases 56–57):** Complete the 7-crate `crates.io` publish set so Rust Foundation reviewers can run `cargo install vox-arch-check` on any Rust repository.

---

### 4.3 AI & Cloud Compute Credit Grants (OpenRouter, Anthropic, Together, Modal, Lambda)

- **Target Value:** \$2,500 – \$25,000 in API/GPU credits.
- **Pitch:** *Pre-Registered Empirical Evaluation of Programming Language Grammar & Compiler Diagnostics on LLM Multi-Turn Repair (`Pass@k` & Token Efficiency).*
- **Why It Wins:** You have already built the statistical evaluation harness (`eval-corpus` and `vox model eval-compare` using seeded nucleus sampling, multi-turn compiler feedback, QLoRA adapter lineage cards, McNemar's test, Holm-Bonferroni family-wise error control, and paired bootstrap confidence intervals). Compute providers actively sponsor open benchmark datasets that evaluate their models objectively.
