#!/bin/bash -eu
# OSS-Fuzz build for crawlkit fuzz targets.
cd "$SRC/crawlkit/crates/crawlkit-engine"

# Each fuzz target builds with the default (libfuzzer) engine; the
# wasm32 conformance targets are excluded (they need a host runtime).
for target in fuzz_parser fuzz_robots fuzz_sitemap; do
  cargo fuzz build -O "$target"
done

OUT_TARGET="target/x86_64-unknown-linux-gnu/release"
# cargo-fuzz places binaries under target/<triple>/release/<name>
# (release profile with -O); copy each into $OUT for OSS-Fuzz packaging.
for target in fuzz_parser fuzz_robots fuzz_sitemap; do
  cp "fuzz/target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"
done
