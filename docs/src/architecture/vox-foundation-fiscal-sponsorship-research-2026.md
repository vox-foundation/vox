---
title: "Vox Foundation Fiscal Sponsorship & Legal Charter Deep Research (2026)"
description: "Adversarially audited evaluation of chartering Vox Foundation via HCB (The Hack Foundation) versus Open Source Collective, SPI, NumFOCUS, Model C sponsors, GitHub Sponsors, and independent 501(c)(3) incorporation alongside NextGen Interactive LLC."
category: "Architecture SSOTs"
status: "research"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-generated"
authorship_model: "google-deepmind-agent"
training_eligible: false
---

# Vox Foundation Fiscal Sponsorship & Legal Charter Deep Research (2026)

<!-- kb:summary -->
This adversarially audited report evaluates chartering the in-formation Vox Foundation under HCB (The Hack Foundation, EIN 81-2908499) versus Open Source Collective, GitHub Sponsors, SPI, NumFOCUS, Model C fiscal sponsors, and independent 501(c)(3) incorporation alongside NextGen Interactive LLC. Combining live browser verification of HCB and Open Collective with 2026 policy, fee, IRS Model A asset-lock, and codebase surface audits, it documents eligibility gates, trademark and domain separation requirements, and a staged charter roadmap.
<!-- /kb:summary -->

> **Scope & Evidence Discipline:** Research reflects live browser verification (`hackclub.com`, `help.hcb.hackclub.com`, `hcb.hackclub.com/applications/new`, and `opencollective.com/vox-foundation`), IRS/ProPublica records, fiscal sponsor policies, and workspace audits performed on `2026-11-18`. This report does **not** alter `README.md`, `docs/src/index.mdx`, `.github/FUNDING.yml`, or `docs/src/contributors/sustainability-and-funding-ssot-2026.md`; those files remain untouched pending a charter decision. Nothing herein constitutes formal legal or tax counsel.

---

## 1. Executive Verdict: Should Vox Charter via HCB ("Step 3")?

**Verdict: Conditionally Viable — Do NOT Apply Blindly as a Self-Serve Step.**

On pure financial mechanics, **HCB (The Hack Foundation, EIN `81-2908499`)** beats **Open Source Collective (`501(c)(6)`)** on fees (`7%` flat absorbing Stripe card processing vs. `10%` plus card/payout fees) and unlocks U.S. `IRC § 170` charitable tax deductions without recruiting a board or filing `Form 1023`. However, three concrete legal and policy catches require **explicit pre-clearance and structural separation** before routing Vox or `voxlang.org` assets into HCB:

1. **Active Policy Contradiction on Adult-Led Intake:** HCB's June 6, 2026 Help Center eligibility rule states that HCB *"is exclusively focused on teen-led organizations, and only accepts new groups that are run by teenagers"* (ages 13–18), whereas the live Step 1 application screen at `https://hcb.hackclub.com/applications/new` displays `"Are you a teenager? *"` with `"No, I'm an adult (Over 18)"` and helper text *"HCB primarily focuses on teen-led projects, so we're more selective with adult-run organizations."* Applying without prior written clearance from `hcb@hackclub.com` carries high rejection risk for an adult solo maintainer.
2. **Model A Permanent Charitable Asset Lock vs. `NextGen Interactive LLC`:** HCB operates under **Model A (Comprehensive / Direct Project)** fiscal sponsorship. "Vox Foundation" does not exist as a separate legal entity under HCB; it is an internal restricted program of The Hack Foundation. Under `Treas. Reg. § 1.501(c)(3)-1(b)(4)`, funds, hardware, cloud accounts, domains (`voxlang.org`), or trademarks acquired or created under the Model A umbrella are permanently dedicated to `501(c)(3)` charitable purposes and **cannot** revert to Bertrand personally or to `NextGen Interactive LLC`—they may only spin out to another verified `501(c)(3)` organization.
3. **Founder & Related-Party Compensation Firewall (`IRC § 4958`):** Because Bertrand leads Vox and owns `NextGen Interactive LLC`, the LLC is a `35%`-controlled **disqualified person** (`Treas. Reg. § 53.4958-3`). HCB prohibits arbitrary self-payouts, requires itemized receipts within 7 days for card charges, and cannot safely contract with `NextGen Interactive LLC` without an independent board comparability safe harbor (`Treas. Reg. § 53.4958-6`) and strict prevention of private commercial benefit.

**Recommended Strategy:** Keep **HCB conditional** on written confirmation of adult FOSS eligibility and Fiscal Sponsorship Agreement review (`hcb@hackclub.com`), while relying immediately on **GitHub Sponsors Direct (`0%` personal fee)** + **Open Collective Self-Hosted Stripe (`0%` platform fee, already blueprinted in [sustainability-and-funding-ssot-2026.md](../contributors/sustainability-and-funding-ssot-2026.md))** and direct non-charity-gated R&D grants (**Sovereign Tech Fund** / **NLnet Open Internet Stack**). Use **Model C** (e.g., Social Good Fund) if `501(c)(3)` status is required without Model A IP lock, and graduate to an independent **Vox Foundation Inc.** (`Form 1023-EZ` or `Form 1023`, following the **Zig Software Foundation** model) once `≥2` unrelated independent directors and steady funding justify incorporation.

---

## 2. Claim-by-Claim Audit of the "Step 3" Proposal

| Claim in Proposal | Audit Status | Verified 2026 Reality & Nuances | Primary Evidence |
| :--- | :--- | :--- | :--- |
| **Open Source Collective is a `501(c)(6)` trade group taking `10%` + `~3.5%` card fees with `$0` charitable tax deduction** | **Verified (with nuance)** | OSC is a `501(c)(6)` nonprofit charging a `10%` host fee plus separate Stripe/PayPal/Wise fees (`~2.9% + $0.30` card rate, plus up to `6%` on GitHub Org sponsors). Individual donations get **zero `IRC § 170` charitable deduction**, *but* corporate sponsors may deduct payments as `IRC § 162` business expenses, and OSC leaves IP/trademarks with the project. | `docs.oscollective.org`, `sustainability-and-funding-ssot-2026.md` |
| **Incorporating your own nonprofit requires recruiting a board and filing IRS Form 990s** | **Verified (with nuance)** | U.S. nonprofit incorporation typically requires `≥3` directors with an unrelated independent majority and `Form 1023-EZ` (`$275` if `≤$50k/yr` receipts & `≤$250k` assets) or `Form 1023` (`$600`). However, projects under `$50,000/yr` file the free, 8-field **Form 990-N (e-Postcard)** rather than full Form 990 schedules. | `irs.gov`, `ziglang.org/zsf` |
| **Vox operates legally under `501(c)(3)` EIN `81-2908499`, separate from `NextGen Interactive LLC`, with 100% tax-deductible donations/grants** | **Verified with Legal Caveats** | **The Hack Foundation** (EIN `81-2908499`, IRS ruling Dec 2016; FY24 revenue `$22,351,707`, net assets `$18,308,371`) is an active `IRC §§ 509(a)(1)` / `170(b)(1)(A)` public charity. Donations are `§ 170`-deductible. **Caveat:** Under Model A, Vox Foundation is not a separate legal entity—assets sit inside The Hack Foundation, triggering charitable lock and `IRC § 4958` related-party constraints toward `NextGen Interactive LLC`. | ProPublica `812908499`, `the.hackfoundation.org`, `help.hcb.hackclub.com` |
| **HCB explicitly accepts builder-led open-source projects, including a "team of one"** | **Outdated / Policy-Gated** | Neither `hackclub.com/fiscal-sponsorship` nor `/fiscal-sponsorship/open-source` mentions `"team of one"`. HCB onboarded adult FOSS projects like `htop` and `PCP` during the 2024 OCF shutdown and marketing copy still references *"hundreds of adult-run organizations such as mutual aid groups, open source projects, and community spaces"* (plus an AI safety fast-track at `hcb@hackclub.com`). Yet the June 6, 2026 Help Center eligibility page states HCB **only accepts new groups run by teenagers (13–18)**, and the live form warns *"we're more selective with adult-run organizations."* | `help.hcb.hackclub.com/en/articles/15409923`, `hcb.hackclub.com/applications/new`, `daniel-lange.com` |
| **HCB charges `7%` flat with `$0` startup, `$0` monthly, `$0` credit card processing fees, and files Form 990s** | **Verified (with GitHub Org & FX caveats)** | Verified: `7%` flat on incoming revenue (`93%` net), `$0` startup/monthly/card-issuing/check-deposit/ACH fees, Stripe card fees absorbed inside the `7%`, and Form 990s handled by The Hack Foundation. **Caveats:** GitHub Org sponsors still incur up to `6%` on GitHub's side; international Wise transfers incur Wise FX fees (`~3%–5%`); non-Wise international wires carry a `$500` minimum until `$50k+` raised. | `hackclub.com/fiscal-sponsorship`, `help.hcb.hackclub.com/en/articles/15410000`, `16609038` |
| **Use HCB virtual Visa cards for domains/servers/CI or reimburse out-of-pocket receipts via ACH** | **Verified (with 7-Day Receipt & Title rules)** | Virtual/physical HCB Visa Commercial cards (Stripe + Celtic Bank, `$0` FX fee) and ACH reimbursements (`~2` business days after org manager + HCB review) work as stated. **Caveats:** Itemized receipts are mandatory (lost receipts are non-reimbursable; Visa receipts are due within **7 days of settlement** or cards lock), and assets/domains purchased under Model A legally belong to The Hack Foundation. | `help.hcb.hackclub.com/en/articles/15410232`, `15409924` |
| **Keep Vox funds strictly for FOSS mission; never mix commercial SaaS revenue into HCB** | **Verified (Mandatory IRS Law)** | Mixing commercial SaaS receipts or private business expenses into a `501(c)(3)` fund violates `Treas. Reg. § 1.501(c)(3)-1(d)(1)(ii)` (private benefit / inurement) and risks `IRC § 4958` penalties or sponsorship termination. | `IRC § 4958`, `Treas. Reg. § 1.501(c)(3)-1` |

---

## 3. Live Browser Verification Findings

Live inspection confirmed all four target origins (`https://hackclub.com`, `https://help.hcb.hackclub.com`, `https://opencollective.com`, and `https://hcb.hackclub.com`) with zero blocked origins:

1. **Marketing & Open-Source Pages (`hackclub.com/fiscal-sponsorship` & `/fiscal-sponsorship/open-source`):**
   - Verbatim fee promise: *"One simple, transparent fee: 7% of revenue."* and *"No legal fees. No startup fees. No transaction fees. No card issuing fees. No subscription fees. No check deposit fees. No credit card processing fees."*
   - Youth framing sits directly alongside legacy broad claims: footer reads *"For teens, by teens"*, and the `7%` fee explicitly pays for *"Hack Club's operations staff, including teen interns working under mentors"*, while copy still references adult-run open source projects and AI safety fast-tracking at `hcb@hackclub.com`.
   - The March 29, 2025 announcement at `/fiscal-sponsorship/open-source` open-sourced HCB's own Ruby on Rails platform under **AGPL** (`github.com/hackclub/hcb`) and linked `nonprofit.new`, without establishing a dedicated FOSS-project intake track or mentioning `"team of one"`.
2. **HCB Help Center (`help.hcb.hackclub.com`, verified June 2026 articles):**
   - **`Who can apply for fiscal sponsorship?` (`articles/15409923`, June 6, 2026):** Explicitly restricts new intake to teenagers ages `13–18` (no under-13 accounts even with parental consent) and classifies adult-led groups as out of mission focus (*"HCB is exclusively focused on teen-led organizations, and only accepts new groups that are run by teenagers"*). Also excludes India-based orgs and limits support to English.
   - **`What is the fiscal sponsorship agreement?` (`articles/15409924`, June 6, 2026):** Confirms pre-existing IP/code remains under existing licensing, while **IP created with `501(c)(3)` donor funds is held under The Hack Foundation's EIN** and can transfer on exit **only to another `501(c)(3)` successor**.
   - **`How do I request a reimbursement?` (`articles/15410232`, June 10, 2026):** Supports ACH, check, Wise, and wire payouts; targets `~2` business days post-manager approval; strictly forbids reimbursing without itemized receipts.
   - **`Collecting donations with GitHub Sponsors` (`articles/16609038`) & Donation Pages (`articles/15410000`, June 12, 2026):** Confirms `7%` all-in with optional `"donors cover the fees"` toggle; HCB must be selected as Fiscal Host during GitHub Sponsors setup/onboarding.
3. **Open Collective Profile (`https://opencollective.com/vox-foundation`):**
   - Loads `HTTP 200` as an **active but unhosted `COLLECTIVE`** (`Vox Foundation - Open Collective`).
   - Shows **zero fiscal host**, **zero budget/balance**, **zero contribution tiers**, and no public `"Contribute"` button. Only `Contributors` (`1`: `The Vox Foundation` listed as `Admin`), `About`, and `Submit Expense` tabs appear, linking out to `github.com/vox-foundation/vox` and `x.com/voxlang`.
   - This matches the prior October 2026 diagnosis in [sustainability-and-funding-ssot-2026.md](../contributors/sustainability-and-funding-ssot-2026.md).
4. **Live HCB Application Form (`https://hcb.hackclub.com/applications/new` — Active Open Tab):**
   - No login/email wall gates Step 1. Banking/sponsor disclosures identify **Column N.A., Member FDIC** as banking partner and offer `$50k+` expedited onboarding via `meagan@hackclub.com`.
   - Step 1 requires answering `"Are you a teenager? *"` (`Yes, I'm a teenager (18 and under)` vs. `No, I'm an adult (Over 18)`), displaying: *"HCB primarily focuses on teen-led projects, so we're more selective with adult-run organizations."* No Fiscal Sponsorship Agreement link appears on Step 1 itself; the full agreement is only surfaced inside onboarding or by emailing `hcb@hackclub.com`.

---

## 4. Model A vs. Model C vs. 501(c)(6) vs. Independent 501(c)(3): IP, Trademark & LLC Firewall

### 4.1 Code Copyrights, Apache-2.0, and Model A Asset Lock

Currently, root `LICENSE` uses standard **Apache-2.0** (inbound=outbound Section 5 grant with no separate CLA or DCO), `NOTICE` holds `Copyright 2024-2026 Bert Brainerd`, and `Cargo.toml` sets workspace `authors = ["Bert Brainerd"]`. Under HCB's Model A rules:

- **Pre-existing code & Apache-2.0:** Existing repositories keep their Apache-2.0 public license, so neither HCB nor `NextGen Interactive LLC` loses the right to use or build commercially on Apache-2.0 code.
- **Donor-funded IP & physical/cloud assets:** Under Model A (`Rev. Rul. 68-489` and `Treas. Reg. § 1.501(c)(3)-1(b)(4)`), assets purchased with HCB funds (hardware, Hetzner/Coolify VPS accounts paid directly by HCB cards, paid contractor IP without carve-out, and registered domains) legally belong to **The Hack Foundation**.
- **Irreversible exit lock:** Leaving HCB requires incorporating an independent `501(c)(3)` and obtaining an IRS Determination Letter before accumulated funds or Model A assets can transfer out. **Zero funds or HCB-owned assets can ever be transferred to `NextGen Interactive LLC` or Bertrand personally.**

### 4.2 Trademarks (`Vox`, `VoxLang`) & Domains (`voxlang.org`)

Paying for `voxlang.org`, `eval.voxlang.org`, or `telemetry.voxlang.org` registration/renewals or USPTO trademark filings directly from an HCB virtual card risks vesting legal title to the domain or trademark inside The Hack Foundation's Model A estate. To preserve strategic flexibility alongside `NextGen Interactive LLC`:

- **Do NOT transfer ownership of `voxlang.org` or the `Vox` / `VoxLang` trademarks into a Model A sponsor** unless you intend those marks to remain permanently bound to `501(c)(3)` charitable ownership and inspect a written carve-out in HCB's Fiscal Sponsorship Agreement first.
- **Safer boundary while sponsored:** Retain personal or `NextGen Interactive LLC` title to existing domain registrations (or transfer them directly to an independent `Vox Foundation Inc.` once incorporated), and allow the Foundation project to use `eval.voxlang.org` / `telemetry.voxlang.org` under a documented royalty-free non-exclusive license—reimbursing only discrete hosting/compute bills (`Coolify`/`Hetzner`/`Cloudflare` public-tier usage) cleanly attributable to FOSS operations.

### 4.3 `NextGen Interactive LLC` & Founder Self-Compensation Rules

Under **IRC § 4958 (Excess Benefit Transactions)**, any entity `>35%` owned by a person with substantial influence over a sponsored `501(c)(3)` project is a **disqualified person (DQP)**:

- **Self-reimbursement vs. Self-compensation:** Reimbursing verified, third-party open-source infrastructure receipts (cloud hosts, DNS, public CI runners, benchmark API tokens) via ACH is low-risk because zero net personal profit accrues. By contrast, paying founder wages or contracting `NextGen Interactive LLC` for engineering hours from `501(c)(3)` HCB funds carries severe compliance risk (`25%` initial excise tax, `200%` uncorrected penalty) unless pre-approved under `Treas. Reg. § 53.4958-6` by an independent, conflict-free body of The Hack Foundation using market comparability data and concurrent written minutes—something fiscal hosts rarely undertake for early solo maintainers.
- **Operational firewall:** Never route commercial SaaS revenues, proprietary FableForge workloads (`docs/src/ci/deploy-contract.md` already separates `Vox Ecosystem` from `FableForge Production` in Coolify), or commercial consulting invoices through a `501(c)(3)` fiscal sponsor.

---

## 5. 2026 Head-to-Head Comparison Matrix

| Structure / Option | Legal & Tax Status | Upfront Cost | Ongoing Fee + Processor Cut | `IRC § 170` Tax Deduction | Adult Solo Eligibility (2026) | IP & Trademark Retention | Self-Pay / LLC Compatibility | Admin Burden |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **HCB (The Hack Foundation)** | `501(c)(3)` Model A | `$0` | `7%` flat (CC fees absorbed; Wise `~3%–5%` FX) | **Yes** | **Policy-Gated** (Help Center says 13–18 only; form says *"more selective"*) | **Low** (Model A vests donor-funded IP/assets in Hack Foundation) | **Very Low** (receipt reimbursements OK; LLC contracting blocked/high-risk) | Low (upload receipts in 7d) |
| **GitHub Sponsors Direct** (Personal / SMLLC) | Taxable Schedule C | `$0` | `0%` personal; up to `6%` org sponsors | **No** (corp `§ 162` business expense only) | **100% Open** (immediate) | **100% Retained** | **Full** (offset gross receipts with `§ 162` compute/CI expenses) | Very Low (`1099-NEC` at `$2,000+` in 2026) |
| **Open Collective — Discover Self-Hosted Stripe** | Unincorporated / Org (`0%` tier) | `$0/mo` (`≤10` exp/mo) | `0%` platform + Stripe (`~2.9% + $0.30`) | **No** | **100% Open** (1 admin) | **100% Retained** | **Full** (via connected personal/LLC Stripe) | Low (public ledger) |
| **Open Source Collective (OSC)** | `501(c)(6)` | `$0` | `10%` host + Stripe/Wise/PayPal fees | **No** (`§ 162` corporate deduction only) | **Low–Med** (requires GitHub Org, `100+` stars, `≥2` contributors/admins; discourages solo-pay-self) | **High** (project keeps IP; optional OSC mark holding `$1,200` reg / `$350` transfer) | **Low** (disfavors paying sole admin) | Low–Med |
| **SPI (Software in the Public Interest)** | `501(c)(3)` | `$0` | `5%` + processor/wire fees | **Yes** | **Low** (requires *"genuine, substantial, significant"* multi-person community) | **Medium** (SPI holds marks/domains in trust; liaison directs) | **Low–Med** (independent governance expected) | Medium |
| **NumFOCUS** | `501(c)(3)` | `$0` | **PAUSED** (historical `7%` `<$150k`; `0%` Affiliated) | **Yes** (Sponsored only) | **Closed (2026 Pause)** + requires `>1` contributor & scientific/ML scope | Sponsored: held in trust; Affiliated: retained | **Low** | Medium |
| **Software Freedom Conservancy (SFC)** | `501(c)(3)` | `$0` | `10%` gross revenue | **Yes** | **Very Low** (favors multi-person PLC; rejects sole-commercial-interest setups) | **Low** (Model A vests trademarks & goodwill in SFC) | **Very Low** | Medium |
| **Commonhaus / Linux Foundation / LF Charities** | `501(c)(6)` (Commonhaus, LF) / `509(a)(3)` (LF Charities) | Varies | `9%` `<$1M/yr` (LF); `0%` LFX Crowdfunding; custom (Commonhaus) | Commonhaus/LF: **No**; LF Charities: **Yes** | **Very Low** (Commonhaus targets mature multi-maintainer libs; LF expects `$50k–$200k+` corp budgets) | **Low** (trademark assignment required) | **Low** | High |
| **Model C `501(c)(3)` Sponsor** (e.g., Social Good Fund, PPF, CI) | `501(c)(3)` grantor (`Rev. Rul. 68-489`) | `$0` (`$29/mo` refundable at SGF) | `6.5%` SGF (`<$500k`), `6%` PPF (`$30k` min), `10%` CI (`$50k` min) | **Yes** (to sponsor; no direct TechSoup) | **Adult-Eligible** (subject to sponsor mission review & annual mins) | **High** (separate entity/LLC retains IP & marks) | **Viable w/ Firewall** (LLC grants strictly bound to 100% Apache-2.0 deliverables) | Medium |
| **Direct R&D Grantors** (NLnet OIS / Sovereign Tech Fund) | Direct Contract / Grant (`0%` fee) | `$0` | `0%` | N/A | **Open** (STF `>€50k` accepts US LLCs/nonprofits; NLnet OIS odd-month 3rds accepts US w/ *"exceptional"* EU angle) | **100% Retained** (Apache-2.0 outputs) | **Full** (STF pays commercial LLCs directly for FOSS infra contracts) | Medium (milestones) |
| **Independent `501(c)(3)` ("Vox Foundation Inc.")** | Independent `501(c)(3)` Public Charity | `$275` (`1023-EZ`) or `$600` (`Form 1023`) + state fee | `$0` platform (`Form 990-N` `$0` if `≤$50k/yr`) | **Yes** | **High** (needs `≥3` directors w/ unrelated majority & `§ 509(a)(1)` `33⅓%` public support) | **Maximum** (Foundation owns marks; passive FMV royalty license to LLC per `§ 512(b)(2)`) | **Controlled** (disinterested board sets FMV comp/licenses per `§ 4958`) | High (annual filings, board minutes) |

---

## 6. Recommended Staged Charter Roadmap for Vox Foundation

Rather than forcing an unvetted HCB application or premature `501(c)(3)` incorporation before multi-contributor scale exists, Vox should execute a three-stage legal and funding progression:

```vox-diagram
graph {
    direction: right

    node s0: "Stage 0: GitHub Sponsors 0% + OC Stripe 0% + STF/NLnet"
    node s1a: "Stage 1A: HCB Pre-Clearance (Adult FOSS + IP Carve-Out)"
    node s1b: "Stage 1B: Model C 501(c)(3) Sponsor (Apache-2.0 Wall)"
    node s2: "Stage 2: Independent Vox Foundation Inc. (1023-EZ / 1023)"

    s0 -> s1a: "US 501(c)(3) donor asks"
    s0 -> s1b: "HCB declines or IP lock risk"
    s0 -> s2: "STF/corp funding scales directly"
    s1a -> s2: "2+ unrelated directors and >$25k-$50k/yr"
    s1b -> s2: "2+ unrelated directors and >$25k-$50k/yr"
}
```

### Stage 0 — Immediate Zero-Friction Baseline (`$0` Legal Overhead)
- **Activate GitHub Sponsors Direct (`0%` personal fee) and/or Open Collective Discover Self-Hosted Stripe (`0%` platform fee):** Resolves the live unhosted state of `https://opencollective.com/vox-foundation` without waiting for OSC's `100+` stars / `2+` admins gate or HCB's teen-leadership exception. Gross receipts go to Schedule C (`NextGen Interactive LLC` / sole proprietor) and are offset dollar-for-dollar by `§ 162` infrastructure costs (`eval.voxlang.org`, CI runners, benchmark API tokens).
- **Target Non-501(c)(3)-Gated R&D Grants:**
  - **Sovereign Tech Fund (`>€50,000`, rolling):** Directly contracts with **US LLCs** (`NextGen Interactive LLC`) for foundational open-source infrastructure (`vox-compiler`, `vox-crypto`, `vox-arch-check`) under Apache-2.0.
  - **NLnet Open Internet Stack (`€5k–€50k` initial, odd-month 3rd deadlines):** Note that the November 3, 2026 deadline noted in [sustainability-and-funding-ssot-2026.md](../contributors/sustainability-and-funding-ssot-2026.md) has passed; the next cycles fall on **January 3, 2027** and **March 3, 2027**. Frame proposals around capability sandboxing, cryptographic receipts, and European open-standards adoption to clear the non-EU *"exceptional"* bar.

### Stage 1 — Conditional `501(c)(3)` Bridge (Only When Needed Before Board Formation)
- **Option 1A — HCB with Written Pre-Clearance:** If an institutional US donor or foundation specifically requires a `501(c)(3)` EIN (`81-2908499`) today, email `hcb@hackclub.com` **before** counting on approval, obtain written confirmation of an adult FOSS exception plus inspect the Fiscal Sponsorship Agreement's IP/domain clauses, and never purchase `voxlang.org` domain titles or commercial assets through HCB.
- **Option 1B — Model C Bridge (`Rev. Rul. 68-489`):** If HCB declines adult-led intake or if Model A IP lock conflicts with `NextGen Interactive LLC`, apply to an adult-eligible Model C sponsor (e.g., Social Good Fund at `6.5%` + `$29/mo` refundable) with grant contracts strictly scoping funded deliverables to `100%` Apache-2.0 public outputs.

### Stage 2 — Permanent Endgame: Independent `501(c)(3)` (`Vox Foundation Inc.`, ZSF Model)
- Mirror the **Zig Software Foundation (`EIN 84-5105214`)** blueprint once Vox sustains **`~$25k–$50k+/yr`** in sponsorships/grants and recruits **at least 2 unrelated independent directors** alongside Bertrand (3-director board minimum).
- Choose **IRS Form `1023-EZ` (`$275`)** only if Years 1–3 projected gross receipts stay `≤$50,000/yr` (`Form 990-N` `$0` annual filing); file full **Form `1023` (`$600`)** if pursuing a `$50k+` Sovereign Tech Fund or corporate grant in Years 1–3.
- House the `Vox` / `VoxLang` trademarks and `voxlang.org` inside `Vox Foundation Inc.`, execute an **IRS Appendix A Conflict-of-Interest Policy**, and keep `NextGen Interactive LLC` cleanly walled off through Apache-2.0 usage and (if branded commercially) a disinterested-board-approved, arm's-length **passive FMV trademark license** (`IRC § 512(b)(2)` / `§ 4958`).

---

## 7. Vox Repository Touchpoint Audit (Deferred Until Charter Choice)

No repository files were edited during this research pass. Once a legal charter path is chosen, align these audited surfaces:

- **Stewardship & Funding Claims:** `README.md:32,237,453,455,460`, `docs/src/index.mdx:469`, `.github/FUNDING.yml:1-2`, `docs/src/contributors/sustainability-and-funding-ssot-2026.md:15-93`, `docs/src/architecture/front-facing-honesty-audit-2026.md:41,54`, and `docs/src/architecture/v1-release-criteria.md:10,312` (removing `"in formation"` or replacing with the exact `501(c)(3)` fiscal sponsorship attribution if HCB/Model C is adopted).
- **Copyright, Authorship & Contributor Governance:** `NOTICE:1-5`, `Cargo.toml:28-30`, `CITATION.cff:12-16`, `CONTRIBUTING.md:1-95` (no CLA/DCO currently), `SECURITY.md:3,22` (`security@vox-foundation.org`), `CODE_OF_CONDUCT.md:63` (`conduct@vox-foundation.org`), and `.github/CODEOWNERS:3-4,9-35` (currently placeholder `@vox-foundation/maintainers`).
- **Operational Infrastructure & Ed25519 Key Custody:** `docs/src/ci/deploy-contract.md:86-118` (Coolify separation between `Vox Ecosystem` and `FableForge Production`; `telemetry.voxlang.org` currently dormant/NXDOMAIN), `.github/workflows/deploy-telemetry.yml:3-4,77,96`, `.github/workflows/docker-eval.yml:23-51` & `docs/src/architecture/eval-sandbox-deployment.md:21,35,97` (currently `ghcr.io/brbrainerd/vox-eval`), and `docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md:29,102,161,189,209,212,306` (Foundation Ed25519 bundle signing keys and `brbrainerd/vox-ops` ownership).

---

## 8. Ready-to-Use HCB Pre-Clearance Email & Transparent Application Copy

If you choose to test HCB eligibility directly in the live browser tab (`https://hcb.hackclub.com/applications/new`) alongside a pre-clearance inquiry, use this transparent wording so there is zero ambiguity during underwriting:

### 8.1 Pre-Clearance Email to `hcb@hackclub.com`

```text
Subject: Fiscal Sponsorship Eligibility & FSA Question — Vox Foundation (Apache-2.0 Language & Toolchain)

Hi HCB team,

I am an adult solo maintainer (Bertrand Reyna-Brainerd) preparing to apply for fiscal sponsorship for "Vox Foundation", an open-source (Apache-2.0) programming language and deterministic Rust developer toolchain (https://github.com/vox-foundation/vox, https://voxlang.org).

Before completing the application at hcb.hackclub.com/applications/new, could you confirm two items?

1. Adult Open-Source Eligibility: Your Help Center eligibility page restricts new intakes to teen-led (13-18) projects, while hackclub.com/fiscal-sponsorship notes support for adult-run open-source projects and the application form asks whether the applicant is an adult. Are you currently accepting adult-led open-source compiler/toolchain projects?
2. Fiscal Sponsorship Agreement & Domain/Trademark Boundaries: Could you share a copy of the current HCB Fiscal Sponsorship Agreement? Specifically, I want to verify that pre-existing Apache-2.0 repositories, the voxlang.org domain registration, and the Vox trademark remain cleanly separable from any separate commercial consulting entity I own, while 100% of HCB donations go exclusively toward public FOSS infrastructure (eval.voxlang.org, CI, reproducible compiler benchmarks, and external contributor bounties).

Thank you,
Bertrand Reyna-Brainerd
https://github.com/brbrainerd
```

### 8.2 Transparent Copy for `hcb.hackclub.com/applications/new`

- **Age Selection (Step 1):** Select **`No, I'm an adult (Over 18)`**.
- **Project Description:**

```text
Vox Foundation stewards Vox (https://voxlang.org, https://github.com/vox-foundation/vox), an open-source (Apache-2.0) programming language, compiler, and verification toolchain written in Rust.

Leader disclosure: I am an adult solo maintainer (Bertrand Reyna-Brainerd) applying transparently to verify whether HCB still accepts independent adult-led open-source public-goods projects. I separately own a commercial LLC (NextGen Interactive LLC); zero commercial revenues, consulting funds, or proprietary SaaS workloads will ever touch HCB.

Charitable FOSS mission & restricted use of funds: 100% of HCB funds will pay strictly for public open-source infrastructure — hosting the free deterministic browser sandbox (eval.voxlang.org), documentation (voxlang.org), cross-platform CI runners, reproducible multi-model compiler evaluation runs, and external open-source contributor bounties.
```
