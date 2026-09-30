# Flex: d flips the selected box between row and column in MOVE

## User Story

Noor has a box showing `Hello [box] World` in a row, and they're in MOVE. They press `d`. "Hello", the inner box and "World" now stack top to bottom in the same order, each centred horizontally in the box. The outer box reshapes to fit the column and stays centred on the screen. Noor presses `d` again, and everything is back in a row. In WRITE, pressing `d` just types a "d".

## Acceptance Criteria

- In MOVE, `d` switches the selected box from row to column. Pressing `d` again switches it back to row.
- After `d`, `dre-flex` stays in MOVE.
- In column, the siblings stack top to bottom in the order they were added.
- In column, each sibling is centred horizontally in the box.
- In column, there's a 1-row gap between neighbouring siblings.
- The outer box grows or shrinks to fit the column and stays centred on the screen.
- `d` only changes the selected box. Other boxes keep their direction.
- In column, `g` changes nothing on screen. Pressing `d` to go back to row shows the spread again if `g` is still on.
- In WRITE, `d` types a "d" and doesn't change the direction.

## Technical Design

Depends on spec 196, which is done. The model is CSS flexbox. Every box has a `flex-direction`. `justify` (`g`) works along that direction, the main axis. Alignment on the other axis, the cross axis, is always `center`.

### State (`src/flex/state.rs`)

- New `Direction { #[default] Row, Column }` with `toggle()`, the same shape as `FlexWidth` and `Justify`.
- `FlexBox` gets a `direction: Direction` field. New boxes, inner ones included, start as `Row`.
- `move_key`: `"d"` toggles `selected_box().direction` and stays in MOVE. `d` only changes the selected box.
- WRITE needs no change, because `printable_char` already types `d`.

### Layout (`src/flex/view.rs`)

The measure and place passes from spec 196 stop assuming x is the main axis. Each box reads its own `direction` to decide which axis is main.

- `Size` gets `main(dir)` and `cross(dir)`, plus `Size::along(dir, main, cross)` to build one back.
- `measure`: for a box, main = the sum of the children's main sizes, plus `FLEX_GAP` between neighbours, plus the border. Cross = the largest child cross size, plus the border. A text is still `interior(text) × 1`.
- `Frame` holds the box's `direction`, a `main_cursor`, `cross_start` and `inner_cross`. The gap and `widened_from` come from main-axis free space.
  - `justify` spreads the free space on the main axis, using the same remainder rule as today.
  - Each child is centred on the cross axis: `cross_start + (inner_cross - child_cross) / 2`.
  - `place` converts each child's `(main, cross)` to `(x, y)` using the parent's direction.
- `Full` still only widens the outer box to `window.cols`, which is x. There's no full height.

With this model the column criteria need no special cases:

- **`g` changes nothing in column.** A column's main size is always its fitted height, so the free space is `FLEX_GAP × gaps` and SpaceBetween gives the same gaps as Start.
- **`g` comes back in row.** `d` never touches `justify`, so switching back to row with `g` still on shows the spread again.
- **Centred horizontally.** In column the cross axis is x, so each sibling is centred in the box's inner width. For a `Full` box that's the whole window.
- **Box reshapes and stays centred.** `measure` sizes the column, and `scene` still stacks the outer boxes and centres everything with `view::centre`.

### Collaborators

- `types::Tree<FlexNode>`: no change.
- `view::interior`, `view::centre`, `Placement`: no change.
- Only `src/flex/state.rs` and `src/flex/view.rs` change.

### Testing plan (TDD, thin slices)

1. State: `d` in MOVE flips the selected box `Row` → `Column` → `Row`, stays in MOVE, and leaves the other boxes alone. `d` in WRITE is typed.
2. View, measure: a column of `Hello`, an inner box and `World` is as wide as the widest sibling plus the border, and as tall as the siblings plus one gap between each, plus the border.
3. View, place: in column the siblings stack top to bottom in order, with a 1-row gap, each centred horizontally in the box.
4. View: a column with `g` on places its siblings exactly as it does with `g` off. A full-width column centres its siblings across the window.
5. View: the row tests from spec 196 still pass unchanged. This is the regression check for the axis refactor.

### Out of scope

- Cross-axis alignment other than centre (`align-items`).
- Full height, and any main-axis spread in column.
- Choosing which box `d` acts on when a text is selected. That follows `selected_box_path()` from spec 199.
