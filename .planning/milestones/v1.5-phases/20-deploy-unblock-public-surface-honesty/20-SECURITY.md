---
phase: "20"
slug: "deploy-unblock-public-surface-honesty"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-09"
---

# Phase 20 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail. Register taken from the `<threat_model>` blocks of plans 20-01 to 20-12; `block_on: high`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CI job ↔ third-party npm packages | `pnpm install` runs dependency code inside docs workflows | `GITHUB_TOKEN`, Cloudflare token (deploy step only), OIDC (Pages job only) |
| Workflow ↔ GitHub Issues API | `notify-on-failure` / `notify-on-success` write issues | Run URLs and counters (via `env:`), `issues: write` job-scoped |
| Build scripts ↔ repo working tree | `setup-content.mjs` creates/removes the content mirror and `/repo/` mounts | Filesystem deletes and writes under `docs-astro/` |
| Repo Markdown ↔ public site | Repo docs and research notes are rendered on voxlang.org and llms*.txt | Internal planning content, link targets |
| Agent ↔ public GitHub state | Push, PR, issue comments and drills during 20-11 | Public writes on the user's behalf |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-1 | Elevation of privilege | docs-deploy permissions | high | mitigate | Top-level `contents: read`; `pages`/`id-token: write` only on `deploy-pages` (no install step) | closed |
| T-01-2 | Tampering | wrangler supply chain | high | mitigate | `wrangler` pinned `4.146.0` in package.json + lockfile; `pnpm exec`; `--frozen-lockfile` | closed |
| T-01-3 | Tampering | postinstall scripts | medium | mitigate | `allowBuilds: workerd: false`; CF secrets only in the deploy step `env:` | closed |
| T-01-4 | Tampering / injection | workflow `run:` scripts | medium | mitigate | 0 `${{ }}` inside `run:` in docs-deploy, docs-quality, ci-liveness | closed |
| T-01-5 | Denial of service | failure notifications | medium | mitigate | Single edit-in-place issue; live drill: 0 comments on 2nd failure (#639) | closed |
| T-01-6 | Repudiation | ci-liveness | low | mitigate | Deploy-success-age row with dispatch override | closed |
| T-01-7 | Information disclosure | `docs-dist` artifact | low | accept | Public site content; 3-day retention | closed |
| T-02-1 | Tampering | git-dates subprocess | low | mitigate | `execFileSync` argv, no shell | closed |
| T-02-2 | Denial of service | git-dates history scan | low | mitigate | 256 MiB maxBuffer; one cached log call | closed |
| T-02-3 | Repudiation | page dates | medium | mitigate | Ignore-revs/subject/bulk rules; `git-dates.test.mjs` + `dates.spec.ts` | closed |
| T-03-1 | Repudiation | superseded specs | medium | mitigate | `status: deprecated` + superseded notice (verification truth 5) | closed |
| T-03-2 | Information integrity | governance doc | low | mitigate | False date-derivation claim removed | closed |
| T-04-1 | Tampering | tutorial-verify | low | mitigate | argv-only process calls after token filtering | closed |
| T-04-2 | Repudiation | tutorial record | medium | mitigate | Blob-SHA freshness asserted by `tutorial-record.test.mjs` in docs-quality | closed |
| T-04-3 | Information integrity | tutorials | medium | mitigate | `doctest-md --strict` per tutorial; record lists 7/7 | closed |
| T-05-1 | Information integrity | dead-link fixes | low | mitigate | Retired targets unlinked, not redirected | closed |
| T-05-2 | Denial of service | link guard | low | mitigate | `decodeURI` + `git ls-files` listing | closed |
| T-06-1 | Tampering (XSS) | status banners | medium | mitigate | Banner text from constants in `page-status.mjs`; status is a lookup key | closed |
| T-06-2 | Information disclosure | research notes in search engines | low | mitigate | noindex + sitemap filter | closed |
| T-06-3 | Tampering | new dependencies | low | mitigate | Exact pins of lockfile versions | closed |
| T-07-1 | Tampering | patched dependency | low | mitigate | `patchedDependencies` in lockfile; frozen install | closed |
| T-07-2 | Information integrity | llms*.txt | medium | mitigate | `llms.spec.ts` asserts no Internals in any variant | closed |
| T-08-1 | Tampering (path traversal) | link rewriter | low | mitigate | Paths outside repoRoot throw; `existsSync` only | closed |
| T-08-2 | Information integrity | link rewriter | medium | mitigate | Strict throw for docs/src + `links.spec.ts` + lychee | closed |
| T-08-3 | Tampering (open redirect) | link rewriter | low | mitigate | Only the fixed `repoUrl` prefix | closed |
| T-09-1 | Tampering / data loss | `/repo/` mounts | high | mitigate | Mounts written inside the marker-guarded mirror; throw if `docs/src/repo` exists (`setup-content.mjs:108`) | closed |
| T-09-2 | Information disclosure | repo-doc auto-discovery | medium | mitigate | `never_mount_prefixes: ["docs/superpowers/"]`; `repo-docs.spec.ts` asserts no superpowers route | closed |
| T-09-3 | Tampering (XSS) | generated wrapper titles | low | mitigate | JSON-quoted YAML title | closed |
| T-09-4 | Tampering (CI) | workflow path filters | low | mitigate | No `**/*.md`; `workflow-paths.test.mjs` coverage check | closed |
| T-10-1 | Tampering (CI injection) | docs-quality | medium | mitigate | Outputs via `env:`; 0 `${{ }}` in `run:` | closed |
| T-10-2 | Repudiation (silent gate) | lychee | medium | mitigate | "Link checker self-test (fixture must fail)" + assert step | closed |
| T-10-3 | Tampering | lychee binary | low | mitigate | `lycheeVersion` pinned 0.24.2 | closed |
| T-10-4 | Denial of service | CI cache | low | mitigate | Nightly on main only; cache-key-lint | closed |
| T-11-1 | Repudiation / integrity | agent public writes | high | mitigate | Every push, PR, merge, #462 comment and drill dispatch ran only after explicit user approval in session | closed |
| T-11-2 | Denial of service | live smoke | low | mitigate | Concurrency 8, same-origin, once per deploy | closed |
| T-11-3 | Information integrity | #462 root-cause comment | low | mitigate | Draft approved by user before posting | closed |
| T-11-4 | Tampering | drill leftovers | low | mitigate | Recovery run closed #639; 0 open `docs-deploy-broken` | closed |
| T-12-1 | Tampering / data loss | mirror rebuild | high | mitigate | `prepareMirrorDir`: symlink unlinked only; real dir removed only with `.vox-docs-mirror` marker | closed |
| T-12-2 | Information disclosure | archive | medium | mitigate | Archive not built; `archive.spec.ts`; noindex retired page | closed |
| T-12-3 | Spoofing | retired page | low | mitigate | Same-site links and fixed GitHub URL only | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-20-1 | T-01-7 | `docs-dist` is the public site build; repo readers gain nothing not already on voxlang.org; 3-day retention | plan 20-01 threat model | 2026-10-08 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-09 | 40 | 40 | 0 | orchestrator (ASVS L1 grep-depth; plan-time register) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-09
