> **Personal fork build.** This repository is
> [sahenjp/solidcraft-perf](https://github.com/sahenjp/solidcraft-perf): upstream SolidCraft 0.10.0
> with a smaller release profile and the measurement harness in `perf/`. The kernel, cache and
> STEP changes this fork measured first are upstream as of 0.10.0
> ([PR #714](https://github.com/bherbruck/solidcraft/pull/714)), and so is the Japanese interface.
> Upstream development happens at
> [bherbruck/solidcraft](https://github.com/bherbruck/solidcraft).
SolidCraft is open-source parametric 3D CAD in Rust: sketch, constrain, extrude, and change a
parameter to watch the timeline rebuild. [Documentation](https://bherbruck.github.io/solidcraft/)
· [try it in the browser](https://bherbruck.github.io/solidcraft/app/) ·
[contributing](https://github.com/bherbruck/solidcraft/blob/main/CONTRIBUTING.md) ·
[issues](https://github.com/bherbruck/solidcraft/issues). AI agents can model with it over MCP:
`solidcraft-cli mcp` ([how](https://bherbruck.github.io/solidcraft/guide/mcp.html)).

**Why 0.9:** this is a minor bump because the design file format moved to version 17, and
because it adds many new features.

**What's new**
- **Face actions work on about 90% of flat faces.** Sketches, offset planes and press-pull now
  attach to sheet metal rims and holes, plastic features (Boss, Lip, Rest, Snap Fit), lofts,
  revolved faces, pieces Offset Face moved and walls a Join merged
  ([#677](https://github.com/bherbruck/solidcraft/pull/677), fixing
  [#681](https://github.com/bherbruck/solidcraft/issues/681)). Across the test models: sketch
  75% → 90%, offset plane 75% → 90%, press-pull 64% → 85%.
- **Joins:** a logo-style part with pieces extruded up and down from one sketch now joins
  ([#679](https://github.com/bherbruck/solidcraft/issues/679)), faces that only partly overlap
  fuse correctly ([#704](https://github.com/bherbruck/solidcraft/pull/704), fixing
  [#674](https://github.com/bherbruck/solidcraft/issues/674)), and booleans build the same body
  every time ([#748](https://github.com/bherbruck/solidcraft/pull/748)), so a design rebuilds
  identically on every run.
- **Sketch:** type a dimension right after picking (D, click, type;
  [#695](https://github.com/bherbruck/solidcraft/pull/695)); redundant but consistent
  constraints are accepted, such as a rectangle clover
  ([#699](https://github.com/bherbruck/solidcraft/pull/699)); tool windows for Scale, Offset,
  Mirror, Move/Copy, Fillet and Chamfer and the rectangular pattern (thanks @wdunn001), shown
  next to the Sketch Palette, which now lists the running tool's options; a typed Rho for the
  conic; a Construction Geometries row in the Sketch Palette that hides construction geometry
  ([#804](https://github.com/bherbruck/solidcraft/pull/804)); sketch tool shortcuts open a sketch on the selected plane; Edit Sketch from the viewport
  menu.
- **Features:** feature history on the left side with folders (opt in under Preferences;
  [#449](https://github.com/bherbruck/solidcraft/pull/449)); Convert to Sheet Metal from a solid
  or shelled body by its bend edges ([#725](https://github.com/bherbruck/solidcraft/pull/725));
  a tear corner relief ([#809](https://github.com/bherbruck/solidcraft/pull/809)); tapered and
  variable fillet radii ([#675](https://github.com/bherbruck/solidcraft/pull/675),
  [#749](https://github.com/bherbruck/solidcraft/pull/749)) with a radius spline you drag on the
  edge ([#752](https://github.com/bherbruck/solidcraft/pull/752)); free-form cut-outs stay exact
  in sheet-metal flat patterns ([#692](https://github.com/bherbruck/solidcraft/pull/692)) and edge
  flanges run along what cut-outs leave of an edge
  ([#702](https://github.com/bherbruck/solidcraft/pull/702)); and Surface Extrude, sheets from
  open curves and profile walls ([#815](https://github.com/bherbruck/solidcraft/pull/815)).
- **View:** Home and Fit frame only what is shown, and a middle double-click goes Home
  ([#701](https://github.com/bherbruck/solidcraft/pull/701)); a cube navigator option
  (thanks @Karl-VonRichter, [#664](https://github.com/bherbruck/solidcraft/pull/664)); the window
  stays responsive and draggable while a design opens, with a Cancel button that stays put
  ([#643](https://github.com/bherbruck/solidcraft/pull/643),
  [#686](https://github.com/bherbruck/solidcraft/pull/686)).
- **Quality:** cuts no longer split walls they don't touch
  ([#670](https://github.com/bherbruck/solidcraft/pull/670)); booleans are faster and use less
  memory ([#714](https://github.com/bherbruck/solidcraft/pull/714)); STL export welds shared
  vertices ([#816](https://github.com/bherbruck/solidcraft/pull/816)); complete Japanese
  translation ([#753](https://github.com/bherbruck/solidcraft/pull/753)).
- **Fixes:** pasted features act on the pasted bodies (#613); a refused OK's error clears when
  you change the inputs (#641); dialogs show the units of their sizes (#630); the wheel over a
  dialog's list scrolls only the list (#678); a Single hole picked at a circle's centre lands on
  the centre (#657); axis or tangent plane on a drill tip works (#567); sketch text edits from
  letter interiors and moves as a whole; a projected edge stays the edge you picked; a size from
  a bare diameter keeps its unit (#716); Convert Mesh says how big a mesh is and what converts;
  the Save changes prompt takes arrow keys and Enter.
- **MCP:** agent skills for modeling and editing over MCP
  ([#728](https://github.com/bherbruck/solidcraft/pull/728)).
- A new README demo and the SELENE rover showcase.

**File format 17.** Designs are now format 17 (`solidcraft/17`). SolidCraft 0.8 and older can't
open designs saved by 0.9, as a container or as JSON; they ask you to update instead. Keep a
copy of a file you may still open in 0.8. Files from every earlier version still open in 0.9.

**Upgrading from 0.8.** Nothing to do: open your designs and save (they're saved as format 17
from then on). Feature-history folders on
the left are off by default; turn them on in Preferences.

**Known limitations**
- Some through-all holes along a wall or beside curved faces still fail
  ([#559](https://github.com/bherbruck/solidcraft/issues/559)), as does a hole crossing an
  equal-radius hole ([#788](https://github.com/bherbruck/solidcraft/issues/788)).
- Some fillets are still refused, such as along an outline of lines and arcs
  ([#431](https://github.com/bherbruck/solidcraft/issues/431)).
- Some faces still can't take an attached sketch: a revolve's flat faces before a later feature
  ([#726](https://github.com/bherbruck/solidcraft/issues/726)) and faces after Split Body or
  Combine Intersect ([#565](https://github.com/bherbruck/solidcraft/issues/565)).
- Fillet and chamfer accept an edge pick far from any edge and change the nearest one
  ([#783](https://github.com/bherbruck/solidcraft/issues/783)); Press Pull fails on faces beside
  rounds ([#562](https://github.com/bherbruck/solidcraft/issues/562)).
- `sheet.fold` refuses a fixed point exactly on the sheet face
  ([#685](https://github.com/bherbruck/solidcraft/issues/685)), and some tabs on L-shaped or
  collinear blanks can't be folded ([#638](https://github.com/bherbruck/solidcraft/issues/638),
  [#639](https://github.com/bherbruck/solidcraft/issues/639)); an edge flange with different
  per-edge angles leaves gaps at the corners
  ([#501](https://github.com/bherbruck/solidcraft/issues/501)).
- More in [ROADMAP.md](https://github.com/bherbruck/solidcraft/blob/main/ROADMAP.md#known-limitations).

**Upgrading from 0.7 or earlier.** 0.8 changed the Linux app ID to `ai.storyteller.solidcraft`:
`flatpak update` offers to replace the old app, and the `.deb`, `.rpm` and AppImage remove the old
menu entry.

**Upgrading from 0.5 or earlier.** 0.6 renamed SolveCraft to SolidCraft and made the zip
container the default save format. `.solvecraft` designs, old settings folders and
`SOLVECRAFT_*` variables are still picked up, and `solvecraft-cli` still forwards to
`solidcraft-cli`. Want plain JSON? Use **File ▸ Save As JSON…** or `--save design.json`. On
macOS, replace `SolveCraft.app` with `SolidCraft.app`. Details:
[compatibility notes](https://github.com/bherbruck/solidcraft/blob/main/docs/compatibility.md).

Documentation, the browser app and the Flatpak feed use the SolidCraft project Pages URLs.

**Which file?**
- **Windows:** `SolidCraft-{version}-windows-x64.exe` runs as is. Or install the `.msi`, or unzip the
  `-portable.zip` (x64, x86 and arm64).
- **macOS 11+** (Apple silicon and Intel): open the `.dmg` and drag `SolidCraft.app` onto Applications.
  The CLI is in `solidcraft-cli-{version}-macos-universal.zip`.
- **Linux** (x86_64 and aarch64, glibc 2.35+): the `.AppImage` (`chmod +x`, then run it), the
  `.deb` or `.rpm`, the `.flatpak` (`flatpak install --user <file>`), the `.snap`
  (`sudo snap install --dangerous <file>`, or `sudo snap install solidcraft` from the Snap Store), or the `.tar.gz`.
- **FreeBSD:** `tar -xzf solidcraft-{version}-freebsd-x86_64.tar.gz --strip-components 1 -C /usr/local`.
- **Web:** `solidcraft-web-{version}.zip` is a static site to host yourself.
- `SHA256SUMS.txt`: `sha256sum -c SHA256SUMS.txt --ignore-missing`.
- Files without a version in the name (`solidcraft-windows-x64.msi`, `solidcraft-linux-x86_64.AppImage`, …)
  are the same packages under fixed names, for download links that always fetch the latest release.

For Flatpak feed installation and updates, add Flathub for the required runtime before
installing SolidCraft; keep both remotes in the same user installation:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --user --if-not-exists solidcraft https://bherbruck.github.io/solidcraft/flatpak/solidcraft.flatpakrepo
flatpak install --user solidcraft ai.storyteller.solidcraft
flatpak run ai.storyteller.solidcraft
```

Flatpak installs from 0.7 or earlier use the old ID `ai.storyteller.solvecraft`: `flatpak update --user`
offers to replace it with `ai.storyteller.solidcraft`, and settings move along.

Single-file `.flatpak` bundles include the runtime repository address; missing runtimes
still need an internet connection. [Linux installation details](https://bherbruck.github.io/solidcraft/guide/getting-started.html#linux-flatpak).

The builds aren't signed yet. On macOS, right-click `SolidCraft.app` in Applications and choose
**Open** the first time (or run `xattr -dr com.apple.quarantine /Applications/SolidCraft.app`).
On Windows, SmartScreen may say "Windows protected your PC": **More info**, then **Run anyway**.
Licences are built in (Help ▸ About ▸ Licences, `solidcraft-cli licences`); the packages also
install `THIRD-PARTY-LICENSES.txt`.

