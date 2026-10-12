# SolidCraft

**An open-source parametric 3D CAD application built in pure Rust.**

Sketch, constrain, and build solids with a parametric feature timeline. Change a dimension
and watch your model rebuild. Run it on your desktop, in your browser, or let AI agents
model through the built-in MCP server. Part of the storytold "craft" family.

---

**This is a personal fork.** It is SolidCraft `07e263d` plus a measured performance and memory
pass. Nothing here is planned as a pull request; the app is developed upstream at
[bherbruck/solidcraft](https://github.com/bherbruck/solidcraft). The changes, the measurements
behind them and the harness that reproduces them live in [perf/](perf/README.md) and
[docs/performance-optimization.md](docs/performance-optimization.md).

### What it changes

Speed and memory, without touching geometry, file formats, the command set or the interface:
whole-solid triangulations the kernel did not need (a cut or an intersection of prism pairs is
decided exactly in 2D, placed copies take their orientation from the placement matrix, a body
remembers its bounding-box diagonal), a byte budget for the evaluation cache, three redundant
copies in the STEP reader, a smaller release profile, and a complete Japanese translation with
font loading on demand.

### Measured (median of five runs, one process per run, nothing else on the machine)

| Design | Upstream `07e263d` | This branch | Ratio | Peak RSS |
|---|---:|---:|---:|---:|
| 100 holes cut into one plate | 16.78 s | 4.09 s | **4.1x** | 121 MB → 114 MB |
| vent plate sample | 0.16 s | 0.05 s | **3.2x** | 19 MB → 15 MB |
| vent plate, 80-cut pattern | 0.83 s | 0.36 s | **2.3x** | 21 MB → 18 MB |
| bolted flange sample | 2.02 s | 1.89 s | 1.07x | 153 MB → 152 MB |
| 196-copy body pattern | 6.36 s | 6.10 s | 1.04x | 62 MB → 60 MB |
| 121-feature join chain | 7.98 s | 7.80 s | 1.02x | 193 MB → 188 MB |

Medians of five runs, one process each; the ranges, the CPU times and the raw output are in
[perf/results](perf/results). Built with the same profile, the code changes alone are 4.2x on
the 100-hole plate and 2.4x on the vent pattern; the release profile adds a few percent of run
time and 3-5 MB less peak RSS, and a third of the binary size.

Raw output, the code-only and profile-only tables, the corpus equivalence check and the
byte-identical STEP round trips are all in [perf/results](perf/results).

### Reproduce

```
cargo build --release -p solvecraft-cli
perf/build-baseline.sh 07e263d              # the untouched baseline, in ~/.cache
perf/bench.sh <baseline-cli> target/release/solvecraft-cli 5 | tee perf/results/bench.csv
python3 perf/summarize.py perf/results/bench.csv
```

### Status

`cargo xtask ci` passes on the committed revision (26 test binaries, including 209 kernel
tests); its only failure is the `cargo xtask wasm` check, because the `wasm32-unknown-unknown`
target is not installed on the machine that produced these numbers and installing it needs root.

---
![SolidCraft: draw a rounded electronics enclosure, sketch its USB port directly on a face, add four mounting bosses and resize it through Parameters](docs/screenshots/enclosure-demo-0.2.1.gif)

*Start with an empty design and draw the original parametric electronics enclosure in the English desktop UI: rounded corners, a shell, a USB port sketched directly on the wall and a four-boss pattern. Change width from 80 → 100 mm and wall thickness from 2 → 3 mm in Parameters and watch it rebuild. Original animated pointer and click cues; 4× playback. [Full video](https://github.com/bherbruck/solidcraft/raw/refs/heads/main/docs/screenshots/enclosure-demo-0.2.1.mp4), generated with the [demo script](examples/demo/record_ui.sh).*

**Documentation:** the [SolidCraft book](https://bherbruck.github.io/solidcraft/) (user guide,
developer guide and command reference; sources in [docs/book](docs/book)), and the
[web app](https://bherbruck.github.io/solidcraft/app/) to try it in the browser.

Linux Flatpak installation needs the Freedesktop runtime from Flathub as well as the
SolidCraft application feed. See [install and launch commands](https://bherbruck.github.io/solidcraft/guide/getting-started.html#linux-flatpak).

**Status: early but usable for simple parts.** The historical benchmark report records
76 of 76 reference parts matching volume, area and topology ([docs/oracle.md](docs/oracle.md)).
These checks do not establish complete geometric or workflow equivalence. Feature coverage
is tracked in [docs/parity.md](docs/parity.md) and [ROADMAP.md](ROADMAP.md).
Open work and known bugs are in [GitHub issues](https://github.com/bherbruck/solidcraft/issues),
and help is welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).

## Name and compatibility

SolidCraft was previously called SolveCraft. The product display name and repository are
changing; existing identifiers remain compatible: `.solvecraft` designs, `solvecraft-cli`,
Cargo package names, settings/recovery locations and package IDs keep their existing names.
Published 0.2.0 and 0.2.1 packages and demo recordings retain their original SolveCraft branding
and filenames.

Use the SolidCraft documentation and web-app URLs above. GitHub redirects links to the
renamed repository; old-name Pages and Flatpak URLs are not maintained as permanent aliases.

## What works

- **Sketches** on origin planes, offset planes or planar faces: lines, rectangles, circles,
  arcs, polygons, slots, ellipses, fit/control splines, conics and text; construction geometry,
  projection, trimming, fillets and chamfers, including corners at the origin.
  New face attachments retain supported construction identities and chosen occurrences;
  missing sources expose an explicit plane repair workflow. See [face attachments](docs/face-attachments.md).
  Explicitly closed fit splines and closed associative offsets use periodic C2 interpolation.
  Supported convex line-and-arc offsets retain their identity when an inset consumes corner
  arcs, and linked extrusions follow the current boundary. Breaking the association freezes
  the boundary and keeps existing extrusions usable.
  Curve drawing and picking adapt together to zoom and display scale, including small circles
  at Fit and sketches in placed components.
- **Constraints and dimensions** solved by our own solver (damped Gauss–Newton with
  degree-of-freedom analysis): coincident, horizontal/vertical, parallel, perpendicular, tangent,
  equal, concentric, collinear, midpoint, symmetry, fix; linear, aligned, horizontal, vertical,
  radius, diameter and angle dimensions driven by parameters. Over-constraining edits are
  rejected; fully constrained geometry turns black.
  Select a line or its two endpoints, type its dimension and press Enter to commit one undo
  step. Fit keeps sketch geometry clear of the palette and navigation controls.
- **Profiles** found automatically, including regions split by lines and holes inside outlines.
  Newly captured complete adaptive boundaries retain their outer and hole source identities
  when spline span counts change. Partial or ambiguous boundaries do not qualify.
- **Features** on a parametric timeline: extrude, revolve, sweep, loft, fillet/chamfer,
  shell, draft, holes, threads, patterns, mirror, split, combine, move and construction geometry.
  The timeline rebuilds incrementally; features can be edited, suppressed, renamed, reordered
  and deleted. Supported geometry and current limitations are described in [ROADMAP.md](ROADMAP.md).
  New revolves preserve symmetric and two-sided angular placement across reversed axes;
  older designs retain their saved convention when reopened or edited.
- **Parameters and configurations** with unit-aware expressions (`2 * width + 5 mm`,
  `angle / 2`), ordered rules, feature suppression and per-row body materials/appearances.
- **Assemblies** with nested components and occurrences, joints, motion studies, contact sets,
  interference and material-aware mass properties. Recorded body transfers preserve modelling
  history and world placement; 3D sketch wires retain their coordinate frames through edits.
- **Sheet metal** with flanges, hems, fold/unfold/refold, material cutouts, flat patterns,
  retained shallow prismatic pockets, exact line/arc/circle DXF outlines and bend tables;
  plastic enclosure features and rules. Pocket depth and side survive unfold/refold and appear
  separately from through-hole outlines in the flat preview and DXF.
- **Files**: designs as JSON (`.solvecraft`), export to STEP (AP242 with body names, colours and
  components as assemblies; revolved faces as native planes, cylinders, cones, spheres and tori),
  IGES 5.3 (manifold solid B-reps with names and colours), 3MF, STL (binary/ASCII) and OBJ;
  3MF, STL and OBJ import as mesh bodies (they render, measure, move and export; solid features
  need B-rep bodies);
  STEP import (AP203/AP214/AP242 solids, assemblies, units, names, colours) and IGES import
  (manifold solid B-reps, or trimmed and bounded surfaces sewn into solids) as a base feature
  that later features build on — open a `.step`/`.stp`/`.igs`/`.iges` file, insert one into a
  design, or drop it on the window ([docs/step-import.md](docs/step-import.md)).
- **Desktop app** (egui + wgpu): toolbar with workspace tabs, browser, timeline, 3D viewport with
  axis view and navigation bar, sketch tools, feature dialogs, command palette, and a JSON-lines
  control channel so agents and tests can drive it.
  Feature errors and Help can open an editable **Create Issue** draft with relevant context
  and an optional reviewed screenshot; opening GitHub does not submit the report.
- **Headless CLI** `solvecraft-cli`: run command scripts, measure (volume, area, centre of mass,
  face/edge/vertex counts), export, render PNG snapshots and replay reference recipes.
- **MCP server** `solvecraft-cli mcp`: AI agents model, measure and look at parts headless or
  in the running app (see below).

## Build and run

```sh
cargo run --release -p solvecraft -- --sample          # desktop app with a sample part
cargo run --release -p solvecraft-cli -- commands      # the command registry
cargo run --release -p solvecraft-cli -- run examples/bracket.json --out bracket.step
cargo run --release -p solvecraft-cli -- snapshot examples/bracket.json --out bracket.png
cargo run --release -p solvecraft -- part.step          # open a STEP file as a new design
cargo run --release -p solvecraft-cli -- eval part.step # measure an imported STEP file
cargo xtask ci                                          # fmt, clippy, tests, assets, layers, wasm
```

Windows builds: `cargo xwin build --release --target x86_64-pc-windows-msvc -p solvecraft`.

On Windows, SolidCraft detects a connected 3Dconnexion SpaceNavigator or SpaceMouse through its
multi-axis HID interface. Move the cap to pan and zoom, and tilt or twist it to rotate the 3D view.
While editing a sketch, the cap pans and zooms without changing the sketch view. The Navigation
page in Preferences has an on/off switch and a speed control for the 3D mouse.

In the browser: `cd apps/solvecraft-web && trunk build --release --public-url ./` (WebGPU, or
WebGL2 with `?webgl`; `?sample` opens the sample part).

## Scripts

A script is a JSON list of commands, the same ones the UI runs:

```json
{"commands": [
  {"command": "sketch.create", "params": {"plane": "XY"}},
  {"command": "sketch.rectangle.two_point", "params": {"p0": [0, 0], "p1": [40, 30]}},
  {"command": "solid.extrude", "params": {"distance": 20}},
  {"command": "solid.fillet", "params": {"edges": [[0, 0, 10]], "radius": 3}}
]}
```

## AI agents (MCP)

`solvecraft-cli mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) server on
stdio. Headless it runs its own design session; with `--connect 127.0.0.1:PORT` it drives the app
started with `solvecraft --control PORT`, so you can watch the agent model. Tools:
`list_commands`, `execute`, `batch`, `inspect_design`, `measure`, `body_topology`,
`set_parameter`, `screenshot` (returns the image), `export`, `undo`, `redo`, `new_design`, `open`,
`save`. Claude Code: `claude mcp add solvecraft -- solvecraft-cli mcp`. Details, client
configuration and an example session: [docs/mcp.md](docs/mcp.md).

## Architecture

<!-- ANCHOR: architecture -->
| Layer | Crates | Role |
|---|---|---|
| L0 | `geom` | vectors, planes, profiles, meshes and their measures |
| L1 | `sketch`, `kernel`, `render` | constraint solver and profiles; the B-rep kernel boundary (truck); view math and CPU rasterizer |
| L2 | `doc` | parameters, expressions, the feature timeline and its incremental evaluation |
| L3 | `io` | design files, STL/OBJ/STEP/3MF export, STEP and 3MF/STL import features |
| L4 | `engine` | the session and the command registry (everything is a command) |
| L5 | `ui-egui`, `mcp` | the swappable desktop front end; the MCP server (headless or bridged to the app) |
| apps | `solvecraft`, `solvecraft-cli` | desktop app, headless CLI |
<!-- ANCHOR_END: architecture -->

## Contributing

Coming from Autodesk Fusion? SolidCraft uses familiar parametric CAD workflows: constrained
sketches, solid features, an editable timeline, parameters and assemblies. See the
[roadmap](ROADMAP.md) for supported operations and current limitations.

See [CONTRIBUTING.md](CONTRIBUTING.md): how to build and test, the code map, the clean-room
and licence rules, and where help is wanted most.

## Licence

MIT OR Apache-2.0, at your option. Developed under the
[clean-room contribution rules](CLAUDE.md); third-party code and assets retain their
documented licences and attribution.

References to other products describe workflow comparisons and benchmark provenance.
SolidCraft is not affiliated with, sponsored by or endorsed by Autodesk. Autodesk and
Fusion are trademarks of Autodesk, Inc.
