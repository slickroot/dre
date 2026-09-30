# Extract status_line.rs

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`StatusLine`, `Segment`, `Style` and `status_line()` are presentation, but they live in `state.rs` with the editing session and import colour constants.

## Acceptance Criteria

- A new `src/status_line.rs` holds `StatusLine`, `Segment`, `Style`, `dim`, `ModeLabel`, `StatusInput` and `status_line(&StatusInput)`.
- `status_line.rs` does not import `state`.
- `state.rs` no longer contains the presentation types, and the terminal renderer imports from `status_line`.
- The status line the terminal shows is identical. Existing tests move or split with the code and pass.

## Technical Design

Amends spec 117: `status_line.rs` does not read `State`. The rule is that no
module outside `state` reads `State` for the status line, so `state` builds a
plain input and `status_line` only turns it into styled segments. The
dependency runs one way: `state` imports `status_line`, never the reverse.

### `status_line.rs` (new)

- `ModeLabel { Commanding, Editing }`: owned by `status_line`, rendered as
  `" COMMANDING "` / `" EDITING "`.
- `StatusInput { mode: ModeLabel, filename: String, box_count: usize }`:
  resolved facts. `filename` already carries the `[+]` marker or the
  `Save as: x█` prompt text.
- `status_line(&StatusInput) -> StatusLine`: builds the mode segment (bold,
  `palette(0)` on `BACKGROUND`) and the dim segments.
- Moved unchanged: `StatusLine`, `Segment`, `Style`, `DIM_ALPHA`, `dim`.
- Imports: `palette` only.

### `state.rs`

- Gains `State::status_input(&self) -> StatusInput`: maps `Mode::Command` and
  `Mode::SavePrompt` to `ModeLabel::Commanding` and `Mode::Insert` to
  `ModeLabel::Editing`, resolves the filename (`save_to` or `DEFAULT_FILENAME`,
  `[+]` when dirty, or `Save as: {filename}{CURSOR}` during the save prompt),
  and counts boxes.
- Keeps `CURSOR`, `count_boxes` and `DEFAULT_FILENAME`. Loses the presentation
  types, `status_line` and its `palette` imports.

### Callers and visibility

- `render/terminal.rs` calls `status_line(&state.status_input())` and imports
  from `status_line`. It already reads `State`, so it gains no new dependency.
  Its `Style` reference in a test changes to `crate::status_line::Style`.
- Everything in `status_line.rs` and `State::status_input` is `pub(crate)`.
  `lib.rs` adds `mod status_line;` and drops `status_line, Segment,
  StatusLine, Style` from its `pub use`.
- `web/src/lib.rs`: delete `status_line_left`, `status_line_right`, the `join`
  helper and their four tests. Nothing in the web sources calls them.

### Tests

- Split by what each checks:
  - Stay in `state.rs`, as tests of `status_input`: the mode mapping, the save
    prompt keeping `Commanding`, filename cases (default, dirty, save-to path,
    prompt with cursor), and box counting (empty, nested, always plural).
  - Move to `status_line.rs`, built from a `StatusInput`: label padding, mode
    segment style, every other segment dim, separator, and the trailing
    `" • dre"`.
- The terminal renderer tests keep their assertions. The five that call
  `status_line(&state)` directly change to `status_line(&state.status_input())`.

### Order

1. Add `status_line.rs` with the new types and function, and its tests.
2. Add `State::status_input` with its tests.
3. Cut `render/terminal.rs` over, then delete the old code and tests from
   `state.rs`.
4. Narrow the `lib.rs` exports and delete the dead web methods.

### Considered and rejected

- `status_line(&State)` in a new file: it would be a reader of `State` outside
  `state`.
- `State::status_line()` composing internally: `state` would then depend on the
  presentation function, not just its input type.
- Passing rawer pieces (mode, dirty, path) and composing the filename in
  `status_line.rs`: it would move state rules (`DEFAULT_FILENAME`, the save
  prompt) out of `state`.
- A `Session::status_line()` facade or keeping the types public for the web
  crate: the web crate has no consumer of the status line.
