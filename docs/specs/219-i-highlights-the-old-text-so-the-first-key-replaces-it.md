# Flex: i highlights the old text so the first key replaces it

## User Story

Doug has a box that reads "Hello" and he wants it to read "World". He presses i,
the old word lights up on a filled background so he can see exactly what he is
about to overwrite, and he types W — the whole old word is gone in one
keystroke. No backspacing through five letters, no risk of leaving half the old
word stuck at the end.

## Acceptance Criteria

- Pressing i on a box that has text highlights that entire text with a filled
  background behind it.
- The first printable key Doug types after i replaces the whole highlighted
  text: "Hello" then W leaves the box reading "W".
- Every key he types after that first one appends, exactly as it does today.
- Backspace as that first key leaves the text empty — the old text is already
  gone, so there is nothing left to delete.
- Enter as that first key keeps the old text and moves him on to the next text
  node, exactly as it does today.
- Pressing i on a box that has no text behaves exactly as it does today: no
  highlight, and typing just types.
- Only i produces a highlight. o and the Enter-adds-a-box flow both start on
  empty text, so they never highlight anything.

## Technical Design
