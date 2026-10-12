#!/usr/bin/env bash
# Wall time, CPU time and peak RSS for one or more designs, alternating two binaries.
#
#   perf/bench.sh <binary-a> <binary-b> [runs] [design ...]
#
# Defaults: runs=5 and four generated designs (medium, holes_100, pattern_196,
# many_sketches_60). Each measurement is a fresh process:
#
#   /usr/bin/time -v <binary> eval <design> --quiet
#
# and one CSV row per run is printed on stdout:
#
#   design,label,run,wall_s,user_s,sys_s,max_rss_mb
#
# The rules the numbers depend on:
#   * nothing else may build, test or benchmark while this runs - cargo across the whole
#     machine makes the numbers meaningless (the script refuses to start while cargo runs,
#     unless PERF_ALLOW_LOAD=1 is set, and then the result must be treated as contended);
#   * runs alternate A/B, so a machine that drifts affects both sides equally;
#   * compare binaries built the way perf/README.md describes - a release profile change
#     moves the numbers by itself.
set -euo pipefail
[ $# -ge 2 ] || { sed -n "2,20p" "$0"; exit 2; }
A=$1; B=$2; shift 2
RUNS=5; if [ $# -gt 0 ]; then RUNS=$1; shift; fi
case "$RUNS" in (*[!0-9]*|"") echo "runs must be a number" >&2; exit 2;; esac
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DESIGNS=("$@")
if [ ${#DESIGNS[@]} -eq 0 ]; then
  DESIGNS=("$ROOT/perf/designs/medium.json" "$ROOT/perf/designs/holes_100.json" "$ROOT/perf/designs/pattern_196.json" "$ROOT/perf/designs/many_sketches_60.json")
fi
for f in "$A" "$B"; do
  [ -x "$f" ] || { echo "not executable: $f" >&2; exit 2; }
done
[ -x /usr/bin/time ] || { echo "GNU time (/usr/bin/time) is required" >&2; exit 2; }
if pgrep -x cargo >/dev/null 2>&1 && [ "${PERF_ALLOW_LOAD:-0}" != 1 ]; then
  echo "cargo is running: the numbers would be contended. Wait for it, or set PERF_ALLOW_LOAD=1." >&2
  exit 3
fi
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
echo "design,label,run,wall_s,user_s,sys_s,max_rss_mb"
run_one() {  # <binary> <label> <design> <run>
  /usr/bin/time -v -o "$TMP" "$1" eval "$3" --quiet >/dev/null 2>/dev/null || true
  awk -F': ' -v design="$(basename "$3" .json)" -v label="$2" -v run="$4" '
    function secs(t, p) { n = split(t, p, ":"); if (n == 3) return p[1]*3600 + p[2]*60 + p[3]; if (n == 2) return p[1]*60 + p[2]; return t + 0 }
    /Elapsed \(wall/ { wall = $2 }
    /User time/     { user = $2 }
    /System time/   { sys = $2 }
    /Maximum resident/ { rss = $2 }
    END { printf "%s,%s,%s,%.2f,%.2f,%.2f,%.1f\n", design, label, run, secs(wall), user, sys, rss/1024 }' "$TMP"
}
for design in "${DESIGNS[@]}"; do
  for run in $(seq 1 "$RUNS"); do
    if [ $((run % 2)) -eq 1 ]; then
      run_one "$A" before "$design" "$run"
      run_one "$B" after  "$design" "$run"
    else
      run_one "$B" after  "$design" "$run"
      run_one "$A" before "$design" "$run"
    fi
  done
done