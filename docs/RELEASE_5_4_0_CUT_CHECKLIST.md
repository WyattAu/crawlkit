# 5.4.0 Stable — Cut Checklist

**Status:** EXECUTED 2026-09-12 — v5.4.0 tagged, published stable (18
assets, byte-exact against signed checksums), CI green. Retained as the
reference procedure for the next stable cut; the version-bearing items
below record what was done, and §D remains the live scheduling table.

**Purpose:** the executable runbook for cutting 5.4.0 stable, written while
the remaining work is fresh. Every item names its evidence location; nothing
here requires rediscovery. Follow in order; do not skip the gate rows.

**Baseline:** v5.4.0-alpha.1 is published (18 assets, deterministic-SBOM
pipeline, signed checksums). All ADR-015 code obligations are implemented
and CI-proven. The GA decision is the only open product question.

---

## Gate 0 — the GA decision (human; blocks item C only)

**Resolved at cut time:** the CI-streak item is MET (5 of 5 consecutive
green runs, counted per the runbook rule) and the drills are recorded;
**§7 remains unsigned**, so the not-signed branch was taken —
`hosted_scanner` stays `prototype` and §A was skipped. The deferral is
framed in docs/RELEASE_5_4_0_ALPHA_1.md §"What's next".

- [x] §6 checklist human items complete (except the signature):
  - [x] N consecutive green CI runs observed since the evidence suites
        landed — **N = 5**, with the counting rule and live streak
        recorded in the runbook checklist item itself (docs/SCANNER_RUNBOOK.md §6);
        they accumulate automatically, no action needed beyond checking
        **(met: 5/5 verified 2026-09-12)**
  - [x] §6.1 drills: done 2026-09-12, recorded in the runbook
  - [ ] §7 ownership line signed (docs/SCANNER_RUNBOOK.md §7) with the
        escalation path confirmed — **still open; the one remaining GA item**
- [ ] If signed: flip `hosted_scanner` → `stable-with-configuration` in
      docs/capabilities.toml, land public copy, include §A below.
- [x] If not signed: keep `prototype`, note the deferral in the release
      notes (docs/RELEASE_5_4_0_ALPHA_1.md §"What's next" already frames
      this), skip §A.

Decision material: docs/GA_SIGNOFF_PACKAGE.md.

## A — if scanner GA lands (skip when Gate 0 says no)

- [ ] Flip `hosted_scanner` status + notes in docs/capabilities.toml
- [ ] Public copy review against the honest-scope rule (runbook §1):
      page cap stated, "analyzes what it fetches, not what a browser
      renders", no link-graph/rank/keyword claims
- [ ] Runbook §4 dashboard row completed (see D status below)
- [ ] Release notes section: scanner GA with its exact scope and limits

## B — release engineering (executed 2026-09-12: v5.4.0 published stable,
18 assets, checksums verified, SBOM pipeline unchanged)

- [x] Version bumps: workspace `Cargo.toml` → `5.4.0` (3 places: members,
      deps, binary versions), `VERSION.md`, `docs/capabilities.toml`
      `[project].version`
- [x] CHANGELOG.md: move the 5.4.0 section from the rolling-prerelease
      entry to a dated stable entry; keep the alpha.1 history line
- [x] `cargo check -p crawlkit -p crawlkit-api -p crawlkit-scanner -p
      crawlkit-engine` (version-consistency gate)
- [x] Tag `v5.4.0` on the release commit; push triggers the Release
      workflow
- [x] Verify release is published, **not draft**, and is flagged stable
      (v5.3.0 must stop being "Latest" only when 5.4.0 is stable)
- [x] Download all assets; verify byte-exact against the signed
      `checksums.txt` (script from the 5.3.0/alpha.1 cuts)
- [ ] Verify SBOM reproducibility (re-run the workflow's SBOM step on the
      same commit; artifacts must be byte-identical)

## C — capabilities manifest audit at cut time (repeat; statuses drift)

- [ ] `cargo run -p crawlkit-engine --example manifest_drift_check -- --print`
      matches the committed `[counts]` table
- [ ] No capability's `evidence` paths 404 (CI compiles them, but the
      manifest check is path-string only)
- [ ] Statuses updated for anything that changed since this document:
      - `redis_queue` → `stable-with-configuration` (done 2026-09-12)
      - `ga4_integration`, `alert_channels` →
        `stable-with-configuration` (done 2026-09-12)
      - `hosted_scanner` → per Gate 0
- [ ] CHANGELOG cross-references match manifest statuses (a capability
      claimed "GA" in notes must not read `experimental` in the table)

## D — known open items to schedule (not 5.4.0 blockers)

| Item | Where it lands | Notes |
|---|---|---|
| Monitoring dashboards consuming `/metrics` | Post-GA sprint | Data plane shipped (metrics.rs, /metrics endpoint); dashboards scrape per-replica and aggregate |
| Alert routing integration (ADR-014 webhook delivery → scanner alerts) | 6.0.0 design | Runbook §4 alerting row |
| Insights engine surface (CLI/API) | 6.x product surface | Implemented in engine, unexposed — manifest `experimental` is accurate |
| CLI forwarding of `wasi-preview2` | Product decision + sccache in CI | docs/wasi-promotion-evaluation.md step 2; +6.2 MB binary, ~+7 min release compile |

## E — final gates (the standard release battery)

- [ ] Full workspace test battery green across feature combos
      (`cargo test --workspace`, scanner with and without
      `shared-budget`, CLI with `queue-ops`)
- [ ] Feature matrix workflow green (includes the new feature combos)
- [ ] Supply-chain / dependency audit workflows green (pnpm + cargo)
- [ ] Secret scan green
- [ ] Docs deploy green on the release commit
- [ ] Post-release: watch the first scheduled jobs (dependency audit) on
      the release day

---

**Rollback posture:** the deterministic-SBOM pipeline plus signed
checksums make every artifact reproducible; a bad cut is re-cut from the
same tag after a workflow fix with zero manual artifact surgery (proven
on the alpha.2 asset race and the 5.3.0 cut).
