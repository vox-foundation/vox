# `vox-kb-facets`

Layer-0 SKOS-lite taxonomy, YAML frontmatter facet types, `<!-- kb:summary -->` block extractors, and provenance validators from the [Vox](https://github.com/vox-foundation/vox) knowledge-base pipeline.

## Overview

`vox-kb-facets` provides a zero-DB, pure-data contract for validating technical documentation metadata:
- Controlled `domain` and `topic` vocabulary lookup with `altLabel` synonym expansion and `deprecatedAliases` diagnostics.
- Authorship and provenance validation (`human`, `llm-assisted`, `llm-generated`) and training-eligibility rules.
- Summary block (`<!-- kb:summary -->`) word-count and structural checks.

## License

Licensed under the [Apache License, Version 2.0](../../LICENSE).
