# Measuring this fork

This directory holds the measurement harness and the raw results behind the numbers in the
project README and `docs/performance-optimization.md`. Everything here runs against the
`solvecraft-cli` built from this checkout; no measurement code is part of the application.

## The rules the numbers depend on

- One process per measurement (`solvecraft-cli eval <design> --quiet`), so nothing is warm.
- Nothing else may build, test or benchmark at the same time. cargo on any core moves both the
  wall time and the peak RSS; `perf/bench.sh` refuses to start while cargo is running.
- Runs alternate A/B, so machine drift hits both binaries equally.
- Medians of the runs, with the observed range: a range wider than the difference means the
  difference is not a result. `perf/summarize.py` prints both.
- Peak RSS is the `Maximum resident set size` of that one process (GNU `time -v`).
- When the comparison is about code, both binaries must be built with the same release
  profile. `perf/build-baseline.sh` builds the baseline with the profile its own commit
  carries, which is why a profile change and a code change are reported separately.

## Build the two binaries

```
cargo build --release -p solvecraft-cli          # the candidate: this branch
perf/build-baseline.sh 07e263d                   # the untouched baseline; prints its path
```

`perf/build-baseline.sh` keeps a git worktree and its target directory under
`$PERF_CACHE` (default `~/.cache/solidcraft-perf`), so the checkout stays clean.

## Time, CPU time and peak memory

```
perf/bench.sh <baseline-cli> <candidate-cli> 5 | tee perf/results/bench.csv
python3 perf/summarize.py perf/results/bench.csv
```

The default designs are `perf/designs/{medium,holes_100,pattern_196,many_sketches_60}.json`.
Further design paths go after the run count, for example:

```
perf/bench.sh "$BASE" target/release/solvecraft-cli 5 perf/designs/samples/*.json
```

## Output equivalence

The optimisations must not change what the app computes.

```
perf/corpus.sh <baseline-cli>  perf/results/corpus-before
perf/corpus.sh <candidate-cli> perf/results/corpus-after
python3 perf/cmp_all.py perf/results/corpus-before perf/results/corpus-after
```

`cmp_all.py` ignores the per-feature millisecond fields and reports every structural and
numeric difference with its relative size. The line to look for is `no structural differences`;
floating-point noise in the last bits is expected and is printed with its magnitude.

## Byte-identical STEP round trips

```
perf/identity.sh <baseline-cli> <candidate-cli> perf/fixtures/*.step perf/designs/*.json
```

Evaluates every file with both binaries (the normalised output must match byte for byte) and
runs every STEP file through import and export in two directories, comparing the exported
bytes. An untouched body must also stay the very same body, which is what
`ops::prism_fast_path_tests` covers at the kernel level.

## Large-CAD fixtures

The generated STEP files are 60 MB together and are not committed, and they are written
outside the checkout so the asset gate never sees them ($PERF_CACHE/fixtures, default
~/.cache/solidcraft-perf/fixtures). Regenerate them from the designs with the CLI:

```
python3 perf/gen_step_fixtures.py                 # -> ~/.cache/solidcraft-perf/fixtures
perf/identity.sh <baseline-cli> <candidate-cli> ~/.cache/solidcraft-perf/fixtures/*.step
```

## Where a session's heap goes

`perf/probe` is a scratch crate with a counting global allocator. It is not part of the
application and not in the workspace.

```
cargo build --release --manifest-path perf/probe/Cargo.toml
perf/probe/target/release/scperf-probe perf/designs/many_sketches_60.json --report
perf/probe/target/release/scperf-probe perf/designs/many_sketches_60.json --dismantle
```

`--report` prints the retained bytes of the model states; `--dismantle` clears one holder at a
time, so each release shows up in the live-heap counter. What it showed for a long timeline is
in `docs/performance-optimization.md`.

## Results and reports

- `perf/results/summary.md` - the tables, the equivalence check and the identity check in one
  place
- `perf/results/bench.csv` - raw `perf/bench.sh` output, one row per run; `bench.md` is the
  table `perf/summarize.py` produced from it
- `perf/results/bench-code-only.csv` and `bench-profile.csv` - the same measurement with the
  code change isolated (one profile, two sources) and the profile isolated (one source, two
  profiles)
- `perf/results/equivalence.txt` - the corpus comparison (`perf/corpus.sh` + `cmp_all.py`)
- `perf/results/identity.txt` - the evaluation and STEP round-trip check (`perf/identity.sh`)
- `perf/reports/step-import.md`, `perf/reports/release-profile.md` - the two focused
  measurement records, kept as they were written (their paths point at the scratch directory
  the runs used)

## What is not measured here

`cargo xtask wasm` needs the `wasm32-unknown-unknown` target. It was not installed on the
machine that produced these numbers, and installing it needs root, so that gate is still
open; `cargo xtask ci` was run on the committed revision and everything except the wasm check
passed.
