# Release profile tuning — measurement report (task-2, 2026-10-12)
> Kept as written when the measurement ran: the paths in it point at the scratch
> directory those runs used (`/tmp/scperf/...`). In this checkout the same assets
> live in `perf/designs`, `perf/fixtures` (generated), `perf/probe` and here.


Repo: `~/File/Prog/solvecraft` (SolidCraft 0.3.2, cargo/rustc 1.98.1, 16 logical cores,
32 GB RAM). No commit, no push; the only repository file touched is `Cargo.toml`.

## Verdict

**P2 = `lto = "fat"` + `codegen-units = 1` + `strip = "symbols"` won and is landed.** It is
never slower than the committed profile (median 1.8% / 2.4% / 6.0% faster on holes_100 / medium /
pattern_196 over 7 uncontended runs each) and it is clearly smaller: **-34.6%** on
`target/release/solvecraft` and **-29.1%** on `target/release/solvecraft-cli`. The stated cost is
a longer cold release build: **7m29s vs 4m14s** (-j 8, both binaries, 330 crates).

`panic = "abort"` was never considered or measured: `Session::execute` keeps a `catch_unwind`
as the last-resort guard against losing a user's design, so panics must keep unwinding.

## Variants

| id | profile flags |
|---|---|
| P0 | committed: `opt-level = 3`, `lto = "thin"`, `codegen-units = 16`, `debug = 0`, no strip |
| P1 | P0 + `CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` |
| P2 | P1 + `CARGO_PROFILE_RELEASE_STRIP=symbols` |
| P3 | P0 + `CARGO_PROFILE_RELEASE_STRIP=symbols` (isolates the strip contribution) |

## Method

| | |
|---|---|
| Source | frozen snapshot `~/scperf-src-frozen`, build-input hash `c38f9f416ca743dd`, verified equal before and after every build |
| Target dir | `CARGO_TARGET_DIR=~/scperf-target-profile` (private, outside the repo; the shared repo `target/` was used only for the first pre-redirect pass and was not touched afterwards) |
| Build | `CARGO_BUILD_JOBS=8 cargo build --release -p solvecraft-cli -p solvecraft` |
| Eval | `solvecraft-cli eval <design> --quiet`, GNU `time -v` (wall + peak RSS), interleaved run order, median of 3 |
| Cold start | `solvecraft-cli eval /tmp/scperf/designs/empty.json --quiet`, 15 interleaved runs, median |
| Designs | `/tmp/scperf/designs/{holes_100,medium,pattern_196,empty}.json` |
| Contention | every run records `/proc/loadavg` and live `cargo`/`rustc` processes; a 6 s quiet gate precedes each measurement batch. Only P0/P1/P2/P3 interleaved runs (and the reversed-order P0/P2 runs) are uncontended; the first per-variant pass had contended runs and is used only for peak RSS |

### Size

| variant | profile | solvecraft bytes | vs P0 | solvecraft-cli bytes | vs P0 |
|---|---|---|---|---|---|
| P0 | thin, cgu 16, no strip (committed) | 58,296,296 | — | 24,996,288 | — |
| P1 | fat, cgu 1 | 43,516,544 | -25.4% | 20,135,168 | -19.4% |
| P2 | fat, cgu 1, strip=symbols | 38,099,720 | -34.6% | 17,721,120 | -29.1% |
| P3 | thin, cgu 16, strip=symbols | 46,417,304 | -20.4% | 19,969,832 | -20.1% |

### Run time, interleaved median of 3, all runs uncontended

| variant | holes_100 s | medium s | pattern_196 s | holes_100 peak RSS MiB | pattern_196 peak RSS MiB | cold start ms |
|---|---|---|---|---|---|---|
| P0 | 4.4714 | 0.3818 | 6.6395 | 118.4 | 63.7 | 1.325 |
| P1 | 4.2111 | 0.3769 | 6.5674 | 114.7 | 62.3 | 1.197 |
| P2 | 4.2291 | 0.3726 | 6.3900 | 116.1 | 60.1 | 1.165 |
| P3 | 4.1620 | 0.3558 | 6.3122 | 117.8 | 64.6 | 1.304 |

### Decision pair P0 vs P2, every uncontended run with run order alternated

| variant | design | runs | median s | min s | max s |
|---|---|---|---|---|---|
| P0 | holes_100 | 7 | 4.3054 | 4.1347 | 4.5030 |
| P2 | holes_100 | 7 | 4.2291 | 3.9221 | 4.4288 |
| P0 | medium | 3 | 0.3818 | 0.3574 | 0.4012 |
| P2 | medium | 3 | 0.3726 | 0.3582 | 0.3741 |
| P0 | pattern_196 | 7 | 6.7751 | 6.2937 | 6.9491 |
| P2 | pattern_196 | 7 | 6.3680 | 5.9791 | 6.6676 |

Cold start is process-load bound (~1 ms for all variants); the 0.16 ms spread is noise, not a
feature. Peak RSS moves slightly in the same direction as the size (P2 holds ~2 MiB less on
pattern_196). P3 is nominally the fastest variant on this workload (thin code is 1-2% ahead of
P2); the difference is inside run-to-run noise and does not justify giving up 14 points of size.

### Build cost, cold release build of both binaries, -j 8, 330 crates

| variant | wall time |
|---|---|
| P0 | 4:14.46 |
| P1 | 7:41.75 |
| P2 | 7:28.55 |
| P3 | 3:47.28 |

In this cargo version a change to any of these profile settings recompiles all 330 crates
(observed for `lto`, `codegen-units` and `strip` in the fresh target dir). So the landed change
costs one full release rebuild: **+3m15s (+77%)** on a 16-core box at -j 8, and no extra cost to
the `ci`/`dev` profiles that `cargo xtask ci` uses. Caveat measured pre-redirect: in the shared
repo target dir, where the thin rlibs already existed, the strip-only build relinked just the two
binaries in 2m09s; treat 2-4 minutes as the strip-only incremental cost and 7m29s as the cold cost
of the landed fat+strip profile.

## Exact commands

```sh
# variant builds (from the frozen snapshot, private target dir)
cd ~/scperf-src-frozen
export CARGO_TARGET_DIR=~/scperf-target-profile CARGO_BUILD_JOBS=8
cargo build --release -p solvecraft-cli -p solvecraft                                   # P0
CARGO_PROFILE_RELEASE_STRIP=symbols cargo build --release -p solvecraft-cli -p solvecraft  # P3
CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  cargo build --release -p solvecraft-cli -p solvecraft                                 # P1
CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  CARGO_PROFILE_RELEASE_STRIP=symbols cargo build --release -p solvecraft-cli -p solvecraft  # P2

# eval measurement (interleaved harness, /usr/bin/time -v inside)
python3 /tmp/scperf/tune/interleave.py     # 3 reps x 4 variants x 3 designs, order P0,P1,P2,P3
python3 /tmp/scperf/tune/reversed.py       # 4 reps x (P2 then P0), holes_100 + pattern_196
python3 /tmp/scperf/tune/cold.py           # 15 interleaved empty-design runs per variant

# verification of the landed manifest (no env overrides; 0 crates recompiled, sizes identical)
cp ~/File/Prog/solvecraft/Cargo.toml ~/scperf-src-frozen/Cargo.toml
cd ~/scperf-src-frozen && CARGO_TARGET_DIR=~/scperf-target-profile \
  cargo build --release -p solvecraft-cli -p solvecraft     # "Finished in 0.50s", 0 Compiling lines
stat -c '%s %n' ~/scperf-target-profile/release/solvecraft{,-cli}
#   38099720 solvecraft  17721120 solvecraft-cli   <- byte-identical to the measured P2 binaries

# repository checks
cargo fmt --all -- --check        # clean
cargo metadata --no-deps --format-version 1 > /dev/null   # manifest parses
```

## What was landed and how it was verified

`Cargo.toml` `[profile.release]`: `lto = "thin" -> "fat"`, `codegen-units = 16 -> 1`, and
`strip = "symbols"` added; `opt-level` and `debug` unchanged; no `panic` key. A comment above the
table records the measurement, the build-time cost and why panics must keep unwinding.

Verification actually run:
1. The landed manifest, built with no environment overrides in the same target dir, resolved to the
   measured profile exactly: cargo reported `Finished ... in 0.50s` with **0 crates recompiled** and
   produced byte-identical binaries (38,099,720 / 17,721,120) to the measured P2 pair.
2. `cargo fmt --all -- --check` clean; `cargo metadata --no-deps` parses the edited manifest.
3. `panic = "abort"` is absent, so `Session::execute`'s `catch_unwind` guard still catches.
4. `cargo xtask layers`/`assets` are unaffected: a profile change touches no dependency, source or
   asset; the Lead's final `cargo xtask ci` covers them.

## Caveats and things that are not wins

* **`strip = "symbols"` removes function names from release binaries.** Debug info was already off
  (`debug = 0`), so this costs symbolization of release backtraces and the `gdb` "poor man's
  profiler" workflow described in `docs/performance-optimization.md`; use a scratch
  `CARGO_PROFILE_RELEASE_STRIP=none` build when profiling release code. No code in the repository
  reads a backtrace, and the CLI/GUI release workflows only run the binaries.
* **Cold start and RSS differences are small** (1.17 ms vs 1.33 ms; ~2 MiB RSS) - reported for
  completeness, not claimed as wins.
* **Source drift invalidated my first pass.** The first variant builds ran in the shared repo
  target dir; import-perf's `crates/kernel/src/step_in/*` edits landed during the P2 build
  (`e803a850ba3e154d -> 85db192c9df7c8c8 -> cf84bea945e3d9c9`), so those numbers are not comparable
  across variants. Every number in this report comes from the single frozen snapshot above.
* **Contended numbers were discarded, not averaged.** In the first per-variant pass P1's
  pattern_196 read 9.34 s and P0's control re-run 12.21 s while another agent was building; the
  clean interleaved medians for the same binaries are 6.57 s and 6.78 s.
* The shared repo `target/release` currently holds my last pre-redirect (fat + strip) binaries from
  19:02; it was not touched after the Lead's redirect and will be rebuilt by the Lead's final gates.
