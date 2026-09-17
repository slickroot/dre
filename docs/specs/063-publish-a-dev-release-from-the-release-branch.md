# 063 - Publish a dev release from the release branch

## Story

Marouane pushes to the `release` branch. The tests pass, and a `v0.1.0-dev`
pre-release appears on the `dre` GitHub repo with a ready-made `dre` for Apple
Silicon Macs attached.

## Acceptance Criteria

- Pushing to the `release` branch publishes a GitHub release automatically.
- The release name comes from the project version, and it's the only place
  the version is kept. The project version is `0.1.0-dev`, so the release is
  `v0.1.0-dev`.
- The release is marked as a pre-release.
- The release has a ready-made `dre` for Apple Silicon Macs attached.
- If a test fails, nothing is published.
- If a release for that version already exists, nothing is published and the
  build fails.

## Technical Design
