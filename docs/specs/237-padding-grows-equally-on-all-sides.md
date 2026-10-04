# Padding grows equally on all sides

Doug selects a box — whether or not it has a border — and presses `]`. He sees the space around the text grow equally on all four sides: top, bottom, left, and right. He presses `]` again and the space grows further, still equally on every side. Happy, he goes back to sleep!

## Acceptance Criteria

- Pressing `]` on a selected box with no border makes padding appear on all four sides equally (not just left/right).
- Pressing `]` on a selected box with a border grows the padding on all four sides equally (currently only left/right grow).
- Each additional press of `]` keeps growing all four sides together, by the same amount.

## Technical Design
