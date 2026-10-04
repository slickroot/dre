## User Story

Maya selects a box on the flex canvas (in Move mode). Instead of the box's border or label text changing color, she sees a thin outline drawn around the box with a small visible gap between the outline and the box's own edge — the same treatment whether the box has a border of its own or is a plain text leaf. When she selects a different box, the outline moves to it and the previous box returns to its normal appearance.

## Acceptance Criteria

1. Selecting any box in Move mode draws a rectangle outline around it, offset outward by a small visible gap from the box's actual edges — this applies the same way to bordered boxes and borderless text leaves.
2. The box's own border color and label text color are no longer changed when it's selected (removing the current blue-recolor behavior).
3. The outline uses the existing selection blue color.
4. Deselecting / moving selection to another box removes the outline from the old box and draws it on the new one.
5. This applies only to Move mode (Replace mode's text-highlight behavior is untouched).

## Technical Design
