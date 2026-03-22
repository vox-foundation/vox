# Introduction

Vox is the AI-native language built from the ground up to compile to Rust (backend) and TypeScript (frontend). This documentation will outline the language design and compiler infrastructure.

## Tooling

- **[`vox` CLI reference](ref-cli.md)** — commands shipped by `crates/vox-cli` today

## Architecture

- **[ADR 004: Codex over Arca over Turso](adr/004-codex-arca-turso-ssot.md)** — storage single source of truth (**Codex** = public API, **Arca** = internal schema, **Turso** = engine)
- **[Codex vNext schema domains](architecture/codex-vnext-schema.md)**
- **[Codex BaaS scaffolding](architecture/codex-baas.md)**
- **[Orphan surface inventory](architecture/orphan-surface-inventory.md)**
- **[Forward-only migration charter](architecture/forward-migration-charter.md)**
- **[CLI scope policy (minimal binary)](architecture/cli-scope-policy.md)**
- **[Codex / Arca import policy](architecture/codex-arca-import-policy.md)**
- **[Codex legacy migration (importers)](architecture/codex-legacy-migration.md)**
- **[CI runner contract](ci/runner-contract.md)**
