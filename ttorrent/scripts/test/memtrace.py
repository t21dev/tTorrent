"""Samples a running app's memory every 200 ms, independently of bench.ps1 (ttorrent/VERIFY.md › Benchmarks).

    python -I ttorrent/scripts/test/memtrace.py <exe path> <seconds> <out.csv>

Waits for a process started from <exe path>, then writes seconds since it
started, working set and private bytes (MB) to <out.csv>, and prints any
sample more than 40 MB above the running median: a spike.
"""
import csv
import os
import statistics
import sys
import time

import psutil


def main():
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    exe, seconds, out = os.path.normcase(os.path.abspath(sys.argv[1])), float(sys.argv[2]), sys.argv[3]
    proc = None
    deadline = time.time() + 120
    while proc is None and time.time() < deadline:
        for p in psutil.process_iter(["exe"]):
            if p.info["exe"] and os.path.normcase(p.info["exe"]) == exe:
                proc = p
                break
        time.sleep(0.1)
    if proc is None:
        sys.exit("no process started from " + exe)
    started = proc.create_time()
    rows, spikes = [], []
    while time.time() - started < seconds and proc.is_running():
        try:
            m = proc.memory_info()
        except psutil.Error:
            break
        t = time.time() - started
        rows.append((round(t, 1), round(m.wset / 2**20, 1), round(m.private / 2**20, 1)))
        recent = [r[2] for r in rows[-50:]]
        if len(recent) > 10 and rows[-1][2] > statistics.median(recent) + 40:
            spikes.append(rows[-1])
        time.sleep(0.2)
    with open(out, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["seconds", "working_set_mb", "private_bytes_mb"])
        w.writerows(rows)
    print(f"{len(rows)} samples; peak private bytes {max(r[2] for r in rows)} MB")
    print("spikes:", spikes[:20] if spikes else "none")


if __name__ == "__main__":
    main()
