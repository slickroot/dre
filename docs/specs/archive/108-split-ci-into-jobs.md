# 108: Split CI into separate jobs

This is a technical spec, not a user story.

## Goal

A failing pull request shows which check broke in the PR checks list, without
opening the workflow run. Today fmt, clippy and test are unnamed steps in one
`test` job, so a red `test` could mean any of the three.

## Acceptance Criteria

- `.github/workflows/ci.yml` has three jobs: `fmt`, `clippy` and `test`.
- Each job appears as its own check on a pull request.
- The jobs run in parallel and don't depend on each other. A failure in one
  doesn't hide a failure in another.
- The commands are unchanged.
- `release.yml` is unchanged.

## Technical Design

### Jobs (`.github/workflows/ci.yml`)

- The single `test` job is replaced by three jobs on `ubuntu-latest`. None of
  them uses `needs`, so they run in parallel.
- `fmt`: checkout, `dtolnay/rust-toolchain@stable` with `components: rustfmt`,
  then `cargo fmt --all -- --check`. No rust cache, since fmt compiles nothing.
- `clippy`: checkout, `dtolnay/rust-toolchain@stable` with
  `components: clippy`, `Swatinem/rust-cache@v2`, then
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `test`: checkout, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`,
  then `cargo test --workspace --all-targets`. It no longer installs the clippy
  component.
- The job id is the check name, so the checks read `ci / fmt`, `ci / clippy`
  and `ci / test`.

### Shared setup

- The setup steps are repeated in each job, not moved into a composite action.
  The jobs need different components and caching, so a shared action would need
  inputs to cover the differences.
- `Swatinem/rust-cache@v2` keys its cache by job id by default, so `clippy` and
  `test` keep separate caches without a `key` input.

### Out of scope

- `release.yml` stays as it is. Its `build` matrix already shows the target in
  the job name.
- Branch protection is not touched. Nothing in the rulesets requires the old
  `test` check. If classic branch protection on `main` requires `ci / test`,
  add `ci / fmt` and `ci / clippy` in the repo settings.

### Tests

- No code tests. Open the pull request and check that the checks list shows
  `ci / fmt`, `ci / clippy` and `ci / test`.
- Break formatting on a scratch branch and check that only `ci / fmt` goes red.
