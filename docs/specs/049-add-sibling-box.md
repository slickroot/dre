# Add a Sibling Box

## User Story

As someone editing a diagram, I want to press `s` while a box is selected to add a new sibling box, so that I can grow the tree sideways without navigating back to the parent and pressing `b`.

## Acceptance Criteria

- Pressing `s` while a box with a parent is selected appends a new empty box to the end of that parent's children (same append logic as `b` uses today).
- Pressing `s` while a top-level box (no parent) is selected appends a new empty top-level box to the end of the top-level list.
- The newly created sibling becomes the selected box, and the editor immediately enters label-editing mode so you can start typing right away.
- Pressing `s` does nothing if no box is currently selected.

## Technical Design
