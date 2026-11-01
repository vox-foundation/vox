#!/usr/bin/env bash
# 4B gsm8k/math500 compare, collateral and gate on the think128 binary (plan c, 2026-10-31). Ops evidence helper.
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
LEGS=${LEGS:-compare-vg baseline-vg collateral-vg compare-full baseline-full collateral-full serve-file2 serve-dir2}
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
V=target/release-think128/vox-ml-cli   # old held-out protocol, so collateral matches 0.6B/1.7B (see binaries.txt)
B=$RUN/evidence
leg compare-vg $CMP/Qwen3-4B-1cfa9a720891-vox_gsm8k.json \
  $V mens eval-reasoning-compare --base Qwen3-4B --adapter $RUN --slices vox_think,gsm8k_think \
  --out $CMP/Qwen3-4B-1cfa9a720891-vox_gsm8k.json || exit 1
leg baseline-vg $B/collateral_baseline_vox_gsm8k.json \
  $V mens baseline-from-compare --compare $CMP/Qwen3-4B-1cfa9a720891-vox_gsm8k.json -o $B/collateral_baseline_vox_gsm8k.json || exit 1
leg collateral-vg "" \
  $V mens eval-collateral-damage --pre-score $B/collateral_baseline_vox_gsm8k.json --post-adapter $RUN \
  --fail-fast vox_think,gsm8k_think,math500_think
# Only when fail-fast got past gsm8k_think (stopped for want of a math500 baseline) is math500 scored.
if grep -q "no 'math500_think'" $E/score-4b-collateral-vg.log; then
  leg compare-full $CMP/Qwen3-4B-1cfa9a720891.json \
    $V mens eval-reasoning-compare --base Qwen3-4B --adapter $RUN --out $CMP/Qwen3-4B-1cfa9a720891.json || exit 1
  leg baseline-full $B/collateral_baseline_full.json \
    $V mens baseline-from-compare --compare $CMP/Qwen3-4B-1cfa9a720891.json -o $B/collateral_baseline_full.json || exit 1
  leg collateral-full "" \
    $V mens eval-collateral-damage --pre-score $B/collateral_baseline_full.json --post-adapter $RUN \
    --fail-fast vox_think,gsm8k_think,math500_think
fi
leg serve-file2 "" timeout 180 $V mens serve --model $RUN/candle_qlora_adapter.safetensors --host 127.0.0.1 --port 11437
leg serve-dir2 "" timeout 180 $V mens serve --model $RUN --host 127.0.0.1 --port 11437
echo "DONE-THINK128 $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> $E/score-4b-legs.txt
