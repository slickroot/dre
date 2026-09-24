## Story

Doug is editing a box and has typed "Chche". He notices the missing "a". He
presses ← three times, so the cursor sits between "C" and "h". He types "a" and
the label reads "Cache". Happy, he presses Esc and moves on. If he had typed
"Cxache" instead, he would have pressed Backspace to remove the "x" before the
cursor.

## Acceptance Criteria

- In edit mode, ← moves the cursor one letter to the left and → moves it one
  letter to the right. The cursor is drawn where it is.
- Typing a letter puts it in at the cursor.
- Backspace deletes the letter before the cursor.
- ← at the start of the label and → at the end of the label do nothing. Doug
  stays in edit mode.
- When Doug starts an edit with `i` on a box that already has text, the cursor
  starts at the end of the label.
- Enter and Esc keep the whole label, not just the text before the cursor.
  Enter still adds a child box and Esc still leaves edit mode.

## Technical Design
