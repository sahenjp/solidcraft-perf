#!/usr/bin/env bash
# Evaluate a corpus of designs with one binary, one JSON file per design.
#
#   perf/corpus.sh <binary> <outdir> [design ...]
#
# Default designs: every JSON under perf/designs (the generated corpus) plus the bundled
# samples under perf/designs/samples. Compare two runs with:
#
#   python3 perf/cmp_all.py <outdir-a> <outdir-b>
#
# which ignores the per-feature millisecond fields and reports every structural or numeric
# difference; "no structural differences" is the line that matters.
set -euo pipefail
[ $# -ge 2 ] || { sed -n "2,12p" "$0"; exit 2; }
BIN=$1; OUT=$2; shift 2
ROOT=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$OUT"
DESIGNS=("$@")
if [ ${#DESIGNS[@]} -eq 0 ]; then
  while IFS= read -r f; do DESIGNS+=("$f"); done < <(find "$ROOT/perf/designs" -name "*.json" | sort)
fi
for d in "${DESIGNS[@]}"; do
  name=$(basename "$d" .json)
  # A sample with the same name as a generated design keeps its own directory prefix.
  case "$d" in */samples/*) name="sample_$name";; esac
  "$BIN" eval "$d" --quiet >"$OUT/$name.json" 2>"$OUT/$name.err" || true
  printf "%s\n" "$name"
done