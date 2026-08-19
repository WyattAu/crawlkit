# ADR-006: WASM Plugin Trust Chain (ed25519 Manifest Signing)

## Status
Accepted (2026-08-19)

## Context

crawlkit's plugin system (ADR-001) executes third-party analyzer code as WASM
inside a wasmtime sandbox (ADR-003). The marketplace ambitions documented in
`docs/PLUGIN_MARKETPLACE.md` imply a future where users install plugins from a
catalog by name — which means the artifact a user *asks for* and the artifact
that *arrives on disk* are controlled by different parties.

Prior to v3.0.0 the load path compiled arbitrary `.wasm` bytes with zero
verification: if a manifest pointed at a module, wasmtime compiled it. The
sandbox constrains what a plugin can *do*, but it says nothing about *whose*
code is running. That gap matters because the sandbox threat model
(ADR-003) assumes an untrusted-but-nonmalicious author making honest mistakes;
it does not defend against a deliberately hostile artifact.

Threat model addressed by this ADR:

1. **Swapped artifacts** — a mirror, CDN, or supply-chain incident replaces a
   legitimate `.wasm` with a trojaned build. The manifest name is unchanged;
   only the bytes differ.
2. **Typosquatting names** — a plugin published as `seo-analyzer` when the user
   meant `seo-analyzers`. Hash pinning alone cannot help here because the user
   pins the *wrong* artifact; only a named-key identity gives the operator a
   chance to notice the publisher is not who they expected.
3. **Malicious publisher** — a signed-but-hostile plugin. Signing does *not*
   solve this; it converts an anonymous attack into an attributable one that
   the trust store can revoke. Sandbox limits (ADR-003) remain the primary
   control; the trust chain is attribution and revocation.

Constraints: verification must work offline (air-gapped and CI crawls must not
phone home), must add negligible latency to plugin load, and must not drag a
heavy PKI dependency tree into the engine (the workspace deliberately keeps
`unsafe_code = "deny"` and a minimal dependency set per the engineering
standards in `.adrs/coding-standards.md`).

## Decision

Plugin manifests carry three trust fields, and the loader verifies them
**before** handing the module to wasmtime for compilation:

- `wasm_hash`: hex sha256 of the raw `.wasm` bytes.
- `signature`: hex ed25519 signature over the raw 32-byte sha256 *digest*
  (not over the hex string, and not over the file — signing the digest keeps
  signatures constant-size and independent of module size).
- `signed_by`: a key id that must resolve against the trust store.

Verification policy (see `PluginVerification` in
`crates/crawlkit-engine/src/plugin.rs`):

1. A *declared* `wasm_hash` must match the actual bytes on disk. Mismatch is
   an immediate load failure regardless of policy — this is the
   swapped-artifact check.
2. `signature` and `signed_by` must appear together; a lone field is a load
   error (ambiguous manifest, fail closed).
3. **`Required` (the default):** missing `wasm_hash`, missing signature pair,
   or a signature that does not verify against a trust-store key with the
   given `key_id` all reject the plugin at load.
4. **`AllowUnsigned`:** escape hatch for local development and private plugin
   builds. Unsigned plugins load with a warning; a declared-but-invalid hash
   or a *malformed* signature still fails. Exposed as the
   `allow_unsigned_plugins` config knob so operators must opt in explicitly.

The trust store is **compiled into the engine**, not read from disk at load
time. A key that is not in the built-in store does not verify, even with a
mathematically valid signature — this is what makes typosquatting and
unknown-publisher attacks loud instead of silent.

Tooling ships with the engine: `crawlkit plugin keygen`, `crawlkit plugin
sign`, and `crawlkit plugin verify` let authors produce signed manifests and
let operators audit them without running the plugin.

## Consequences

### Positive
- Swapped-artifact and unknown-publisher attacks fail closed at load, before
  any wasmtime compilation or execution — malicious bytes never reach JIT
  surface.
- Fully offline-verifiable: ed25519 over a digest needs no network, no
  timestamp authority, no revocation server.
- Verification cost is one sha256 pass plus one ed25519 verify (~tens of
  microseconds) at plugin load, which happens once per crawl run — negligible
  against the sandbox's per-call costs.
- Defense in depth with ADR-003: the signature decides *whether* code runs;
  the sandbox bounds *what it can do* if it lies its way in. Neither layer
  is asked to do the other's job.

### Negative
- **Key rotation requires a PR and an engine release.** Adding or revoking a
  trust-store key means recompiling and shipping the engine. Rotation is
  therefore rare and deliberate — acceptable for a small publisher set,
  painful at marketplace scale (a fetchable, pinned key bundle is the
  eventual growth path).
- **Trust store centralization:** the engine maintainers become the root of
  trust for all signed plugins. Publishers cannot self-certify.
- Signatures attest bytes + key id, not manifest semantics or behavioral
  safety. A signed plugin can still be a malicious publisher (threat 3);
  attribution and sandboxing, not prevention.
- Signing is one more step for plugin authors, and `AllowUnsigned` output
  cannot be distributed to default-configured installs.

### Risks
- Key compromise: mitigated by release-based revocation + sandbox containment
  (ADR-003 capability checks are fail-closed: `network`, `filesystem`,
  `env_vars` requests are rejected at load).
- Operator habituation to `AllowUnsigned` eroding the default: mitigated by
  making it a loud config option, not a CLI flag that silently persists.
- Digest-only signing means the signature covers the `.wasm`, not the rest of
  the manifest (name, version, permissions). A re-signed manifest with an
  unchanged module keeps its valid signature; permission changes are caught
  separately by the capability fail-closed checks.

## Alternatives Considered

- **Sigstore / Cosign (keyless, identity-based signing):** rejected as the
  engine-side mechanism — verification pulls in OIDC identity assumptions and
  (for revocation and trust-root freshness) online checks; the offline,
  zero-network constraint rules it out. Revisit if the marketplace grows a
  real publishing identity story.
- **TUF / Notation (full update framework / signature envelope formats):**
  solves repository metadata, key rotation, and delegation far better than
  this design, at the cost of a metadata server, snapshot/root roles, and a
  dependency tree the engine does not want yet. Right answer for the
  marketplace backend; premature inside the loader.
- **Hash-only pinning (`wasm_hash` with no signature):** catches swapped
  artifacts but carries *no key identity* — a typosquat or a publisher swap
  that re-pins the hash in its own manifest verifies cleanly. Rejected as a
  default; retained as the integrity floor that even `AllowUnsigned` enforces
  for declared hashes.
- **KMS-backed signing (AWS KMS, Vault Transit):** moves private-key safety
  out of band but makes offline signing impossible and adds an ops dependency
  for every release. The built-in key ids are high-entropy and few; local
  key hygiene is proportionate for now.
- **Reusing GPG (already used for release artifacts, see CHANGELOG 3.0.0):**
  deliberately not reused here. GPG signs *release archives for humans*; the
  web-of-trust model and toolchain audience (packagers, distro maintainers)
  differ from *machine-verified plugin manifests at load time*. Bundling a
  GPG implementation in the engine for verification would be a heavy,
  wrong-audience dependency; ed25519 is ~100 lines against `ed25519-dalek`
  and verifies deterministically offline.

## References

- ADR-001 (Plugin System Architecture) — what is being trusted
- ADR-003 (WASM Plugin Sandboxing) — containment if trust fails
- `crates/crawlkit-engine/src/plugin.rs` — `PluginVerification`,
  `verify_ed25519_signature`, built-in trust store
- RFC 8032 (EdDSA), `docs/PLUGIN_DEVELOPMENT.md` (signing workflow)
