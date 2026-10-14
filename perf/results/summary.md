# Measured results

Environment: Fedora 44, x86_64, 16 logical cores, 32 GB RAM, rustc 1.98.1. Every number below
was taken with nothing else building, testing or benchmarking on the machine; each measurement
is a fresh process of `solvecraft-cli eval <design> --quiet`.

| binary | source | release profile | bytes |
|---|---|---|---:|
| baseline | upstream `07e263d` | thin LTO, 16 codegen units | 24,877,632 |
| code only | this branch | thin LTO, 16 codegen units | 24,918,416 |
| this branch | this branch | fat LTO, 1 unit, stripped | 17,721,120 |

`perf/bench.sh` alternates the two binaries run by run, five runs each; the tables give the
median with the observed range. Raw output: `bench.csv`, `bench-code-only.csv` and
`bench-profile.csv`.

## Upstream against this branch (`bench.csv`)

| Design | Before | After | Ratio | Peak RSS |
|---|---:|---:|---:|---:|
| holes_100 | 16.78 s (16.78-16.84) | 4.09 s (4.07-4.12) | 4.10x | 120.8 -> 113.7 MB |
| medium | 0.83 s (0.81-0.85) | 0.36 s (0.34-0.36) | 2.31x | 20.9 -> 17.9 MB |
| Vent_Plate | 0.16 s (0.16-0.17) | 0.05 s (0.05-0.06) | 3.20x | 18.5 -> 14.7 MB |
| Bolted_Flange | 2.02 s (2.01-2.03) | 1.89 s (1.86-1.90) | 1.07x | 153.3 -> 151.7 MB |
| pattern_196 | 6.36 s (6.21-6.39) | 6.10 s (5.97-6.28) | 1.04x | 61.9 -> 60.4 MB |
| many_sketches_60 | 7.98 s (7.92-8.04) | 7.80 s (7.76-7.93) | 1.02x | 193.0 -> 187.6 MB |

## The kernel and document changes alone (`bench-code-only.csv`)

Both binaries with the profile upstream ships, so this is the code change by itself.

| Design | Before | After | Ratio | Peak RSS |
|---|---:|---:|---:|---:|
| holes_100 | 18.06 s (17.87-18.40) | 4.30 s (4.20-4.57) | 4.20x | 119.2 -> 117.9 MB |
| medium | 0.90 s (0.89-0.94) | 0.38 s (0.36-0.41) | 2.37x | 21.2 -> 22.3 MB |
| pattern_196 | 6.50 s (6.35-6.94) | 6.33 s (6.23-6.55) | 1.03x | 61.4 -> 60.1 MB |
| many_sketches_60 | 8.45 s (8.41-8.65) | 8.35 s (8.06-8.55) | 1.01x | 193.1 -> 193.5 MB |

The vent plate pattern is the case the prism fast path targets: 2.4x with nothing else changed.

## The release profile alone (`bench-profile.csv`)

One source, two profiles, so this is the profile by itself. Run time is level (the ranges
overlap and the medians move by 1%); the profile buys binary size and 2-5 MB less peak RSS.

| Design | Before | After | Ratio | Peak RSS |
|---|---:|---:|---:|---:|
| holes_100 | 4.06 s (3.98-5.72) | 4.12 s (4.10-6.79) | 0.99x | 118.2 -> 114.4 MB |
| medium | 0.36 s (0.35-0.38) | 0.36 s (0.35-0.38) | 1.00x | 21.9 -> 17.3 MB |
| pattern_196 | 6.14 s (6.10-6.16) | 6.00 s (5.96-6.20) | 1.02x | 63.5 -> 60.1 MB |
| many_sketches_60 | 7.86 s (7.79-8.00) | 7.79 s (7.76-7.95) | 1.01x | 192.5 -> 188.8 MB |

## Equivalence (`equivalence.txt`, `perf/corpus.sh` + `perf/cmp_all.py`)

Every design under `perf/designs` was evaluated with both binaries, per-feature milliseconds
ignored. **No structural differences** in any of the 26 outputs: same bodies with the same
names, face/edge/vertex/shell counts, bounds, volumes, areas, centroids, masses and per-feature
errors. 24 are byte-identical after normalisation; the 100-hole and 400-hole plates differ in
the last bits, worst 5.1e-9 relative on the 400-hole plate's area (1.8e-3 mm2 of 352,225 mm2)
and 6.7e-9 mm on its centre of mass, and their bounding-box minimum reads exactly 0 where the
baseline read -3.5e-15.

## Identity (`identity.txt`, `perf/identity.sh`)

Six STEP fixtures (256 to 2500 solids, 963 KB to 31 MB) and the 20 designs, evaluated with
both binaries and compared with `perf/same_model.py` at 1e-8 relative / 1e-9 absolute - the
noise floor measured above. Every model matches, and all six STEP files round-trip through
import and export to byte-identical files.

## Large STEP imports (`perf/reports/step-import.md`)

The reader used to deep-clone every converted B-rep, clone every solid again to validate it,
and mesh every body over 100 mm a second time for a count it had already cached. On the
generated fixtures that is 2-11% of the import and up to 8% of its peak RSS: the 2500-solid
31 MB file went from 4.85 s / 460 MB to 4.75 s / 422 MB, and the missing-face pass over 961
boxes of 200 mm from 90.8 ms to nothing.
