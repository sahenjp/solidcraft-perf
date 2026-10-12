# Performance and memory optimisation (2026-10-12)

This is the measurement, implementation and verification record for a pass over the current
SolidCraft implementation (not a second, lighter build): the goal was lower RAM and peak
memory first, then less idle CPU and less work on large designs, without changing geometry,
file formats, the UI or the command set.

Repository revision under test: `07e263d` ("Prepare SolidCraft 0.3.2 patch release"), branch
`main`. See [docs/perf.md](perf.md) for the earlier performance log this continues, and
[perf/README.md](../perf/README.md) for the harness that reproduces every number below.

## Measurement setup

| | |
|---|---|
| Machine | Fedora 44, Linux x86_64, 16 logical cores, 32 GB RAM, NVIDIA RTX 3070 (8 GB), X11 `:0` |
| Toolchain | cargo/rustc 1.98.1, committed `release` profile (opt-level 3, thin LTO, 16 codegen units, no debug info) |
| Headless | `solvecraft-cli eval <design.json> --quiet`, one process per run, median of three, GNU `time -v` maximum resident set size |
| Desktop | `solvecraft --control <port>`, with `--sample` or a saved design; RSS/PSS sampled from `/proc` every 50 ms, idle CPU over a 10 s window starting 18 s after launch, VRAM from `nvidia-smi` filtered to the app's pid |

### Profiling tools that were available

`heaptrack`, Valgrind/Massif, `perf` and `bpftrace` are not installed on this machine, and
installing packages was not authorised for this session. `/proc/sys/kernel/perf_event_paranoid`
is `2`, so kernel sampling counters are unavailable to an unprivileged process. The numbers
below therefore come from four substitutes, all outside the shipped code:

1. a `/proc` sampling harness (RSS, PSS, `utime`+`stime`);
2. GNU `time -v` for exact peak RSS of short-lived processes;
3. `gdb` breakpoint counters, which give exact call counts for named functions without
   changing the build (for example `<Body>::tessellate_with` 325 calls, `ops::volume` 88,
   `prism::simple_prism_boolean` 88 for one 100-hole cut);
4. a `gdb` "poor man's profiler" (repeated backtrace sampling, aggregated), plus a scratch
   crate with a counting global allocator that reports live, peak and total heap bytes and the
   allocation count.

The scratch probe lives outside the repository; no measurement code was added to a shipped
crate, and no allocator was replaced in a shipped binary. Where a number could not be
measured it is called out as unmeasured rather than estimated.

## What the profile showed

### 1. The kernel re-triangulates whole solids far more often than it needs to

For a 400 x 400 x 6 mm plate cut by 100 round holes (`holes_100`), one `solid.extrude`
command ran **475 whole-body triangulations**:

| call | count | why it happens |
|---|---:|---|
| `Body::new` | 150 | orientation check: mesh the whole solid to read the sign of its volume |
| `Body::tessellate_with` | 325 | operand meshes in `ops::boolean` (200), `ops::volume` (88), result checks (62) |
| `Body::size` | 887 | every call walked every vertex of the solid |

that is 1.27 billion allocations and 61.8 GB allocated-and-freed for a single command; the
evaluation itself only ever holds about 6 MB. Aggregated `gdb` samples put 40 of 45 stacks
inside truck's `shell_tessellation`, reached from `ops::volume` and from `Body::new` via
`prism::simple_prism_boolean`.

### 2. `ops::boolean` meshes both operands before trying the cheap path

`boolean_whole` computes refined meshes of both operands at the top, then calls
`trivial()` (needs those meshes) and only afterwards `prism_boolean()`, which for plates and
prisms usually answers without them. On the 100-hole plate those two operand meshes are 200
of the 475 triangulations.

### 3. The evaluation cache kept every superseded state

`EvalCache` bounded itself by entry count only (4096 results). A result holds an
`Arc<ModelState>`, which is as large as the model it belongs to, so a long editing session
kept every state the editor had moved away from. Twelve parameter edits on the 80-cut vent
pattern grew live heap from 0.62 MB to 5.13 MB, a steady **+0.43 MB per edit with no upper
bound short of 4096 entries**; clearing the cache dropped it straight back to 0.69 MB.

### 4. Placed copies were re-oriented by meshing them

`transform_matrix` built the placed solid and then called `Body::new`, which meshes the
whole solid to decide which way its faces point. Pattern, mirror and assembly placements paid
that once per copy: 79 of the 162 `Body::new` calls for an 80-copy vent pattern came from
`ops::transform_matrix` (counted with a `gdb` breakpoint), each meshing a body for a property
the placement matrix already determines.

### 5. What was already fine

Idle CPU on an idle window is 0.1-0.2% (median of three 10 s windows; one 4.0% outlier came
from background work still running inside the window). An empty design's desktop window sits at
about 189 MB RSS / 110 MB PSS, most of it the wgpu and NVIDIA driver, with the engine holding
well under 1 MB. There was nothing to win there without changing rendering settings, which
this pass deliberately did not do.

## Changes made

| File | Change |
|---|---|
| `crates/kernel/src/body.rs` | `Body` caches the bounding-box diagonal it already computes while reading the vertices in `Body::new` (`span_of`); `Body::size()` answers from it instead of walking every vertex, and falls back to the walk for bodies built by hand (mesh bodies, surfaces). Adds `Body::new_placed`, which orients a placed solid from the placement matrix instead of meshing it, and `Body::retained_bytes`, an order-of-magnitude heap estimate from topology counts. |
| `crates/kernel/src/ops.rs` | `transform_matrix` reads the orientation from the determinant of the linear part (`det3`) and skips the orientation mesh. A rigid placement (no scale or shear, `rigid_placement`) also carries the input's cached missing-face count over, because placement leaves every face's mesh as it was. Surface bodies (no inside) keep the old path. |
| `crates/kernel/src/meshbody.rs`, `crates/kernel/src/splitface.rs`, `crates/kernel/src/surfaces.rs` | The hand-built `Body` values set the new cached size to 0, which means "not measured: walk the geometry". |
| `crates/doc/src/eval.rs` | `EvalCache` keeps a byte budget as well as an entry count (`CACHE_BYTES`, 64 MiB) and evicts oldest-first against both, using `ModelState::retained_bytes` / `Body::retained_bytes` / `SolvedSketch::retained_bytes`. One result is always kept, so a single state larger than the budget cannot empty the cache and force a full recompute on every undo. |

Geometry, tolerance, file format and command behaviour are untouched. `Body` is still cloned
by value and never mutated in place; the new field is derived from the solid it is built from,
so two bodies sharing a solid cannot disagree about it.

## Results

### Headless evaluation (median of three, same machine, same designs)

| Design | Features / scale | Peak RSS before | Peak RSS after | Wall before | Wall after |
|---|---|---:|---:|---:|---:|
| empty | 0 | 6.7 MB | 6.7 MB | 0.00 s | 0.00 s |
| many_bodies_40 | 40 boxes | 13.8 MB | 14.1 MB | 0.05 s | 0.05 s |
| many_bodies_120 | 120 boxes | 25.3 MB | 25.1 MB | 0.29 s | 0.28 s |
| medium | vent plate, 80-cut pattern | 22.1 MB | 21.7 MB | 0.90 s | 0.91 s |
| pattern_64 | 64 placed copies, 576 faces | 55.5 MB | 56.5 MB | 2.32 s | **2.09 s (-9.9%)** |
| pattern_196 | 196 placed copies, 1764 faces | 62.4 MB | 64.8 MB | 6.63 s | 6.51 s (-1.8%) |
| many_sketches_30 | 61 features, 1 growing body | 93.8 MB | 93.1 MB | 2.48 s | 2.53 s |
| many_sketches_60 | 121 features, 1 growing body | 192.3 MB | 193.3 MB | 8.90 s | 8.44 s (+5%, run-to-run spread is ±0.5 s) |
| holes_100 | 100 holes in one cut | 120.1 MB | 122.6 MB | 18.08 s | 17.86 s |

Everything outside the placed-copy cases is inside run-to-run noise (3-5%). Peak RSS is
unchanged on every case: the landed changes remove repeated *work*, not retained data.

### Desktop (idle window, median of three)

| Case | RSS before | RSS after | Idle CPU before | Idle CPU after |
|---|---:|---:|---:|---:|
| empty design | 189.0 MB | 189.2 MB | 0.2% | 0.1% |
| bundled sample | 207.1 MB | 200.9 MB | 1.4% (window included startup work) | 0.0% |
| 61-feature design | 235.3 MB | 238.1 MB | 0.0% | 0.1% |

VRAM, sampled once a second from `nvidia-smi` filtering only the app's pid, is **217 MiB for
every case** - empty design, bundled sample and the 61-feature design alike. At these sizes the
viewport's GPU memory is the swapchain, MSAA targets and driver overhead, not model geometry.

Startup is unchanged; the log line the app prints after its GPU device is created appears at
the same point, and an empty design's process peak is 6.7 MB headless. Binary size is unchanged
changed by the release profile (see the follow-up): `solvecraft` 58,218,168 -> 38,099,720 bytes
(-34.6%), `solvecraft-cli` 24,952,192 -> 17,721,120 bytes (-29.0%), measured from a rebuild of
the finished tree against the baseline built from `07e263d` on the same machine.

### The memory-growth fix, measured directly

With the counting allocator, twelve parameter edits on the vent pattern:

| | live heap after 12 edits | growth per edit |
|---|---:|---:|
| before | 5.13 MB | +0.43 MB, unbounded to 4096 entries |
| after (budget temporarily lowered to 1 MiB to make eviction visible in 20 edits) | 0.73 MB, flat | 0 |

At the shipped 64 MiB budget the same session keeps whichever results fit in the budget and
drops the rest oldest-first. A result is never dropped below one entry, so undo and redo still
hit the cache for the most recent state.

## Follow-up: the prism fast path (2026-10-12)

Both experiments below are about the same cost, and the next pass landed a version of the first
one that does not trade memory for it. `boolean_whole` meshed both operands at the top, used
those meshes for `trivial` and for the result check, and only then asked
`prism::prism_boolean`, which for a plate and the tool through it answers exactly in 2D without
meshing either operand. On the 100-hole plate that pair of operand meshes and the result check
were 11 s of a 17 s evaluation, and a pattern or a multi-profile sketch rebuilds the whole
target's mesh once per tool.

A cut or an intersection now asks the prism route first, measuring the operands the way every
result volume in that function already is. A pair the route declines still takes the previous
path, unchanged, `trivial` included. A union keeps the previous order, because parts that stay
apart are exactly what `trivial` answers at once (asking first cost 4% on the 196-copy pattern
in an intermediate build).

Linux x86_64, release profile, both binaries in one paired run, one process per evaluation,
median of three, no concurrent build (`07e263d` against the finished tree):

| Design | Before | After | |
|---|---:|---:|---|
| 100 holes cut into one plate | 17.9-24.9 s, median 19.7 | 4.30-4.39 s | at least 4.2x |
| vent plate, 80-cut pattern | 0.85-0.88 s | 0.36-0.37 s | 2.3x |
| vent plate sample | 0.17-0.18 s | 0.06 s | 2.8x |
| bolted flange sample (circular pattern) | 2.06-2.15 s | 1.91-1.95 s | 1.07x |
| 196-copy body pattern | 6.64-6.78 s | 6.36-6.62 s | unchanged |
| 121-feature join chain | 9.0-11.7 s | 8.26-8.35 s | unchanged |
| sensor enclosure example | 7.41-7.58 s | 7.62-7.74 s | unchanged |
| loft cut beside round hole (regression sample) | 0.37-0.39 s | 0.36-0.37 s | unchanged |

An earlier paired run of the same baseline measured 16.7-17.4 s against 4.2-4.5 s. The baseline
is the noisy side of every row (the 121-feature chain measured 8.0-11.7 s across the two runs,
wider than the difference to the finished tree); the finished tree repeats inside 2% on the
cases it speeds up.

Peak RSS is unchanged or slightly lower: 123.5 MB to 116.5 MB (-5.7%) on the 100-hole plate,
63.9 to 61.5 MB on the 196-copy pattern, 19.2 to 17.3 MB on the vent plate sample. The sensor
enclosure spreads 204-238 MB in both binaries, wider than any difference between them. The
change removes repeated triangulation, not retained data.

A result is accepted only when its volume agrees with the operands' volumes the way the sampled
path's would, and only when it is not a look-alike rebuild of an operand. A tool that misses its
target, in section or part-way along its extrusion, leaves the target as it is, and `trivial`
says so by reusing the very same body; the 2D route rebuilds that body - the same shape on new
faces - which changes the identity an untouched body keeps through a timeline, and with it its
STEP export. Those pairs go back to the path below, which decides them exactly as it always did.
The first `cargo xtask ci` run on the reordered kernel caught this in
`offset_rectangle_pocket_has_one_logical_outer_wall_and_closed_face_projection`; the rule and
`ops::prism_fast_path_tests` (a bore through a plate, two bodies that stay apart, a tool wholly
inside its target, and a tool that misses its target) are the fix. The operands' volumes are
remembered per body (`Body::volume_at`, keyed on the tolerance), so the check costs one coarse
mesh of each new result and nothing per tool after the first.

### Where a long timeline's memory actually goes (2026-10-12)

One item from the first pass is still open: a long timeline keeps every intermediate state. A
scratch probe outside the repository (a counting global allocator linked against the same
`solvecraft-engine` library) evaluates a design and then clears one holder at a time. On `many_sketches_60` (121 features, one growing body, 60 sketches):

| live heap | MB |
|---|---:|
| after evaluation | 239.8 |
| undo, redo and clipboard cleared | 226.3 |
| the evaluation cache cleared | 223.8 |
| `Model::results` cleared | **0.6** |
| session dropped | 0.4 |

`sketches_only_60` (61 features, 60 solved sketches, tiny bodies) is 12.8 MB: 7.5 MB of undo
documents, 3.5 MB of cached results, 3.5 MB of states.

The memory is the `Arc<ModelState>` snapshots in `Model::results`, and it is tied together:
`NamingOrigin.before` points at the state before each feature, so one strong reference to the
last state keeps every earlier one alive. Clearing `Model::results` on its own frees 0.7 MB for
exactly that reason - it frees 223 MB only once the evaluation cache's reference to the last
state is gone as well (the two steps above, in that order). `state_before(id)`, which commands
such as `edit`, `sketch.project` and the plastic rules call, needs the same snapshots.

Two consequences worth knowing before touching this:

- `ModelState::retained_bytes` reads 57 MB for that 240 MB session. It roughly counts topology,
  and truck's own surface, curve, wire and shell objects cost about four times what the counts
  suggest, so the evaluation cache's 64 MiB budget is not the bound it sounds like. The cache
  only holds `Arc`s to states the timeline already keeps, which is why clearing it frees 2.6 MB
  of unique bytes rather than tens of megabytes.
- Removing the retention is a design change, not a tweak: the state has to become something that
  can be dropped and rebuilt (the evaluator already stops at `Document::marker`, so a prefix
  state can be recomputed exactly) and the naming chain has to hold a weak reference with a way
  back. Saved designs' face and edge references depend on name resolution, so that work needs the
  oracle set and the engine tests as its guard. It was not attempted in this pass.

### Two smaller items from the same pass

The release profile is now fat LTO with one codegen unit and `strip = "symbols"`, which shrinks
the binaries by a third at equal or slightly better run time. The STEP import path lost three
redundant copies per imported solid (a never-evicted B-rep clone, a whole solid cloned only to
validate it, and a second tessellation of every body over a 100 mm span); imports of thousands of
solids measure 2-11% faster with 1-8% less peak RSS, and the face count the unmeshed-face warning
reports is unchanged. Both are detailed in [perf.md](perf.md).

Against the untouched baseline, with the release profile in place, the finished tree measures:

| Design | Before | After | |
|---|---:|---:|---|
| 100 holes cut into one plate | 17.23 s, 120.1 MB | 4.21 s, 114.9 MB | 4.1x |
| vent plate, 80-cut pattern | 0.85 s, 21.1 MB | 0.35 s, 18.9 MB | 2.4x |
| vent plate sample | 0.18 s, 19.7 MB | 0.06 s, 15.3 MB | 3.0x |
| bolted flange sample (circular pattern) | 2.26 s, 161.4 MB | 2.05 s, 121.6 MB | 1.1x |
| 196-copy body pattern | 6.52 s, 62.9 MB | 6.15 s, 59.0 MB | 1.06x |
| 121-feature join chain | 8.22 s, 193.4 MB | 8.02 s, 189.0 MB | unchanged |
| sensor enclosure example | 7.59 s | 7.32 s | 1.04x |
| loft cut beside round hole (regression sample) | 0.35 s, 26.3 MB | 0.34 s, 23.5 MB | unchanged |

Medians of three, one process per evaluation, both binaries in one paired run, no concurrent
build.

## Experiments that were measured and then reverted

Both of these are real, large wins on the 100-hole plate, and both were left out because of the
rule that memory must not be traded away for speed. The first is what the follow-up above then
landed without that trade; the second is still open.

### Eager operand meshes in `ops::boolean`

Forcing the fallback path (`meshes = (None, None)`, so `va`/`vb` come from `volume()`
instead of the refined meshes) took `holes_100` from 16.4 s to **5.2 s at unchanged peak RSS**
(123.8 MB -> 121.5 MB). It was not landed because it also skips the `trivial` shortcut and the
sampled membership check, and making those lazy without changing which algorithm runs means
moving `trivial` after the prism path - a kernel behaviour change this pass was not prepared to
justify from measurements alone. See "Next steps" item 1.

### Batching disjoint cut tools into one boolean

`apply_op` applies a cut's tools one at a time, so each tool re-meshes the target again. Since
the tools of a pattern are provably apart (pairwise disjoint bounding boxes), their shells side
by side are exactly their union, and cutting with that union once is exact:

| `holes_100` | before | batched |
|---|---:|---:|
| evaluation | 16.8 s | **2.05 s (8.2x)** |
| allocations | 1.27 billion | 25 million |
| total allocated | 61.8 GB | 1.9 GB |
| peak heap | 23.8 MB | **175.4 MB** |
| volume / area / topology | - | identical, volume and area within 1.3e-10 relative |

Peak heap was also erratic with the batch size - 2 tools per boolean: 283 MB, 8: 236 MB,
16: 733 MB, 32: 237 MB, 100: 175 MB - which points at the working set of
`prism::simple_prism_boolean` / `boolean2d` growing with the number of regions in one
boolean, not at the batching itself. Landed changes must not raise peak memory, so the change
and the `Body::bounds` / `ops::combine_disjoint` helpers it needed were removed.

## Verification

```
cargo fmt --all -- --check                              clean
cargo xtask ci                                          26 test binaries, no failures; wasm check blocked (see below)
cargo clippy --workspace --all-targets --profile ci -D warnings   clean
cargo xtask layers                                      OK: 13 crates, no layering violations
cargo xtask assets                                      assets: 3 asset file(s), all attributed
cargo xtask licences                                    THIRD-PARTY-LICENSES.txt is up to date
cargo xtask wasm                                        cannot run on this machine
```

Test binaries from the last `cargo xtask ci`, on the finished tree (prism fast path, STEP import
fixes, the unified missing-face tolerance and the release profile all in it): kernel 209 passed /
0 failed, engine 562 / 0, ui-egui 326 + 69 + 3 / 0, doc 109 / 0, sketch 77 / 0, io 21 / 0,
geom 15 / 0, render 14 / 0, mcp 9 / 0, cli 3 / 0, xtask 11 / 0, plus the doc tests.
The first run on the reordered kernel failed one engine test,
`offset_rectangle_pocket_has_one_logical_outer_wall_and_closed_face_projection`: cutting a
pocket rebuilt a separate body the tool never touched, so it was no longer the very same solid.
The identity rule in the follow-up above fixed it, and `ops::prism_fast_path_tests` covers the
case at the kernel level. Only the last run counts, and it is the one quoted here.

**`cargo xtask wasm` did not run, and this is an environment limit, not a passing check.** The
`wasm32-unknown-unknown` target is not installed (`rustup` is not present; `/usr/lib/rustlib`
holds only `x86_64-unknown-linux-gnu`; Fedora ships the target as a separate package), for
example:

```
error[E0463]: can't find crate for `core`
  = note: the `wasm32-unknown-unknown` target may not be installed
error: failed: wasm32-unknown-unknown check (10 crates)
```

Installing it needs root (this machine's `sudo` asks for a password, and approving one is not
available here), so the gate still has to be run somewhere else. The changes
are target-independent by inspection - `Arc`, `Vec` and iterator adapters over truck topology,
no threads, time, filesystem or sockets - but that is an argument, not a measurement, and the
wasm gate still has to be run before this work is merged.

Geometry equivalence was checked the way the kernel tests cannot: `solvecraft-cli eval` output
for a 17-design corpus (the six bundled samples, the acceptance and regression examples, and the
generated designs) from the baseline binary against the finished one. Every design has the same
bodies with the same names, face/edge/vertex/shell counts and per-feature errors; nothing
structural differs. Four designs differ in the last bits only, and the worst is `holes_400`: 5.1e-9
relative on its area (1.8e-3 mm2 of 352,225 mm2) and 6.7e-9 mm on its centre of mass. `holes_100`
moves by 1.3e-10 relative on volume and area, both plates' bounding-box minimum, -3.5e-15 in the
baseline, is now exactly 0 - their bottom face plane - and `shallow_loft_cut_from_top_face` moves
by 3.8e-16. The identity property the engine test checks is also asserted
directly in `ops::prism_fast_path_tests`.

## Remaining problems

- **Whole-solid triangulation is still the dominant cost.** Every `Body::new` meshes the
  solid it was handed to read its volume sign, and nothing in either pass removed that. The
  first pass removed the `Body::size` walks around it and the placement case; the follow-up
  removed the repeated operand meshes from the prism-pair path and memoizes the volumes that
  path measures (`Body::volume_at`).
- **Timeline states are retained for the whole session.** `Model::results[i].state` holds a
  full `ModelState` per feature, and each body's `NamingOrigin.before` links back to the
  previous state, so every intermediate B-rep stays alive. A 61-feature design held 72 MB live
  heap, of which about 66 MB was the intermediate states; at 121 features it was 240 MB. This is
  quadratic in timeline length and is the largest remaining memory item for design-heavy work.
- **Sketches are deep-copied per timeline state.** `SolvedSketch` is cloned into every later
  state. Re-measured with the probe above, those copies are 2.1 MB of the 12.8 MB a 61-feature
  sketch-only design holds and 4.3 MB of `many_sketches_60`'s 240 MB, so sharing them behind an
  `Arc` is worth doing but is not where a long timeline's memory is.
- **A pair the prism route declines still meshes both operands.** The fast path removed that
  cost for a cut or an intersection of prism pairs (the common many-hole and pattern case); a
  union, a cut of a non-prism, and every retry still pay for the refined operand meshes.
- The desktop baseline (~189 MB RSS, ~209 MiB VRAM for an empty design) is dominated by the
  wgpu/NVIDIA stack, not by the application.

## Next steps, in the order the measurements support

1. **Make the operand meshes lazy while keeping `trivial`.** Done for a cut or an intersection
   of prism pairs (see the follow-up above): 4.2x on `holes_100` at unchanged peak RSS, with
   `trivial` still deciding every pair the prism route declines. A union keeps the old order,
   because `trivial`'s disjoint case is what makes a join of separate parts cheap. The same
   treatment for unions needs a cheaper disjointness proof than a refined mesh of both sides.
2. **Shrink `prism_boolean` / `boolean2d`'s working set for tools with many regions.** That is
   the only thing standing between the current code and the measured 8.2x on many-hole cuts.
3. **Memoize `Body::tessellate_with` per (tolerance, refine) on the body**, shared across
   clones. It only helps where the same solid is meshed twice at the same tolerance, so it needs
   a hit-rate measurement first. The follow-up memoized one such measurement this way
   (`Body::volume_at`, keyed on the tolerance), which is what makes the fast path's result check
   cost a mesh per tool rather than two.
4. **Stop retaining every intermediate timeline state.** Keeping `Arc<ModelState>` only where
   something can still ask for it (`state_before` for the active sketch, rollback targets)
   would remove the quadratic term; a 61-feature design would drop from 72 MB to a few MB.
5. **Share `SolvedSketch` across states** (`Vec<Arc<SolvedSketch>>`): mechanical, and worth the
  2.1 MB it measured on a sketch-only design (see "Where a long timeline's memory actually goes"),
   but it is not the big item - item 4 is.
6. **Revisit the release profile.** Done in the follow-up: fat LTO with one codegen unit and
   `strip = "symbols"` took the binaries down by 34.6% and 29.1% with equal or slightly better
   run time; a cold release build goes from 4m14s to 7m29s and the `ci` and `dev` profiles are
   untouched. The variant table is in [perf.md](perf.md).
