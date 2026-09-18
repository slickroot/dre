# 071: Enter creates a child box

## User Story

As a user building a diagram, I can press Enter while typing a box's label to finish that box
and drop straight into a new, empty child box — one key instead of Esc then `b`.

## Acceptance Criteria

- While typing a label, pressing Enter finishes the box and a new empty child box appears below, ready for me to type its label
- Pressing Enter on a box whose label is still empty still creates an empty child box
- The action is undoable
- The start-up/README guidance shows Enter as the way to add a child box while typing

## Technical Design