# 088: A Broken dre-web Test Fails The Pull Request Check

As the maintainer, when I open a pull request that breaks a `dre-web` test, the CI check fails, so I don't merge a broken wasm build.

## Acceptance Criteria

- The pull request check runs the `dre-web` tests as well as the `dre` tests.
- If a `dre-web` test fails, the pull request check fails.

## Technical Design
