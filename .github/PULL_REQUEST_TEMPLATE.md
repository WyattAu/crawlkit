## Summary

<!-- What does this PR change and why? Link related issues/ADRs. -->

Fixes #

## Risk

<!-- What could this change break? Consider blast radius: engine, API, clients, dashboard, CI. -->

- [ ] Low — additive or isolated change
- [ ] Medium — modifies existing behavior
- [ ] High — touches core crawling, auth, tenancy, or data storage

## Testing

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] Manual verification described below (if applicable)

### Manual tests

<!-- Commands run, output observed, screenshots for dashboard/UI changes. -->

## Security review

<!-- Required for changes to authentication, authorization, multi-tenancy, secrets handling, or processing of untrusted network input. -->

- [ ] No auth/tenancy changes, OR changes reviewed for privilege escalation, tenant data isolation, and secret exposure
