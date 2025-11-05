# Solvra Benchmark Suite

This directory hosts the cross-language benchmarking harness for SolvraScript.
The current scaffold targets C, C++, Rust, Python, Java, C#, JavaScript,
TypeScript, and SolvraScript itself. Each language directory contains stub
scripts and build runners that can be replaced with production-grade
implementations.

## Layout
- `bench_config.toml` — global thresholds and benchmark list.
- `<language>/build.sh` — entrypoint invoked by `run_all.sh` to compile/run tests.
- `<language>/results/` — per-benchmark JSON metrics and logs.
- `run_all.sh` — orchestrates all languages, aggregates metrics, and produces a
  Markdown summary plus JSON dataset.
- `compare.py` — merges metrics, computes Solvra Efficiency Index (SEI), and
  assigns 1–5 ratings.

Run the entire suite with:

```bash
cd solvra_script/benchmarks
./run_all.sh
```

The run produces `results/summary.json` and `results/summary_report.md`, which
can be consumed by dashboards or CI reporters.
