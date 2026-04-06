---
title: "Journey: Native Rust LLM Training"
description: "How to use Vox's native ML toolchain to fine-tune open weights directly from your application data without diving into Python environments."
category: "journey"
sort_order: 4
---

# Journey: Native Rust LLM Training

## The Curse of Python ML Environments

When you have domain-specific application data housed in a Rust or typical structured backend and want to use it to fine-tune a model, you hit a massive tooling disconnect.

You have to pull the data directly from production, dump it into JSONL files, transfer them, spin up complex Virtual Environments (venv/Conda), manage nested CUDA PyTorch dependencies, and fight Python multi-threading environments in Jupyter notebooks. Your application logic effectively divorces the ML operations layer.

## The Vox Paradigm: Zero-Python Native Fine-tuning

The Vox toolchain resolves this tension by providing native hardware-accelerated LLM QLoRA training right from your ecosystem via **MENS** (the Vox ML Subsystem) powered by `vox-tensor` (built atop Candle).

You can extract your `@table` records, instruct Vox to assemble training pairs directly from your canonical data, and launch a full forward/backward training pass executing entirely within Rust. It bridges the data layer and ML architecture without needing any Python binding layers.

## Core Snippet: Triggering QLoRA Training

To orchestrate a training pipeline, you specify the data source, the checkpoint strategy, and the hyperparameters straight from Vox constructs.

```vox
// vox:skip
// Import the native tensor functions and MENS integration logic
import vox.mens.training
import vox.mens.qlora

// We assume we have a table of high-quality agent queries and outputs.
@table type AgentTelemetry {
    query: str
    optimal_response: str
}

@action
fn finetune_from_telemetry() -> Result[str] {
    // 1. Fetch training subset directly from your database
    let records = db.query(AgentTelemetry).take(5000);
    
    // 2. Map structural DB logic into instruction dataset layout
    let dataset = records.map(fn(r) {
        { prompt: r.query, completion: r.optimal_response }
    });
    
    // 3. Initiate a hardware-accelerated QLoRA training session (Candle backend)
    let session = training.qlora_finetune(
        dataset,
        "base_models/Meta-Llama-3-8B-Instruct",
        {
            r: 16,
            lora_alpha: 32,
            target_modules: ["q_proj", "v_proj"],
            batch_size: 4,
            epochs: 3
        }
    )?
    
    return Ok("Trained adapter saved to: " + session.adapter_path)
}
```

## Running the Process

NVIDIA GPU training directly off Rust requires specific compilation features so the compiler binds to CUDA efficiently.

Instead of the standard run, use the CLI's native ML capabilities:

```bash
# This fetches the model base components and leverages PyTorch/Candle-compatible GPU bindings
vox run server.vox --mens-training-enabled
```

## Deep Dives

To evaluate how Vox bypassed Python boundaries and the performance implications:

- **[ADR 003 — Native Rust Training Over Python](../adr/003-native-training-over-python.md)**: The technical motivation explaining why Vox transitioned away from `PyO3` architectures towards pure Rust.
- **[Native ML Training Pipeline](../explanation/expl-ml-pipeline.md)**: End-to-end overview showing data staging and tokenization processes prior to QLoRA execution.
- **[Mens native training SSOT (Candle QLoRA)](../reference/mens-training.md)**: Highly specific architectural guarantees, limits, and configurations for `vox-tensor` training capabilities.
