# Extract status_line.rs

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`StatusLine`, `Segment`, `Style` and `status_line()` are presentation, but they live in `state.rs` with the editing session and import colour constants.

## Acceptance Criteria

- A new `src/status_line.rs` holds `StatusLine`, `Segment`, `Style`, `dim` and `status_line(&State)`, unchanged.
- `state.rs` no longer contains them, and the terminal renderer imports from `status_line`.
- No behaviour changes. Existing tests move with the code and pass.

## Technical Design

Mechanical move, after the palette extraction. See spec 117.
