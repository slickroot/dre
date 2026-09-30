# Brackets are not tiled

## Refactoring Goal

Selection brackets go through the tiler (`draw_brackets` → `place_tiles` with `TileStyle::Brackets`) as if they were box-sized sprites. In fact, `BracketsShape` only paints four fixed L's (arms of `BRACKET_ARM` = 14px, starting `BRACKET_OFFSET` = 8px outside each box corner), each about 2×2 cells. The rest of the placement is always empty. The rendered screen should stay the same, but brackets should stop being tiled.

Split out of spec 187.

## Technical Design
