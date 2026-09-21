# 088: A Broken dre-web Test Fails The Pull Request Check

As the maintainer, when I open a pull request that breaks a `dre-web` test, the CI check fails, so I don't merge a broken wasm build.

## Acceptance Criteria

- The pull request check runs the `dre-web` tests as well as the `dre` tests.
- If a `dre-web` test fails, the pull request check fails.

## Technical Design

**Cause:** the workspace root is the `dre` package, so `cargo test --all-targets` only tests `dre`. The native `#[test]`s in `web/src/lib.rs` never run in CI.

**Change:** in `.github/workflows/ci.yml`, change the test step from `cargo test --all-targets` to `cargo test --workspace --all-targets`.

- One `test` job, one check. A failing test in any workspace member fails it, and future members are covered automatically.
- No new component and no new dependency. The `dre-web` tests are native, so the job needs no wasm target or `wasm-bindgen`.
- `release.yml` is unchanged. Its per-target test step stays as is, because spec 087 makes a failed wasm build stop the release, and the PR check now catches `dre-web` test failures before merge. Adding `--workspace` there would run the `dre-web` tests four times, once per matrix target.

**Verification:** locally, run `cargo test --workspace --all-targets` and confirm the `dre-web` tests appear in the output. Then open the PR with a temporarily failing `dre-web` test to confirm the check goes red, and revert it.
