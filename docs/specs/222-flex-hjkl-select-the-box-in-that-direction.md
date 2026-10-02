# Flex: h j k l select the box you see in that direction

## User Story

Doug has three boxes stacked down the screen and a fourth nested inside the top
one. He is on the top box and he wants the one below it. Today `j` means "next
sibling", so he has to hold the shape of the tree in his head and count. He
wants to hold the shape of the *screen* in his head instead: `j` is down, `k` is
up, `h` is left, `l` is right, and each one lands on the box nearest in that
direction. `Enter` goes into a box and `Backspace` comes back out, so descending
is always deliberate even when the geometry would happily do it for him.

## Acceptance Criteria

1. `h`, `j`, `k` and `l` in move mode select the box nearest in that direction.
2. "Nearest" is decided by the top-left corner: the candidate whose corner is
   closest to the current box's corner on that axis wins.
3. A box is only in a direction if it is strictly beyond the current box on that
   axis. A box that shares the same row or column is not to the left or right of
   anything.
4. When two candidates are equally near on the leading axis, the one closest to
   the current box across the other axis wins.
5. When nothing is in that direction, the selection does not move.
6. `k` from a box that has its parent above and to its left, and nothing else
   above it, selects that parent.
7. `j` from a box whose first child is below it and which has nothing else below
   it selects that first child.
8. `Enter` in move mode selects the first child, or does nothing if the box has
   no children. `Backspace` in move mode selects the parent, or does nothing if
   the box is an outer box.
9. `Enter` and `Backspace` are move-mode keys only. Typing mode is unchanged.
10. Navigating after a resize uses the layout at the new size.
11. Moving the selection is not undoable, exactly as it is not today.

## Technical Design

### Layout becomes a value the reducer is handed

The model must ask a geometry question it cannot answer alone, and the wrong
move is to let it try. `FlexState` gains no field. It already knows the tree, the
selection and the mode; geometry is not a fact about the document, it is a fact
about the document *at a size*, and it is derived fresh on every frame anyway.

`run_loop` (`src/flex/mod.rs:59-74`) already renders before it reads a key:

```rust
loop {
    screen.render(&state)?;       // lays out THIS state at THIS window size
    let key = keys.next_key()?;
    (state, effect) = state::reduce(state, &key);
}
```

So at the moment `reduce` runs, the layout for the current state at the current
window size is guaranteed fresh. `RESIZE` calls `screen.resize()` and
`continue`s (`src/flex/mod.rs:62-64`), which loops back through `screen.render`
before any key is read, so a resized terminal cannot hand `reduce` a stale
layout. **That loop order is the reason this design is safe, and it is why the
alternative of caching rects inside `FlexState` is rejected below.**

So `FlexScreen::render` grows a return value:

```rust
fn render(&mut self, state: &FlexState) -> io::Result<Layout>;
```

`run_loop` holds the returned layout in a local and passes it down:

```rust
let layout = screen.render(&state)?;
let key = keys.next_key()?;
(state, effect) = state::reduce(state, &key, &layout);
```

`MockFlexScreen` gets `returning(|_| Ok(Layout::empty()))` and the existing
`run_loop` tests are unaffected apart from that.

### `src/flex/layout.rs`, and why it is a file

Layout is currently private to the view: `Size`, `Rect`, `padding`, `text_size`,
`item_sizes`, `measure`, `distribute`, `share`, `arrange` and `Arranged` occupy
`src/flex/view.rs:19-246`. All of it is a pure function of `(tree, window)`, and
none of it mentions a colour.

`state` needs `Rect` and the answer to "which box is nearest in this direction".
If that lives in `view.rs`, then `state` imports the painter, and the arrow
`view → state` (`src/flex/view.rs:9`) becomes a cycle that spans paint, layout
and key handling. So the file splits:

- **`src/flex/layout.rs`** — `Size`, `Rect`, `measure`, `distribute`, `share`,
  `Layout`, `Layout::arrange`, `Layout::nearest`, `Heading`. Geometry only.
- **`src/flex/view.rs`** — `paint`, `scene`, and the colour constants. It
  consumes a `Layout` instead of building one.

`layout` needs `Direction` and `Justify`, which are model types in `state.rs`, and
`state` needs `Layout::nearest`. That is a module cycle between `state` and
`layout`. Rust permits it (modules are not compilation units) and it is kept
honest by the rule that `layout` owns geometry and `state` owns the tree:
`state` may call exactly one thing on `layout`, which is `nearest`.

### `Heading` is not `Direction`

`Direction` (`src/flex/state.rs:37`) means a box's own flex axis: `Row` or
`Column`. A heading means a screen direction. `Row` is not `Right` and conflating
them would make `nearest`'s signature lie about what it does. So:

```rust
pub(crate) enum Heading { Left, Right, Up, Down }
```

`h`/`j`/`k`/`l` map to `Left`/`Down`/`Up`/`Right` at the `move_key` call site.

### `nearest`: corners, strictly, with a cross-axis tie-break

```rust
impl Layout {
    pub(crate) fn nearest(&self, from: &[usize], heading: Heading) -> Option<Vec<usize>>;
}
```

`Layout` is `Vec<Arranged>` plus the `Area` it was arranged into. It does **not**
hold the tree, and it does not need to: every comparison here is between `Rect`s
and between `path`s, and a path is enough. The candidate set is **every arranged
node except the one named by `from`** — no ancestor filter, no descendant filter.

```rust
fn nearest(&self, from: &[usize], heading: Heading) -> Option<Vec<usize>> {
    let current = self.rect(from)?;
    self.arranged
        .iter()
        .filter(|a| a.path != from)
        .filter(|a| is_beyond(a.rect, current, heading))
        .min_by_key(|a| (leading_gap(a.rect, current, heading), cross_offset(a.rect, current, heading)))
        .map(|a| a.path.clone())
}
```

`is_beyond` is strict — `Down` needs `r.y > c.y`, `Up` needs `r.y < c.y`,
`Right` needs `r.x > c.x`, `Left` needs `r.x < c.x`. `leading_gap` is the
distance on the leading axis, so `min_by_key` takes the nearest corner;
`cross_offset` is `|r.x - c.x|` for vertical headings and `|r.y - c.y|` for
horizontal ones, so criterion 4 falls out of the tuple ordering and needs no
separate branch. `Vec<Arranged>` is in `walk()` order (`types/src/tree.rs:88`,
pre-order), and `min_by_key` keeps the first of equal elements, so the ordering
is total and the tests are deterministic without a third tie-break.

**Strictness matters and is not an edge case.** A borderless box insets its
children by nothing (`padding` returns `(0, 0)` when `!flex_box.border`,
`src/flex/view.rs:55-67`), so a borderless container's first child is laid out on
*exactly* its parent's rect. Under `>` / `<` neither can select the other on any
key, which is the right outcome and arrives for free.

### No ancestor filter, no descendant filter

This is the decision the playground disagrees with, so it is worth recording with
numbers. `A` twice from the default state, arranged at 40×20:

```
[]    x:0  y:0   w:40 h:20
[0]    x:0  y:6   w:40 h:7
[0,0]  x:2  y:7   w:36 h:2      ← its own first child, one cell below
[0,1]  x:2  y:10  w:36 h:2
```

`[0,0]` is exactly one cell below `[0]`, and `[0]` is exactly one cell above
`[0,0]`. Parent and child are mirror images at equal distance, so no geometric
rule can distinguish them — any filter has to be policy, and the policy here is
**none**. `k` from `[0,0]` selects `[0]` because `[0]` is nearer on that axis,
which is exactly what `h` does today (`src/flex/state.rs:224`). `j` from `[0]`
selects `[0,0]` for the same reason, and `Enter` is still there when Doug wants to
descend on purpose.

The playground filters both (`dre-playground.html:190-192`) because a DOM box can
be positioned anywhere relative to anything and it needed a `1e5 + gap + cross`
sentinel to make overlap dominate distance. dre-flex generates its own geometry:
children are strictly inset from bordered parents by at least `(2, 1)`
(`FLEX_SPACE`, `src/flex/view.rs:12-15`), siblings are stacked by `distribute`
with a non-zero gap, and rows share width via `share`. There is nothing for a
heuristic to defend against, so the heuristic goes and the corner comparison is
the whole rule.

### Enter and Backspace stay structural

They never needed geometry, so they keep using the tree directly:
`Tree::child` (`types/src/tree.rs:23`) and `Tree::parent` (`:38`). Both are
already no-ops at their limits — `child` returns the same path for a childless
box, and `parent` returns the same path for an outer box — so criterion 8 needs no
guarding code. `Backspace` consequently never selects the window box at `[]`.

Both keys are currently unbound in move mode: `"\r"` and `"\x7f"` appear only in
`write_key` (`src/flex/state.rs:159-190`), and `move_key` (`:211`) falls through
to `_ => {}`. Neither is in `history::undoable`'s whitelist (`src/flex/history.rs:14-22`),
so neither is recorded, which satisfies criterion 11 for free. The existing test
`selection_undo_and_move_keys_are_not_undoable` (`src/flex/history.rs:50`) already
lists `h`, `j`, `k`, `l` and stays true.

### Collaborators and responsibilities

**`Layout`** (`src/flex/layout.rs`) — knows the window it was arranged into and
one `Rect` per node. Answers "where is the box nearest in this direction". Knows
nothing about `FlexBox`, the tree, selection, or colour.

**`view::scene`** (`src/flex/view.rs:306`) — takes `&Layout` instead of arranging,
so painting and hit-testing agree by construction: the rect that decides `j` is
the rect that draws the box.

**`TerminalFlexScreen::render`** (`src/flex/mod.rs:28`) — arranges once from
`self.renderer.area()`, hands the layout to `view::scene`, and returns the same
layout so `run_loop` can pass it to `reduce`. One arrangement per frame, exactly
as today.

**`move_key`** (`src/flex/state.rs:211`) — maps the four keys to `Heading` and
assigns `state.selected` from `nearest`. Four arms, no scoring logic. Everything
else in `move_key` is untouched.

**`history`** — untouched. `Snapshot` (`src/flex/history.rs:5`) keeps storing a
`selected` path, which is still what state holds.

### Existing tests

The `typed` (`:469`) and `moved` (`:945`) helpers absorb the new parameter:
`moved` arranges at a fixed 40×20 `Area` and threads the resulting layout through
the fold, so every existing test keeps its current call shape and the nav tests
now exercise real geometry rather than a stand-in.

Tests that pinned the *old* meaning of `l` and `h` change meaning and are
re-pointed at `"\r"` and `"\x7f"`, which now own first-child and parent —
`l_on_a_box_with_children_goes_to_the_first_child` and
`h_goes_to_the_parent_and_stays_when_already_outer` among them. The `j`/`k` tests
that happen to still hold — `j_and_k_after_capital_a_only_move_between_outer_boxes`
(`:1131`), for instance — are left alone and now pass for a new reason, which is
fine; they assert the selected path, not the rule that produced it.

### Tests

Framework is inline `#[test]` with plain `assert!`/`assert_eq!`, no snapshots.

In `src/flex/layout.rs`, on hand-built rects so the rule is tested without running
layout at all:
- `down_picks_the_box_whose_top_left_corner_is_lowest`
- `up_picks_the_box_whose_top_left_corner_is_highest`
- `left_and_right_pick_on_the_x_corner`
- `a_box_level_with_the_current_one_is_not_to_its_left_or_right`
- `an_equal_leading_corner_is_broken_by_the_closest_across_axis`
- `nothing_in_that_direction_selects_nothing`
- `a_descendant_below_is_picked_by_down`
- `an_ancestor_up_and_left_is_picked_by_up`
- `the_root_is_a_candidate_like_any_other_box`
- `a_missing_from_path_selects_nothing`

In `src/flex/view.rs`, against real `arrange` output, for the geometry the rule
depends on:
- `a_bordered_box_insets_its_children_by_at_least_one_row`
- `a_borderless_box_lays_its_first_child_on_its_own_rect`

In `src/flex/state.rs`:
- `j_selects_the_box_below_and_k_the_box_above`
- `l_and_h_walk_across_and_back_by_corner`
- `j_with_nothing_below_leaves_the_selection_alone`
- `k_from_a_top_level_child_selects_its_parent`
- `enter_selects_the_first_child_and_stays_on_a_childless_box`
- `backspace_selects_the_parent_and_stays_on_an_outer_box`
- `enter_and_backspace_type_themselves_in_write_mode`
- `moving_the_selection_is_not_undoable`

In `src/flex/mod.rs`:
- `the_layout_handed_to_reduce_matches_the_one_that_was_drawn` — a `Sequence`
  of two `expect_render` calls whose first return value feeds an
  `expect_next_key`, pinning that the layout `reduce` receives came from the
  frame that just drew
- `a_resize_re_arranges_before_the_next_key_is_reduced`

### Rejected

- **Caching rects in `FlexState`** — a derived field with a staleness invariant,
  an `eq` carve-out on top of `PartialEq` (`src/flex/state.rs:106`), and a
  recompute after every `undo` since `Snapshot` holds only `boxes` and `selected`.
  All of that to avoid passing one argument.
- **A list of spatial moves instead of `selected`** — a move has to be validated
  before it is committed (`j` on the last box leaves you put,
  `src/flex/state.rs:1229`), so the rects are needed anyway; and `A`, `a`, `o`
  and `Enter` each restructure the tree, so a positional description would have to
  be re-derived after every one of them. It also retires `selected`, which has 201
  references in `src/flex/state.rs`.
- **Assuming a canonical 80×24 window** — keeps `reduce`'s signature but navigates
  a fiction. `Rect::inner` (`src/flex/view.rs:135`) goes negative when a box is
  too small for its content, so a box can be off-screen and still be the nearest
  corner in the fiction.
- **`state` calling into `view`** — a cycle spanning painting, layout and key
  handling, rather than the narrow `state ↔ layout` cycle this takes.
- **Copying the playground's overlap heuristic** — `1e5 + gap + cross` exists to
  make overlap dominate distance among boxes a browser placed arbitrarily. See
  "No ancestor filter, no descendant filter".