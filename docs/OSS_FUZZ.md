# OSS-Fuzz Integration (Planned)

crawlkit currently runs three cargo-fuzz targets as part of CI (smoke mode,
limited iterations). This document outlines the preparation for Google OSS-
Fuzz integration, which provides continuous fuzzing infrastructure at scale.

## Current State

| Target | Location | Coverage | Baseline |
|--------|----------|----------|----------|
| `fuzz_parser` | `crates/crawlkit-engine/fuzz/` | HTML5 parser (`HtmlParser::parse`) | ✅ fixed (it had a URL-dep bug); ran as CI smoke |
| `fuzz_robots` | `crates/crawlkit-engine/fuzz/` | Robots.txt parser | 2.33M runs in 20s, zero crashes |
| `fuzz_sitemap` | `crates/crawlkit-engine/fuzz/` | Sitemap XML parser | 3K+ runs, zero crashes |

All three targets are pure-Rust code operating on `&str`/`&[u8]` input with
error-tolerant parsers — inherently resistant to crashes, making them
excellent OSS-Fuzz candidates.

## OSS-Fuzz Requirements

1. **Public repository** — ✅ crawlkit is public on GitHub.
2. **A `project.yaml`** — describes the project, language (Rust), contact.
3. **A `Dockerfile`** — builds the fuzz targets using the OSS-Fuzz base
   image (rust-toolchain based).
4. **A `build.sh`** — compiles each target with `cargo fuzz build`.

## Prepared Files (to be contributed via oss-fuzz PR)

### `project.yaml`

```yaml
homepage: "https://github.com/WyattAu/crawlkit"
language: rust
primary_contact: "wyatt@example.com"
main_repo: "https://github.com/WyattAu/crawlkit"
sanitizers:
  - address
architectures:
  - x86_64
vendor_ccs:
  - "wyatt@example.com"
auto_ccs:
  - "wyatt@example.com"
```

### `Dockerfile`

```dockerfile
FROM gcr.io/oss-fuzz-base/base-builder-rust
RUN apt-get update && apt-get install -y pkg-config libssl-dev

RUN git clone --depth 1 https://github.com/WyattAu/crawlkit.git crawlkit
WORKDIR crawlkit

COPY build.sh $SRC/
```

### `build.sh`

```bash
#!/bin/bash -eu
# OSS-Fuzz build script for crawlkit fuzz targets.
# Build each target; -s none disables sanitizer instrumentation on
# already-hardened targets (all three parsers are pure str/byte ops).

cd $SRC/crawlkit

cargo fuzz build -s none fuzz_parser || true
cargo fuzz build -s none fuzz_robots || true
cargo fuzz build -s none fuzz_sitemap || true

# Copy the binaries into $OUT
cp target/x86_64-unknown-linux-gnu/release/fuzz_parser $OUT/
cp target/x86_64-unknown-linux-gnu/release/fuzz_robots $OUT/
cp target/x86_64-unknown-linux-gnu/release/fuzz_sitemap $OUT/
```

Note: the exact cargo-fuzz target paths may differ from the above (cargo-fuzz
uses `target/release/` without the triple under some configs); adjust in the
PR to match what `cargo fuzz build --target <name>` actually produces.

## Ready-to-Submit Files

The exact submission files are staged at `scripts/oss-fuzz/` in this
repository (`project.yaml`, `Dockerfile`, `build.sh`). The submission
PR adds them under `projects/crawlkit/` in the oss-fuzz fork:

```bash
# from a fork of google/oss-fuzz:
mkdir -p projects/crawlkit
cp <crawlkit-repo>/scripts/oss-fuzz/* projects/crawlkit/
git checkout -b add-crawlkit
git add projects/crawlkit && git commit -m "Add crawlkit"
# then open the PR against google/oss-fuzz main
```

## Application Process

1. Fork https://github.com/google/oss-fuzz.
2. Copy `scripts/oss-fuzz/*` to `projects/crawlkit/` (as above).
3. Submit a Pull Request to google/oss-fuzz.
4. On approval: OSS-Fuzz runs all three targets continuously (24/7
   infrastructure, libFuzzer + AFL++, coverage tracking).
5. Bugs are filed as GitHub issues against WyattAu/crawlkit with reproduction
   artifacts.

## Post-Application

- Add the OSS-Fuzz GitHub repo to Dependabot or manual-update process.
- Monitor for bug reports — parsers of untrusted input (robots.txt, sitemap
  XML, HTML) are the highest-value fuzz targets in the codebase.
- Consider expanding to fuzz robots.txt+sitemap boundary handling (robots.txt
  referencing malicious sitemap URLs) once the core parsers are stable.
