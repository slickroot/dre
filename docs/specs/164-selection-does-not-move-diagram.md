## Story

Maya is navigating a diagram in dre, moving selection between nodes with the keyboard. When she selects a node near the edge of the diagram, the diagram no longer shifts position on screen — only the glowing selection border appears around the newly selected node, and everything else stays exactly where it was.

## Acceptance Criteria

- Moving selection from one node to another never shifts the diagram's rendered position on screen, regardless of which node is selected (including nodes at the diagram's edges).
- Centering still recalculates normally when the diagram's actual content changes (e.g. nodes added or removed).

## Technical Design
