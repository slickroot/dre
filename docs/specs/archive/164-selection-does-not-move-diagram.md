## Story

Maya is navigating a diagram in dre, moving selection between nodes with the keyboard. When she selects a node near the edge of the diagram, the diagram no longer shifts position on screen — only the glowing selection border appears around the newly selected node, and everything else stays exactly where it was.

## Acceptance Criteria

- Moving selection from one node to another never shifts the diagram's rendered position on screen, regardless of which node is selected (including nodes at the diagram's edges).
- Centering still recalculates normally when the diagram's actual content changes (e.g. nodes added or removed).

## Technical Design

### Root cause

`centre()` (`src/view.rs`) computes the diagram's bounding box (`min_x`, span, height) from *every* `Placement` it's given, then centers that box in the terminal `Area`. The selection glow border is emitted as a `Placement` too (`PlacementNode::Glow`, in `src/layout/tree.rs`), positioned `GLOW_MARGIN` (1 cell) outside the selected node's box on every side. Because the glow placement is mixed into the same `Vec<Placement>` fed to `centre()`, selecting a node at the diagram's edge extends the bounding box by up to 1 cell on that edge, shifting the computed centering offset.

### Fix

1. Add `PlacementNode::is_decoration(&self) -> bool`, implemented as an **exhaustive match** (no wildcard arm) over every `PlacementNode` variant — `Glow` and `Caret` return `true`; all content variants (boxes, edges, labels, etc.) return `false`. The exhaustive match means adding a new `PlacementNode` variant in the future won't compile until someone explicitly decides whether it's a decoration.
2. In `centre()`, compute `min_x` / span / height only from placements where `!node.is_decoration()` — decorations no longer participate in the bounding-box calculation.
3. Still call `offset(...)`/`shift(...)` with the resulting horizontal/vertical offset on the **full, unfiltered** `placements` vec, so the glow and caret move together with the content instead of staying fixed while the diagram shifts.

This keeps `centre()` purely derived from content placements (so adding/removing nodes still recalculates centering normally), while selection/editing decorations can no longer affect it.
