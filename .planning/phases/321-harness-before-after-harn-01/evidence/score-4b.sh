#!/usr/bin/env bash
# 4B scoring legs, same commands as 0.6B/1.7B (321-02/03), one GPU job at a time.
# Ops evidence helper, not project automation. A leg whose output exists is skipped.
# Collateral uses fail-fast order vox_think,gsm8k_think,math500_think (user decision 2026-10-30).
set -uo pipefail
R=/Users/brbrainerd/dev/vox-mens-compare
E=$R/.planning/phases/321-harness-before-after-harn-01/evidence
RUN=mens/runs/qwen3-4b
SNAP=$HOME/.cache/huggingface/hub/models--Qwen--Qwen3-4B/snapshots/1cfa9a7208912126459214e8b04321603b3df60c
BENCH=mens/data/heldout_bench
cd "$R"; mkdir -p $RUN/evidence

# LEGS: space-separated leg names to run (default: all, in file order).
LEGS=${LEGS:-compare heldout-base heldout-adapter baseline collateral serve-file serve-dir}
leg() { # name out cmd...
  local name=$1 out=$2; shift 2
  case " $LEGS " in *" $name "*) ;; *) return 0 ;; esac
  local log=$E/score-4b-$name.log
  if [ -n "$out" ] && [ -s "$out" ]; then echo "SKIP $name ($out exists)" >> $E/score-4b-legs.txt; return 0; fi
  echo "START $name 4B $(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$log"
  echo "CMD: $*" >> "$log"
  "$@" >> "$log" 2>&1; local ec=$?
  echo "END $name 4B exit=$ec $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$log"
  echo "$name exit=$ec $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> $E/score-4b-legs.txt
  return $ec
}

V=target/release/vox
leg compare mens/runs/reasoning_compare/Qwen3-4B-1cfa9a720891.json \
  $V mens eval-reasoning-compare --base Qwen3-4B --adapter $RUN --out mens/runs/reasoning_compare/Qwen3-4B-1cfa9a720891.json || exit 1
leg heldout-base $RUN/evidence/heldout_base.json \
  $V mens eval-local --base "$SNAP" --bench $BENCH -o $RUN/evidence/heldout_base.json || exit 1
leg heldout-adapter $RUN/evidence/heldout_adapter.json \
  $V mens eval-local --model $RUN --bench $BENCH -o $RUN/evidence/heldout_adapter.json || exit 1
leg baseline $RUN/evidence/collateral_baseline.json \
  $V mens baseline-from-compare --compare mens/runs/reasoning_compare/Qwen3-4B-1cfa9a720891.json -o $RUN/evidence/collateral_baseline.json || exit 1
# exit 1 = collateral fail, a valid outcome; continue to the gate either way
leg collateral "" \
  $V mens eval-collateral-damage --pre-score $RUN/evidence/collateral_baseline.json --post-adapter $RUN \
  --fail-fast vox_think,gsm8k_think,math500_think
# serve gate: adapter file must be refused; run dir must be refused on a failing report.
# A run that passes the gate would start listening, so each attempt is bounded.
leg serve-file "" timeout 180 $V mens serve --model $RUN/candle_qlora_adapter.safetensors --host 127.0.0.1 --port 11437
leg serve-dir "" timeout 180 $V mens serve --model $RUN --host 127.0.0.1 --port 11437
echo "DONE $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> $E/score-4b-legs.txt
