---
title: "Populi Cloud GPU Training Strategy"
description: "Official documentation for Populi Cloud GPU Training Strategy for the Vox language. Detailed technical reference, architecture guides, an"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Populi Cloud GPU Training Strategy

> This document is the **Single Source of Truth** for cloud GPU selection, pricing tiers, and
> self-terminating training job design for `vox populi train`. All preset VRAM thresholds, 
> hardware tier names, and cloud provider recommendations **must** align with this document.

## Core Design Principle: Pay Only For Training Time

The goal is a **fire-and-forget training job**: you start it, model trains to completion,
artifacts upload to HuggingFace Hub (or S3 / GCS), and the instance terminates. No idle
billing, no custom monitoring loop.

### Provider Comparison (March 2026)

All prices are **spot/interruptible** unless noted. On-demand is noted separately.
GPU pricing excludes base VM / storage costs.

| Provider | GPU | VRAM | Spot $/hr | On-Demand $/hr | Auto-Terminate | Best For |
|----------|-----|------|-----------|----------------|---------------|---------|
| **Vast.ai** | RTX 3090 | 24 GB | ~$0.12 | ~$0.22 | ✅ `--onstart cmd` | Dev / small runs |
| **Vast.ai** | RTX 4090 | 24 GB | ~$0.28 | ~$0.45 | ✅ shell on-start hook | Dev / medium runs |
| **Vast.ai** | A100 80G | 80 GB | ~$0.67 | ~$1.05 | ✅ shell on-start hook | Production 7B+ |
| **Vast.ai** | H100 SXM | 80 GB | ~$1.53 | ~$2.20 | ✅ shell on-start hook | Large scale |
| **RunPod** | RTX 4090 | 24 GB | ~$0.20 | ~$0.34 | ⚠️ credit limit only | Quick experiments |
| **RunPod** | L40S | 48 GB | ~$0.40 | ~$0.86 | ⚠️ credit limit only | Mid-scale |
| **RunPod** | A100 80G | 80 GB | ~$0.79 | ~$1.49 | ⚠️ credit limit only | Production |
| **RunPod** | H100 SXM | 80 GB | ~$1.50 | ~$2.69 | ⚠️ credit limit only | Large scale |
| **GCP Spot** | L4 | 24 GB | ~$0.40 | ~$0.80 | ✅ managed instance group | Cloud-native |
| **GCP Spot** | A100 40G | 40 GB | ~$0.74 | ~$2.93 | ✅ preemptible VM | Production |
| **GCP Spot** | A100 80G | 80 GB | ~$1.10 | ~$4.37 | ✅ preemptible VM | Production |
| **GCP Spot** | H100 80G | 80 GB | ~$1.52 | ~$10.98 | ✅ preemptible VM | Datacenter |
| **fal.ai** | A100 40G | 40 GB | ~$0.99/hr | — | ✅ serverless auto-scale | Quick API jobs |
| **fal.ai** | H100 80G | 80 GB | ~$1.89/hr | — | ✅ serverless auto-scale | Quality runs |
| **Lambda Labs** | A100 SXM | 80 GB | ~$1.29/hr | ~$1.79/hr | ❌ manual | Sustained runs |

### Winner for Populi Training: **Vast.ai + `--onstart` hook**

**Reasoning:**
1. **True fire-and-forget**: the `--onstart` hook runs your command when the instance starts.
   Your training script calls `huggingface-cli upload` at the end, then `vastai stop self`.
2. **Per-second billing**: you only pay for active time.
3. **Interruptible is fine for QLoRA**: checkpointing is built in (`CheckpointState`). A 5-second
   warning before preemption is enough to save the checkpoint to persistent volume.
4. **Cheapest A100 access**: ~$0.67/hr vs RunPod $0.79/hr for equivalent workload.

**Best local fallback (you own this):** RTX 4080 Super @ 16 GB → preset `4080_super`.

---

## GPU × Preset Mapping (SSOT)

This table defines the canonical VRAM thresholds used in `auto_preset_from_vram()`.  
**Any change here MUST be reflected in `device.rs` `auto_preset_from_vram`.**

| VRAM Range (MB) | Auto Preset | Example GPUs | Est. 3B Qwen2 Train Time (5k pairs, 3 epochs) |
|----------------|-------------|-------------|----------------------------------------------|
| 0–7,999 | `safe` | GTX 1080 (8 GB), V100 16G | ~8-12h CPU / not recommended |
| 8,000–11,999 | `safe` | RTX 3070 Ti (8 GB), RTX 3080 (10 GB) | ~6h |
| 12,000–15,999 | `4080_safe` | RTX 3060 12G, RTX 4060 Ti 12G | ~4h |
| 16,000–23,999 | `4080_super` | **RTX 4080 Super**, RTX 4060 Ti 16G | ~2.5h |
| 24,000–31,999 | `prosumer_24g` | RTX 3090, RTX 4090, A10G, GCP L4 | ~1.5h |
| 32,000–47,999 | `prosumer_32g` | RTX 5090 (32 GB) | ~1h |
| 48,000–79,999 | `l40s` | L40S (48 GB), A6000 Ada (48 GB) | ~40m |
| 80,000–140,999 | `a100` | A100 80G, H100 SXM | ~20m |
| 141,000–191,999 | `h200` | H200 NVL (141 GB) | ~12m |
| 192,000+ | `b200` | B200 (192 GB), MI300X | ~8m |

---

## Estimated Training Costs (Qwen 2.5 3B, 5k pairs, 3 epochs)

| Hardware | Provider | Time Estimate | Est. Cost |
|----------|---------|--------------|----------|
| RTX 4080 Super | Local | 2.5h | $0 (your power) |
| RTX 4090 | Vast.ai spot | 1.5h | ~$0.42 |
| A100 80G | Vast.ai spot | 20m | ~$0.22 |
| H100 SXM | Vast.ai spot | 12m | ~$0.31 |
| L40S | RunPod spot | 40m | ~$0.27 |
| H100 80G | GCP Spot | 12m | ~$0.30 |

**Key insight**: even at H100 rates, a full training run costs under $1 USD. The largest cost
risk is not GPU time but **idle time if you don't auto-terminate**.

---

## Cloud Training Command Pattern

### Vast.ai (Recommended)

```bash
# 1. Search for cheapest available A100
vastai search offers 'gpu_name=A100_SXM4_80GB num_gpus=1 disk_space>50 reliability>0.95' \
  --order dph_total

# 2. Rent and provision with on-start training command
vastai create instance <offer_id> \
  --image pytorch/pytorch:2.4-cuda12.4-cudnn9-devel \
  --disk 80 \
  --env '-e HF_TOKEN=$HF_TOKEN -e VOX_MODEL=Qwen/Qwen2.5-Coder-3B-Instruct' \
  --onstart-cmd 'bash -c "
    pip install -q huggingface_hub vox-populi &&
    vox populi train --model $VOX_MODEL --preset auto &&
    huggingface-cli upload my-org/vox-populi populi/runs/latest &&
    vastai stop self
  "'
```

### RunPod (No auto-terminate — set a spend limit)

```bash
# RunPod terminates on credit depletion (set a budget before starting)
runpodctl create pod \
  --gpuType NVIDIA_H100_80GB_HBM3 \
  --imageName "runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04" \
  --containerDiskSize 80 \
  --env "VOX_MODEL=Qwen/Qwen2.5-Coder-3B-Instruct,HF_TOKEN=$HF_TOKEN"
# SSH in and run training + shutdown manually, or build into Docker entry point
```

### GCP Spot VM

```bash
gcloud compute instances create vox-train-$(date +%s) \
  --machine-type=a2-highgpu-1g \
  --accelerator=type=nvidia-tesla-a100,count=1 \
  --provisioning-model=SPOT \
  --instance-termination-action=DELETE \
  --boot-disk-size=100GB \
  --metadata=startup-script='
    #!/bin/bash
    cd /workspace
    vox populi train --model Qwen/Qwen2.5-Coder-3B-Instruct --preset auto
    gsutil -m cp -r populi/runs/latest gs://my-bucket/vox-populi/
    # GCP Spot VMs can be set to auto-delete on shutdown
    gcloud compute instances delete $(hostname) --zone=$(curl metadata/zone) -q
  '
```

---

## What To Build in Vox

For native integration, `vox populi train --cloud vast|runpod|gcp` should:

1. Call provider API to find cheapest available matching GPU (VRAM ≥ threshold from preset)
2. Upload workspace tarball or configure Docker env with `HF_TOKEN`
3. Start instance with on-completion hook: upload artifacts → terminate
4. Stream training logs via provider API
5. Return exit code 0 when adapter is on HuggingFace Hub

This is **Wave 8** work — not required for local training to be fully functional.

---

## Checkpoint Safety with Cloud Interruption

All cloud providers except fal.ai can interrupt running instances with minimal warning.
The QLoRA training loop already writes `checkpoint.state.json` at configurable intervals
(default every 100 steps). To survive spot preemption:

1. Mount persistent disk / volume (not ephemeral container storage)
2. Set `--output-dir` to the persistent mount path
3. Resume automatically on restart: `--force-restart` is NOT passed; checkpoint is detected

Example Vast.ai volume mount:
```
vastai create instance ... --disk 80   # 80 GB persistent disk
# Output dir on persistent volume:
vox populi train ... --output-dir /workspace/populi/runs/v1
```
