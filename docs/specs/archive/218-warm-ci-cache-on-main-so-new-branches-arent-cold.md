# Warm the CI cache on main so new branches aren't cold

## Problem

The `test` job in `.github/workflows/ci.yml` routinely takes ~55s-1m on CI,
even though the suite itself runs in well under a second locally. Most of
that time is `cargo test` downloading and compiling crates from scratch.

`ci.yml` already uses `Swatinem/rust-cache@v2` on the `clippy` and `test`
jobs, so this isn't a missing-cache problem — it's a cache-scoping problem,
confirmed by inspecting actual CI runs and `gh cache list`:

1. **No durable cache on `main`.** `ci.yml` only triggers on `pull_request`.
   GitHub Actions caches are scoped to the branch that created them, with
   fallback only to the PR's base branch or the repo's default branch — never
   to a sibling branch. This repo cuts a fresh branch per spec
   (`spec-213-...`, `spec-214-...`, ...), so each one is a sibling of the
   others under `main`, not a descendant. Since `main` never runs the
   workflow, it never has a cache for new branches to fall back to, and every
   new spec branch starts cold.

2. **A failing job never saves its cache.** `rust-cache` defaults to
   `cache-on-failure: false`. When the `test` job fails mid-iteration (which
   is expected during red-green-refactor), it saves nothing, so the *next*
   push on the *same* branch is cold again too. Logs confirm this: on
   spec-214's two runs, `clippy` passed on the first (failing) run and showed
   `Cache up-to-date.` on the retry, while `test` failed on the first run and
   showed `No cache found.` on the retry.

## Technical Design

### Decisions

- **Add a `push: branches: [main]` trigger to `ci.yml`.** Running `clippy` and
  `test` on every merge to `main` gives those jobs a cache scoped to `main`,
  which every new feature branch can use as a fallback on its first run.
  `fmt` is excluded from the push trigger: it has no cache step and gains
  nothing from running on `main`.
- **Set `cache-on-failure: true` on the `Swatinem/rust-cache@v2` step in both
  `clippy` and `test`.** This banks whatever was downloaded/compiled even
  when the job itself fails, so a red run on a branch doesn't force the next
  push on that same branch back to a cold cache.

### Collaborators

- `.github/workflows/ci.yml`: add `push: branches: [main]` alongside the
  existing `pull_request` trigger; add `cache-on-failure: true` to the
  `Swatinem/rust-cache@v2` step in `clippy` and in `test`. No change to the
  `fmt` job or to `cargo test`/`cargo clippy` invocations.

### Testing plan

No unit tests apply to a workflow file. Verify by merging to `main` and
confirming a `v0-rust-clippy-...` and `v0-rust-test-...` cache appears
scoped to `main` in `gh cache list`; then open a new feature branch's PR and
confirm its `clippy`/`test` jobs restore from that cache (`Cache up-to-date.`
or a quick restore) instead of printing `No cache found.`.
