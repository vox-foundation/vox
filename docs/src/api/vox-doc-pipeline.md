---
title: "Crate API: vox-doc-pipeline"
description: "Official documentation for Crate API: vox-doc-pipeline for the Vox language. Detailed technical reference, architecture guides, and imple"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Crate API: vox-doc-pipeline

## Overview

Documentation generation tool for the Vox project. Scans `docs/src/` for Markdown files and generates a `SUMMARY.md` navigation index.

## Usage

```bash
cargo run -p vox-doc-pipeline
```

This scans `docs/src/` for all `.md` files (excluding `SUMMARY.md` itself), sorts them alphabetically, and generates a `SUMMARY.md` with title-cased links.

## How It Works

1. Reads all `.md` files in `docs/src/`
2. Converts filenames to title-case headings (e.g., `language-guide.md` → `Language guide`)
3. Writes a `# Summary` with links to each page

## Future Plans

- Copy crate `README.md` files into `docs/src/api/` as individual pages
- Generate cross-reference links between docs and rustdoc
- Integrate with mdBook or Pagefind for full-text search

---

## Module: `vox-doc-pipeline/src/main.rs`

The crate currently **only** walks `docs/src/*.md`, skips `SUMMARY.md`, title-cases filenames, and writes `docs/src/SUMMARY.md` for mdBook navigation. It does **not** parse Rust with `syn` or extract rustdoc; that paragraph described a future / alternate design and is intentionally not implemented in `main.rs` yet.


