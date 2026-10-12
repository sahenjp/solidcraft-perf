# Design loading

## Cold geometry rebuilds after 0.3.1

Three alternating cold-process comparisons on the same Windows machine and optimized
`ci` profile, with no concurrent build, compare the unchanged 0.3.1 source with the
straight-line projection and exact surface-refinement reuse changes:

| Saved design | Before, median evaluation | After | Reduction |
|---|---:|---:|---:|
| Public sensor enclosure, 21 features, 52 KB | 11.96 s | 8.85 s | 26.0% |
| Local Pi case assembly, 25 features, 2,990 bodies | 14.75 s | 11.57 s | 21.6% |

These are geometry rebuild times, not complete desktop opening times. The public fixture
can be generated with `solvecraft-cli run examples/acceptance/sensor_enclosure.json
--save sensor.solvecraft --quiet`, then measured with `solvecraft-cli bench-open
sensor.solvecraft`. Each benchmark starts a new process; this does not measure a warm
timeline-cache hit. The local assembly is private and is not part of the repository.

Straight segments now use guarded algebraic closest-point projection instead of sampling
65 points and running Newton iterations. Other curve types retain their existing behavior.
Curved-face refinement reuses derivative and midpoint evaluations with exact finite input
keys and bounded, local caches. It keeps the original tolerance, seam and singularity
checks, refinement passes, and triangle limits.

Identity assembly placements retain the definition body's shared display-mesh cache.
Changed placements continue to build separate meshes in their own coordinates. Face-name
preparation remains eager during placement: deferring it merely shifted expensive work to
thumbnails and the first face query, so that approach was excluded.

Both saved fixtures retain exactly the same body names, face/edge/vertex/shell counts,
bounds, volumes, and surface areas. Focused tests compare cached and uncached refinement
coordinates and normals bit for bit on periodic, singular and transformed surfaces.

For desktop phase profiling, set `SOLVECRAFT_PROFILE_OPEN=1` before launching the app.
Interactive Open reports geometry, placement, tessellation, thumbnail and total worker
times to stderr without logging the source path. Thumbnails were about 0.1 seconds of
the slow assembly's open and were not the main bottleneck. Further cold Boolean and
import costs remain; these changes do not make all documents open instantly.

### Prepared assembly geometry and selection

Prepared world geometry is now keyed by the document, evaluated model and exact placement
context instead of the session revision. Selecting a face or edge, changing visibility,
or moving the view no longer discards already prepared assembly bodies and meshes.
Geometry edits, pending occurrence moves, authoring context changes, undo/redo and timeline
rollback still invalidate the cache. Scratch sessions share prepared geometry while their
inputs match, and adopting a computed session retains its warm cache.

On the private 2,990-body Pi assembly, a matched Linux desktop control probe reduced an
edge-selection round trip from 9.24 s to 0.57 s. Initial opening stayed essentially unchanged
(33.54 s to 33.32 s); this fixes a separate selection stall, not cold geometry evaluation.
Three fresh Windows desktop runs with the corrected cache measured a median 15.87 s to
ready: 13.04 s geometry, 2.20 s placement, 0.53 s meshing and 0.07 s thumbnails. These used
the optimized `ci` profile and actual Vulkan rendering, with isolated settings and caches.
Linux and Windows timings are separate measurements and must not be compared as a speedup.

`solvecraft-cli bench-open design.solvecraft` reports JSON lines for file reading,
JSON decoding and migrations (including input-name repair), timeline evaluation, individual
features, and world placement/display meshing. `--parse-only` stops before geometry evaluation.
The parse timer includes document validation and repair, not just the JSON parser.

Windows x64, optimized `ci` profile, measured during other builds:

| Design | Parse/repair | Evaluate | Display meshes |
|---|---:|---:|---:|
| 19-feature enclosure, 34 KB | 1.2 ms | 14.9 s | 6 ms |
| 25-feature assembly, 15.2 MB, 2,990 bodies | 21 ms | 12.5 s | 2.1 s |
| Generated 1,000 independent boxes, 185 KB | 5 ms | 0.82 s | 0.15 s |
| Generated 5,000 boxes in five grids, 10 features | 0.45 ms | 17.6 s | 0.81 s |

The enclosures spend most of evaluation in joins and cuts, not parsing. Designs save feature
definitions; opening them rebuilds their geometry. Those times remain after the repair fix.

A separate generated timeline with missing input names reproduced minutes in the reader:

| Timeline (geometry rolled back) | Parse/repair before | After |
|---|---:|---:|
| 2,000 boxes, 6,000 unnamed inputs | 78.1 s | 10–20 ms |
| 5,000 boxes, 15,000 unnamed inputs | interrupted after 131 s, still repairing | 26–46 ms |

Previously, each missing input restarted the `d1`, `d2`, … search, scanning the document for
every candidate. This approaches cubic work as unnamed inputs accumulate. Loading now reserves
names once and assigns missing names with an advancing cursor. Existing names, names on later
features, parameter collisions and reuse of surplus names retain the previous behavior.
Regression tests compare the old and bulk repairs and load a 5,000-feature unnamed timeline.

# Viewport performance

Measured with the control channel (`ui.inspect` `frame_ms`: CPU time of one UI frame, which
includes picking under the cursor) on the sample plate patterned 20 × 20 (400 bodies, about
600k triangles), 1600 × 1000 window under Xvfb, release build. Hover: 60 cursor moves across
the model; orbit: a right-drag.

| | hover median | hover p90 | orbit median |
|---|---|---|---|
| before | 18.7 ms | 37.5 ms | 21.0 ms |
| after | 1.9 ms | 2.4 ms | 2.6 ms |
| after, Browser hidden | 0.4 ms | — | 0.4 ms |

What changed:
- **Picking culls by bounding box.** Per-body boxes are cached until the model changes. The
  face raycast skips bodies whose box the ray misses or enters behind the nearest hit so far.
  Edge and vertex tests skip bodies whose box, projected on screen, is not under the cursor.
- **Picks are reused** while the cursor, camera and model are unchanged, and **no picking runs
  while the view moves** (orbit, pan, zoom).
- **The model extent is cached** (it sets the view's depth range and was recomputed from every
  mesh several times per frame).

With 49 bodies the frame was already 2.8 ms (hover) before these changes. The remaining frame
time with the Browser shown is mostly the Browser listing every body row. GPU ID-buffer picking
was not needed at this size.

## Browser

Same model and method (400 bodies, Bodies folder open, 60 hover moves over the viewport):

| | median | p90 |
|---|---|---|
| Browser shown, before | 1.95 ms | 2.71 ms |
| Browser shown, after | 0.72 ms | 1.19 ms |
| Browser hidden | 0.30 ms | 0.72 ms |

What changed: rows scrolled out of view only reserve their height. They register no widgets
and paint nothing (no text layout, icons or eye/fold buttons), so the cost follows the rows on
screen rather than the rows in the tree. Scrolling, Find in Browser and drag and drop still see
every row's place.

# Recompute performance

`solvecraft-cli bench-edit <script> <param> "(<old>) * 1.02"` builds a design from its script
(timing the build), then times a parameter edit, setting the parameter back to its old
expression, undo and redo. Each cell is before → after the evaluation cache, with the number of
features recomputed in parentheses. Release build, on a machine shared with other build jobs (load average about 45; the
times vary by ±50 % run to run, so compare the recompute counts and the zeros). "Early" edits a parameter of the
first sketch or feature, "late" one of the last feature.

| Design | Edit | Build | Edit | Set back | Undo | Redo |
|---|---|---|---|---|---|---|
| sample plate, 20 × 20 pattern | early (`width`) | 7587 ms (10) → 1764 ms (10) | 7969 ms (10) → 1691 ms (10) | 7557 ms (10) → 0 ms (0) | 6534 ms (0) → 0 ms (0) | 4570 ms (0) → 0 ms (0) |
| sample plate, 20 × 20 pattern | late (`d9`) | 7870 ms (10) → 1699 ms (10) | 3944 ms (1) → 812 ms (1) | 4039 ms (1) → 0 ms (0) | 1464 ms (0) → 0 ms (0) | 1398 ms (0) → 0 ms (0) |
| oracle 58 phone stand | early (`d1`) | 163 ms (6) → 140 ms (6) | 228 ms (5) → 140 ms (5) | 171 ms (5) → 0 ms (0) | 158 ms (0) → 0 ms (0) | 130 ms (0) → 0 ms (0) |
| oracle 58 phone stand | late (`d4`) | 144 ms (6) → 195 ms (6) | 118 ms (1) → 118 ms (1) | 145 ms (1) → 0 ms (0) | 149 ms (0) → 0 ms (0) | 187 ms (0) → 0 ms (0) |
| oracle 59 enclosure base | late (`d8`) | 11582 ms (11) → 11179 ms (11) | 5772 ms (1) → 5742 ms (1) | 5606 ms (1) → 0 ms (0) | 6023 ms (0) → 0 ms (0) | 6119 ms (0) → 0 ms (0) |
| oracle 60 pipe flange | early (`d1`) | 8137 ms (7) → 8348 ms (7) | 7569 ms (6) → 10143 ms (6) | 8354 ms (6) → 0 ms (0) | 8607 ms (0) → 0 ms (0) | 10034 ms (0) → 0 ms (0) |
| oracle 60 pipe flange | late (`d6`) | 7661 ms (7) → 11990 ms (7) | 67 ms (1) → 361 ms (1) | 69 ms (1) → 0 ms (0) | 84 ms (0) → 0 ms (0) | 66 ms (0) → 0 ms (0) |
| oracle 61 shaft keyway | early (`d1`) | 1723 ms (6) → 3016 ms (6) | 1785 ms (5) → 2745 ms (5) | 1760 ms (5) → 2323 ms (5) | 1667 ms (0) → 0 ms (0) | 1719 ms (0) → 0 ms (0) |
| oracle 61 shaft keyway | late (`d4`) | 1905 ms (6) → 2247 ms (6) | 1734 ms (1) → 2101 ms (1) | 1682 ms (1) → 0 ms (0) | 1719 ms (0) → 0 ms (0) | 1876 ms (0) → 0 ms (0) |
| oracle 62 heat sink | early (`d1`) | 382 ms (6) → 643 ms (6) | 362 ms (5) → 697 ms (5) | 233 ms (5) → 568 ms (5) | 245 ms (0) → 0 ms (0) | 360 ms (0) → 0 ms (0) |
| oracle 62 heat sink | late (`d5`) | 479 ms (6) → 688 ms (6) | 270 ms (1) → 657 ms (1) | 321 ms (1) → 0 ms (0) | 413 ms (0) → 0 ms (0) | 283 ms (0) → 0 ms (0) |
| oracle 63 hinge knuckle | early (`d1`) | 1305 ms (8) → 1671 ms (8) | 1329 ms (7) → 2397 ms (7) | 1971 ms (7) → 0 ms (0) | 1705 ms (0) → 0 ms (0) | 1321 ms (0) → 0 ms (0) |
| oracle 63 hinge knuckle | late (`d4`) | 1430 ms (8) → 2072 ms (8) | 1387 ms (1) → 2073 ms (1) | 1480 ms (1) → 0 ms (0) | 1192 ms (0) → 0 ms (0) | 1035 ms (0) → 0 ms (0) |

What changed: every feature evaluation is cached by a key made of the feature's own inputs
(definition and parameter values), the document context it may read (construction planes,
sheet metal and plastic rules, components) and a digest of the model it starts from, in which
each body is identified by the evaluation that produced it (bodies a feature passes through
unchanged keep their identity). The same feature on the same inputs is never evaluated twice:
setting a value back, undo, redo, a preview followed by the same command, and the repeated
evaluations while a script builds the design all reuse earlier results. The cache is shared by a
model and its copies (previews, scratch sessions) and keeps up to 4096 results and 64 MiB of
them, whichever comes first (see [performance-optimization.md](performance-optimization.md)).

An edit still recomputes the edited feature and every later feature whose starting model
changed; in these parts every later feature works on the edited body, so early edits recompute
almost everything, as they must. `cached_evaluation_matches_from_scratch` checks that cached and
from-scratch evaluation give the same models (bodies, volumes and areas to 1e-6, topology,
errors) through edits, suppression, roll-back, undo and redo. Setting a sketch dimension back
can still recompute (oracle 61, 62): solving the sketch moves its points by rounding noise, so
its inputs are not exactly the old ones. Oracle 59's first parameter edit fails (a recipe
expression the bench's `* 1.02` can't scale) and is left out.


## Desktop loading and recent thumbnails (2026-10-10)

The Arduino Uno enclosure (19 features, 34 KB) and Raspberry Pi 5 case (24 features,
61 KB) each decode in about 1 ms. A release build spends roughly 12.1 s and 8.2 s,
respectively, evaluating their geometry on this Windows machine. Changing the document
encoding does not eliminate these boolean operations.

Interactive Open, recent-file clicks, drag/drop and STEP/mesh insertion now execute on a
worker, including preparation of the world-space bodies and display meshes. The existing
design stays available until the result is ready; a failed or cancelled job cannot replace
it. Insertion preserves the file, saved state, earlier undo history and view settings.
Timeline copies share their lazy mesh cell, and adopting a prepared session keeps its warm
world-space cache instead of transforming and tessellating its components again on the UI.
A STEP body's missing-face check reuses its orientation tessellation when the tolerances
match and its orientation was not reversed.

Live desktop tests (`ci` profile, with other builds/tests running) opened the Arduino and
Pi case in 15.2 s and 10.5 s. During those jobs, median `ui.inspect` round trips were 20.5 ms
and 8.3 ms; the slowest were 33.7 ms and 28.6 ms. This fixes the window freeze, not the
remaining cost of cold geometry evaluation.

Recent thumbnails are 240 × 150 PNG files in `SolveCraft/thumbnails` under the platform
cache directory (`LOCALAPPDATA` on Windows). Their keys include the source path, file size
and modification time. Open and Save render from the model already in memory. Cached
recents decode the PNG without reading or rebuilding a document, and run before cold
recents and sample thumbnails. Missing or invalid pictures fall back to a background rebuild.
No thumbnail or base64 image is added to the design JSON. A regression test verifies the
source bytes remain unchanged, a cached picture works even without a parseable source
file, and changing the source invalidates the cache.

`solvecraft-cli bench-open` also accepts STEP/IGES. For exchange files its `parse` phase
includes constructing and checking the imported B-rep, not merely decoding STEP syntax.

The live test loaded both cached recent cards in 21 ms and inserted the 2 MB lightweight
Pi STEP (27 bodies) into the case in 279 ms; undo restored the original two bodies. The
full 77 MB board STEP (2689 solid definitions, 45,689 faces) still had not completed after
300 s. Its window continued handling inspection and Cancel, and cancelling preserved the
case. Complex STEP construction remains an outstanding kernel performance problem.

## Boolean fast path for prism pairs (2026-10-12)

A plate and the tool through it are decided exactly in 2D by `prism::prism_boolean`. That route
needs no mesh of either operand, while the sampled shortcuts in `boolean_whole` (`trivial`,
and `mismatch` inside its result check) need a refined mesh of both at the meeting tolerance.
For a plate with many holes that mesh is most of what a boolean costs - on the 100-hole plate
the operand meshes and the result check were 11 s of 17 s - and a pattern or a multi-profile
sketch rebuilds the whole target's mesh once per tool. The prism route now runs before them for
a cut or an intersection, measuring the operands the way every result volume in that function
already is; a pair it declines still takes the previous path, unchanged, `trivial` included. A
union keeps the previous order: parts that stay apart are exactly what `trivial` answers at
once.

Linux x86_64, release profile, one process per evaluation, both binaries in one paired run,
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

An earlier paired run of the same baseline measured 16.7-17.4 s against 4.2-4.5 s, so the
identity rule below costs nothing measurable. The baseline is the noisy side of every row (the
121-feature chain measured 8.0-11.7 s across the two runs, wider than the difference to the
finished tree); the finished tree repeats inside 2% on the cases it speeds up.

Peak RSS is unchanged or slightly lower: 123.5 MB to 116.5 MB (-5.7%) on the 100-hole plate,
63.9 to 61.5 MB on the 196-copy pattern, 19.2 to 17.3 MB on the vent plate sample. The sensor
enclosure spreads 204-238 MB in both binaries, wider than any difference between them. The
change removes repeated triangulation, not retained data.

The evaluation output is unchanged - same bodies, names, face/edge/vertex counts, bounds,
volumes, areas, centroids, masses and per-feature errors - across the six bundled samples, the
example designs and the generated corpus. Four designs differ only in the last bits, and the
worst is the 400-hole plate at 5.1e-9 relative on its area (1.8e-3 mm2 of 352,225 mm2) and 6.7e-9
mm on its centre of mass. The 100-hole plate moves by 1.3e-10 relative on volume and area, both
plates' bounding-box minimum, which the baseline reported as -3.5e-15, is now exactly 0, their
bottom face plane, and `shallow_loft_cut_from_top_face` moves by 3.8e-16.

A pair the route answers is accepted only when its volume agrees with the operand volumes the way
the sampled path's would, and only when the result is not a look-alike rebuild of an operand. A
tool that misses its target, in section or part-way along its extrusion, leaves the target as it
is, and `trivial` says so by reusing the very same body; the 2D route would rebuild that body -
the same shape on new faces - which changes the identity an untouched body keeps through a
timeline, and with it its STEP export. Those pairs go back to the path below, which decides them
exactly as it always did. The check costs one coarse mesh of each new result: the operands'
volumes are remembered per body (`Body::volume_at`), so a pattern cutting one target measures
each tool and result once.

`ops::prism_fast_path_tests` pins the configurations the reordering touches: a bore through a
plate, two bodies that stay apart, a tool wholly inside its target (a closed void, not a through
cut), and a tool that misses its target, which must leave the very same body behind.

## Large STEP imports (2026-10-12)

A STEP file is read in three stages: the p21 text is parsed, every representation item is
converted to a B-rep, and `Body::new` runs per body. On a generated 31 MB file (2,500 solids,
22,500 faces, 483,358 entities - the closest stand-in this machine can build for the private
77 MB board file that had not finished in 300 s) the text is 13% of the 3.0 s the import takes and
**54% is `Body::new`'s orientation mesh**, a kernel-wide design point this pass did not touch
(see [performance-optimization.md](performance-optimization.md)). Three redundant copies around
it were removed:

- `step_in::Reader::item_solid` deep-cloned every converted B-rep into a cache that is never
  evicted and kept the clone for the whole import, although a flat file asks for each item once.
  An item the file cannot ask for twice - no assembly occurrence, one representation, that
  representation reached from one product definition - is now handed over as its single
  reference. A file that can repeat a representation keeps every conversion exactly as before,
  and a walk budget falls back to that behaviour wholesale, so a misjudged file costs one
  reconversion and nothing else.
- `step_in::brep::read_item` built every solid with `Solid::try_new(shells.clone())` only to fall
  back to `Solid::new_unchecked(shells)` when a shell failed a check. truck's `try_new` checks
  four shell properties and returns the shells unchanged
  (`truck-topology-0.6.0/src/solid.rs`), so the same four checks now run on the borrowed shells
  and the deep clone of every face, edge, vertex, curve and surface per item is gone.
- `Body::unmeshed_faces` counted faces missing from a mesh at `span/100`, while the orientation
  mesh in `Body::new` runs at `span/100` clamped between 0.001 and 1 mm. Above a 100 mm span the
  two never matched, so the count missed the cache and every large body was meshed a second time:
  measured at 90.8 ms for 961 boxes of 200 mm. Both now use one `orientation_tolerance`.

Linux x86_64, release profile as committed before the change below, median of three, one process
per run:

| | before | after | |
|---|---:|---:|---|
| 2,500-solid 31 MB STEP, `--parse-only` | 2.970 s, 428.5 MB | 2.910 s, 423.2 MB | -2.0% / -1.2% |
| 961 boxes of 200 mm, missing-face pass | 90.8 ms | 0.0 ms | second mesh removed |

The remaining win is small because the removed copy is about 20 us per solid; the orientation mesh
that dominates the import is untouched. Identity: `eval` output is byte-identical for eight
designs (256, 961, 2,500, 2,500 and 500 imported bodies, `holes_100`, `many_bodies_120`,
`many_sketches_30`) and three STEP in/out round trips are byte-identical. The missing-face count
is unchanged on every fixture measured, and a kernel test pins that the orientation mesh's count
is the one `unmeshed_faces` answers with.

## Release profile (2026-10-12)

`[profile.release]` is now fat LTO with one codegen unit and `strip = "symbols"`; opt-level 3 and
debug 0 are unchanged and no `panic` key was added, because `Session::execute` keeps a
`catch_unwind` as the last-resort guard against losing a design. Four variants, built from one
frozen source snapshot, with no other build running:

| variant | solvecraft | solvecraft-cli | 100 holes | 80-cut pattern | 196-copy pattern | cold start |
|---|---:|---:|---:|---:|---:|---:|
| thin LTO, 16 units (before) | 58,296,296 | 24,996,288 | 4.471 s | 0.382 s | 6.640 s | 1.33 ms |
| fat LTO, 1 unit | 43,516,544 | 20,135,168 | 4.211 s | 0.377 s | 6.567 s | 1.20 ms |
| fat LTO, 1 unit, stripped (landed) | 38,099,720 | 17,721,120 | 4.229 s | 0.373 s | 6.390 s | 1.17 ms |
| thin LTO, 16 units, stripped | 46,417,304 | 19,969,832 | 4.162 s | 0.356 s | 6.312 s | 1.30 ms |

The binaries shrink by 34.6% and 29.1%, run time is equal or slightly better (seven uncontended
runs per binary with the order alternated), peak RSS about 2 MiB lower, and a cold release build
goes from 4m14s to 7m29s. The `ci` and `dev` profiles are untouched, so the quality gates and
tests are unaffected. `strip` removes symbol names from release backtraces; there is no backtrace
code in the repository, and profiling a release build needs
`CARGO_PROFILE_RELEASE_STRIP=none`.
