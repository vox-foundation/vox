# QLoRA Fine-tuning Data Strategy & SSoT

last_updated: 2026-03-22

> [!IMPORTANT]
> This document is the Single Source of Truth for Vox Populi's QLoRA data scaling requirements and continuous assimilation pipeline. DO NOT attempt to "pad" the pipeline with a stale `examples/` directory.

## 1. Minimal Data Size Requirements

Research on code-style adaptation in Large Language Models via QLoRA concludes that data **quality** trumps raw quantity, but a strict minimum threshold exists to prevent catastrophic overfitting:
- **General Style Changes / Simple Tasks:** 400 to 1,000 high-quality examples minimally required.
- **Complex Domain Inference (Vox Native Rules):** 1,000 to 5,000 examples.
- **Anti-pattern to avoid:** Finetuning with extremely small sets (< 120 samples) practically guarantees catastrophic overfitting, essentially treating the tuning target like a few-shot prompt.

Historically, Vox accumulated ~19 files in an `examples/` directory. This was vastly too small for QLoRA, leading to severe model degradation and overfitting.

## 2. Continuous Ingestion Pipeline

To satisfy the `> 1000` sample requirement without building a stale monolithic examples folder, Vox's native `vox populi corpus` data pipeline implements a continuous ingestion strategy. This guarantees zero architectural drift by generating ML instructional pairs from live code:

1. **Rust Crate Source (`crates/**/*.rs`)**
   - Extracts live function definitions, `docstrings`, and signatures mapping to Vox internal patterns.
   - Yields ~3,000+ samples naturally.
2. **Markdown Documentation (`docs/src/**/*.md`)**
   - Parses the actual documentation site, building Q&A instructional pairs dynamically based on `vox` code blocks.
   - Yields ~1,500+ samples.
3. **Synthetic Generation (`crates/vox-cli/src/training/datagen.rs`)**
   - Template-based dynamic code expansion to satisfy complex component and workflow structural coverage.
   - Yields ~2,000+ samples.

This pipeline seamlessly creates a training corpus of >10,000 pairs, ensuring perfectly aligned Populi models as the Vox compiler automatically scales learning alongside real logic changes.
