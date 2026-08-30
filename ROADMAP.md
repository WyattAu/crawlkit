# crawlkit Engineering Roadmap

**Status:** Living implementation contract
**Version baseline:** 4.4.1
**Roadmap owner:** Maintainers
**Last reviewed:** 2026-08-30

This roadmap is intentionally evidence-driven. A feature is not considered complete because code exists or a document describes it. It is complete only when the implementation, tests, operational behavior, documentation, and release evidence all agree.

---

## 1. Product scope

crawlkit is a Rust web crawler and website-analysis platform with:

- asynchronous HTTP crawling;
- robots.txt and sitemap handling;
- SEO, content, schema, security, accessibility and performance analysis;
- deterministic crawl/report output when configured;
- SQLite and PostgreSQL storage paths;
- REST API, CLI, dashboard and client libraries;
- optional JavaScript rendering;
- signed, sandboxed WASM analyzers;
- exports, comparisons, monitoring and integrations.

### Explicit non-goals

The project must not claim to be:

- a high-frequency trading system;
- a kernel-bypass or microsecond-latency system;
- formally verified;
- defence-certified;
- a replacement for a full SIEM, vulnerability scanner, search engine or marketing platform.

HFT, FAANG, ECN and defence terminology is used only as a comparison against engineering practices, never as a certification claim.

---

## 2. Current verified baseline

The following baseline was verified on 2026-08-30:

| Area | Verified state |
|---|---|
| Rust workspace | Builds and type-checks through Clippy compilation paths |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` passes |
| Engine unit tests | 3,819 passed, 1 ignored, using one test thread |
| Documentation tests | 45 passed |
| Selected integration tests | API, crawler, parallel pipeline, property, WASM, plugin-index, backlinks and RUM suites pass |
| Core engine | Uses semaphore-bounded `tokio::spawn` + `FuturesUnordered` |
| Storage seam | `CrawlEngine` accepts `Arc<dyn StorageBackend>` |
| SSRF | Shared engine SSRF module is used by API/plugin paths observed in current source |
| Plugin isolation | WASM fuel, memory, capability and ABI tests exist |
| CI | Format, Clippy, unit/doc/integration tests, coverage, dependency checks, build and release workflows exist |
| Main risks | Documentation drift, unclear feature counts, unsupported performance claims, large modules, optional infrastructure maturity, unsafe FFI wording |

### Baseline rules

Every future roadmap update must include:

1. date and toolchain;
2. commands run;
3. pass/fail result;
4. changed metrics;
5. known limitations;
6. links to implementation and tests.

---

## 3. Definition of done

No roadmap item may be marked `DONE` unless all applicable gates pass.

### Code gate

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- no new compiler or lint suppressions without an ADR;
- no new production `unwrap`, `expect`, `panic`, `unreachable` or `unsafe` without documented justification;
- public APIs have rustdoc;
- errors preserve actionable context;
- cancellation and resource limits are defined.

### Test gate

- unit tests for normal and boundary behavior;
- negative/error-path tests;
- integration or contract tests at every external boundary;
- property tests for parsers, URL handling, queues, limits and serialization where useful;
- concurrency tests for shared state;
- regression test for every fixed defect;
- no test relies on an unavailable external service unless explicitly service-backed and isolated.

### Security gate

- threat model updated when trust boundaries change;
- SSRF, redirect, DNS rebinding and resource-limit behavior tested;
- dependency audit passes or has a reviewed exception;
- secrets are absent from source, logs, fixtures and artifacts;
- unsafe code has a local safety invariant and tests;
- plugin and external-input paths fail closed.

### Documentation gate

- docs describe current behavior, not intended behavior;
- every numeric claim has methodology, environment, sample size and raw result;
- unsupported features are labelled `planned` or `experimental`;
- CLI/API examples execute in CI where practical;
- version, MSRV, test count and analyzer/check count are generated or verified automatically.

### Release gate

- reproducible release build or documented variance;
- release artifact checksums;
- SBOM or dependency inventory;
- migration/rollback procedure;
- upgrade notes;
- smoke test against the release artifact;
- no unresolved P0/P1 issue.

---

## 4. Priority definitions

| Priority | Meaning | Maximum acceptable state |
|---|---|---|
| P0 | Security, data integrity, release-blocking correctness or misleading public claim | Must be resolved before the next release |
| P1 | Significant architecture, reliability, operability or test gap | Must have an owner and target release |
| P2 | Maintainability, performance or usability improvement | Schedule after P0/P1 work |
| P3 | Optional enhancement | Only pursue with clear user value |

---

# Phase 0 — Truth baseline and release freeze

**Target:** 4.5.0 readiness
**Priority:** P0
**Exit condition:** Repository claims are internally consistent and all release-critical behavior is classified.

## 0.1 Establish a generated project manifest

Create one machine-readable manifest containing:

- version;
- MSRV;
- supported platforms;
- registered built-in analyzers;
- analyzer checks versus analyzer components;
- test counts by category;
- supported CLI commands;
- optional features;
- benchmark names and baseline metadata;
- experimental features;
- unsafe-code inventory.

Generate README badges/tables from the manifest or validate them in CI.

**Acceptance criteria**

- A CI job fails when README counts disagree with code-derived counts.
- “Analyzer,” “check,” “plugin,” and “integration” are defined separately.
- No manually maintained test-count badge remains.

## 0.2 Reconcile all documentation

Review and classify every architecture and performance document:

- current and verified;
- current but optional;
- experimental;
- planned;
- historical/archive;
- unsupported and removable.

Correct stale claims including:

- Rust/MSRV versions;
- CLI commands;
- bind addresses;
- actor/channel architecture;
- Bloom-filter claims;
- mock implementation claims;
- OpenTelemetry scope;
- SSL certificate validation status;
- encryption status;
- unsafe-code wording;
- test/analyzer counts.

**Acceptance criteria**

- One current architecture document matches source.
- Archived documents are clearly marked and excluded from current claims.
- `cargo run -- --help` is compared against documented CLI commands in CI.

## 0.3 Establish a claims policy

Every public quantitative or capability claim must include:

```text
Claim owner
Measurement date
Version/commit
Hardware/OS/toolchain
Workload and configuration
Sample size
Statistic or confidence interval
Raw artifact location
Known exclusions
```

Unsupported competitor numbers must be removed or explicitly labelled as third-party estimates.

---

# Phase 1 — Security and trust-boundary hardening

**Target:** 4.5.x
**Priority:** P0/P1
**Exit condition:** Security claims are scoped, tested and operationally true.

## 1.1 Unsafe-code inventory and policy

Inventory all unsafe blocks and FFI boundaries, especially:

- plugin SDK allocator/export ABI;
- host ABI calls;
- native plugin loading;
- any transitive FFI boundary.

For every unsafe block document:

- ownership and lifetime invariant;
- pointer validity invariant;
- alignment and size invariant;
- panic/exception boundary;
- allocator/deallocator pairing;
- malformed-input behavior;
- test covering the invariant.

**Acceptance criteria**

- Public documentation says “unsafe denied in core crates; scoped unsafe FFI exists where required,” or equivalent.
- No document says “zero unsafe code.”
- Unsafe-code CI checks every exception explicitly rather than silently excluding files.
- Fuzz/property tests cover malformed ABI inputs.

## 1.2 SSRF and network policy conformance

Maintain one canonical policy for:

- URL schemes;
- localhost and metadata endpoints;
- IPv4 private, loopback, link-local, multicast and reserved ranges;
- IPv6 equivalents;
- DNS rebinding;
- redirect revalidation;
- proxy behavior;
- plugin host fetches;
- webhook/OIDC/external integration targets.

**Acceptance criteria**

- API, crawler, plugin and webhook paths use shared policy code.
- Every redirect hop is revalidated.
- DNS resolution is checked immediately before connection where feasible.
- Tests cover alternate textual IP forms, IPv6, encoded hosts, redirects and rebinding scenarios.
- Security policy changes require an ADR or security review.

## 1.3 TLS and certificate analysis

Choose one of two explicit outcomes:

- wire live certificate metadata from the TLS client into `SslCertificateValidator`; or
- remove/disable the analyzer from the default registry and label it planned.

**Acceptance criteria**

- A local TLS integration test proves certificate data reaches the analyzer.
- Expired, hostname-mismatch, incomplete-chain and weak-certificate cases are covered.
- The README reflects the actual scope: transport validation versus certificate analysis.

## 1.4 Secrets and authentication review

Audit:

- JWT algorithm and key management;
- key rotation;
- session revocation;
- password reset and lockout behavior;
- API-key storage and display;
- OIDC issuer/audience/nonce/PKCE validation;
- webhook secret handling;
- logs and error responses.

**Acceptance criteria**

- Algorithm choice is documented accurately.
- Secret rotation is tested without invalidating unrelated tenants unexpectedly.
- No secrets appear in normal logs, errors or exports.
- Security-sensitive configuration has safe defaults and explicit production warnings.

## 1.5 Plugin security model

Document and test:

- trust-store lifecycle;
- signing key rotation/revocation;
- artifact hash verification;
- capability grants;
- memory/fuel/epoch limits;
- host ABI validation;
- network and filesystem denial;
- plugin failure isolation;
- plugin result size limits.

**Acceptance criteria**

- Tampered, unsigned, revoked and malformed plugins fail closed.
- A runaway plugin cannot stall the main runtime.
- Plugin failures are observable and attributable without aborting unrelated pages.
- Release documentation distinguishes WASM support from native plugin support.

---

# Phase 2 — Architectural simplification and boundary quality

**Target:** 4.6.0
**Priority:** P1
**Exit condition:** Production architecture contains only supported paths, with clear ownership and contracts.

## 2.1 Split high-responsibility modules

Refactor without changing behavior:

```text
crawl_engine/
  orchestration.rs
  dispatch.rs
  lifecycle.rs
  enrichment.rs
  monitoring.rs
  resource_limits.rs

plugin/
  manifest.rs
  trust.rs
  verification.rs
  sandbox.rs
  network_policy.rs
  registry.rs
  runtime.rs
```

Split analyzer files by cohesive domain where file size impairs review or testing.

**Acceptance criteria**

- No module combines more than one major security or lifecycle responsibility without an explicit rationale.
- New modules have focused tests.
- Public API remains stable unless an ADR approves a breaking change.
- Refactor does not increase duplicated logic or suppressions.

## 2.2 Storage contract and backend parity

Define a conformance suite for `StorageBackend` covering:

- lifecycle;
- pages and issues;
- tenant isolation;
- conditional fetch data;
- links;
- comparisons;
- purge behavior;
- transaction/error semantics;
- ordering and pagination.

Run the same contract suite against SQLite and PostgreSQL.

**Acceptance criteria**

- Both backends pass required contract tests.
- Unsupported operations return explicit errors, not empty successful results.
- Backend capabilities are documented.
- Crawl engine behavior is backend-independent for supported operations.

## 2.3 Queue abstraction and distributed mode decision

Decide whether Redis/distributed queue is:

1. production-supported;
2. experimental feature-gated;
3. removed until needed.

If production-supported:

- define queue delivery semantics;
- document at-least-once/exactly-once boundaries;
- handle leases, retries, poison entries and worker loss;
- test duplicate and crash recovery.

**Acceptance criteria**

- No documentation calls Redis “wired” unless a real crawl uses it.
- Queue behavior is tested under worker interruption and duplicate delivery.
- Memory queue and distributed queue share a documented behavioral contract.

## 2.4 Backpressure and resource budgets

Formalize limits for:

- concurrent fetches;
- queued URLs;
- page body bytes;
- total crawl bytes;
- findings per page/crawl;
- extracted links;
- plugin memory/fuel/time;
- storage batch size;
- API crawl submissions;
- per-tenant concurrency and rate.

**Acceptance criteria**

- Every limit has a default, configuration source, enforcement location and metric.
- Limit exhaustion produces a typed reason.
- No unbounded production queue or collection remains undocumented.
- Backpressure tests assert bounded memory/queue behavior.

## 2.5 Async runtime hygiene

Audit all async functions for:

- synchronous database calls;
- filesystem calls;
- subprocess execution;
- CPU-heavy parsing or plugin execution;
- blocking mutexes held across await points.

Move work to `spawn_blocking` or a bounded dedicated pool where appropriate.

**Acceptance criteria**

- No known blocking database or subprocess operations run on Tokio worker threads.
- A runtime-stall test or instrumentation confirms event-loop responsiveness under representative load.
- Documentation states which operations are intentionally blocking.

---

# Phase 3 — Correctness and analyzer quality

**Target:** 4.6.x
**Priority:** P1
**Exit condition:** Analyzer outputs are reproducible, semantically specified and regression-protected.

## 3.1 Analyzer contract

Specify the analyzer contract:

- input completeness;
- whether missing data means “not applicable” or “finding”;
- severity semantics;
- finding identity and deduplication;
- ordering guarantees;
- error behavior;
- plugin compatibility;
- versioning.

**Acceptance criteria**

- Contract tests run against representative built-in analyzers and plugins.
- Findings are deterministically ordered where deterministic mode is enabled.
- Analyzer count and check count are separate metrics.

## 3.2 SEO analyzer mutation-survivor reduction

Use mutation testing and targeted fixtures to close surviving mutants, prioritizing:

- boundary conditions;
- empty/malformed structured data;
- locale and URL validation;
- canonical/hreflang logic;
- severity and finding cardinality;
- duplicate and conflicting metadata.

**Acceptance criteria**

- Every surviving mutant is either killed or documented as equivalent/unreachable.
- No blanket weakening of mutation configuration is used to improve the score.
- Critical analyzer branches have explicit expected-output fixtures.

## 3.3 Parser and hostile-input robustness

Expand fuzz/property tests for:

- malformed HTML;
- oversized tags/attributes;
- invalid encodings;
- deeply nested DOMs;
- pathological URLs;
- malformed robots/sitemap XML;
- JSON-LD arrays and recursive structures;
- enormous link sets.

**Acceptance criteria**

- Fuzz targets have a documented run budget and corpus policy.
- No panic, unbounded memory growth or excessive runtime for bounded inputs.
- Parser limits are explicit and observable.

## 3.4 CLI/API contract tests

Add tests for every documented user-facing command and major API endpoint:

- help and invalid arguments;
- crawl lifecycle;
- output formats;
- comparison and report generation;
- authentication failures;
- tenant isolation;
- pagination and filters;
- idempotency;
- retryable versus terminal errors.

**Acceptance criteria**

- CLI help output is tested against documented commands.
- API OpenAPI output is generated and checked.
- Error status codes and response schemas are stable and documented.

---

# Phase 4 — Performance, capacity and reproducibility

**Target:** 4.7.0
**Priority:** P1/P2
**Exit condition:** Performance claims are reproducible, workload-specific and regression-tested.

## 4.1 Benchmark taxonomy

Maintain separate benchmark classes:

1. parser microbenchmarks;
2. analyzer microbenchmarks;
3. storage microbenchmarks;
4. local end-to-end crawler benchmarks;
5. network-like benchmarks with controlled latency/body sizes;
6. API throughput/latency benchmarks;
7. plugin overhead benchmarks;
8. memory/capacity tests.

Never combine these into one “pages/sec” claim.

## 4.2 Reproducible benchmark harness

Record:

- commit and version;
- compiler/toolchain;
- CPU, RAM, OS and filesystem;
- build profile;
- test-server topology;
- page count and body size;
- latency distribution;
- concurrency and rate limits;
- analyzer/plugin configuration;
- warmup and repetitions;
- median, p95 and p99;
- raw Criterion or benchmark output.

**Acceptance criteria**

- Performance documentation links to committed or CI-retained raw artifacts.
- README contains only a conservative representative result.
- Any regression threshold is based on repeated measurements and noise analysis.
- No competitor performance comparison uses a different workload without stating so.

## 4.3 Capacity and soak testing

Test:

- 10k and 100k URL crawls;
- large response bodies;
- slow and failing origins;
- high-cardinality domains;
- long-running rate-limit maps;
- plugin-heavy crawls;
- repeated API submissions;
- storage growth and purge.

**Acceptance criteria**

- Memory, file descriptors, task count and queue size remain bounded.
- Failure recovery does not leak permits, tasks or storage locks.
- Capacity results include confidence limits and known bottlenecks.

## 4.4 Runtime profiling

Use flamegraphs/allocation profiling to identify real hotspots before optimization.

**Acceptance criteria**

- No HFT terminology is used for ordinary batch optimizations.
- Optimizations include before/after benchmark evidence.
- Complexity or allocation reductions do not weaken correctness or limits.

---

# Phase 5 — Observability and operations

**Target:** 4.7.x
**Priority:** P1
**Exit condition:** Operators can diagnose, limit, recover and audit production behavior.

## 5.1 Metrics contract

Define stable metrics for:

- crawl submissions/completions/failures;
- pages fetched/stored/skipped;
- status classes;
- retries and circuit-breaker transitions;
- queue depth and wait time;
- rate-limit waits;
- parser/analyzer/plugin durations;
- storage latency/errors;
- bytes read/written;
- resource-limit terminations;
- tenant/API rate limits.

Avoid high-cardinality labels such as raw URLs unless explicitly bounded.

## 5.2 Tracing contract

Document:

- span names;
- correlation IDs;
- tenant/request/crawl identifiers;
- sensitive-field redaction;
- exporter behavior;
- sampling;
- shutdown flushing.

**Acceptance criteria**

- “OpenTelemetry support” specifies whether this means instrumentation, SDK, exporter or production deployment.
- A local collector integration test or documented smoke test proves export when enabled.

## 5.3 Operational runbooks

Create runbooks for:

- database unavailable;
- Redis unavailable, if supported;
- stuck crawl;
- memory limit reached;
- plugin failure;
- signing-key compromise;
- JWT secret rotation;
- migration failure;
- high error rate;
- SSRF/security alert;
- corrupted crawl data.

## 5.4 Graceful shutdown and recovery

Define behavior for:

- SIGTERM/cancellation;
- in-flight fetches;
- plugin execution;
- storage transactions;
- partial crawl completion;
- resumability versus restart;
- idempotent retries.

**Acceptance criteria**

- Shutdown tests prove permits/tasks/connections are released.
- Crawl state is not reported as completed when finalization failed.
- Recovery behavior is documented and observable.

---

# Phase 6 — Release engineering and supply-chain assurance

**Target:** 5.0.0
**Priority:** P1
**Exit condition:** Releases are reproducible enough to trust, verify and roll back.

## 6.1 CI consistency

Unify all versions and commands across:

- `Cargo.toml`;
- CI workflows;
- `justfile`;
- pre-commit/pre-push scripts;
- Dockerfiles;
- CONTRIBUTING documentation;
- release metadata.

**Acceptance criteria**

- One MSRV value is generated or checked everywhere.
- CI and local gate commands are equivalent.
- Resource-constrained test jobs use deliberate parallelism.
- Build job dependencies include all required test/security jobs.

## 6.2 Reproducible release artifacts

Produce:

- platform-specific binaries;
- checksums;
- SBOM/dependency inventory;
- source archive;
- provenance/attestation where supported;
- release smoke-test report;
- migration compatibility statement.

## 6.3 Upgrade and rollback

Document and test:

- schema migrations;
- backup/restore;
- downgrade limitations;
- configuration compatibility;
- API version compatibility;
- plugin ABI compatibility;
- release rollback.

**Acceptance criteria**

- A clean environment can verify and run a release artifact.
- A failed migration has a tested recovery procedure.
- Plugin ABI compatibility is versioned and tested.

---

# Phase 7 — Ecosystem and product completeness

**Target:** after 5.0.0
**Priority:** P2/P3
**Exit condition:** Only pursue features with demonstrated user value and maintained ownership.

Candidate work:

- client-library parity;
- scheduled crawls;
- continuous monitoring;
- GSC/CrUX integrations;
- custom extraction;
- log analysis;
- visual crawl maps;
- richer dashboard workflows;
- formal queue/circuit-breaker models;
- additional plugin marketplace capabilities.

Every candidate must include:

- user problem;
- supported lifecycle;
- security model;
- operational cost;
- API/CLI contract;
- test plan;
- maintenance owner;
- deprecation plan.

No candidate becomes “implemented” merely because a module or type exists.

---

# 5. Release gates

## 4.5.0 gate — Truth and security

Must have:

- corrected unsafe-code claims;
- reconciled current documentation;
- generated/validated counts;
- canonical SSRF policy;
- SSL analyzer disposition;
- CI consistency audit;
- no unresolved P0 issue;
- complete security claim inventory.

## 4.6.0 gate — Architecture and correctness

Must have:

- storage backend conformance suite;
- queue/distributed-mode decision;
- blocking-operation audit;
- resource-limit contract;
- CLI/API contract tests;
- analyzer contract;
- plugin security contract.

## 4.7.0 gate — Performance and operations

Must have:

- reproducible benchmark harness;
- raw benchmark artifacts;
- capacity/soak baseline;
- metrics and tracing contract;
- operational runbooks;
- graceful shutdown/recovery tests.

## 5.0.0 gate — Trustworthy release

Must have:

- all prior gates complete;
- no critical/high unresolved security issue;
- release artifact smoke tests;
- checksums and dependency inventory;
- migration/rollback evidence;
- API/CLI documentation generated or tested;
- public claims reviewed against the manifest;
- explicit statement that FAANG/HFT/ECN/defence comparisons are non-certification assessments.

---

# 6. Continuous controls

Run on every change:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins
cargo test --doc --workspace
relevant integration tests
security/dependency checks
claim/documentation consistency checks
```

Run nightly or scheduled:

```text
full integration suite
service-backed PostgreSQL/Redis tests, if supported
fuzzing budget
mutation testing on changed critical modules
benchmark regression suite
large-scale capacity tests
dependency/license/SBOM refresh
```

Run before release:

```text
MSRV build/test
cross-platform builds
release artifact smoke tests
migration upgrade/rollback tests
secret and provenance checks
manual security and claims review
```

---

# 7. Decision log requirements

An ADR is required for:

- public API or ABI changes;
- storage schema changes;
- new trust boundaries;
- new unsafe code;
- new external service or credential;
- new persistence or queue semantics;
- claims of determinism, exactly-once behavior or formal assurance;
- new “enterprise,” “defence,” “HFT” or compliance wording;
- removing or materially changing a supported feature.

Each ADR must state:

- context;
- alternatives;
- decision;
- security impact;
- performance impact;
- operational impact;
- testing strategy;
- migration/rollback plan;
- documentation changes;
- exit/deprecation criteria.

---

## Final roadmap principle

The project should optimize for **truthful capability, bounded behavior, simple production paths and reproducible evidence**. A smaller set of fully verified features is preferable to a larger set of partially wired modules and stronger-sounding claims.
