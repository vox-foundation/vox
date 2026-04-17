# Documentation Rules (docs/src/ scope)

## Code block policy
- `.vox` and `.tsx` blocks: use `{{#include}}` from `examples/golden/` or tag `// vox:skip`
- All code blocks must specify a language identifier
- Do not place golden examples inline without the include directive

## Frontmatter requirements (all new pages)
- `title`, `description`, `category`, `status`, `last_updated`, `training_eligible` are required
- Use `status: research` for evidence docs; `status: roadmap` for unshipped plans
- Do NOT label a page SSOT unless it is the sole B-canon in `contracts/documentation/canonical-map.v1.yaml`

## Research storage
- Research findings → `docs/src/architecture/`
- Naming: `*-research-2026.md` or `*-findings-2026.md`
- After writing, update `docs/src/architecture/research-index.md`

## Archive prohibition
- Do NOT read or modify files in `docs/src/archive/` for new work
