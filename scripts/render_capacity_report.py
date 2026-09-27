#!/usr/bin/env python3
"""Render capacity run records into REPORT.md (docs/CAPACITY_EVIDENCE_PLAN.md §5.7).

The generated section introduces no numbers of its own: every value is read
from the committed run_record.json files. Reports may carry hand-written
narrative (purpose, session context) above the generated section; the drift
gate verifies only the marked section, so narrative and numbers can never
silently disagree.

Usage:

    python3 scripts/render_capacity_report.py docs/capacity/<dir>      # render to stdout
    python3 scripts/render_capacity_report.py --update                 # (re)write marked sections in place
    python3 scripts/render_capacity_report.py --check                  # gate: marked section must match a fresh render

Handled record layouts (dispatched automatically per directory):

- `crawlkit.capacity.run_record/v1`, single `run_record.json` — the 1k CI
  smoke class.
- `crawlkit.capacity.run_record/v1`, `run_record_run*.json` — inline 3-run
  valid sets (10k reference class).
- `crawlkit.capacity.distributed_run_record/v1`, `run_record_run*.json` —
  distributed-posture sets; records with a distinct `workload` field (e.g.
  the 100k headline run) render as their own group.
- `run_record_inline_N.json` + `run_record_queue_N.json` — paired
  same-commit comparison sets; the paired overhead ratio is computed
  mean-to-mean from the two medians, never hand-copied.

Directories without any `run_record*.json` (narrative/diagnostic artifacts
such as heap attribution) are skipped.
"""

from __future__ import annotations

import json
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CAPACITY = ROOT / "docs" / "capacity"

BEGIN = "<!-- GENERATED-NUMBERS:BEGIN -->"
END = "<!-- GENERATED-NUMBERS:END -->"

# Plan §2 targets (throughput floor and RSS cap apply to the 1k CI class;
# they are the published §2 rows for all classes, compared honestly).
THROUGHPUT_TARGET = 50.0  # pages/s (ROADMAP: >= 50)
RSS_TARGET_KB = 500_000  # 500 MB (ROADMAP: < 500 MB @ 10k)
SPREAD_LIMIT = 0.15  # plan §5.6: >15% spread invalidates the set

PAIRED_RUN_RE = re.compile(r"^run_record_(inline|queue)_(\d+)\.json$")


def load(path: Path) -> dict:
    return json.loads(path.read_text())


def spread(values: list[float]) -> float:
    lo = min(values)
    return (max(values) - lo) / lo if lo else float("inf")


def machine_line(r: dict) -> str:
    mem = r.get("mem_total_kb")
    mem_s = f"{round(mem / 1024 / 1024)} GB RAM · " if mem else ""
    return f"{r['cpu_model']} · {mem_s}Linux {r['kernel']}"


def honest_notes(rss_note: str) -> list[str]:
    return [
        "",
        "Honest notes:",
        "",
        "- Loopback serving inflates throughput vs the real internet by design; "
        "this is an engine-capacity number (plan §9).",
        rss_note,
        "- Every number above is derived from the committed records; nothing is hand-copied.",
        "",
    ]


INLINE_RSS_MISSED = (
    "- The RSS target is the plan §2 row sized for the 1k CI class; at this "
    "workload class it is **missed** — the raw records above are the evidence, "
    "and the gap is a 6.0.0 engineering item, not a reporting artifact."
)
INLINE_RSS_MET = (
    "- The RSS target is the plan §2 row sized for the 1k CI class; at this "
    "workload class it is **met**."
)
DISTRIBUTED_RSS = (
    "- The relevant RSS bound for this class is the per-worker / worker-sum cap "
    "enforced by the record's own gates (shown per run below), not the inline "
    "500 MB row."
)


# --------------------------------------------------------------------------
# inline run_record/v1
# --------------------------------------------------------------------------


def render_inline(records: list[dict]) -> list[str]:
    med = {
        "throughput": statistics.median(r["results"]["throughput_pages_per_sec"] for r in records),
        "rss_peak_kb": statistics.median(int(r["resources"]["rss_peak_kb"]) for r in records),
        "elapsed": statistics.median(float(r["results"]["elapsed_secs"]) for r in records),
        "tasks_peak": statistics.median(int(r["resources"]["tasks_peak"]) for r in records),
    }
    config = records[0]["config"]
    thr_spread = spread([float(r["results"]["throughput_pages_per_sec"]) for r in records])
    thr_target_met = med["throughput"] >= THROUGHPUT_TARGET
    rss_target_met = med["rss_peak_kb"] < RSS_TARGET_KB

    lines = [
        f"**Workload:** `{config['pages']}` pages · storage `{config['storage']}` · "
        f"concurrency {config['concurrency']} · delay {config['request_delay_ms']} ms · "
        f"build profile `{config['profile']}` · mode `{config['mode']}`",
        f"**Engine:** crawlkit {records[0]['crawlkit_version']}",
        f"**Machine:** {machine_line(records[0])}",
        f"**Runs:** {len(records)} committed raw records · throughput spread {thr_spread:.1%} → set "
        + ("VALID" if thr_spread <= SPREAD_LIMIT else f"INVALID (> {SPREAD_LIMIT:.0%}, rerun required)"),
        "",
        "| Metric | Median | Target (plan §2) | Verdict |",
        "|---|---|---|---|",
        f"| Throughput | **{med['throughput']:.1f} pages/s** | ≥ {THROUGHPUT_TARGET:.0f} | "
        + ("✅ met |" if thr_target_met else "❌ missed |"),
        f"| Peak RSS | **{med['rss_peak_kb'] / 1024:.0f} MB** | < {RSS_TARGET_KB // 1000} MB | "
        + ("✅ met |" if rss_target_met else "❌ missed |"),
        f"| Crawl wall time | {med['elapsed']:.1f} s | — | — |",
        f"| Peak tasks | {med['tasks_peak']:.0f} | bounded, returns to baseline | "
        + ("✅ (fd gate passed on every run) |" if all(r["gates"]["fds_returned_to_baseline"] for r in records) else "❌ |"),
        "",
        "| Run | Throughput | Peak RSS | Pages | fds start→end | All absolute gates |",
        "|---|---|---|---|---|---|",
    ]
    for r in records:
        res, reso, gates = r["results"], r["resources"], r["gates"]
        lines.append(
            f"| {r['workload']} @ {r['recorded_at_unix']} | {res['throughput_pages_per_sec']:.1f} p/s | "
            f"{reso['rss_peak_kb'] / 1024:.0f} MB | {res['pages_crawled']} | "
            f"{reso['fd_baseline']}→{reso['fd_end']} | "
            + ("✅" if gates["all_absolute_pass"] else "❌ (see gates in raw record)")
        )
    lines += honest_notes(INLINE_RSS_MET if rss_target_met else INLINE_RSS_MISSED)
    return lines


# --------------------------------------------------------------------------
# distributed_run_record/v1 (grouped by workload label)
# --------------------------------------------------------------------------


def render_distributed_group(records: list[dict], workload: str) -> list[str]:
    agg = [float(r["results"]["throughput_aggregate_pages_per_sec"]) for r in records]
    rss_sum = [int(r["resources"]["worker_rss_peak_sum_kb"]) for r in records]
    thr_spread = spread(agg)
    thr_target_met = statistics.median(agg) >= THROUGHPUT_TARGET
    topo = records[0]["topology"]

    lines = [
        f"### Workload `{workload}`",
        "",
        f"**Topology:** {topo['workers']} worker processes · storage `{topo['storage']}` · "
        f"queue `{topo.get('queue', 'n/a')}` · pages/worker {topo.get('pages_per_worker', '?')} · "
        f"concurrency/worker {topo.get('concurrency_per_worker', '?')} · profile `{topo.get('profile', '?')}`",
        f"**Engine:** crawlkit {records[0]['crawlkit_version']}",
        f"**Machine:** {machine_line(records[0])}",
        f"**Runs:** {len(records)} committed raw records · aggregate-throughput spread "
        f"{thr_spread:.1%} → set "
        + ("VALID" if thr_spread <= SPREAD_LIMIT else f"INVALID (> {SPREAD_LIMIT:.0%}, rerun required)"),
        "",
        "| Metric | Median | Target (plan §2) | Verdict |",
        "|---|---|---|---|",
        f"| Aggregate throughput | **{statistics.median(agg):.1f} pages/s** | ≥ {THROUGHPUT_TARGET:.0f} | "
        + ("✅ met |" if thr_target_met else "❌ missed |"),
        f"| Worker RSS sum (peak) | **{statistics.median(rss_sum) / 1024:.0f} MB** | per-run caps (gates) | — |",
        "",
        "| Run | Aggregate p/s | Worker RSS sum peak | Pages | All gates |",
        "|---|---|---|---|---|",
    ]
    for r in records:
        lines.append(
            f"| {r['workload']} @ {r['recorded_at_unix']} | "
            f"{r['results']['throughput_aggregate_pages_per_sec']:.1f} | "
            f"{int(r['resources']['worker_rss_peak_sum_kb']) / 1024:.0f} MB | "
            f"{r['results']['pages_total']} | "
            + ("✅" if r["gates"]["all_pass"] else "❌ (see gates in raw record)")
        )
    lines += honest_notes(DISTRIBUTED_RSS)
    return lines


# --------------------------------------------------------------------------
# paired inline/queue sets (same-commit comparison)
# --------------------------------------------------------------------------


def render_paired(sets: dict[str, list[dict]]) -> list[str]:
    lines: list[str] = []
    medians: dict[str, float] = {}
    r0 = next(iter(sets.values()))[0]
    lines += [
        f"**Engine:** crawlkit {r0['crawlkit_version']}",
        f"**Machine:** {machine_line(r0)}",
        "",
    ]
    for name in sorted(sets):
        records = sets[name]
        agg = [float(r["results"]["throughput_aggregate_pages_per_sec"]) for r in records]
        rss_sum = [int(r["resources"]["worker_rss_peak_sum_kb"]) for r in records]
        medians[name] = statistics.median(agg)
        topo = records[0]["topology"]
        lines += [
            f"### `{name}` set — {len(records)} runs (topology: "
            f"{topo['workers']} workers · {topo.get('queue', 'n/a')})",
            "",
            "| Run | Aggregate p/s | Worker RSS sum peak | Pages | All gates |",
            "|---|---|---|---|---|",
        ]
        for r in records:
            lines.append(
                f"| run {r['recorded_at_unix']} | "
                f"{r['results']['throughput_aggregate_pages_per_sec']:.1f} | "
                f"{int(r['resources']['worker_rss_peak_sum_kb']) / 1024:.0f} MB | "
                f"{r['results']['pages_total']} | "
                + ("✅" if r["gates"]["all_pass"] else "❌")
            )
        sp = spread(agg)
        lines += [
            "",
            f"Median aggregate throughput: **{medians[name]:.1f} pages/s** "
            f"(range {min(agg):.1f}–{max(agg):.1f}, spread {sp:.1%} → set "
            + ("VALID" if sp <= SPREAD_LIMIT else f"INVALID (> {SPREAD_LIMIT:.0%})")
            + f"); median worker RSS sum peak {statistics.median(rss_sum) / 1024:.0f} MB.",
            "",
        ]
    if "inline" in medians and "queue" in medians and medians["inline"]:
        overhead = 1 - medians["queue"] / medians["inline"]
        lines += [
            "### Paired overhead (computed from the medians above)",
            "",
            f"Lease-queue overhead ≈ **{overhead:.1%}** median-to-median "
            f"(plan §5.6: publish the median; inline median {medians['inline']:.1f} p/s "
            f"vs queue median {medians['queue']:.1f} p/s). "
            "The paired ratio is the citable number; cross-session absolute "
            "comparisons remain invalid by the plan's rules.",
            "",
        ]
    lines += honest_notes(DISTRIBUTED_RSS)
    return lines


# --------------------------------------------------------------------------
# single-shot smoke record
# --------------------------------------------------------------------------


def render_smoke(records: list[dict]) -> list[str]:
    r = records[0]
    res, reso, gates = r["results"], r["resources"], r["gates"]
    config = r["config"]
    lines = [
        f"**Workload:** `{config['pages']}` pages · storage `{config['storage']}` · "
        f"concurrency {config['concurrency']} · delay {config['request_delay_ms']} ms · "
        f"mode `{config['mode']}` · build profile `{config.get('profile', 'debug (CI smoke)')}`",
        f"**Engine:** crawlkit {r['crawlkit_version']}",
        f"**Machine:** {machine_line(r)}",
        "",
        "| Metric | Value | Target | Verdict |",
        "|---|---|---|---|",
        f"| Throughput | **{res['throughput_pages_per_sec']:.1f} pages/s** | ≥ 20 (CI smoke floor) | "
        + ("✅ met |" if res["throughput_pages_per_sec"] >= 20 else "❌ missed |"),
        f"| Peak RSS | **{reso['rss_peak_kb'] / 1024:.0f} MB** | < 500 MB | "
        + ("✅ met |" if reso["rss_peak_kb"] < RSS_TARGET_KB else "❌ missed |"),
        f"| Pages | {res['pages_crawled']} | exact budget | "
        + ("✅ |" if gates.get("pages_exact", True) else "❌ |"),
        f"| fds returned to baseline | {reso['fd_end']} (baseline {reso['fd_baseline']}) | +10 after settle | "
        + ("✅ |" if gates["fds_returned_to_baseline"] else "❌ |"),
        "",
        "Single-shot CI smoke record (relative-gate baseline candidate); "
        "absolute numbers from CI are never published (plan §6).",
    ]
    lines += honest_notes(INLINE_RSS_MET if reso["rss_peak_kb"] < RSS_TARGET_KB else INLINE_RSS_MISSED)
    return lines


# --------------------------------------------------------------------------
# dispatch
# --------------------------------------------------------------------------


def render_numbers(directory: Path) -> str | None:
    paths = sorted(directory.glob("run_record*.json"))
    if not paths:
        return None
    records = [load(p) for p in paths]

    paired: dict[str, list[dict]] = {}
    rest: list[tuple[Path, dict]] = []
    for p, r in zip(paths, records):
        m = PAIRED_RUN_RE.match(p.name)
        if m:
            paired.setdefault(m.group(1), []).append(r)
        else:
            rest.append((p, r))

    body: list[str] | None = None
    if paired:
        body = render_paired(paired)
        if rest:
            body += ["---", ""]
    if body is None or rest:
        inline_v1 = [r for _, r in rest if r["schema"] == "crawlkit.capacity.run_record/v1"]
        distributed = [r for _, r in rest if r["schema"] == "crawlkit.capacity.distributed_run_record/v1"]
        if rest and not inline_v1 and not distributed:
            sys.exit(
                f"unrecognized record schema(s) in {directory}: "
                f"{sorted({r['schema'] for _, r in rest})}"
            )
        sections: list[str] = []
        if inline_v1:
            if len(inline_v1) == 1 and not distributed:
                sections += render_smoke(inline_v1)
            else:
                sections += render_inline(inline_v1)
        if distributed:
            by_workload: dict[str, list[dict]] = {}
            for r in distributed:
                by_workload.setdefault(r["workload"], []).append(r)
            for workload in sorted(by_workload):
                if sections:
                    sections += ["---", ""]
                sections += render_distributed_group(by_workload[workload], workload)
        body = (body or []) + sections
    return "\n".join(body)


def marked(fresh: str) -> str:
    return f"{BEGIN}\n\n{fresh}\n{END}\n"


def update() -> int:
    changed = 0
    for directory in sorted(CAPACITY.iterdir()):
        if not directory.is_dir() or not sorted(directory.glob("run_record*.json")):
            continue
        fresh = render_numbers(directory)
        assert fresh is not None
        report = directory / "REPORT.md"
        if not report.exists():
            content = f"# Capacity Report — {directory.name}\n\n{marked(fresh)}"
            action = "created"
        else:
            content = report.read_text()
            if BEGIN in content:
                content = re.sub(
                    re.escape(BEGIN) + r".*?" + re.escape(END),
                    marked(fresh).rstrip("\n"),
                    content,
                    flags=re.DOTALL,
                )
                action = "updated marked section"
            else:
                content = content.rstrip("\n") + "\n\n" + marked(fresh)
                action = "appended marked section"
        report.write_text(content)
        print(f"{directory.name}: {action}")
        changed += 1
    print(f"{changed} report(s) processed.")
    return 0


def check() -> int:
    failures: list[str] = []
    checked = 0
    for directory in sorted(CAPACITY.iterdir()):
        if not directory.is_dir() or not sorted(directory.glob("run_record*.json")):
            continue
        checked += 1
        fresh = render_numbers(directory)
        assert fresh is not None
        report = directory / "REPORT.md"
        if not report.exists():
            failures.append(
                f"{directory.name}: REPORT.md missing — run "
                "`python3 scripts/render_capacity_report.py --update` and commit"
            )
            continue
        content = report.read_text()
        m = re.search(re.escape(BEGIN) + r"\n\n(.*?)\n" + re.escape(END), content, re.DOTALL)
        if not m:
            failures.append(
                f"{directory.name}: REPORT.md has no {BEGIN} section — run "
                "`python3 scripts/render_capacity_report.py --update` and commit"
            )
        elif m.group(1) != fresh:
            failures.append(
                f"{directory.name}: generated numbers drift from the committed "
                "records — run `python3 scripts/render_capacity_report.py --update` "
                "and commit"
            )
    for f in failures:
        print(f"FAIL: {f}", file=sys.stderr)
    if not failures:
        print(f"Capacity report drift check passed ({checked} report(s) verified).")
    return 1 if failures else 0


def main() -> None:
    argv = sys.argv[1:]
    if "--check" in argv:
        sys.exit(check())
    if "--update" in argv:
        sys.exit(update())
    if len(argv) != 2:
        sys.exit("usage: render_capacity_report.py <docs/capacity/dir> | --update | --check")
    directory = Path(argv[1])
    fresh = render_numbers(directory)
    if fresh is None:
        sys.exit(f"no run_record*.json files in {directory}")
    print(fresh)


if __name__ == "__main__":
    main()
