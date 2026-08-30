# Benchmark protocol

Performance claims are valid only when accompanied by the following metadata:

- commit SHA and Rust/toolchain version;
- host CPU, memory, operating system, and governor/container limits;
- build profile and enabled Cargo features;
- workload seed, URL count, depth, response sizes, status distribution, and
  analyzer/plugin configuration;
- concurrency, rate limits, storage backend, and database configuration;
- warm-up policy and number of measured repetitions;
- throughput, latency percentiles, peak memory, error rate, and completed pages;
- raw result artifact and command used to produce it.

## Required capacity scenarios

1. parser-only synthetic pages;
2. fetch pipeline with a deterministic local fixture server;
3. SQLite persistence with bounded concurrency;
4. analyzer-heavy workload;
5. plugin-enabled workload, separately from the core baseline.

Results must not be compared across different workloads or environments.
Regression thresholds should be checked against a stored baseline and should
fail CI only after the baseline has been reviewed and versioned.

Example metadata command:

```bash
cargo bench --workspace -- --output-format bencher
```

The command alone is not evidence; retain the environment and workload
metadata with its output.
