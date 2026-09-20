# 084: Cursor hides when idle

## User Story

When I stop pressing keys for a moment while working in the diagram, the block cursor
over the selected box disappears so I can read the box's full label — and it reappears
the instant I press any key.

## Acceptance Criteria

1. In command mode with a box selected, the cursor covering that box's label disappears
   after roughly one second with no keypresses.
2. While the cursor is hidden, the selected box's label is fully readable — the last
   character is never covered.
3. Pressing any key makes the cursor reappear immediately, on the currently selected box.
4. If I go quiet again, it hides again — the cycle repeats.
5. While I'm typing inside a label (insert mode), the caret stays visible and never
   auto-hides.

## Technical Design