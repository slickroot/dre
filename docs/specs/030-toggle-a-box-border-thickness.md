# 030 - Toggle a box's border thickness

## Story

Bob selects a box and presses `t`. Its border becomes visibly thicker. He
presses `t` again and it goes back to thin.

## Acceptance Criteria

- `t` toggles the selected box's border between thin and thick.
- A new box starts with a thin border.
- `t` only changes the selected box's border; other boxes are untouched.
- The box's size and position stay exactly as they are today — only the drawn
  line gets thicker, it doesn't eat into the label area.
- On a canvas with no boxes, `t` does nothing.
- In insert mode, `t` types the letter "t" into the label.

## Technical Design
