#!/usr/bin/env python3
"""Aggregate benchmark results and compute Solvra Efficiency Index (SEI).

Usage: python compare.py summary.json summary_report.md
"""
from __future__ import annotations

import json
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from statistics import mean

@dataclass
class Metric:
    benchmark: str
    language: str
    execution_ms: float
    memory_mb: float


def load_metrics(path: Path) -> list[Metric]:
    data = json.loads(path.read_text(encoding="utf-8"))
    metrics: list[Metric] = []
    for entry in data:
        metrics.append(
            Metric(
                benchmark=entry.get("benchmark", "unknown"),
                language=entry.get("language", "unknown"),
                execution_ms=float(entry.get("execution_time_ms", 0.0)),
                memory_mb=float(entry.get("memory_usage_mb", 0.0)),
            )
        )
    return metrics


def compute_native_summaries(items: list[Metric]) -> tuple[float, float]:
    native_times = [m.execution_ms for m in items if m.language in {"c", "cpp", "rust"}]
    native_mem = [m.memory_mb for m in items if m.language in {"c", "cpp", "rust"}]
    native_time = mean(native_times) if native_times else 1.0
    native_memory = mean(native_mem) if native_mem else 1.0
    return native_time or 1.0, native_memory or 1.0


def compute_sei_and_rating(metrics: list[Metric]) -> dict[str, tuple[float, int]]:
    grouped: dict[str, list[Metric]] = defaultdict(list)
    for metric in metrics:
        grouped[metric.benchmark].append(metric)

    results: dict[str, tuple[float, int]] = {}
    for bench, items in grouped.items():
        native_time, native_memory = compute_native_summaries(items)
        solvra_items = [m for m in items if m.language == "solvrascript"]
        if not solvra_items:
            continue
        solvra_time = mean(m.execution_ms for m in solvra_items)
        solvra_mem = mean(m.memory_mb for m in solvra_items)
        if solvra_time <= 0:
            solvra_time = native_time
        if solvra_mem <= 0:
            solvra_mem = native_memory

        speed_ratio = solvra_time / native_time if native_time else 1.0
        memory_ratio = solvra_mem / native_memory if native_memory else 1.0
        sei = (native_time / solvra_time) * (native_memory / solvra_mem)
        rating = compute_rating(speed_ratio, memory_ratio)
        results[bench] = (sei, rating)
    return results


def compute_rating(speed_ratio: float, memory_ratio: float) -> int:
    if speed_ratio > 2.0 or memory_ratio > 2.0:
        return 1
    if speed_ratio > 1.5 or memory_ratio > 1.8:
        return 2
    if speed_ratio > 1.3 or memory_ratio > 1.6:
        return 3
    if speed_ratio > 1.1 or memory_ratio > 1.4:
        return 4
    return 5


def append_report(report: Path, results: dict[str, tuple[float, int]]) -> None:
    lines = report.read_text(encoding="utf-8").splitlines()
    lines.append("")
    lines.append("## Solvra Efficiency Index (SEI)")
    lines.append("")
    lines.append("| Benchmark | SEI | Rating | Interpretation |")
    lines.append("|-----------|-----|--------|----------------|")
    for bench, (score, rating) in sorted(results.items()):
        interpretation = "ok" if rating >= 4 else "needs review"
        lines.append(f"| {bench} | {score:.2f} | {rating} | {interpretation} |")
    report.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("Usage: compare.py summary.json summary_report.md", file=sys.stderr)
        return 2
    summary = Path(argv[1])
    report = Path(argv[2])

    if not summary.exists() or not report.exists():
        print("summary or report missing", file=sys.stderr)
        return 1

    metrics = load_metrics(summary)
    results = compute_sei_and_rating(metrics)
    append_report(report, results)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
