#!/usr/bin/env python3
"""Sample RSS/PSS/CPU of a process tree while it runs (no profiler needed).

usage:
  measure.py --label NAME [--idle-after S] [--idle-window S] [--timeout S] -- CMD...

Prints one JSON object: peak/final RSS, peak PSS, CPU seconds, wall time, idle CPU%.
"""
import argparse, json, os, subprocess, sys, time

CLK = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")


def pids_of(pid):
    """pid plus its children (one level is enough here)."""
    out = [pid]
    try:
        with open(f"/proc/{pid}/task/{pid}/children") as f:
            for c in f.read().split():
                out.append(int(c))
    except OSError:
        pass
    return out


def stat_of(pid):
    try:
        with open(f"/proc/{pid}/stat") as f:
            parts = f.read().rsplit(") ", 1)[1].split()
        return int(parts[11]) + int(parts[12])  # utime+stime in ticks
    except (OSError, IndexError, ValueError):
        return None


def rss_kb(pid):
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return None


def pss_kb(pid):
    try:
        with open(f"/proc/{pid}/smaps_rollup") as f:
            for line in f:
                if line.startswith("Pss:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--label", required=True)
    ap.add_argument("--timeout", type=float, default=600)
    ap.add_argument("--idle-after", type=float, default=None,
                    help="seconds after which the process is considered settled")
    ap.add_argument("--idle-window", type=float, default=5.0)
    ap.add_argument("--stdout", default=None)
    ap.add_argument("--stderr", default=None)
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    cmd = a.cmd[1:] if a.cmd and a.cmd[0] == "--" else a.cmd
    if not cmd:
        ap.error("no command")

    so = open(a.stdout, "wb") if a.stdout else subprocess.DEVNULL
    se = open(a.stderr, "wb") if a.stderr else subprocess.DEVNULL
    t0 = time.monotonic()
    p = subprocess.Popen(cmd, stdout=so, stderr=se)
    peak_rss = final_rss = peak_pss = 0
    samples = []
    idle_start = None
    cpu_at_idle = None
    try:
        while True:
            now = time.monotonic()
            if now - t0 > a.timeout:
                p.kill()
                break
            r = rss_kb(p.pid)
            if r is not None:
                peak_rss = max(peak_rss, r)
                final_rss = r
                ps = pss_kb(p.pid)
                if ps:
                    peak_pss = max(peak_pss, ps)
                samples.append((round(now - t0, 2), r, ps or 0))
            if a.idle_after is not None and idle_start is None and now - t0 >= a.idle_after:
                idle_start = now
                cpu_at_idle = stat_of(p.pid)
            if idle_start is not None and now - idle_start >= a.idle_window:
                cpu_end = stat_of(p.pid)
                if cpu_at_idle is not None and cpu_end is not None:
                    idle_cpu = (cpu_end - cpu_at_idle) / CLK / (now - idle_start) * 100.0
                else:
                    idle_cpu = None
                p.terminate()
                try:
                    p.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    p.kill()
                break
            if p.poll() is not None:
                break
            time.sleep(0.05)
    finally:
        if p.poll() is None:
            p.terminate()
            try:
                p.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p.kill()
        if so != subprocess.DEVNULL:
            so.close()
        if se != subprocess.DEVNULL:
            se.close()
    wall = time.monotonic() - t0
    print(json.dumps({
        "label": a.label,
        "exit": p.returncode,
        "wall_s": round(wall, 3),
        "peak_rss_mb": round(peak_rss / 1024, 1),
        "final_rss_mb": round(final_rss / 1024, 1),
        "peak_pss_mb": round(peak_pss / 1024, 1),
        "idle_cpu_pct": None if a.idle_after is None else round(idle_cpu, 2),
        "samples": len(samples),
    }))


main()
