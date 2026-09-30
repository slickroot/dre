# Flex: layout is measure, arrange, paint

## Refactoring Goal

`src/flex/view.rs` lays out a scene in four steps that overlap. `measure` fills a `HashMap` keyed by path. `place` walks the tree once and, for each node, handles outer boxes as special cases (`Full`, stacking, `x = -width/2`), moves a `Frame` cursor, builds a new `Frame`, and builds the `Placement` with its colours. `scene` then undoes the `-width/2` with a min-x shift, and `view::centre` centres everything again. So centring happens three times, and `measure` and `place` disagree about the width of a `Full` box.

Spec 197 (direction) would add main and cross axes to `Frame`, and spec 200 (depth) would add another field to that same loop. After this refactor, layout is three recursive steps, each with one job: **measure** a node, **arrange** it inside the rectangle its parent gives it, and **paint** it. There is no user story: nothing changes on screen or on the keyboard.

## Findings

Problems in today's `src/flex/view.rs` that this refactor fixes. Each one is removed, not moved somewhere else.

1. **Centring happens three times.** Outer boxes start at `x = -width/2`, `scene` shifts everything by the smallest x, and `view::centre` centres it all again. *Fixed by:* `lay_out_window` puts each outer box at its final position in one step. The min-x shift and the `view::centre` call are deleted.
2. **`measure` and `place` disagree about size.** `measure` gives a `Full` box its fitted width, then `place` overrides it with `window.cols`. *Fixed by:* one rule, as in CSS. `measure` always returns the natural size, and the parent decides the final rectangle. Only `lay_out_window` turns a natural size into a `Full` width.
3. **Outer boxes are special-cased in three places.** `path.len() == 1` is checked for `Full` width, for fill, and for stacking. *Fixed by:* `Full` width and stacking live only in `lay_out_window`. Fill stays in `paint` until the open question in Out of scope is settled.
4. **Layout and looks are mixed in one loop.** The same loop works out x and y and picks colours, selection and fill. *Fixed by:* `arrange` returns only rectangles, and `paint` owns everything visual. Each can be tested without the other.
5. **The walk stands in for recursion.** Because `Tree` hides children, a size `HashMap` and a mutable `Frame` cursor in a second `HashMap`, both keyed by path, do the job a call stack would do. *Fixed by:* `Tree::children(path)` and recursive `measure` and `arrange`. Both maps and `Frame` are deleted.
6. **`child_sizes` finds children by probing.** It builds `[path, i]` for i = 0, 1, … and calls `contains` until one is missing. *Fixed by:* `Tree::children(path)`.
7. **The space-between remainder rule is hidden in cursor state.** `widened_from` and `gap_after` spread the rule across `Frame::new` and every child step. *Fixed by:* `distribute`, a pure function that returns every offset at once and has its own tests.
8. **`row_shift` is dead code.** A `Fit` box's inner width always equals its row width, and a `Full` box starts its row at the border, so the shift is always 0. *Fixed by:* deleting it.

## Technical Design

Decisions:

- **Nothing user-visible changes.** Every view test in `src/flex/view.rs` keeps its assertions. Only the three `measure` tests change their call shape (see Testing plan). Placements come out in the same pre-order as `Tree::walk()`, so paint order is the same too.
- **The layout recurses, so the `types` crate gets one accessor.** `Tree::children(&self, path: &[usize]) -> Vec<Vec<usize>>` returns the paths of a node's children, in order. It takes a path like the rest of the `Tree` API, and the empty path gives the root's children. This replaces `child_sizes`, which finds children by building paths and checking `contains` until one is missing.
- **Geometry** (`src/flex/view.rs`):
  ```rust
  struct Size { width: i64, height: i64 }        // unchanged
  struct Rect { x: i64, y: i64, width: i64, height: i64 }
  ```
- **`measure(tree, path) -> Size`** is recursive and gives the natural (`Fit`) size, using the same formula as today. A `Text` is `interior(text) × 1`. A `Box` is the sum of its children's widths plus `FLEX_GAP` between neighbours plus the border, by the tallest child plus the border. There is no map and no cache: `arrange` measures a box's children when it needs them. Trees are small, so the repeated work doesn't matter.
- **`distribute(widths: &[i64], room: i64, justify: Justify) -> Vec<i64>`** returns each child's x offset from the start of the row. It is the only place that knows the gap rules.
  - `Start`: children sit `FLEX_GAP` apart.
  - `SpaceBetween` with two or more children: every gap is `free / gaps`, and the remainder goes one cell each to the last gaps. That's today's `widened_from` rule.
  - `SpaceBetween` with fewer than two children behaves like `Start`.
- **`arrange(tree, path, rect, out: &mut Vec<(Vec<usize>, Rect)>)`** pushes `(path, rect)`. For a `Box`, it measures the children, calls `distribute` with the inner width, and recurses into each child with:
  - `x = rect.x + FLEX_BORDER + offset`
  - `y = rect.y + FLEX_BORDER + (inner_height - child.height) / 2`
  - the child's measured size. Inner boxes are always `Fit`, as today.
- **`lay_out_window(state, window) -> Vec<(Vec<usize>, Rect)>`** is the one place the outer boxes are special. It measures each outer box and gives a `Full` box `window.cols` as its width. It stacks the boxes `FLEX_GAP` apart and centres the stack, reproducing today's numbers exactly (the `odd_widths_lean_right` and `the_stack_is_centred_in_the_window` tests pin them). With `W` the widest box and `H` the stack height, for each outer box of width `w` at stack offset `dy`:
  - `x = window.col + (window.cols - W).div_euclid(2) + W.div_euclid(2) - w.div_euclid(2)`
  - `y = window.row + (window.rows - H).div_euclid(2) + dy`

  It then calls `arrange` for each outer box. Spec 197 replaces this function with a root column box that is not drawn.
- **`paint(state, path, node, rect) -> Placement`** holds all the looks: label colour, border colour (`FLEX_SELECTED_COLOUR` when the box is at `selected` in MOVE), and fill. Fill still applies only to outer boxes, so the `path.len() == 1` check moves here unchanged. Spec 200's `depth` will be one line here.
- **`scene`** is `lay_out_window`, then `paint` on each entry. It no longer calls `view::centre` or shifts by min-x.
- **Deleted:** `Frame`, `child_sizes`, the size `HashMap`, the min-x shift, and `row_shift` (finding 8).

### Collaborators

- `types::Tree<FlexNode>`: gains `children(path)`. `walk`, `value` and `contains` stay as they are.
- `view::interior`, `Placement`, `PlacementNode`, `Area`: no change. `view::centre` is no longer used by flex, but the main editor still uses it.
- `src/flex/state.rs` and `src/flex/mod.rs`: no change.

### Testing plan (TDD, thin slices)

1. `types`: `children` returns child paths in order, returns the root's children for `[]`, and returns an empty list for a leaf.
2. `distribute`: `Start` with 0, 1 and 3 children; `SpaceBetween` with an even split; `SpaceBetween` where the remainder goes to the last gaps; `SpaceBetween` with 1 child.
3. `measure`: the three existing measure tests, rewritten from `sizes[&path]` to `measure(&tree, &path)` with the same expected sizes.
4. Swap `place` for `lay_out_window` + `arrange` + `paint`. Every other view test passes unchanged, and that's the guard.
5. Delete `Frame`, `child_sizes` and the size map.

### Out of scope

- The root as a real box, and a column direction. Both come with spec 197, which should now depend on this spec.
- Letting inner boxes be `Full` or filled. That's still an open question, and this refactor keeps both outer-only.
- Spec 200's `depth`, and spec 199's selectable texts.
