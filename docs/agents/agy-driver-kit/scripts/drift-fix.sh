#!/bin/zsh
# Run ssot-drift; when it names a `vox ci ... --write` fix, run that and retry (max 8 rounds).
cd /Users/brbrainerd/dev/vox
for i in {1..8}; do
  out=$(timeout 1200s cargo run -q -p vox-cli -- ci ssot-drift 2>&1)
  err=$(print -r -- "$out" | grep -m1 "^Error")
  if [[ -z $err ]]; then print -r -- "$out" | tail -3; echo "DRIFT OK after $i round(s)"; exit 0; fi
  echo "round $i: $err"
  fix=$(print -r -- "$err" | grep -oE 'vox ci [a-z0-9-]+( --target [a-z]+)? --write')
  [[ -z $fix ]] && { echo "no auto-fix hint; stopping"; exit 1; }
  echo "  running: $fix"
  timeout 1200s cargo run -q -p vox-cli -- ${=${fix#vox }} 2>&1 | grep -E "wrote|Error" | head -3
done
echo "gave up after 8 rounds"; exit 1
