# 067 - Fix the Node.js 20 deprecation warning in the GitHub Actions workflows

## Technical Design

### Scope

GitHub is deprecating the Node.js 20 runtime for actions and forcing affected
actions to run on Node.js 24. Both workflows (`ci.yml` and `release.yml`)
currently pin `actions/checkout@v4`, which declares `using: node20`, so every
run prints the deprecation warning. This spec bumps `actions/checkout` to `@v5`
in both workflows to eliminate the warning.

Only `actions/checkout` changes. The other two actions in each workflow are
left untouched:

- `dtolnay/rust-toolchain@stable` is a composite action (`using: composite`)
  with all `bash` steps — it does not run on a Node runtime at all, so it
  produces no deprecation warning.
- `Swatinem/rust-cache@v2` resolves to the node24 runtime as of v2.9.0, so it
  produces no deprecation warning.

### Workflow changes

Both files change exactly one line each:

- `.github/workflows/ci.yml`: `uses: actions/checkout@v4` →
  `uses: actions/checkout@v5`
- `.github/workflows/release.yml`: `uses: actions/checkout@v4` →
  `uses: actions/checkout@v5`

`actions/checkout@v5` runs on the node24 runtime. It requires a minimum
Actions runner version of v2.327.1, which both GitHub-hosted `ubuntu-latest`
and `macos-14` runners satisfy, so the change is safe with no other edits.

No arguments are passed to the step in either workflow today, and checkout v5
is drop-in compatible for this zero-configuration use, so the steps remain
argument-free.