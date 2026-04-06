#!/usr/bin/env bash
# Vox Mens cloud training/serving/agent entrypoint.
#
# Runs on both Vast.ai (via `onstart: exec /entrypoint.sh`) and RunPod (container CMD).
# Environment variables are injected at dispatch time by CloudResolver.
#
# VOX_JOB_KIND controls which mode to run:
#   train  — download data, run training, optionally upload adapter, self-terminate
#   serve  — HTTP inference (vox mens serve); see script-surface-audit.md for checkpoint path / image caveats
#   agent  — not mapped to a single vox subcommand yet; fails fast with instructions
#
# Termination safety (layered — all must be in place):
#   1. This script self-terminates on completion (primary)
#   2. CloudWatchdog terminates on time/budget/idle (mandatory fallback)
#   3. absolute_max_runtime_secs in CloudProviderConfig (hard cap)
set -euo pipefail

log() { echo "[vox-populi][$(date -u +%H:%M:%S)] $*" >&2; }

JOB_KIND="${VOX_JOB_KIND:-train}"
MODEL_ID="${VOX_MODEL_ID:-Qwen/Qwen3.5-4B}"
SERVE_PORT="${VOX_SERVE_PORT:-8080}"

log "=== Vox Mens Cloud — job_kind=$JOB_KIND model=$MODEL_ID ==="

# ─────────────────────────────────────────────────────────────────────────────
# Shared: self-termination functions
# ─────────────────────────────────────────────────────────────────────────────

self_terminate_vast() {
    # VOX_VAST_API_KEY is injected by VastClient into the container env
    if [ -n "${VOX_VAST_API_KEY:-}" ] && [ -n "${CONTAINER_ID:-}" ]; then
        log "Self-terminating Vast.ai instance: $CONTAINER_ID"
        curl -s -X DELETE \
            "https://cloud.vast.ai/api/v0/instances/${CONTAINER_ID}/" \
            -H "Authorization: Bearer ${VOX_VAST_API_KEY}" \
            -o /dev/null --fail-with-body || {
            log "WARNING: Vast.ai self-terminate failed (watchdog will clean up)."
        }
    fi
}

self_terminate_runpod() {
    # RUNPOD_POD_ID is injected automatically by RunPod into every pod
    if [ "${VOX_RUNPOD_SELF_TERMINATE:-0}" = "1" ] && [ -n "${RUNPOD_POD_ID:-}" ]; then
        log "Self-terminating RunPod pod: $RUNPOD_POD_ID"
        # Graceful stop first
        curl -s -X POST \
            "https://rest.runpod.io/v1/pods/${RUNPOD_POD_ID}/stop" \
            -H "Authorization: Bearer ${RUNPOD_API_KEY}" \
            -o /dev/null || true
    fi
}

self_terminate() {
    self_terminate_vast
    self_terminate_runpod
    log "Self-termination complete."
}

# Ensure termination fires even on unexpected exit
trap self_terminate EXIT

# ─────────────────────────────────────────────────────────────────────────────
# Job kind routing
# ─────────────────────────────────────────────────────────────────────────────

case "$JOB_KIND" in

# ── TRAINING ─────────────────────────────────────────────────────────────────
train)
    log "Mode: TRAINING"

    # 1. Download training dataset from HuggingFace Hub
    if [ -n "${VOX_TRAIN_DATA_HF:-}" ]; then
        log "Downloading dataset: $VOX_TRAIN_DATA_HF"
        huggingface-cli download \
            --repo-type dataset \
            "$VOX_TRAIN_DATA_HF" \
            --local-dir /workspace/data
        log "Dataset ready at /workspace/data"
    elif [ ! -d "/workspace/data" ] || [ -z "$(ls -A /workspace/data 2>/dev/null)" ]; then
        log "ERROR: No training data. Set VOX_TRAIN_DATA_HF or mount /workspace/data."
        exit 1
    fi

    # 2. Run training (--preset auto reads gpu-specs.yaml to select config)
    log "Starting training..."
    vox mens train \
        --backend qlora \
        --tokenizer hf \
        --model "$MODEL_ID" \
        --preset auto \
        --data-dir /workspace/data \
        --output-dir /workspace/output \
        --device cuda \
        ${VOX_TRAIN_EXTRA_ARGS:-}
    log "Training complete — adapter at /workspace/output"

    # 3. Upload adapter
    if [ -n "${VOX_ADAPTER_UPLOAD_HF:-}" ]; then
        log "Uploading adapter to: $VOX_ADAPTER_UPLOAD_HF"
        huggingface-cli upload \
            "$VOX_ADAPTER_UPLOAD_HF" \
            /workspace/output \
            --repo-type model
        log "Adapter uploaded."
    fi
    ;;

# ── INFERENCE / SERVE ─────────────────────────────────────────────────────────
serve)
    log "Mode: SERVE (port=$SERVE_PORT)"
    # Local checkpoint after train: /workspace/output. Requires `vox-schola` on PATH (shipped in vox-populi-cuda image).
    _SERVE_MODEL="${VOX_SERVE_MODEL_PATH:-/workspace/output}"
    vox mens serve \
        --host 0.0.0.0 \
        --port "$SERVE_PORT" \
        --model "$_SERVE_MODEL" \
        ${VOX_SERVE_EXTRA_ARGS:-}
    ;;

# ── AGENT ─────────────────────────────────────────────────────────────────────
agent)
    log "Mode: AGENT — forwarding to vox run --mode script"
    exec vox run --mode script "${VOX_AGENT_SCRIPT:-/opt/vox/mesh-noop.vox}"
    ;;

*)
    log "ERROR: Unknown VOX_JOB_KIND='$JOB_KIND'. Expected: train, serve, agent."
    exit 1
    ;;
esac

log "=== Done ==="
# EXIT trap fires self_terminate automatically
