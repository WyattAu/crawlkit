# Change review checklist

The current roadmap work includes a repository-wide formatter baseline as well
as functional documentation and CI changes. Reviewers should classify changes
before approval:

## Functional changes

- capability and claims validation;
- security/unsafe inventory scripts;
- contract and release-control scripts;
- benchmark metadata capture;
- CI workflow gates and artifacts;
- documentation corrections and release assurance;
- removal of hard-coded analyzer-count output.

## Formatting-only changes

Files changed only by `cargo fmt --all` should be reviewed separately from
behavioral changes. They are intended to be behavior-preserving, but the large
volume makes semantic review harder when mixed with roadmap changes.

## Required review evidence

- `git diff --check` passes;
- `cargo fmt --all -- --check` passes;
- capability and unsafe inventory checks pass;
- bounded contract/release checks pass;
- functional diffs have focused tests or documentation evidence;
- no unrelated file is included in a release change.

Do not use a formatter-only change to obscure behavior changes, and do not
remove formatter changes without restoring a passing formatting baseline.
