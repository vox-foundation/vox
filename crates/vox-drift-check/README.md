# vox-drift-check

Workspace-wide multi-language drift and pattern-repetition linter for the [Vox](https://github.com/vox-foundation/vox) project.

## Overview

`vox-drift-check` analyzes Rust, TypeScript, and Vox source trees to detect codebase drift, duplicate constants, and architectural rule violations before they merge:

- **Literal dedup**: flags duplicate string literals and duration constants that should be centralized.
- **Body hash dedup**: finds identical function implementations across crates.
- **Architectural rules**:
  - `reqwest_bypass`: detects direct HTTP client construction outside sanctioned wrappers.
  - `vox_dir_unanchored`: flags unanchored `.vox` directory paths.
  - `timeout_literal`: flags magic timeout literals.
  - `bearer_header`: catches raw bearer token header constructions.
  - `version_string`: identifies hardcoded version strings.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-drift-check = "0.6"
```

CLI execution:

```sh
vox-drift-check --root .
```

## License

Apache-2.0
