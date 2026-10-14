# Performance work in this fork

This branch tracks upstream SolidCraft, with one deliberate difference - a smaller release
profile - and it keeps the measurement harness behind the numbers that started the fork. The
kernel, document and STEP work it was built for is upstream as of 0.10.0.

## What this branch is

- Base: upstream `0.10.0` (`626fe6f`), synced from
  [bherbruck/solidcraft](https://github.com/bherbruck/solidcraft). Nothing in the kernel, the
  document model or the STEP reader is patched here any more.
- `Cargo.toml`: `lto = "fat"`, `codegen-units = 1`, `debug = 0`, `strip = "symbols"`. Upstream
  keeps thin LTO, 16 codegen units and line-tables-only debug on purpose, so that crash reports
  name `file:line` (see `docs/crash-reports.md`); a personal build trades that for size.
- `apps/solidcraft-web/index.html`: the trunk build skips the wasm-opt pass, because the binaryen
  the release pipeline bundles rejects the saturating float-to-int conversions the current Rust
  toolchain emits. An unoptimised wasm beats no browser build.
- `perf/`: the harness that measured the original pass and still measures this branch.

## What went upstream

[PR #714](https://github.com/bherbruck/solidcraft/pull/714) merged on 2026-10-13 and is in 0.10.0:
the prism route asked before a boolean meshes its operands, a body's size, volume and placement
measured once, an evaluation cache bounded in bytes, and a STEP reader that copies neither the
shells nor one-use item solids. The maintainer moved prism *cut* healing into the early route and
added the two new `Body` fields to a split-face constructor that landed on main meanwhile, then
timed it on his machine (`eval --quiet`, alternating builds, 5 runs each, busy box):

| Design | main | PR #714 | peak RSS |
|---|---:|---:|---:|
| 400 × 400 × 6 plate, 100 holes | 2.74 s | 1.99 s (-27%) | 178 → 135 MB |
| same plate, 400 holes | 9.76 s | 7.84 s (-20%) | 543 → 480 MB |

The Japanese interface came with it: 0.10.0 ships five locales with a complete Japanese catalog
(959 messages), so this fork's own translation pass is no longer carried.

## What the fork still changes: the release profile

Same 0.10.0 code, two release profiles, whole processes timed with the harness
(`perf/bench.sh`, 8 alternating runs each, medians with ranges; the machine was shared with
unrelated cargo test runs, so the wall times are directional and every range is wider than the
difference). Peak RSS is the process's own maximum resident set size.

| | upstream profile | personal profile |
|---|---:|---:|
| `solidcraft-cli` | 166.8 MiB | 22.2 MiB (-86.7%) |
| vent plate, one 10 × 8 pattern of a cut | 0.08 s / 23.8 MB | 0.08 s / 19.2 MB |
| 14 × 14 pattern of a filleted box | 1.55 s / 155.6 MB | 1.46 s / 150.1 MB |
| 60 sketches, 60 extrudes into one body | 9.96 s / 225.2 MB | 10.12 s / 220.1 MB |
| 100 circles cut through one plate | 0.82 s / 121.6 MB | 0.80 s / 116.5 MB |

Size is what the profile is for: upstream ships debug information so that crash reports name
`file:line`, and 145 MB of that binary is `.debug_*` sections; the personal profile builds with
`debug = 0` and `strip = "symbols"` and loses them. Speed is unchanged, peak RSS is a little lower
on all four designs, and the evaluation dumps of the two binaries are identical.

## The harness

`perf/` builds a candidate and a baseline CLI, times whole processes and reports medians with
their ranges, compares evaluation dumps for equality, and checks STEP round trips byte for byte.
`perf/README.md` has the rules the numbers depend on and the commands.

## History

[docs/perf-pass-0.3.2.md](perf-pass-0.3.2.md) is the 0.3.2-era report: the measurements, the
equivalence proofs and the limitations of the pass that became PR #714, kept as the record of
what was measured and how.
