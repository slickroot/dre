# Flex: the canvas is a non-editable root box and the default selection

## Context

This is a **refactoring spec**, not a user story. Today `FlexState::boxes` is a
`Tree<FlexBox>` whose root `[]` is the window/canvas: `new_window(vec![new_box()])`
gives the window at `[]` and user boxes at `[0]`, `[1]`, .... The canvas is a
`FlexBox::window()` (borderless, Column), but it is only *ever* reached as an
**empty selection** (`selected == []`). Every reducer branch that can fall back
to it tests `selected.is_empty()`, and editing it would panic because
`Tree::value_mut(&[])` asserts the path is non-empty.

We are promoting the canvas to a **real, first-class box at `[0]`**, with user
boxes living inside it at `[0,0]`, `[0,1]`, .... It is a normal `FlexBox` but it
can never be edited, it is always borderless and a Column, and it is the
selection we fall back to so that there is **always a selected item**.

## Acceptance Criteria

1. The tree is `[] -> canvas[0] -> boxes [0,0], [0,1], ...`. The empty path `[]`
   is a mechanical Tree root only: it is never selected, never rendered as a box,
   and never edited.
2. On startup the canvas contains one empty box at `[0,0]`, and `[0,0]` is
   selected (`selected == [0, 0]`).
3. `selected` is never empty. Whenever the last box is removed (cut with `d`,
   dropped by Enter/Esc on an empty borderless box, or backspaced away in Move),
   the canvas `[0]` becomes selected.
4. The canvas is a Column and borderless, and no key can change either.
5. On the canvas, `a` adds an empty bordered box as its last child and enters
   Write on it; `\r` selects its first child; `\x7f` and `q`/`u`/`\x03` behave as
   usual (backspace on the canvas stays on the canvas because it has no parent
   box).
6. On the canvas, the self-edit keys `i`, `o`, `O`, `s`, `r`, `f`, `]`, `d`, `J`,
   `K` do nothing and push **no** undo snapshot.
7. `j`/`k`/`h`/`l` on the canvas do nothing (it has no siblings).
8. The canvas paints nothing, so it never shows a border or a selection
   highlight, exactly like the old window at `[]`.
9. Every existing flex behavior (typing, `a`/`o`/`O`, `s`/`r`/`f`/`]`, `d`/`p`,
   `J`/`K`, undo) is unchanged for real boxes, only shifted one level deeper in
   their paths.

## Technical Design

### Decision summary

- **Add a level, do not move the root.** `FlexState::boxes` stays a
  `Tree<FlexBox>`. Its root `[]` becomes an **inert** holder whose only child is
  the canvas at `[0]`; the canvas's children are the user boxes at `[0, i]`. This
  is the literal shape from the design meeting and keeps every box path
  non-empty.
- **The canvas is an ordinary `FlexBox`.** It is built from the existing
  `FlexBox::window()` (borderless, Column, Start, no text, no padding). There is
  no new field and no new type.
- **"Never edited" is enforced by position, not by data.** The canvas is the
  only box at depth 1, so `is_canvas(path) == path.len() == 1`. The guard is a
  single early arm in `reduce`; `is_canvas` is also what lets the same predicate
  describe the inert root (depth 0) and boxes (depth >= 2).
- **No history for canvas no-ops.** The guard runs in `reduce` *before*
  `history::recorded`, so a self-edit key on the canvas returns the state
  untouched and never snapshots. This is the one place the design departs from
  the spec 227/228 "no-op still snapshots" precedent, and it is deliberate:
  the canvas is uneditable, so nothing happened to undo.
- **Selection is total.** Because the canvas always exists, `selected` is always
  a valid non-empty path; all the `selected.is_empty()` fallbacks are removed.
- **The canvas occupies the old window's place in layout.** `scene` arranges
  from `[0]` and hands the canvas the whole window rect, so the canvas is laid
  out exactly where the old `[]` window was. Layout, measurement and painting of
  real boxes are otherwise untouched.

### The new tree

```
[]                    inert holder   FlexBox::window()   never selected/rendered/edited
└── [0]               canvas         FlexBox::window()   selectable, non-editable
    ├── [0,0]         user box       FlexBox::default()  normal box
    ├── [0,1]         user box
    └── [0,0,0] ...   nested box
```

- `parent_path([0, 0]) == [0]` (the canvas), `parent_path([0]) == []` (the inert
  holder, only reachable while the canvas is selected and every such path is
  guarded).
- `Tree::value_mut(&[])` still panics, so the inert root cannot be edited; the
  canvas `[0]` is guarded in `reduce`.

### `flex::state` — constructors

```rust
/// The whole canvas: an inert holder with the canvas as its only child.
pub(crate) fn new_canvas(children: Vec<Tree<FlexBox>>) -> Tree<FlexBox> {
    Tree::new(FlexBox::window(), vec![Tree::new(FlexBox::window(), children)])
}
```

- Replaces `new_window(children)` (`state.rs:88`). `new_box` and `new_child_box`
  are unchanged.
- `FlexState::default()` becomes `boxes: new_canvas(vec![new_box()])` and
  `selected: vec![0, 0]` (`state.rs:113`).

### `flex::state` — the position guard

```rust
fn is_canvas(path: &[usize]) -> bool {
    path.len() == 1
}

fn canvas_self_edit(key: &str) -> bool {
    matches!(key, "i" | "o" | "O" | "s" | "r" | "f" | "]" | "d" | "J" | "K")
}
```

`reduce` (`state.rs:139`) gains one arm **above** the Move catch-all, so the
no-op is decided before any snapshot is taken:

```rust
(_, FlexMode::Move) if is_canvas(&state.selected) && canvas_self_edit(key) => (state, None),
(_, FlexMode::Move) => history::recorded(state, key, |s| move_key(s, key)),
```

- Knows: the selected path and the key.
- Does: for the ten self-edit keys on the canvas, returns the state unchanged
  with no effect and no history entry (AC 6).
- `a`, `\r`, `\x7f`, `j`/`k`/`h`/`l`, `u`, `q`, `\x03` are not self-edit keys,
  so they fall through to `move_key` / the quit arm unchanged (AC 5, 7).

`move_key` (`state.rs:248`) needs no canvas arm: every self-edit arm is already
unreachable on the canvas. `j`/`k`/`h`/`l` on the canvas are a harmless no-op
because `parent_path([0]) == []` and the inert holder's `direction` is
`FlexBox::window()`'s Column with `[0]` as its only child (AC 7).

### `flex::state` — remove the empty-selection branches

With `selected` never empty:

- `move_key`'s `o`/`O` arm (`state.rs:280`) drops its `if state.selected.is_empty()`
  branches and reduces to the sibling-insert path; `o`/`O` on the canvas is
  already guarded.
- `move_key`'s `d` arm (`state.rs:317`) drops `if !state.selected.is_empty()`
  and just calls `cut_selected`; `d` on the canvas is already guarded.
- `cut_selected` (`state.rs:347`) keeps `selected = parent_path(&selected)`,
  which for a top-level box `[0, i]` is the canvas `[0]` (AC 3).

No other reducer branch changes shape.

### `flex::state` — `outer_boxes`

`outer_boxes` (`state.rs:126`) currently means "top-level user boxes"
(`path.len() == 1`). It is test/driver-only and now returns the canvas's
children:

```rust
pub(crate) fn outer_boxes(&self) -> impl Iterator<Item = &FlexBox> {
    self.boxes
        .children(&[0])
        .into_iter()
        .map(|path| self.boxes.value(&path))
}
```

### `flex::view`

- `scene` (`view.rs:375`) arranges from the canvas, not the inert root:
  `arrange(&state.boxes, &[0], window_rect, &mut rects)`. The canvas is handed
  the whole window rect, so real boxes are laid out exactly as before (AC 9).
- `paint` (`view.rs:268`) currently treats `path.len() == 1` as the "outer"
  box that may show a solid fill. Top-level user boxes are now depth 2, so the
  predicate becomes `path.len() == 2` (the canvas at depth 1 is borderless and
  never reaches the fill branch).
- Nothing else changes: the canvas has `border == false` and `text == None`, so
  `paint` emits no placements for it and it can never show a highlight (AC 8).

### Removed / renamed items

- `new_window` -> `new_canvas`, with the extra holder level.
- The `selected.is_empty()` guards in `move_key` (`o`/`O`, `d`).
- `FlexBox::window()` stays and is now used for both the inert holder and the
  canvas.

### Dependencies / collaborators

- `types::Tree::children` / `value` / `parent` / `next` / `previous` — reused.
- `types::Tree::value_mut` — still the reason the inert root `[]` is safe to
  leave in place.
- `history::recorded` / `undoable` — unchanged; `recorded` is simply not reached
  for canvas no-ops.
- `flex::view::arrange` / `measure` — reused with the new start path `[0]`.
- No new crate dependencies.

### Compatibility note (specs 228, 229, 230)

Spec 228 (merged) documents the canvas as `[]` and top-level boxes as `[i]`.
This refactor supersedes that path convention: **the canvas becomes `[0]` and
top-level boxes become `[0, i]`**. The *behavioral* rules of 228 still hold
(canvas is a Column parent, `J`/`K` on the canvas change nothing), but the guard
becomes `is_canvas` (depth 1) instead of `selected.is_empty()`.

The not-yet-designed paste/yank specs 229/230 say:
- 229: "with the canvas selected, `p` adds the clipboard box as a new top-level
  box and the canvas stays selected" — in the new model this is `p` adding a
  child of `[0]`, i.e. a structural key like `a`, so it must **not** be in
  `canvas_self_edit`.
- 230: "with the canvas selected, `y` changes nothing" — `y` is a self-edit key
  and belongs in `canvas_self_edit`.

Whichever of 229/230 lands first must use `is_canvas` rather than
`selected.is_empty()`.

### Test plan

`flex::state` (update plus new):

1. AC 1/2 — `default_has_a_canvas_with_one_box_selected`: `boxes.value(&[0])`
   is `FlexBox::window()`; there is one child `[0, 0]`; `selected == [0, 0]`;
   `boxes.value(&[])` is the inert holder and `walk()` never yields `[]`.
2. AC 4 — `a_new_canvas_is_a_borderless_column`: the canvas's `border` is false
   and `direction` is `Column`.
3. AC 3 — from a box `[0, 0]`, `d` and Enter-on-empty-borderless both leave
   `selected == [0]` and the canvas still has no children.
4. AC 5 — `a_on_the_canvas_adds_a_child_and_enters_write`; `enter_on_the_canvas_selects_the_first_child`;
   `backspace_on_the_canvas_stays_on_the_canvas`.
5. AC 6 — `self_edit_keys_on_the_canvas_change_nothing`: for each of
   `i, o, O, s, r, f, ], d, J, K`, `boxes`, `selected`, `mode` and `history.len()`
   are all unchanged.
6. AC 7 — `j_k_h_l_on_the_canvas_change_nothing`.
7. Update every existing test helper to the new shape: `holding`/`stacked`/
   `box_with` wrap children in `new_canvas(...)`; `selected` literals gain the
   leading `0` (e.g. `[1]` -> `[0, 1]`, `[0, 1]` -> `[0, 0, 1]`); the
   `outer_boxes` counts stay the same number but now mean canvas children.
8. Move the "canvas is at `[]`" expectations from
   `FlexState::default().selected == [0]` to `[0, 0]`, and the fallback
   expectations from `selected == []` to `[0]`.

`flex::history`:

- Fix `record_pushes_a_snapshot_that_undo_can_restore` (`history.rs:137`), which
  sets `selected = vec![]`; use the canvas `vec![0]` instead.
- Add a test that `recorded` under Move with the canvas selected and a self-edit
  key is never reached, i.e. history does not grow (owned by `flex::state`'s
  guard test above).

`flex::view`:

- Update all fixtures built with `new_window(...)` to `new_canvas(...)`.
- `the_window_root_paints_nothing` -> `the_canvas_paints_nothing`, using
  `selected: vec![0]` and `boxes: new_canvas(vec![])`.
- `only_a_new_box_grows` and the fill tests still pass with the depth-2
  top-level predicate; add `a_top_level_box_shows_its_solid_fill` under the new
  depth to pin `path.len() == 2`.

`flex::mod`:

- `last_text` and the `new_boxes` tests keep working because `outer_boxes` now
  returns canvas children and `new_boxes` excludes the borderless canvas.
