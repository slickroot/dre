# 111: Cut and paste a box

## Story

Doug has "API gateway" at the top with "Auth", "Payments" and "Orders"
underneath, and "Payments" has "Stripe" under it. He decides "Payments" belongs
under "Orders". He selects "Payments" and presses `d`. "Payments" and "Stripe"
vanish. He moves the selection to "Orders" and presses `p`. "Payments" comes
back under "Orders" with "Stripe" still under it, and it is now selected. He
never had to rebuild anything.

## Acceptance Criteria

- Pressing `d` puts the deleted box and all its descendants on a clipboard.
- A later `d` replaces what is on the clipboard.
- Pressing `u` to undo a delete does not empty the clipboard.
- Pressing `p` pastes the clipboard's box and its descendants as the last child
  of the selected box.
- After `p`, the pasted box is selected.
- Doug can press `p` many times to paste the same branch again.
- Pressing `u` right after a paste removes the pasted branch and puts the
  selection back where it was before the paste.
- Pressing `p` with an empty clipboard does nothing.
- If Doug deleted the top box and the canvas is empty, `p` brings the branch
  back as the top box.

## Technical Design
