#!/usr/bin/env python3
"""Compare every corpus output ignoring per-feature timings; report the worst relative delta."""
import json, os, sys

def strip(o):
    if isinstance(o, dict): return {k: strip(v) for k, v in o.items() if k != "ms"}
    if isinstance(o, list): return [strip(v) for v in o]
    return o

def walk(x, y, path, out):
    if type(x) is not type(y):
        out.append((path, "TYPE", x, y)); return
    if isinstance(x, dict):
        for k in set(x) | set(y):
            if k not in x: out.append((path + "." + k, "MISSING", None, None))
            elif k not in y: out.append((path + "." + k, "MISSING", None, None))
            else: walk(x[k], y[k], path + "." + k, out)
    elif isinstance(x, list):
        if len(x) != len(y): out.append((path, f"LEN {len(x)}!={len(y)}", None, None))
        else:
            for i, (p, q) in enumerate(zip(x, y)): walk(p, q, f"{path}[{i}]", out)
    elif isinstance(x, float):
        if x != y:
            rel = abs(x - y) / max(abs(x), abs(y), 1e-300)
            out.append((path, f"{rel:.2e}", x, y))
    elif x != y:
        out.append((path, "VALUE", x, y))

base, new = sys.argv[1], sys.argv[2]
worst = 0.0; worst_path = ""
worst_abs = 0.0; worst_abs_path = ""
bad = []
for name in sorted(os.listdir(base)):
    if not name.endswith(".json"): continue
    p2 = os.path.join(new, name)
    if not os.path.exists(p2): print(f"{name:34} MISSING in new"); continue
    try:
        a = json.load(open(os.path.join(base, name)))
        b = json.load(open(p2))
    except (json.JSONDecodeError, OSError):
        # A design whose evaluation fails writes no output; its stderr sits next to it.
        def why(d):
            err = os.path.join(d, name[:-5] + ".err")
            if os.path.exists(err):
                lines = open(err).read().strip().splitlines()
                return lines[0] if lines else "no output"
            return "no output"
        ea, eb = why(base), why(new)
        if ea == eb:
            print(f"{name:34} both failed identically ({ea})")
        else:
            print(f"{name:34} FAILED DIFFERENTLY:\n    base: {ea}\n    new:  {eb}")
            bad.append((name, [("stderr", "VALUE", ea, eb)]))
        continue
    if not a and not b:
        print(f"{name:34} both failed (ok)"); continue
    diffs = []
    walk(strip(a), strip(b), "", diffs)
    hard = [d for d in diffs if d[1] == "TYPE" or d[1] == "MISSING" or d[1].startswith("LEN") or d[1] == "VALUE"]
    rels = [(float(d[1]), d[0]) for d in diffs if d[1] not in ("TYPE", "MISSING", "VALUE") and not d[1].startswith("LEN")]
    m = max(rels) if rels else (0.0, "")
    if m[0] > worst: worst, worst_path = m[0], f"{name}{m[1]}"
    absds = [(abs(d[2] - d[3]), d[0]) for d in diffs if d[1] not in ("TYPE", "MISSING", "VALUE") and not d[1].startswith("LEN")]
    a = max(absds) if absds else (0.0, "")
    if a[0] > worst_abs: worst_abs, worst_abs_path = a[0], f"{name}{a[1]}"
    status = "IDENTICAL" if not diffs else f"{len(diffs)} float diffs, max rel {m[0]:.1e}" + ("" if not hard else f", {len(hard)} STRUCTURAL")
    print(f"{name:34} {status}")
    if hard: bad.append((name, hard[:5]))
print()
print(f"worst float difference: {worst:.2e} relative at {worst_path}")
print(f"worst float difference: {worst_abs:.2e} absolute at {worst_abs_path}")
if bad:
    print("STRUCTURAL DIFFERENCES:")
    for n, h in bad:
        print(" ", n)
        for d in h: print("    ", d)
else:
    print("no structural differences")
