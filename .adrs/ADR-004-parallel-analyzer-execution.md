# ADR-004: Parallel Analyzer Execution

## Status
Accepted

## Context
Sequential analyzer execution is a bottleneck for large crawls.

## Decision
Use Rayon for parallel analyzer execution.

## Consequences
### Positive
- 2-4x throughput improvement for analyzer pipeline
- Automatic work-stealing across CPU cores
- Minimal code changes (iter() -> par_iter())

### Negative
- Slight overhead for small workloads
- Requires Send + Sync on Analyzer trait

### Risks
- Thread contention on shared state (mitigated by atomic operations)
- Debugging complexity (mitigated by structured logging)

## References
- Rayon documentation: https://docs.rs/rayon/
