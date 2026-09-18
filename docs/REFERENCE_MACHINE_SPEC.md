# Pinned Reference Machine — Sourcing Spec

**Status:** Open — needs procurement decision
**Date:** 2026-09-17
**Governs:** `docs/CAPACITY_EVIDENCE_PLAN.md` §3 (published capacity class)
**Purpose:** Make explicit what "the published 8-core/16 GB class" means in
practice — procurement options, acceptance checks, and the fallback rule —
so the 6.0.0 stable gate is not blocked on an undefined asset.

## 1. Requirement (from the evidence plan §3)

| Property | Requirement |
|---|---|
| CPU | 8 physical cores (SMT on or off, but recorded) |
| RAM | 16 GB |
| Disk | NVMe, ≥ 40 GB free for corpora + run records |
| OS | Linux, kernel ≥ 6.x |
| Toolchain | rustc pinned at the workspace `rust-version` (MSRV), reproducible via rustup |
| Network | Loopback-measured (no external egress in the published path) |
| Stability | Not shared with other workloads during a run; no thermal throttling mid-run (turbo off or `performance` governor preferred) |

Every published number embeds `uname`, CPU model, RAM, rustc version, and
the crawl-config hash (evidence plan §3), so the machine must be
*identifiable*, not necessarily literally one box forever.

## 2. Options, with trade-offs

1. **Bare-metal VPS / dedicated server rental** (Hetzner AX-class, OVH
   Advance, Equinix, etc.) — ~€40–80/month for 8 dedicated x86 cores / 16–32 GB.
   Best fidelity per euro; no noisy neighbors. **Recommended.**
2. **Existing workstation** (the dev box that produced the 2026-09-15/16
   records: i9-11980HK, 31 GB) — already proven, zero cost, but it is a
   laptop CPU with thermal/turbo variance and it is shared with development.
   Acceptable as an *interim* pinned machine if runs are taken with turbo
   disabled and nothing else running; must be recorded honestly in the report.
3. **Cloud dedicated host** (AWS `c6id.metal` / GCP sole-tenant) — cleanest
   provenance, highest cost; overkill for this stage.
4. **CI runners** — ruled out by the evidence plan: regression gate only,
   never a source of published numbers.

## 3. Acceptance checks (any option must pass all)

- `nproc` reports ≥ 8; `free -g` reports ≥ 15 usable.
- NVMe-backed storage (`lsblk -d -o NAME,ROTA` shows `ROTA=0`).
- CPU governor `performance` or turbo documented as disabled
  (`cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`).
- `cargo -VV` shows the pinned MSRV toolchain; `rustup show active-toolchain`
  matches the workspace `rust-version`.
- One calibration run of the existing 1k-page capacity smoke
  (`cargo test -p crawlkit-engine --test capacity_smoke --release`): completes
  within the absolute caps (peak RSS < 500 MB, gate in the smoke test), and
  the resulting `run_record.json` embeds the full environment block.
- The same calibration run twice, back-to-back, throughputs within ±10% —
  the machine is quiet enough for mean-to-mean comparisons.

## 4. Decision rule

- If a rented bare-metal box (option 1) is online before 6.0.0-rc: it becomes
  the reference machine; re-run the two headline suites (1k smoke, 10k
  distributed posture) on it and republish the class from those records.
- If not: option 2 is formally accepted as the interim reference machine by
  recording it in the evidence plan §3 (open question resolved), with the
  laptop caveats stated in every report it produces.
- Either way, the choice is recorded once here and referenced — never
  re-decided per report.

## 5. Cost of not deciding

The 6.0.0 stable gate requires published numbers from the pinned class
(evidence plan §3/§8). With the spec above, the decision becomes a purchase
or a signature, not a blocker discovered at rc time.
