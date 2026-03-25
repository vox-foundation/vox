---
title: "SCIENTIA publication automation SSOT"
description: "Research-grounded SSOT for what Vox should automate, assist, or never automate in scientific publication workflows."
category: "reference"
last_updated: 2026-03-25
training_eligible: true
---

## SCIENTIA publication automation SSOT

This is the primary SSOT for turning Vox/Populi findings into publishable scientific artifacts quickly, safely, and reproducibly.

Scope:

- direct publication and self-archival paths (`arXiv`, Zenodo-style deposition, Crossref-grade metadata),
- journal submission readiness (`JMLR`, `TMLR`, `JAIR`, major publisher AI policies),
- Vox-native orchestration (`vox-orchestrator`, Populi mesh, Socrates, eval gates, SCIENTIA manifest lifecycle).

## North-star outcome

Minimize time from validated finding to submission-ready package while preserving:

- epistemic integrity (no fabricated claims/citations/data),
- reproducibility (before/after evidence with replayability),
- policy compliance (journal, ethics, AI disclosure, metadata quality),
- provenance (digest-bound state transitions and auditable pipeline decisions).

## Source anchors

Internal SSOT and implementation anchors:

- `docs/src/architecture/scientia-publication-readiness-audit.md`
- `docs/src/adr/011-scientia-publication-ssot.md`
- `docs/src/how-to/how-to-scientia-publication.md`
- `docs/src/reference/socrates-protocol.md`
- `docs/src/architecture/populi-workflow-guide.md`
- `docs/src/reference/external-repositories.md`
- `crates/vox-publisher/src/publication.rs`
- `crates/vox-publisher/src/publication_preflight.rs`
- `crates/vox-publisher/src/scientific_metadata.rs`
- `crates/vox-publisher/src/zenodo_metadata.rs`
- `crates/vox-cli/src/commands/scientia.rs`
- `crates/vox-cli/src/commands/db.rs`
- `crates/vox-mcp/src/tools/scientia_tools.rs`
- `crates/vox-db/src/schema/domains/publication.rs`

External requirements anchors (authoritative policies/guides):

- JMLR final prep and style requirements
- TMLR author/submission/ethics pages (OpenReview + double-blind + broader impact)
- JAIR formatting/final prep
- arXiv moderation and format requirements
- COPE authorship and AI-tools position
- ICMJE AI recommendations
- Nature Portfolio AI policy
- Elsevier generative AI writing policy
- Crossref required/recommended metadata guidance

## Pipeline SSOT

```mermaid
flowchart LR
findingIntake[FindingIntake] --> evidencePack[EvidencePackBuilder]
evidencePack --> worthinessGate[WorthinessGate]
worthinessGate --> policyGate[JournalPolicyGate]
policyGate --> packageBuild[SubmissionPackageBuilder]
packageBuild --> adapterRoute[AdapterRouter]
adapterRoute --> directPublish[DirectPublishPath]
adapterRoute --> journalSubmit[JournalSubmitPath]
adapterRoute --> archiveDoi[ArchiveDoiPath]
journalSubmit --> revisionLoop[RevisionLoop]
directPublish --> postPublishAudit[PostPublishAudit]
archiveDoi --> postPublishAudit
revisionLoop --> postPublishAudit
postPublishAudit --> codexLedger[CodexLedgerAndMetrics]
```

## Automation boundary matrix

|Workflow element|Automate|Assist|Never automate|
|---|---|---|---|
|Artifact capture (run metadata, hashes, manifests, metrics export)|yes|n/a|no|
|Schema and policy preflight checks|yes|n/a|no|
|Citation syntax and resolvability checks|yes|n/a|no|
|Journal template/package scaffolding|yes|n/a|no|
|Metadata normalization (`authors`, ORCID, funding, license)|yes|n/a|no|
|DOI/adapter payload generation|yes|n/a|no|
|Final scientific claim selection and framing|no|yes|yes (fully autonomous)|
|Novelty/significance judgment|no|yes|yes (fully autonomous)|
|Fabrication-prone narrative sections without evidence|no|no|yes|
|Inclusion of unverifiable benchmark deltas|no|no|yes|
|Undisclosed AI authorship/content generation|no|no|yes|
|Safety/ethics risk acceptance|no|yes|yes (fully autonomous)|
|Final submission button with external legal/accountability implications|no|yes|yes (unless explicitly policy-approved human-in-loop)|

## Biggest AI-slop failure modes and controls

|Failure mode|Why it harms science|Vox control surface|Required gate|
|---|---|---|---|
|Fabricated citations|corrupts scholarly graph and reproducibility|citation parse/resolution checks + Socrates evidence linking|hard fail|
|Benchmark gaming/cherry-picking|false claims of improvement|before/after benchmark protocol + eval gate traces|hard fail|
|Confident unsupported claims|hallucination masquerading as findings|Socrates risk decision (`Answer/Ask/Abstain`) and contradiction metrics|hard fail for publication path|
|Undisclosed AI generation in restricted contexts|policy breach / desk reject risk|policy profile in publication preflight|hard fail|
|AI-generated figures in disallowed venues|legal and integrity breach|policy gate by target venue|hard fail|
|Metadata incompleteness|DOI and discoverability failures|structured scientific metadata + completeness score|fail for external deposit paths|

## Journal/direct-publication requirement-to-gate mapping

|Requirement|Gate in Vox pipeline|Status|
|---|---|---|
|Double-blind + anonymization (`TMLR`)|`publication_preflight` profile `double_blind` + additional anonymization checks|partial (email heuristic present, broader anonymization missing)|
|Camera-ready source bundle and compileability (`JMLR`/`JAIR`)|`SubmissionPackageBuilder` + compile preflight|missing|
|Broader impact / ethics disclosure (`TMLR`, publisher policies)|structured `scientific_publication.ethics_and_impact` + policy gate|partial|
|AI disclosure and no AI authorship (COPE/ICMJE/Nature/Elsevier)|policy gate + metadata declarations|partial|
|arXiv format/moderation constraints|package + format preflight profile `arxiv`|missing|
|DOI-quality metadata (Crossref)|metadata completeness + export mapper|partial|
|Self-archive metadata (`Zenodo`)|`zenodo_metadata` generation|partial (metadata done, upload/deposit not done)|

## Vox capability map for publication automation

### Already usable now

- SCIENTIA canonical manifest lifecycle with digest-bound approvals and submission ledger.
- Structured scholarly metadata in `metadata_json.scientific_publication`.
- Preflight checks with readiness score and profile-aware gating.
- Zenodo deposition metadata JSON generation.
- MCP/CLI parity for core prepare/approve/submit/status and preflight.
- Socrates anti-hallucination telemetry and gate concepts.
- `metadata_json.scientia_evidence` (see `vox_publisher::scientia_evidence`): optional Socrates rollup (merged from VoxDb when using preflight `--with-worthiness`), eval-gate snapshot, benchmark baseline/candidate pair, and human attestations; folded into `publication_worthiness` scoring with manifest preflight heuristics.

### Reusable orchestration/mesh assets

- A2A messaging and handoff payloads for reviewer-style multi-agent workflows.
- Populi coordination patterns (distributed lock, heartbeats, conflict paths).
- Reliability and benchmark telemetry pathways for publication KPIs.

### Non-automatable or human-accountability-critical steps

- final claims and novelty significance assertion,
- ethical risk acceptance and framing,
- legal/publisher final attestation steps,
- submission authorization where account liability is personal/institutional.

## Before/after benchmark protocol (publication-grade)

Required evidence pair per claim:

1. `baseline_run` and `candidate_run` with immutable run IDs and repository context.
2. Identical benchmark manifest and policy profile.
3. Captured outputs:
   - eval JSON,
   - gate JSON,
   - telemetry summary,
   - manifest digest,
   - environment and dependency fingerprints.
4. Reported delta set:
   - effect size,
   - confidence/variance window or repeated-run stability proxy,
   - failure-mode deltas (not only headline wins).
5. Publishability condition:
   - no regression in critical safety/quality gates unless explicitly justified and approved.

## Gap priorities and solutions

### Gap 1: package builder and venue profiles (complex)

- **Where:** `vox-publisher` has metadata/preflight but no camera-ready package builder.
- **Why:** manual packaging dominates cycle time and introduces policy errors.
- **Minimum viable fix:** add `SubmissionPackageBuilder` with profiles `jmlr`, `tmlr`, `jair`, `arxiv`; emit deterministic archive manifest.
- **Expanded solution (how/where/when/why):**
  - add `crates/vox-publisher/src/submission_package.rs` with profile-specific validators;
  - wire CLI/MCP commands `publication-package-build` and `publication-package-validate`;
  - persist package artifact metadata in publication tables with digest linkage;
  - run compile/format checks and include machine-readable report in manifest metadata.
- **Success criteria:** >=95% package validation pass in CI dry-runs before human submission.

### Gap 2: adapter execution beyond local stub (complex)

- **Where:** `LocalLedgerAdapter` is the only scholarly adapter.
- **Why:** no direct publication path means manual bottlenecks and error-prone handoff.
- **Minimum viable fix:** implement Zenodo adapter first (draft create + metadata upload path).
- **Expanded solution:**
  - create adapter implementations for Zenodo then OpenReview/arXiv-assist/Crossref export;
  - add per-adapter idempotency keys and retry taxonomy;
  - store external status/revision IDs in `scholarly_submissions` plus status events.
- **Success criteria:** end-to-end draft submission success rate >=90% in staging/sandbox workflows.

### Gap 3: anti-slop policy gate depth (medium)

- **Where:** current preflight catches core checks but not full anti-slop taxonomy.
- **Why:** fabricated or weakly supported science can still pass narrow checks.
- **Minimum viable fix:** add citation resolvability + claim-evidence linkage completeness checks.
- **Expanded solution:** integrate Socrates outputs as hard publication predicates for factual claims.
- **Success criteria:** zero unresolved fabricated-reference incidents in internal publication trials.

### Gap 4: benchmark provenance unification (complex)

- **Where:** benchmarks, Mens/Populi artifacts, and publication manifests are not fully unified.
- **Why:** difficult to prove reproducibility and before/after integrity at publication time.
- **Minimum viable fix:** define a single `EvidencePack` schema and attach to manifest metadata.
- **Expanded solution:** orchestrated evidence pack builder pulls eval/gate/telemetry + commit/env fingerprints and signs report digest.
- **Success criteria:** every publication candidate has a complete evidence pack with replay instructions.

### Gap 5: worthiness classification consistency (medium)

- **Where:** no dedicated publishability rubric in SSOT form.
- **Why:** inconsistent decisions about what is scientifically worthy.
- **Minimum viable fix:** adopt explicit `Publish/AskForEvidence/Abstain` rubric with numeric thresholds.
- **Expanded solution:** policy engine consuming worthiness metrics and producing deterministic decision traces.
- **Success criteria:** decision disagreement rate between reviewers and rubric <15% after calibration period.

## KPI set for this SSOT

- `submission_readiness_score`
- `metadata_completeness_rate`
- `evidence_pack_completeness_rate`
- `policy_gate_pass_rate`
- `time_to_submission_ms`
- `adapter_submission_success_rate`
- `revision_turnaround_ms`
- `socrates_contradiction_ratio_for_publishables`

## Decision policy

Use the companion rules doc:

- `docs/src/reference/scientia-publication-worthiness-rules.md`

This architecture SSOT defines pipeline shape, boundaries, and implementation priorities; the rules doc defines scientific-worthiness classification and hard red lines.

## External policy URL appendix

- JMLR author and final style information: [https://jmlr.org/author-info.html](https://jmlr.org/author-info.html)
- TMLR overview and policies: [https://jmlr.org/tmlr/](https://jmlr.org/tmlr/)
- TMLR OpenReview venue and submission details: [https://openreview.net/group?id=TMLR](https://openreview.net/group?id=TMLR)
- JAIR submission and formatting guidance: [https://www.jair.org/index.php/jair/about/submissions](https://www.jair.org/index.php/jair/about/submissions)
- arXiv submission and moderation policy: [https://info.arxiv.org/help/submit/index.html](https://info.arxiv.org/help/submit/index.html)
- COPE AI tools position statement: [https://publicationethics.org/cope-position-statements/ai-author](https://publicationethics.org/cope-position-statements/ai-author)
- ICMJE recommendations, including AI guidance: [https://www.icmje.org/recommendations/](https://www.icmje.org/recommendations/)
- Nature Portfolio AI policy for authors: [https://www.nature.com/nature-portfolio/editorial-policies/ai](https://www.nature.com/nature-portfolio/editorial-policies/ai)
- Elsevier generative AI in publishing policy: [https://www.elsevier.com/about/policies-and-standards/the-use-of-ai-and-ai-assisted-writing-technologies-in-scientific-writing](https://www.elsevier.com/about/policies-and-standards/the-use-of-ai-and-ai-assisted-writing-technologies-in-scientific-writing)
- Crossref metadata best practices: [https://www.crossref.org/documentation/schema-library/markup-guide-metadata-segments/](https://www.crossref.org/documentation/schema-library/markup-guide-metadata-segments/)
