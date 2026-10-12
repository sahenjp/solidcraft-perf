# Large-CAD load path: profile and bounded fixes
> Kept as written when the measurement ran: the paths in it point at the scratch
> directory those runs used (`/tmp/scperf/...`). In this checkout the same assets
> live in `perf/designs`, `perf/fixtures` (generated), `perf/probe` and here.


Machine: Fedora 44, 16 cores, 32 GB RAM, rustc 1.98.1. Release profile as committed
(opt-level 3, thin LTO, codegen-units 16, debug 0). All timing with `/usr/bin/time -v`,
3 runs per binary, interleaved before/after, `/tmp/scperf/ab-all.sh`.

## Fixtures (reproduce)

Generator: `/tmp/scperf/gen_fixtures.py`; scripts in `/tmp/scperf/designs/`; exports in
`/tmp/scperf/fixtures/`. Each export is one CLI run:

```
solvecraft-cli run /tmp/scperf/designs/grid_31x31_boxes.json   --out grid_961.step        --quiet   # 961 solids, 3.7 MB, 5.8k faces
solvecraft-cli run /tmp/scperf/designs/grid_16x16_boxes.json   --out grid_16x16_boxes.step --quiet  # 256 solids, 963 kB
solvecraft-cli run /tmp/scperf/designs/grid_multi_2500.json    --out grid_multi_2500.step  --quiet  # 2500 solids, 15k faces, 9.5 MB
solvecraft-cli run /tmp/scperf/designs/grid_multi_2500_fillet.json --out grid_fillet_2500.step --quiet # 2500 solids, 22.5k faces, 483k entities, 31 MB
solvecraft-cli run /tmp/scperf/designs/grid_31x31_large.json   --out grid_961_large.step  --quiet  # 961 solids of 200x200x50 mm (span > 100 mm)
solvecraft-cli run /tmp/scperf/designs/many_bodies_500.json    --out many_bodies_500.step --quiet  # 500 solids, 1.9 MB
```

`grid_fillet_2500` is the closest stand-in for the private 77 MB board file (2689 solids,
45,689 faces): 2500 solids, 22,500 file faces, 483,358 STEP entities, 31 MB. It loads in
~2.9 s where that board file did not finish in 300 s, so the private file must carry geometry
(free-form surfaces, big solids) this synthetic set does not.

## Profile: where the time goes

Tooling is outside the repo: `/tmp/scperf/probe` (counting allocator + phase timers),
`/tmp/scperf/phase.py` / `phase2.py` (gdb finish-breakpoint timers), `/tmp/scperf/sample_run.sh`
+ `/tmp/scperf/agg2.py` (gdb stack sampling), `/tmp/scperf/bin/cli-unstripped` (committed-profile
build saved before the release profile was experimented with, so symbols survive).

gdb timers on `grid_fillet_2500.step --parse-only` (import path only, gdb-inflated):

| stage | time | share |
|---|---:|---:|
| `p21::parse` (text) | 0.431 s | 13% |
| `step_import` total (parse + brep + Body::new) | 3.300 s | 100% |

So 87% is geometry construction, and inside it the per-solid orientation mesh dominates:
re-tessellating every imported body at `Body::new`'s tolerance (`/tmp/scperf/probe`,
`step_tess`) takes **1757 ms of the 3255 ms** import (54%) and is thrown away. gdb stack
sampling agrees: ~45% of main-thread samples sit in `rayon` join inside
`truck_meshalgo::shell_tessellation` called from `kernel::body::mesh_shells` <- `Body::new`.

Memory: `grid_fillet_2500` peaks at 429 MB RSS for a 31 MB file; the p21 `Exchange` (483k
entities, `Vec<Param>` per record) and the 16 rayon arenas dominate. `Body::new`'s tessellation
is a kernel-wide design point (docs/perf.md "Remaining problems"), out of this task's scope.

## Findings 1-2: landed fixes (behaviour-identical)

`crates/kernel/src/step_in/mod.rs` - `Reader::item_solid` used to deep-clone every converted
brep into a cache that is never evicted, and the clone stayed alive for the whole import even
though a flat file only asks for each item once. Items the file cannot ask for twice (no
assembly occurrence, one representation, that representation reached from one product
definition) are now handed over as the single reference (`Arc`, `try_unwrap`); anything that
can repeat a representation keeps every conversion exactly as before and only takes the copy
where an instance needs its own geometry.

`crates/kernel/src/step_in/brep.rs` - `read_item` built the solid with
`Solid::try_new(shells.clone())` only to fall back to `Solid::new_unchecked(shells)`. truck's
`try_new` checks four shell properties and never modifies the shells, so the same four checks
now run on the borrowed shells and the whole solid is no longer deep-cloned (every face, edge,
vertex, curve and surface) per imported solid.

Verified against the dependency source (`~/.cargo/registry/.../truck-topology-0.6.0/src/solid.rs`).

## Measured before/after (isolated-interleaved; the only other load was another agent's
cargo build in its link phase, marked contended where it shows)

```
/tmp/scperf/ab-all.sh /tmp/scperf/bin/cli-before /tmp/scperf/bin/cli-after \
  "/tmp/scperf/fixtures/grid_fillet_2500.step|--parse-only"
```

| `grid_fillet_2500.step --parse-only` | wall s | parse ms | peak RSS MB |
|---|---:|---:|---:|
| before (median of 3) | 2.970 | 2927 | 428.5 |
| after (median of 3) | 2.910 | 2866 | 423.2 |
| change | -2.0% | -2.1% | -1.2% |

Same pair on `grid_961.step --parse-only` (contended run): 0.490 -> 0.480 s wall,
65.8 -> 64.3 MB RSS. A contended run of `grid_fillet_2500` earlier showed 470 -> 433 MB peak
RSS (-8%), so the RSS delta moves with allocator/arena state; -1 to -8% is the honest range.

Where the win is small it is because the removed copy is ~20 us per solid: the dominant cost
(the orientation mesh) is untouched by design.

## Finding 3: NOT landed (reported for the Lead)

`Body::new` caches its missing-face count at `(span*0.01).clamp(1e-3, 1.0)`, but
`Body::unmeshed_faces()` re-derives `(span*0.01).max(1e-3)` with no upper clamp, so for any
solid whose bounding-box diagonal exceeds 100 mm the cache misses and the import meshes the
same solid a second time. Measured (`/tmp/scperf/probe/src/bin/step_unmeshed.rs`, calling
`unmeshed_faces()` on every imported body): `grid_961.step` 0.0 ms, `grid_961_large.step`
(200 mm boxes) **188.9 ms** - about 0.20 ms per body, a second full tessellation per large
solid. Unifying the clamp (one line in `crates/kernel/src/body.rs`, the Lead's file) removes
it for every caller; it changes the warning count for >100 mm solids, so it is the Lead's call,
not a silent perf change. A step_in-side shortcut using `body.coarse_missing` was written,
measured and removed for that reason; the call site carries a comment pointing here.

## Verification

Identity proof (`/tmp/scperf/identity.sh`, eval JSON with timing fields normalised, and STEP
in/out round trips compared byte for byte with identical output file names):

- `eval` byte-identical before/after: grid_16x16_boxes.step (256 bodies), grid_961.step (961),
  grid_multi_2500.step (2500), grid_fillet_2500.step (2500), many_bodies_500.step (500),
  holes_100.json, many_bodies_120.json, many_sketches_30.solvecraft - same bodies, names,
  face/edge/vertex/shell counts, bounds, volumes, areas and timeline warnings.
- STEP round trip byte-identical: grid_16x16_boxes.step (963,663 B), grid_961.step (3,689,603 B),
  grid_961_large.step (3,704,765 B).
- Regression test `step_in::tests::repeated_items_are_shared_and_one_use_items_are_not` pins the
  new classification (a flat file converts each item once; an assembly keeps every conversion
  and each instance still gets its own placed geometry). The brep change is covered by the
  existing `unsupported_face_is_left_out_and_reported` (open shell -> `closed == false`) and
  `hand_written_part_in_inches_with_colour` (closed body).
- `cargo test -p solvecraft-kernel --profile ci`: 26 step_in tests passed, 0 failed.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p solvecraft-kernel --all-targets --profile ci` with `RUSTFLAGS="-D warnings"`:
  clean (exit 0, no warnings).
- `cargo test -p solvecraft-kernel --profile ci`: 209 passed, 0 failed (24 ignored).
  No file outside `crates/kernel/src/step_in/` was modified.

## What did not work / was not pursued

- Avoiding `Body::new`'s orientation mesh for imported solids: out of scope by the Lead's
  decision and the biggest single item (54% of import time).
- Parallelising the reader over items: `Reader` is a `&mut self` state machine and body order,
  warnings and the `visits` cap must stay deterministic; not attempted.
- `p21` allocation shape (a `Vec<Param>` + `HashMap` entry per entity, two `String`s per
  record keyword): 13% of the import, needs an arena-style rewrite, not a bounded change.
- Caching the exported `Arc<Vec<mt::Edge>>` in `brep::Builder` (same double-clone shape as
  `item_solid`, but only ~120k small clones on the big fixture): not measured as worth it.
