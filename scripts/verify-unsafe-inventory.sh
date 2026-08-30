#!/usr/bin/env bash
set -euo pipefail

# Keep the inventory explicit: every unsafe-bearing source file must either
# document its FFI/sandbox boundary or fail this check.
mapfile -t files < <(rg -l --glob '*.rs' 'unsafe\s*\{' crates || true)
failed=0
for file in "${files[@]}"; do
  if ! rg -q 'SAFETY:|# Safety|unsafe-code|FFI' "$file"; then
    printf 'unsafe policy violation: %s lacks a safety justification\n' "$file" >&2
    failed=1
  fi
done

printf 'Unsafe Rust files: %s\n' "${#files[@]}"
printf '%s\n' "${files[@]}"
exit "$failed"
