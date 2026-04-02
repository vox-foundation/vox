---
title: "Crate API: vox-cli"
description: "Official documentation for the main Vox CLI binary crate."
category: "reference"
last_updated: 2026-04-02
---

# Crate API: vox-cli

`vox-cli` is the primary entrypoint for the user-facing command line interface. It coordinates subcommands, parses arguments using `clap`, and sets up global spans and Clavis secret environments over the orchestrator and build pipelines.

## Command Dispatch

The CLI surface is structurally divided into domains:
- `build`, `run`, `check`, `fmt` - Core inner-loop tooling
- `dei`, `ludus`, `scholarly` - Latin-namespace advanced capabilities
- `ci`, `telemetry` - Internal operation boundaries

## Feature Flags

- `--features extras-ludus` - Enables gamification commands.
- `--features stub-check` - Enables the TOESTUB quality gates for CI.
