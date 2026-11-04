#!/usr/bin/env bash
# S4 4B full run + watchdog. Ops evidence helper, not project automation.
# Usage: s4-4B-run.sh <tag> <free_mb_at_launch> [--resume]
#   tag: launch label (a, b, ...) for log/exit file names.
# Watchdog (same rules as the S3 re-probe): SIGINT to the vox-ml-cli PID
# (the launcher does not forward signals), SIGKILL 120 s later, when
#   - max peak_metal_allocated_bytes > (free_at_launch - 4096) MiB
#   - a row takes > 5x the previous row (and > 60 s)
#   - no row for > 5x the last interval (and > 60 s), outside cap/validation/finalize
#   - deadline 90000 s (trainer cap is 86400 s)
set -uo pipefail
R=/Users/brbrainerd/dev/vox-mens-compare
E=$R/.planning/phases/320-train-the-sizes-size-03/evidence
D=$R/mens/runs/qwen3-4b
TAG=$1; FREE=$2; MODE=${3:-}
LOG=$E/s4-4B-$TAG-log.txt; WD=$E/s4-4B-$TAG-watchdog.txt
LIMIT=$(( (FREE - 4096) * 1048576 ))
DEADLINE=90000
cd "$R"

ARGS=(mens train --backend qlora --tokenizer hf --device metal --require-gpu
  --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal
  --model Qwen3-4B --output-dir mens/runs/qwen3-4b --max-runtime-secs 86400)
[ "$MODE" = "--resume" ] && ARGS+=(--resume mens/runs/qwen3-4b)

date -u +%Y-%m-%dT%H:%M:%SZ > "$E/s4-4B-$TAG.start"
LAUNCH_MS=$(( $(date +%s) * 1000 ))
echo "CMD: VOX_MENS_FORCE_TRAIN=${VOX_MENS_FORCE_TRAIN:-} target/release/vox ${ARGS[*]}" > "$LOG"
target/release/vox "${ARGS[@]}" >> "$LOG" 2>&1 &
VPID=$!

echo "limit $LIMIT (free $FREE MiB - 4096 MiB) deadline ${DEADLINE}s vox_pid=$VPID" > "$WD"
MLPID=""; killed=0; T0=$(date +%s)
while kill -0 "$VPID" 2>/dev/null; do
  sleep 5
  [ -z "$MLPID" ] && MLPID=$(pgrep -f 'vox-ml-cli mens train.*qwen3-4b' | head -1)
  [ -z "$MLPID" ] && continue
  [ $killed -ne 0 ] && continue
  now=$(date +%s); reason=""
  # rows from this launch: ts_ms step peak
  read -r n peak t1 t2 t3 < <(jq -r --argjson s "$LAUNCH_MS" \
    'select(.event=="step" and .ts_ms>=$s) | "\(.ts_ms) \(.payload.peak_metal_allocated_bytes // 0)"' \
    "$D/telemetry.jsonl" 2>/dev/null | awk '{n++; if($2>p)p=$2; a=b; b=c; c=$1} END{printf "%d %d %d %d %d\n", n, p, a, b, c}')
  if [ "${peak:-0}" -gt "$LIMIT" ]; then reason="peak $peak B > limit $LIMIT B"; fi
  if [ -z "$reason" ] && [ "${n:-0}" -ge 3 ] && [ "$t1" -gt 0 ]; then
    prev=$(( t2 - t1 )); last=$(( t3 - t2 ))
    if [ $last -gt $(( 5 * prev )) ] && [ $last -gt 60000 ]; then reason="row slowdown ${last}ms > 5x ${prev}ms"; fi
  fi
  if [ -z "$reason" ] && [ "${n:-0}" -ge 2 ] && ! grep -qE 'run cap reached|Training complete|Running validation' "$LOG"; then
    gap=$(( now * 1000 - t3 )); last=$(( t3 - t2 ))
    if [ $gap -gt $(( 5 * last )) ] && [ $gap -gt 60000 ]; then reason="stall ${gap}ms since last row (> 5x ${last}ms)"; fi
  fi
  [ -z "$reason" ] && [ $(( now - T0 )) -gt $DEADLINE ] && reason="deadline ${DEADLINE}s"
  if [ -n "$reason" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) SIGINT ml_pid=$MLPID n=$n: $reason" >> "$WD"
    kill -INT "$MLPID"; killed=$now
    ( sleep 120; kill -0 "$MLPID" 2>/dev/null && kill -KILL "$MLPID" && echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) SIGKILL ml_pid=$MLPID" >> "$WD" ) &
  fi
done
wait "$VPID"; ec=$?
echo "$ec" > "$E/s4-4B-$TAG.exit"
date -u +%Y-%m-%dT%H:%M:%SZ > "$E/s4-4B-$TAG.end"
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) watchdog done; killed_at=$killed; last peak=${peak:-0} n=${n:-0} exit=$ec" >> "$WD"
