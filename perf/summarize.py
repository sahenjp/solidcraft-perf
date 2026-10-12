#!/usr/bin/env python3
"""Summarise perf/bench.sh output as a markdown table.

    perf/bench.sh <baseline> <candidate> 5 > perf/results/bench.csv
    python3 perf/summarize.py perf/results/bench.csv

Reports the median of the runs with the observed range, the ratio between the two labels,
and peak RSS for both. Medians are used because a single slow run (a page-fault storm, a
background timer) does not move them; the range shows how much spread there was.
"""
import csv
import statistics
import sys


def load(path):
    runs = {}
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            runs.setdefault((row["design"], row["label"]), []).append(row)
    return runs


def stats(rows, field):
    values = sorted(float(r[field]) for r in rows)
    return statistics.median(values), values[0], values[-1]


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    runs = load(sys.argv[1])
    designs = sorted({d for d, _ in runs})
    print("| Design | Runs | Before (median, range) | After (median, range) | Ratio | Peak RSS before | Peak RSS after |")
    print("|---|---:|---:|---:|---:|---:|---:|")
    for d in designs:
        before = runs.get((d, "before"))
        after = runs.get((d, "after"))
        if not before or not after:
            print("| %s | missing one side | | | | | |" % d)
            continue
        bw, blo, bhi = stats(before, "wall_s")
        aw, alo, ahi = stats(after, "wall_s")
        br, _, _ = stats(before, "max_rss_mb")
        ar, _, _ = stats(after, "max_rss_mb")
        print(
            "| %s | %d | %.2f s (%.2f-%.2f) | %.2f s (%.2f-%.2f) | %.2fx | %.1f MB | %.1f MB |"
            % (d, len(before), bw, blo, bhi, aw, alo, ahi, bw / aw if aw else float("nan"), br, ar)
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())