# Security boundaries

This document describes implemented controls, not a certification.

## Crawl targets

All user-controlled HTTP(S) targets are checked by `ssrf::is_public_url`.
The check rejects local, private, link-local, multicast, unspecified, and
known metadata destinations. URL validation is also applied to redirects by
the API HTTP client. DNS rebinding and connection-level address pinning remain
operational concerns and must be handled by the deployment network policy.

## WASM plugins

WASM plugins are sandboxed by Wasmtime. Default configuration:

- signed/hash-verified plugins required;
- network capability disabled;
- filesystem and environment capabilities rejected;
- fuel, memory, and epoch timeout limits enabled;
- plugin failures are isolated from the crawl.

The network host function uses no redirects, a 10-second timeout, SSRF
validation, and a 1 MiB response cap. Enabling plugin network access is an
explicit embedding decision and should only be done in a restricted runtime.

## Native plugins

Native plugins are not sandboxed. Loading one grants arbitrary process-level
capability and therefore must be treated as trusted-code execution. The native
ABI contains scoped unsafe FFI and is excluded from claims of zero unsafe code.
Use WASM for untrusted extensions.

## Verification

Run:

```bash
scripts/verify-unsafe-inventory.sh
scripts/verify-contracts.sh
```

These checks provide regression detection; they do not replace penetration
testing, dependency review, or infrastructure controls.
