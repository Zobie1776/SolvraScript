#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RESULTS="$ROOT/results"
LOG_DIR="$RESULTS/logs"
mkdir -p "$RESULTS" "$LOG_DIR"
SUMMARY_JSON="$RESULTS/summary.json"
SUMMARY_MD="$RESULTS/summary_report.md"
: > "$SUMMARY_JSON"
: > "$SUMMARY_MD"

LANGUAGES=(c cpp rust python java csharp javascript typescript solvrascript)
ALL_FILES=()

for lang in "${LANGUAGES[@]}"; do
  BUILD="$ROOT/$lang/build.sh"
  if [[ -x "$BUILD" ]]; then
    echo "Running benchmarks for $lang" | tee -a "$LOG_DIR/run_all.log"
    (cd "$ROOT/$lang" && ./build.sh)
    while IFS= read -r -d '' file; do
      ALL_FILES+=("$file")
    done < <(find "$ROOT/$lang/results" -name "*.json" -print0)
  else
    echo "Skipping $lang (build.sh missing or not executable)" | tee -a "$LOG_DIR/run_all.log"
  fi
done

FILES_STR=$(printf "%s\n" "${ALL_FILES[@]}" )
BENCH_ROOT="$ROOT" BENCH_FILES="$FILES_STR" python3 - <<'PY'
import json
import os
from datetime import datetime, timezone
root = os.environ["BENCH_ROOT"]
results_dir = os.path.join(root, "results")
summary_path = os.path.join(results_dir, "summary.json")
report_path = os.path.join(results_dir, "summary_report.md")
files = os.environ.get("BENCH_FILES", "").split("\n")
metrics = []
for path in files:
    path = path.strip()
    if not path:
        continue
    try:
        with open(path, "r", encoding="utf-8") as handle:
            metrics.append(json.load(handle))
    except Exception as err:
        metrics.append({
            "benchmark": os.path.basename(path),
            "language": "unknown",
            "error": str(err),
            "timestamp": datetime.now(timezone.utc).isoformat()
        })
with open(summary_path, "w", encoding="utf-8") as handle:
    json.dump(metrics, handle, indent=2)
timestamp = datetime.now(timezone.utc).isoformat()
lines = ["# Solvra Benchmark Summary", "", f"Generated: {timestamp}", ""]
lines.append("| Benchmark | Language | Exec ms | Memory MB | Notes |")
lines.append("|-----------|----------|---------|-----------|-------|")
for entry in metrics:
    bench = entry.get("benchmark", "n/a")
    lang = entry.get("language", "n/a")
    exec_ms = entry.get("execution_time_ms", 0)
    mem = entry.get("memory_usage_mb", 0)
    note = entry.get("error", "stub")
    lines.append(f"| {bench} | {lang} | {exec_ms} | {mem} | {note} |")
with open(report_path, "w", encoding="utf-8") as handle:
    handle.write("\n".join(lines))
PY

python3 "$ROOT/compare.py" "$SUMMARY_JSON" "$RESULTS/summary_report.md"

echo "Benchmark run complete."
