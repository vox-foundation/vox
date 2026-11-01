#!/usr/bin/env bash
# Held-out re-score on the non-thinking codegen protocol (448df0192). Ops evidence helper.
# Resumable: a stopped run is rerun; an existing report is skipped.
set -uo pipefail
R=/Users/brbrainerd/dev/vox-mens-compare; E=$R/.planning/phases/321-harness-before-after-harn-01/evidence
cd "$R"; V=target/release/vox; H=$HOME/.cache/huggingface/hub
run() { # size leg args...
  local out=mens/runs/$1/evidence/heldout_$2_nothink.json log=$E/heldout-nothink-$1-$2.log; shift 2
  [ -s $out ] && { echo "SKIP $out" >> $E/heldout-nothink-legs.txt; return; }
  bash /private/tmp/claude-501/-Users-brbrainerd-dev-vox-voxdiagram/29f8e79e-7b0b-4197-9cfe-34ec71ab397b/scratchpad/mem.sh "before heldout-nothink $out" > /dev/null
  echo "START $(date -u +%FT%TZ)" >> $log
  $V mens eval-local "$@" --bench mens/data/heldout_bench -o $out >> $log 2>&1
  echo "$out exit=$? $(date -u +%FT%TZ)" | tee -a $log >> $E/heldout-nothink-legs.txt
}
run qwen3-0.6b base --base $H/models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca
run qwen3-0.6b adapter --model mens/runs/qwen3-0.6b
run qwen3-1.7b base --base $H/models--Qwen--Qwen3-1.7B/snapshots/70d244cc86ccca08cf5af4e1e306ecf908b1ad5e
run qwen3-1.7b adapter --model mens/runs/qwen3-1.7b
run qwen3-4b base --base $H/models--Qwen--Qwen3-4B/snapshots/1cfa9a7208912126459214e8b04321603b3df60c
run qwen3-4b adapter --model mens/runs/qwen3-4b
echo "DONE $(date -u +%FT%TZ)" >> $E/heldout-nothink-legs.txt
