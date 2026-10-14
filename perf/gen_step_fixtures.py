#!/usr/bin/env python3
"""Generate the large-CAD STEP fixtures the import measurements use.

The designs they are exported from are committed under perf/designs; the STEP files (60 MB
together) are not, and they are written outside the checkout (the asset gate only wants
committed assets), so run this once before perf/identity.sh or an import benchmark:

    cargo build --release -p solvecraft-cli
    python3 perf/gen_step_fixtures.py [path-to-solvecraft-cli] [output-directory]

The default output directory is $PERF_CACHE/fixtures (~/.cache/solidcraft-perf/fixtures).
One CLI run per fixture; the sizes are what the tables in perf/reports quote.
"""
import os
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DESIGNS = os.path.join(ROOT, "perf", "designs")
CACHE = os.environ.get("PERF_CACHE") or os.path.join(os.path.expanduser("~"), ".cache", "solidcraft-perf")
DEFAULT_OUT = os.path.join(CACHE, "fixtures")

EXPORTS = [
    ("grid_16x16_boxes.json", "grid_16x16_boxes.step", "256 solids"),
    ("grid_31x31_boxes.json", "grid_31x31_boxes.step", "961 solids"),
    ("many_bodies_500.json", "many_bodies_500.step", "500 solids"),
    ("grid_31x31_large.json", "grid_31x31_large.step", "961 solids of 200x200x50 mm"),
    ("grid_multi_2500.json", "grid_multi_2500.step", "2500 solids, 15k faces"),
    ("grid_multi_2500_fillet.json", "grid_multi_2500_fillet.step", "2500 filleted solids, 22.5k faces, 483k entities"),
]


def main():
    cli = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "release", "solvecraft-cli")
    if not os.path.exists(cli):
        print("build it first: cargo build --release -p solvecraft-cli  (looked for %s)" % cli)
        return 2
    out = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_OUT
    os.makedirs(out, exist_ok=True)
    for design, fixture, note in EXPORTS:
        src = os.path.join(DESIGNS, design)
        dst = os.path.join(out, fixture)
        started = time.time()
        subprocess.run([cli, "run", src, "--out", dst, "--quiet"], check=True)
        print("%-28s %7.1f MB %6.1f s  %s" % (fixture, os.path.getsize(dst) / 1e6, time.time() - started, note))
    return 0


if __name__ == "__main__":
    sys.exit(main())