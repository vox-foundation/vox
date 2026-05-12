# Hardcoded values audit — production code

- **Captured**: 2026-05-11T16:10:13.980Z
- **Total itemized findings**: 1400
- **Total raw matches (pre-cap)**: 5383
- **Cap per category**: 100

## For follow-up LLMs

1. Open `findings.v1.json` — each entry is self-contained.
2. Run `verification_command` from the repository root; expect at least one match.
3. For `confidence: "low"`, confirm in IDE before editing.
4. Regenerate: `node tools/hardcoded-values-audit.mjs .vox\audit\2026-05-11-hardcoded-values`

## Category index

- [01-hardcoded-urls.md](./01-hardcoded-urls.md) — raw: 397, itemized: 100 (cap)
- [02-hardcoded-ports.md](./02-hardcoded-ports.md) — raw: 73, itemized: 73
- [03-hardcoded-ips.md](./03-hardcoded-ips.md) — raw: 91, itemized: 91
- [04-hardcoded-filesystem-paths.md](./04-hardcoded-filesystem-paths.md) — raw: 19, itemized: 19
- [05-hardcoded-timeouts.md](./05-hardcoded-timeouts.md) — raw: 153, itemized: 100 (cap)
- [06-hardcoded-retry-counts.md](./06-hardcoded-retry-counts.md) — raw: 44, itemized: 44
- [07-hardcoded-buffer-sizes.md](./07-hardcoded-buffer-sizes.md) — raw: 116, itemized: 100 (cap)
- [08-hardcoded-version-strings.md](./08-hardcoded-version-strings.md) — raw: 1073, itemized: 100 (cap)
- [09-hardcoded-model-names.md](./09-hardcoded-model-names.md) — raw: 61, itemized: 61
- [10-hardcoded-env-var-names.md](./10-hardcoded-env-var-names.md) — raw: 85, itemized: 85
- [11-brittle-string-needles.md](./11-brittle-string-needles.md) — raw: 2266, itemized: 100 (cap)
- [12-brittle-regex-patterns.md](./12-brittle-regex-patterns.md) — raw: 333, itemized: 100 (cap)
- [13-hardcoded-extensions-globs.md](./13-hardcoded-extensions-globs.md) — raw: 61, itemized: 61
- [14-hardcoded-date-literals.md](./14-hardcoded-date-literals.md) — raw: 62, itemized: 62
- [15-hardcoded-ansi-codes.md](./15-hardcoded-ansi-codes.md) — raw: 5, itemized: 5
- [16-hardcoded-magic-numbers.md](./16-hardcoded-magic-numbers.md) — raw: 307, itemized: 100 (cap)
- [17-hardcoded-canonical-keys.md](./17-hardcoded-canonical-keys.md) — raw: 138, itemized: 100 (cap)
- [18-hardcoded-provider-names.md](./18-hardcoded-provider-names.md) — raw: 91, itemized: 91
- [19-hardcoded-test-data-in-prod.md](./19-hardcoded-test-data-in-prod.md) — raw: 8, itemized: 8
- [20-hardcoded-retired-runtime-names.md](./20-hardcoded-retired-runtime-names.md) — raw: 0, itemized: 0

## Schema

- [findings.v1.schema.json](./findings.v1.schema.json)

## Methodology

- [methodology.md](./methodology.md)
