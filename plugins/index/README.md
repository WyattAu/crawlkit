# crawlkit first-party plugin index

This directory is the **first-party plugin index**: a git-versioned,
content-addressed, ed25519-signed catalog of plugins published by the
crawlkit project. Installing from it requires zero server infrastructure.

## Install

```bash
# From a checkout of this repository:
crawlkit plugin install title-length --index plugins/index/plugin-index.toml

# From GitHub directly (raw URL):
crawlkit plugin install viewport-checker \
  --index https://raw.githubusercontent.com/WyattAu/crawlkit/main/plugins/index/plugin-index.toml
```

Artifacts are verified against the engine's built-in trust store
(ADR-006) **before** anything is written to the install root. Default
install location: `~/.crawlkit/plugins`.

## Published plugins

| Name | Version | Description |
|---|---|---|
| title-length | 1.0.0 | Flags missing and oversized `<title>` elements |
| viewport-checker | 1.0.0 | Flags missing viewport meta tags and fixed-width viewports |

## Maintenance

Rebuild + re-sign after changing an example plugin:

```bash
scripts/build-plugin-index.sh
git add plugins/index && git commit -m "chore(plugins): re-sign index for <change>"
```

Each entry pins the exact `wasm_hash` of the artifact it publishes;
updating a plugin means a new artifact, a new hash, and a deliberate
commit — there is no implicit "latest" pointer.

## Trust

All first-party artifacts are currently signed with the development key
in `TRUSTED_PLUGIN_KEYS` (key id `1f299a0020f6ae90`). Before third-party
submissions open, this key rotates to a release-environment-held one per
ADR-010.
