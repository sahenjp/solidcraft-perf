#!/usr/bin/env bash
# Identity proof: loading a file must give the same model with both binaries, and a STEP
# round trip must produce the same bytes.
#
#   perf/identity.sh <baseline-cli> <candidate-cli> <file...>
#
# For every file: evaluate it with both binaries and compare the models with same_model.py -
# per-feature milliseconds ignored, structural differences fail, floating-point fields pass
# within PERF_MAX_REL relative (default 1e-8: the noise floor a changed tessellation order
# leaves on a wide shape is 5e-9, see perf/results/summary.md). For every STEP file: import it
# and export it again in two directories
# under the same output name (an exported header carries the file name), then compare the
# bytes, which must match exactly. Exits non-zero on any difference.
set -euo pipefail
[ $# -ge 3 ] || { sed -n "2,12p" "$0"; exit 2; }
B=$1; A=$2; shift 2
HERE=$(cd "$(dirname "$0")" && pwd)
TMP=${PERF_CACHE:-$HOME/.cache/solidcraft-perf}/identity
MAXREL=${PERF_MAX_REL:-1e-8}
mkdir -p "$TMP/b" "$TMP/a"
fail=0
for f in "$@"; do
  stem=$(basename "$f")
  "$B" eval "$f" --quiet > "$TMP/$stem.b.json" 2> "$TMP/$stem.err.b" || true
  "$A" eval "$f" --quiet > "$TMP/$stem.a.json" 2> "$TMP/$stem.err.a" || true
  if out=$(python3 "$HERE/same_model.py" "$TMP/$stem.b.json" "$TMP/$stem.a.json" "$MAXREL" 2>&1); then
    echo "eval identical: $stem  ($out)"
  else
    echo "eval DIFFERS: $stem"; echo "$out" | head -12; fail=1
  fi
  if [ -s "$TMP/$stem.err.b" ] || [ -s "$TMP/$stem.err.a" ]; then
    cmp -s "$TMP/$stem.err.b" "$TMP/$stem.err.a" || { echo "stderr DIFFERS: $stem"; diff "$TMP/$stem.err.b" "$TMP/$stem.err.a" | head; fail=1; }
  fi
done
echo "--- STEP round trips"
for f in "$@"; do
  case "$f" in *.step|*.stp) ;; *) continue ;; esac
  stem=$(basename "$f")
  "$B" run --in "$f" --out "$TMP/b/$stem" --quiet 2> "$TMP/$stem.rt.b.err" || true
  "$A" run --in "$f" --out "$TMP/a/$stem" --quiet 2> "$TMP/$stem.rt.a.err" || true
  if cmp -s "$TMP/b/$stem" "$TMP/a/$stem"; then
    echo "round-trip bytes identical: $stem ($(stat -c%s "$TMP/b/$stem") bytes)"
  else
    echo "round-trip DIFFERS: $stem"
    cmp -l "$TMP/b/$stem" "$TMP/a/$stem" | head -5
    fail=1
  fi
  cmp -s "$TMP/$stem.rt.b.err" "$TMP/$stem.rt.a.err" || { echo "round-trip stderr DIFFERS: $stem"; diff "$TMP/$stem.rt.b.err" "$TMP/$stem.rt.a.err" | head; fail=1; }
done
exit $fail