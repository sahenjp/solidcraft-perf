#!/usr/bin/env bash
# Build the untouched baseline this project compares against.
#
#   perf/build-baseline.sh [commit]      default: the 0.10.0 release this branch starts from
#                                        (626fe6f, upstream/main)
#
# A git worktree is checked out under $PERF_CACHE (default ~/.cache/solidcraft-perf) and
# built with the release profile that commit itself carries, so the comparison isolates the
# changes in this branch from a profile change. Prints the binary path on the last line.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
COMMIT=${1:-626fe6f}
CACHE=${PERF_CACHE:-$HOME/.cache/solidcraft-perf}
SRC="$CACHE/baseline-$COMMIT"
TARGET="$CACHE/target-baseline-$COMMIT"
mkdir -p "$CACHE"
if [ ! -d "$SRC" ]; then
  git -C "$ROOT" worktree add --detach "$SRC" "$COMMIT"
fi
CARGO_TARGET_DIR="$TARGET" cargo build --release --manifest-path "$SRC/Cargo.toml" -p solidcraft-cli 2>&1 | tail -2
BIN="$TARGET/release/solidcraft-cli"
[ -x "$BIN" ] || { echo "build did not produce $BIN" >&2; exit 1; }
echo "$BIN"