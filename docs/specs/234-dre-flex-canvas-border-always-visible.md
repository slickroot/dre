Maya opens dre-flex. Even before she adds any boxes, she sees a thin border around the edge of the canvas, in the same style as every other box's border, so she always knows where the canvas boundary is.

## Acceptance Criteria

- The canvas draws a border even when it has no child boxes.
- The border uses the same thickness and colour as a normal, non-selected box border (`FLEX_BORDER` / `FLEX_BORDER_COLOUR`).

## Technical Design

The flex box tree has two borderless wrapper levels above real boxes:
path `[]` (the `Tree` root, required by the `Tree<T>` type but never
arranged, painted, or walked) and path `[0]`, which is "the canvas" that
`is_canvas` and selection already treat as a first-class node. Both are
currently built with `FlexBox::window()` in `new_canvas()`:

```rust
pub(crate) fn new_canvas(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
    Tree::new(
        FlexBox::window(),
        vec![Tree::new(FlexBox::window(), children)],
    )
}
```

`FlexBox::window()` and `FlexBox::default()` differ only in `border`
(`false` vs `true`); every other field already matches. `paint()`
already draws a border generically off `flex_box.border` using
`FLEX_BORDER`/`FLEX_BORDER_COLOUR`, with no dependency on child count.
So the fix is to build the canvas node (`[0]`) with `FlexBox::default()`
instead of `FlexBox::window()`, leaving the outer root (`[]`) untouched:

```rust
pub(crate) fn new_canvas(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
    Tree::new(
        FlexBox::window(),
        vec![Tree::new(FlexBox::default(), children)],
    )
}
```

No changes to `paint()`, `arrange()`, or any constants are needed — the
existing border-drawing path handles it once the canvas's `FlexBox` has
`border: true`.

The "outer wrapper vs. canvas" naming confusion this surfaced (the `[]`
root is dead plumbing with no visible purpose) is tracked as a separate
refactoring spec rather than folded in here.

Test fallout to handle during implementation:

- `a_new_canvas_is_a_borderless_column` currently asserts
  `!canvas.value(&[0]).border`; this is no longer true and the test
  (and its name) must be updated to assert a border instead.
- `default_has_a_canvas_with_one_box_selected` has two assertions that
  are no-ops on real behaviour (`state.boxes.value(&[0]) ==
  &FlexBox::window()` and `state.boxes.value(&[]) == &FlexBox::window()`);
  remove them rather than updating them to `FlexBox::default()`, keeping
  the assertions that test meaningful behaviour (`children`, `selected`).
