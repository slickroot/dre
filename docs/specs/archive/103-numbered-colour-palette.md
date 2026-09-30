# 103: Numbered colour palette

## User Story

Doug selects a box and presses `c`. A numbered palette overlay appears, listing all 7 colors (lime, mint, violet, pink, amber, foreground, background), with the box's current color highlighted. He presses `2` and the box turns mint immediately, the palette closes. Happy with the result, he moves on.

## Acceptance Criteria

- Pressing `c` (or `C`) on a selected box opens a numbered palette overlay listing all 7 palette colors.
- The color currently applied to the box is visually marked/highlighted in the overlay.
- Pressing a digit key matching a listed color applies that color to the box immediately and closes the overlay.
- Pressing any non-digit key closes the overlay without changing the box's color.
- Existing `c`/`C` cycling behavior (cycle this box / cycle siblings) continues to work while the overlay is visible — i.e. this palette is additive, not a replacement.

## Technical Design

### State

- New field `State.colour_overlay: bool` (default `false`) — true while the numbered palette overlay is visible. No other data needed: the overlay always targets `state.doc.selected`, and colour resolution reuses the existing `PALETTE` array in `src/diagram.rs`.

### Opening the overlay and cycling (`c` / `C`)

- `command_mode::parse` keeps mapping `"c"` → `Command::CycleColour` and `"C"` → `Command::CycleSiblingsColour`, unchanged.
- `command_mode::reduce()` resets `state.colour_overlay = false` at the top, for every command, alongside the existing `pending_count.take()`. `cycle_colour` and `cycle_siblings_colour` then set `state.colour_overlay = true` after applying their cycle step.
- Net effect: pressing `c`/`C` always cycles the colour exactly as today, and leaves the overlay open (opening it on the first press, keeping it open — with the highlighted row updated — on subsequent presses). Any other recognized command implicitly closes the overlay as a side effect of the same reset. This requires no special-casing elsewhere in `reduce`.

### Applying a colour by digit

- Digit keys are already intercepted in `state::handle_key`, before `command_mode::parse` is called, accumulating into `state.pending_count` (used for count-prefixed motions like `3j`).
- Extend that branch: after `pending_count` is updated for a digit keystroke, if `state.colour_overlay` is `true`, resolve immediately in the same keystroke:
  - If `pending_count` is in `1..=7`: call `snapshot()` (so the change is undoable, matching `CycleColour`/`CycleSiblingsColour`), set the selected box's `colour` to `Some(pending_count - 1)` (digit `n` → palette index `n - 1`, e.g. `2` → index 1 → mint), then clear `pending_count` and set `colour_overlay = false`.
  - Otherwise (`0`, `8`, `9`, or any other non-matching value): clear `pending_count` and set `colour_overlay = false`, without changing the box's colour.
- Every digit 0-9 is immediately conclusive (either a valid index 1-7 or not), so this only ever resolves on the first digit pressed while the overlay is open — consistent with "pressing a digit applies immediately."
- While `colour_overlay` is `false`, digit accumulation is untouched — it continues to feed `pending_count` for count-prefixed motions as today.
- The digit always applies to the single box selected when the overlay opened, even if the overlay was opened via `C`. Siblings-targeted digit-apply is out of scope for this story.
- Any non-digit key that isn't `c`/`C` reaches `command_mode::parse`/`reduce` as normal (performing its usual action, e.g. moving selection), and `colour_overlay` closes as a side effect of `reduce`'s top-of-function reset — a single keystroke both closes the overlay and performs the key's normal action.

### Rendering

- New `TerminalRenderer::render_colour_overlay` method, alongside the existing `render_diagram`/`render_status_line`, invoked each frame when `state.colour_overlay` is `true`.
- Fixed position: a 7-row panel anchored to the left edge of the terminal, vertically centered.
- Each row shows a digit label (`1`-`7`) next to a small colour swatch (~2 cells wide), built with the existing `SolidShape` + `Canvas::fill` + `screen.place` pattern already used by `draw_cursor`, coloured via `palette(index)`. No colour names are shown.
- The row whose palette index matches the selected box's current `colour` is marked with a bracket/border around its swatch; if the box's `colour` is `None` (no palette colour applied), no row is marked.
