# Flex: `p` pastes the cut box

## User Story

Noor is arranging boxes in dre-flex. She cuts a box she no longer wants with `d` — it vanishes and her parent is selected. Later she changes her mind, selects the box she actually wants it in, and presses `p`. The cut box reappears inside it, exactly as it was, with everything that was nested inside still nested. She presses `p` again and drops another copy, then realises she overdid it, presses `u`, and the last paste is gone. Happy, she carries on drawing!

## Acceptance Criteria

1. In Move mode, with a box on the clipboard and a box selected, `p` pastes the clipboard box as the last child of the selected box.
2. The pasted box keeps everything that was inside it, plus its label, border, fill, padding, justify, and direction.
3. The clipboard is kept, so `p` can paste again.
4. After pasting, the box that received the paste stays selected.
5. With the canvas selected and a box on the clipboard, `p` adds it as a new top-level box, and the canvas stays selected.
6. With nothing on the clipboard, `p` leaves the diagram and the selection unchanged.
7. One `u` after a paste removes the pasted box.
8. In Write and Replace modes, `p` is typed as the letter "p" and pastes nothing.

## Technical Design
