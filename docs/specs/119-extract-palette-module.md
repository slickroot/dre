# Extract palette.rs

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`palette()`, `PALETTE`, `FOREGROUND` and `BACKGROUND` live in `diagram.rs`, but they are the app's colours, used by the renderers and the status line. The drawing does not use them: `Node.colour` is only an index.

## Acceptance Criteria

- A new `src/palette.rs` holds `PALETTE`, `FOREGROUND`, `BACKGROUND` and `palette()`, unchanged.
- `diagram.rs` no longer defines or re-exports them, and every user imports from `palette`.
- No behaviour changes. Existing tests pass unchanged apart from imports.

## Technical Design

Mechanical move. See spec 117, `status_line.rs` and `palette.rs`.
