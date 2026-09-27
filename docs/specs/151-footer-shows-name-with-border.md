# Footer shows NAME while typing the diagram's name

## User Story

Doug presses `n` to name his diagram. The footer's LED lights up amber next to the word NAME, and the footer gains a border around it, making it obvious he's now typing into it. When he confirms or cancels, the footer returns to its normal MOVE state with no border.

## Acceptance Criteria

- In the Name prompt, the footer shows a lit amber LED followed by "NAME" before the name being typed.
- While naming, the footer has a border around it (it currently has no border).
- Confirming or cancelling the name prompt returns the footer to its normal state (dim lime LED, MOVE, no border).

## Technical Design
