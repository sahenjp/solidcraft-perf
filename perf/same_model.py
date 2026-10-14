#!/usr/bin/env python3
"""Is one CLI eval output the same model as another?

    python3 perf/same_model.py <a.json> <b.json> [max_relative] [max_absolute]

Per-feature millisecond fields are ignored. Structural differences (a missing key, a different
length, a non-numeric value, a different type) fail. A floating-point field passes when its
relative difference is within max_relative (default 1e-9) OR its absolute difference is within
max_absolute (default 1e-9), which is what a changed tessellation order produces in the last
bits: a bounding-box minimum that read -3.5e-15 and now reads exactly 0 is a 3.5e-15 mm change,
not a relative one. Exit code 0 means "the same model".
"""
import json
import sys


def strip(o):
    if isinstance(o, dict):
        return {k: strip(v) for k, v in o.items() if k != "ms"}
    if isinstance(o, list):
        return [strip(v) for v in o]
    return o


def walk(x, y, path, limits, hard, worst):
    rel_limit, abs_limit = limits
    if type(x) is not type(y):
        hard.append("%s: type %s vs %s" % (path, type(x).__name__, type(y).__name__))
        return
    if isinstance(x, dict):
        for k in sorted(set(x) | set(y)):
            if k not in x:
                hard.append("%s.%s: only on the right" % (path, k))
            elif k not in y:
                hard.append("%s.%s: only on the left" % (path, k))
            else:
                walk(x[k], y[k], "%s.%s" % (path, k), limits, hard, worst)
    elif isinstance(x, list):
        if len(x) != len(y):
            hard.append("%s: length %d vs %d" % (path, len(x), len(y)))
        else:
            for i, (p, q) in enumerate(zip(x, y)):
                walk(p, q, "%s[%d]" % (path, i), limits, hard, worst)
    elif isinstance(x, float):
        if x != y:
            delta = abs(x - y)
            rel = delta / max(abs(x), abs(y), 1e-300)
            if rel > worst[0]:
                worst[0], worst[1] = rel, path
            if delta > worst[2]:
                worst[2], worst[3] = delta, path
            if rel > rel_limit and delta > abs_limit:
                hard.append("%s: %r vs %r (rel %.1e, abs %.1e)" % (path, x, y, rel, delta))
    elif x != y:
        hard.append("%s: %r vs %r" % (path, x, y))


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    rel_limit = float(sys.argv[3]) if len(sys.argv) > 3 else 1e-9
    abs_limit = float(sys.argv[4]) if len(sys.argv) > 4 else 1e-9
    a = strip(json.load(open(sys.argv[1])))
    b = strip(json.load(open(sys.argv[2])))
    hard, worst = [], [0.0, "", 0.0, ""]
    walk(a, b, "", (rel_limit, abs_limit), hard, worst)
    where = " at %s" % worst[1] if worst[1] else ""
    awhere = " at %s" % worst[3] if worst[3] else ""
    if not hard:
        print("same model (worst relative %.1e%s, worst absolute %.1e%s)" % (worst[0], where, worst[2], awhere))
        return 0
    print("DIFFERENT: %d differences, worst relative %.1e%s, worst absolute %.1e%s" % (len(hard), worst[0], where, worst[2], awhere))
    for d in hard[:10]:
        print("   ", d)
    return 1


if __name__ == "__main__":
    sys.exit(main())