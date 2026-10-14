#!/usr/bin/env python3
"""Compare two CLI eval JSON outputs ignoring per-feature timings."""
import json, sys

def strip(o):
    if isinstance(o, dict):
        return {k: strip(v) for k, v in o.items() if k not in ("ms",)}
    if isinstance(o, list):
        return [strip(v) for v in o]
    return o

a = json.load(open(sys.argv[1]))
b = json.load(open(sys.argv[2]))
sa, sb = strip(a), strip(b)
if sa == sb:
    print("IDENTICAL (ignoring ms)")
else:
    diffs = []
    def walk(x, y, path=""):
        if type(x) is not type(y):
            diffs.append(f"{path}: type {type(x).__name__} vs {type(y).__name__}")
        elif isinstance(x, dict):
            for k in set(x) | set(y):
                if k not in x: diffs.append(f"{path}.{k}: missing left")
                elif k not in y: diffs.append(f"{path}.{k}: missing right")
                else: walk(x[k], y[k], f"{path}.{k}")
        elif isinstance(x, list):
            if len(x) != len(y): diffs.append(f"{path}: len {len(x)} vs {len(y)}")
            else:
                for i, (p, q) in enumerate(zip(x, y)): walk(p, q, f"{path}[{i}]")
        elif isinstance(x, float):
            if x != y:
                rel = abs(x - y) / max(abs(x), abs(y), 1e-300)
                diffs.append(f"{path}: {x!r} vs {y!r} (rel {rel:.2e})")
        elif x != y:
            diffs.append(f"{path}: {x!r} vs {y!r}")
    walk(sa, sb)
    print(f"{len(diffs)} differences")
    for d in diffs[:25]:
        print("  ", d)
