# 031 - Cycle through more box border thicknesses

## Story

Bob selects a box and presses `t` repeatedly. Its border cycles through four
thickness levels — thin, then progressively thicker, then thicker still, then
thickest — and pressing `t` again after the thickest wraps back to thin.

## Acceptance Criteria

- `t` cycles the selected box's border through four levels in order: 1px →
  2px → 3px → 4px → back to 1px.
- A new box starts with a 1px (thin) border.
- `t` only changes the selected box's border; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the
  drawn line gets thicker/thinner.
- On a canvas with no boxes, `t` does nothing.
- In insert mode, `t` types the letter "t" into the label.

## Technical Design

- `Box` (`state.py`) widens `border: Literal[1, 2]` to `Literal[1, 2, 3, 4] = 1`
  — the field was already a plain integer thickness (not a bool), so this is
  a type-level change only; the default of `1` (thin) is unchanged.
- `handle_command`'s `"t"` branch (`state.py`) changes its `rewrite` lambda
  from the 1↔2 toggle `border=3 - box.border` to a 1→2→3→4→1 cycle:
  `border=box.border % 4 + 1`. The surrounding guard
  (`if not state.selected: return state`) is unchanged.
- Insert mode needs no change — `t` still falls through `handle_insert`'s
  printable-character branch into the label.
- `GraphicsRenderer._outline_box` already paints `border` edge pixels
  generically (not hardcoded to 1 or 2), so border=3 and border=4 render
  correctly with no rendering code changes — verified with new tests for
  3px and 4px edges and interior fill.
- `_key`'s cache `shape` tuple already includes `node.border` generically,
  so all four thickness values are cached distinctly with no cache-key
  changes — verified with a new test.

