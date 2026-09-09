# 013 - Add a box to the right

## Story

Bob has a labelled box. He presses `b` then `l`, and a second empty box
appears to the right of the first with a gap between them. He types, and the
new box takes his label while the first one keeps its own.

## Acceptance Criteria

- Pressing `b` then `l` when a box already exists places the new box to the
  right of the last one.
- There are four blank columns between each box and the next.
- The two boxes' vertical centers line up.
- The pair of boxes together is horizontally centered on screen.
- Typing after the new box appears goes into that new box; earlier boxes
  keep their labels unchanged.

## Technical Design

### Components

**`State`** (`state.py`) — gains one field: `pending: str = ""`. It accumulates
keys that start a multi-key command. `b` alone no longer creates a box; it
sets `pending = "b"` and otherwise leaves `State` untouched.

**`handle_command`** (`state.py`) — when `state.pending` is non-empty, the
incoming key either completes a known command or is discarded:

- `pending == "" and key == "b"` → `pending = "b"`, nothing else changes.
- `pending == "b" and key == "j"` → append `[Space(direction="down"), Box("")]`
  (today's "stack below" behaviour, now reached via `bj`), enter insert mode
  on the new box, clear `pending`.
- `pending == "b" and key == "l"` → append `[Space(direction="right"), Box("")]`,
  enter insert mode on the new box, clear `pending`.
- `pending == "b"` and any other key → clear `pending`, no other effect. The
  key is swallowed even if it would otherwise mean something (e.g. `i`).

When `pending` is empty, every existing key (`i`, `j`, `k`, `c`, `f`, `a`, `q`)
behaves exactly as it does today.

**`Space`** (`state.py`) — gains a field: `direction: Literal["down", "right"] = "down"`.
The default keeps every existing call site (`Space()`) meaning "new row",
so no other file needs to change just for this field to exist.

**`layout()`** (`layout.py`) — stops laying nodes out in a single vertical
column and instead places the flat node list onto a grid, then centers the
whole grid once.

### The grid

While scanning `state.nodes` in order, `layout()` tracks a `(row, col)`
cursor for the *last* node it placed, starting at `(0, 0)`:

- a plain node (Box, Arrow, Cursor) occupies the current `(row, col)`.
- `Space(direction="down")` moves the cursor to `(row + 1, col)` — same
  column as whatever came before it.
- `Space(direction="right")` moves the cursor to `(row, col + 1)`.

This makes placement relative to the last node, so `bj` after a box that's
already in column 1 stacks below it in column 1, not back in column 0.

Column width is the max `width(node)` over every node recorded in that
column; row height is the max `height(node)` over every node in that row.
Column x-offsets and row y-offsets are the cumulative sums of the widths/
heights of the columns/rows before them. Total grid width/height is the sum
of all column widths / row heights.

The grid is centered once: `x0 = (cols - total_width) // 2`,
`y0 = (rows - total_height) // 2`. Each node is then centered inside its own
cell: `x = x0 + col_offset[col] + (col_width[col] - node.width) // 2`, and
similarly for `y` inside its row. This is what makes the two boxes' vertical
centers line up when they share a row (same row height, both centered in it).

For a single-column stack (the only case that existed before this story),
this reduces algebraically to today's per-box `(cols - width) // 2`
centering: centering a box inside a column that's centered on the canvas is
the same as centering the box on the canvas directly. That's why every
existing `test_layout.py` case keeps passing unchanged.

### Width and height of `Space`

`width()` grows a case: `Space(direction="right")` is 4 (the four blank
columns from the acceptance criteria); `Space(direction="down")` stays 0, as
today. `height()` is untouched — a `Space` is `GAP_HEIGHT` tall regardless of
direction, matching how a horizontal `Space` still needs *some* row height so
it doesn't collapse a sparse row.

### Worked example

`[Box("a"), Space(direction="right"), Box("bb")]`, `cols=21`, `rows=11`.
Cursor positions: `Box("a")` → `(0,0)`, `Space` → `(0,1)`, `Box("bb")` →
`(0,2)`. Column widths: `col0 = width("a") = 3`, `col1 = 4`, `col2 =
width("bb") = 4`. Total width `= 11`, so `x0 = 5`. `Box("a").x = 5`,
`Space.x = 8`, `Box("bb").x = 12`. Gap between the boxes: `12 - (5 + 3) = 4`.
Single row, so both boxes get the same `y`, satisfying "vertical centers
line up".

### Tests

`tests/test_state.py` gains:

- `b` alone sets `pending` to `"b"` and does not add a node or change mode.
- `bl` appends `Space(direction="right")` and a new `Box`, selects it, and
  enters insert mode; `bj` does the same with `Space(direction="down")`.
- `b` followed by an unrecognized key (e.g. `i`) clears `pending` and has no
  other effect — no node is added, mode is unchanged, and `i`'s normal
  behaviour does not run.

`tests/test_layout.py` gains:

- a box placed via `Space(direction="right")` sits 4 columns to the right of
  its predecessor.
- two boxes sharing a row via `Space(direction="right")` get the same `y`.
- a pair of boxes on one row is centered as a unit: the combined left and
  right margins around the pair are equal (mirrors
  `test_each_box_is_centered_on_its_own_width`, but for the pair).
- `width(Space(direction="right"), editing=False) == 4`.
- placing a box below the *last* box when that box is in column 1 (i.e. a
  `bl` followed by a `bj`) keeps the new box in column 1, not column 0.
