# Round a box and its siblings at once

## Story

Nadia has a row of five service boxes under "API gateway". She selects one of
them and presses `R`, and all five get rounded corners together, so she doesn't
have to round each box one at a time.

## Acceptance Criteria

- `R` on a box whose row is all square or mixed rounds every box in the row.
- `R` on a box whose row is all rounded makes every box in the row square.
- `R` changes only the selected box and the boxes that share its parent. Their
  children and every other box stay as they are.
- `R` works on a row of top-level boxes too.
- One `u` undoes the whole row's change.
- `R` with no box selected does nothing.
- `R` appears in the keymap as "Toggle rounded corners of every sibling".

## Technical Design
