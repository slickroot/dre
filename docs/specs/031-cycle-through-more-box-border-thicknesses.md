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

