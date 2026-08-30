#!/usr/bin/env python3
"""Validate the repository capability manifest and current public claims."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
manifest = ROOT / "docs" / "capabilities.toml"
readme = (ROOT / "README.md").read_text()
cargo = (ROOT / "Cargo.toml").read_text()
main = (ROOT / "crates" / "crawlkit" / "src" / "main.rs").read_text()

errors: list[str] = []

if not manifest.exists():
    errors.append(f"missing manifest: {manifest}")
else:
    text = manifest.read_text()

    version = re.search(r'^version = "([^"]+)"$', cargo, re.MULTILINE)
    manifest_version = re.search(r'^version = "([^"]+)"$', text, re.MULTILINE)
    if not version or not manifest_version or version.group(1) != manifest_version.group(1):
        errors.append("manifest version does not match workspace version")

    msrv = re.search(r'^rust-version = "([^"]+)"$', cargo, re.MULTILINE)
    manifest_msrv = re.search(r'^msrv = "([^"]+)"$', text, re.MULTILINE)
    if not msrv or not manifest_msrv or msrv.group(1) != manifest_msrv.group(1):
        errors.append("manifest MSRV does not match workspace MSRV")

    command_block = re.search(r'^\[commands\]\n(.*?)(?=^\[|\Z)', text, re.MULTILINE | re.DOTALL)
    if command_block:
        declared = set(re.findall(r'"([a-z][a-z-]*)"', command_block.group(1)))
        command_enum = re.search(r'pub enum Commands\s*\{(.*?)\n\}', main, re.DOTALL)
        if command_enum:
            # Clap enum variants are intentionally normalized for comparison;
            # this catches missing/extra documented commands without depending
            # on runtime help formatting.
            variants = set(re.findall(r'^\s*([A-Z][A-Za-z0-9]*)\s*(?:\{|\()', command_enum.group(1), re.MULTILINE))
            normalized = {re.sub(r'([a-z0-9])([A-Z])', r'\1-\2', v).lower() for v in variants}
            missing = normalized - declared
            extra = declared - normalized
            if missing:
                errors.append(f"CLI commands missing from manifest: {sorted(missing)}")
            if extra:
                errors.append(f"manifest commands missing from CLI enum: {sorted(extra)}")

for forbidden in (
    'unsafe_code = "forbid"',
    "zero unsafe code",
    "204 analyzers",
    "500+ pages/sec",
):
    if forbidden.lower() in readme.lower():
        errors.append(f"README contains forbidden overclaim: {forbidden}")

# Historical/audit documents may contain quoted claims, but current competitive
# tables must not present the old unverified analyzer total as current evidence.
competitive = ROOT / "docs" / "COMPETITIVE_ANALYSIS.md"
if competitive.exists() and "README.md — 204 analyzers (badge in README)" in competitive.read_text():
    errors.append("competitive analysis still cites the retired 204-analyzer README claim")

if errors:
    for error in errors:
        print(f"FAIL: {error}", file=sys.stderr)
    sys.exit(1)

print("Capability manifest checks passed.")
