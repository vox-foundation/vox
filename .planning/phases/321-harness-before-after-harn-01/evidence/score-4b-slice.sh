#!/usr/bin/env bash
# 4B scoring after the shared per-item checkpoint (31c692401). Ops evidence helper.
# LEGS picks legs (default: the gate path). A leg whose output exists is skipped.
# Every generation is checkpointed, so a stopped leg is rerun to resume.
# Watchdog: SIGINT to vox-ml-cli when its leg log has not grown for STALL s
# (default 3600; one 4B reasoning item takes about 8 min), SIGKILL 120 s later.
set -uo pipefail
R=/Users/brbrainerd/dev/vox-mens-compare
E=$R/.planning/phases/321-harness-before-after-harn-01/evidence
RUN=mens/runs/qwen3-4b
CMP=mens/runs/reasoning_compare
STALL=${STALL:-3600}
cd "$R"; mkdir -p $RUN/evidence
LEGS=${LEGS:-compare-vox baseline-vox collateral serve-file serve-dir}
leg() { # name out cmd...
  local name=$1 out=$2; shift 2
  case " $LEGS " in *" $name "*) ;; *) return 0 ;; esac
  local log=$E/score-4b-$name.log
  if [ -n "$out" ] && [ -s "$out" ]; then echo "SKIP $name ($out exists)" >> $E/score-4b-legs.txt; return 0; fi
  echo "START $name 4B $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$log"
  echo "CMD: $*" >> "$log"
  "$@" >> "$log" 2>&1 & local pid=$!
  local last=$(date +%s) size=0
  while kill -0 $pid 2>/dev/null; do
    sleep 30
    local s=$(stat -f %z "$log"); local now=$(date +%s)
    if [ "$s" != "$size" ]; then size=$s; last=$now; continue; fi
    if [ $((now - last)) -gt $STALL ]; then
      local ml=$(pgrep -f "vox-ml-cli mens" | head -1)
      echo "WATCHDOG $(date -u +%H:%M:%SZ) stall $((now-last))s, SIGINT $ml" >> "$log"
      [ -n "$ml" ] && kill -INT $ml; sleep 120; [ -n "$ml" ] && kill -0 $ml 2>/dev/null && kill -KILL $ml
      last=$now
    fi
  done
  wait $pid; local ec=$?
  echo "END $name 4B exit=$ec $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$log"
  echo "$name exit=$ec $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> $E/score-4b-legs.txt
  return $ec
}
V=target/release/vox
leg compare-vox $CMP/Qwen3-4B-1cfa9a720891-vox_think.json \
  $V mens eval-reasoning-compare --base Qwen3-4B --adapter $RUN --slices vox_think \
  --out $CMP/Qwen3-4B-1cfa9a720891-vox_think.json || exit 1
leg baseline-vox $RUN/evidence/collateral_baseline.json \
  $V mens baseline-from-compare --compare $CMP/Qwen3-4B-1cfa9a720891-vox_think.json \
  -o $RUN/evidence/collateral_baseline.json || exit 1
# exit 1 = collateral fail, a valid outcome; an Err (unscored slice reached) also continues to the gate
leg collateral "" \
  $V mens eval-collateral-damage --pre-score $RUN/evidence/collateral_baseline.json --post-adapter $RUN \
  --fail-fast vox_think,gsm8k_think,math500_think
leg serve-file "" timeout 180 $V mens serve --model $RUN/candle_qlora_adapter.safetensors --host 127.0.0.1 --port 11437
leg serve-dir "" timeout 180 $V mens serve --model $RUN --host 127.0.0.1 --port 11437
# Full compare: vox_think generations come from the checkpoint; only gsm8k/math500 generate.
leg compare-full $CMP/Qwen3-4B-1cfa9a720891.json \
  $V mens eval-reasoning-compare --base Qwen3-4B --adapter $RUN --out $CMP/Qwen3-4B-1cfa9a720891.json || exit 1
echo "DONE-SLICE $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> $E/score-4b-legs.txt
