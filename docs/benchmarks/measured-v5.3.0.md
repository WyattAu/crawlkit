# Measured Benchmarks — v4.4.1

**Date:** 2026-08-30
**Machine:** Linux msi-ge66 7.2.0-1-cachyos x86_64
**Toolchain:** Rust 1.97.1 (8bab26f4f 2026-07-14)
**Method:** Criterion for microbenchmarks; custom harness for throughput

## Microbenchmarks (Criterion)

| Benchmark | Value |
|-----------|-------|
| HTML parse (small page) | 151 µs |
| HTML parse (5 KB) | 610 µs |
| Analyzer full suite (200 analyzers) | 1.48 ms/page |
| URL queue push 1000 | 1.72 ms |
| URL queue pop 1000 | 774 µs |
| URL queue push/pop mixed priorities | 2.34 ms |
| Storage insert 100 pages | 1.49 ms |
| Storage query 100 pages | 724 µs |
| Storage insert+query issues | 536 µs |
| Circuit breaker state check | 1.04 ns |
| PageRank (100 nodes, 20 iterations) | 981 µs |
| Feature flags get | 43.5 ns |

## Throughput (TestServer, local HTTP/1.1)

Zero-delay pages served from a local TCP server. Concurrency = 8.

| Pages | Time | Pages/sec |
|-------|------|-----------|
| 10 | 0.04 s | 301.5 |
| 25 | 1.28 s | 20.3 |
| 50 | 4.43 s | 11.5 |

> Throughput drops at higher page counts because the test server
> serves pages serially per TCP connection. Real-world throughput
> against a concurrent web server will be higher.

## System

| Metric | Value |
|--------|-------|
| Binary size (crawlkit, release, LTO+strip) | 23 MB |
| Startup time (median, N=20) | 3.9 ms |
| Peak RSS (50 pages, in-memory storage) | 33.6 MB |
