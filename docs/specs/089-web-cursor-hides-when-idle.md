# 089: Web cursor hides when idle

## User Story

When Bob stops pressing keys for a moment on dre.elaich.com with a box selected, the
cursor over that box disappears so he can read the full label. It comes back as soon as
he presses a key.

## Acceptance Criteria

1. In command mode with a box selected, the cursor disappears after roughly one second
   with no keypresses.
2. While the cursor is hidden, the selected box's label is fully readable — the last
   character is never covered.
3. Pressing any key makes the cursor reappear immediately, on the box that was selected.
4. If Bob goes quiet again, it hides again — the cycle repeats.
5. While Bob is typing inside a label (insert mode), the caret stays visible and never
   auto-hides.

## Technical Design
