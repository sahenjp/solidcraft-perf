> **Personal fork build, not the upstream 0.3.2 release.** This repository is
> [sahenjp/solidcraft-perf](https://github.com/sahenjp/solidcraft-perf): upstream SolidCraft `0.3.2`
> plus a measured performance and memory pass - 4.1x on a 100-hole cut, 3.2x on the sample vent
> plate, 29-35% smaller release binaries, three redundant copies removed from the STEP reader -
> and a complete Japanese interface. The measurements, the harness that reproduces them and the
> open limitations are in the [README](https://github.com/sahenjp/solidcraft-perf#readme) and
> `perf/`. Upstream development happens at
> [bherbruck/solidcraft](https://github.com/bherbruck/solidcraft).
SolidCraft is open-source parametric 3D CAD in Rust: sketch, constrain, extrude, and change a
parameter to watch the timeline rebuild. [Documentation](https://bherbruck.github.io/solidcraft/)
· [try it in the browser](https://bherbruck.github.io/solidcraft/app/) ·
[contributing](https://github.com/bherbruck/solidcraft/blob/main/CONTRIBUTING.md) ·
[issues](https://github.com/bherbruck/solidcraft/issues). AI agents can model with it over MCP:
`solvecraft-cli mcp` ([how](https://bherbruck.github.io/solidcraft/guide/mcp.html)).

Compatibility names are unchanged: existing `.solvecraft` designs, `solvecraft-cli`, package
IDs and archive filenames still work. Published 0.2.0 and 0.2.1 releases keep their original
SolveCraft branding. Documentation, the browser app and the Flatpak feed use the new
SolidCraft project Pages URLs; old-name Pages addresses are not permanent aliases.

**Which file?**
- **Windows:** `SolveCraft-{version}-windows-x64.exe` runs as is. Or install the `.msi`, or unzip the
  `-portable.zip` (x64, x86 and arm64).
- **macOS 11+** (Apple silicon and Intel): open the `.dmg` and drag `SolveCraft.app` onto Applications.
  The CLI is in `solvecraft-cli-{version}-macos-universal.zip`.
- **Linux** (x86_64 and aarch64, glibc 2.35+): the `.AppImage` (`chmod +x`, then run it), the
  `.deb` or `.rpm`, the `.flatpak` (`flatpak install --user <file>`), or the `.tar.gz`.
- **FreeBSD:** `tar -xzf solvecraft-{version}-freebsd-x86_64.tar.gz --strip-components 1 -C /usr/local`.
- **Web:** `solvecraft-web-{version}.zip` is a static site to host yourself.
- `SHA256SUMS.txt`: `sha256sum -c SHA256SUMS.txt --ignore-missing`.

For Flatpak feed installation and updates, add Flathub for the required runtime before
installing SolidCraft; keep both remotes in the same user installation:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --user --if-not-exists solvecraft https://bherbruck.github.io/solidcraft/flatpak/solvecraft.flatpakrepo
flatpak install --user solvecraft ai.storyteller.solvecraft
flatpak run ai.storyteller.solvecraft
```

Single-file `.flatpak` bundles include the runtime repository address; missing runtimes
still need an internet connection. [Linux installation details](https://bherbruck.github.io/solidcraft/guide/getting-started.html#linux-flatpak).

The builds aren't signed yet. On macOS, right-click `SolveCraft.app` in Applications and choose
**Open** the first time (or run `xattr -dr com.apple.quarantine /Applications/SolveCraft.app`).
On Windows, SmartScreen may say "Windows protected your PC": **More info**, then **Run anyway**.
Licences are built in (Help ▸ About ▸ Licences, `solvecraft-cli licences`); the packages also
install `THIRD-PARTY-LICENSES.txt`. Known limitations:
[ROADMAP.md](https://github.com/bherbruck/solidcraft/blob/main/ROADMAP.md#known-limitations).

