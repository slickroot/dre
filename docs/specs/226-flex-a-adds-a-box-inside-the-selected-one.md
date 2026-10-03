# Flex: `a` adds a box inside the selected one

## User Story

Doug is in move mode with a box selected. He presses `a`. A new empty box
appears inside the selected box, and Doug is now in write mode, ready to type
into it.

## Acceptance Criteria

1. In move mode, `a` adds a new empty box inside the selected box, as its last
   child.
2. The selection moves to the new box.
3. After `a`, dre-flex is in write mode and typing goes into the new box.
4. `a` no longer adds a box at the root.
5. `A` is ignored completely: it changes neither the diagram nor the undo
   history.
6. In write mode, `a` still types an "a".
7. `a` creates a child inside whatever is selected, box or text.
8. Adding with `a` is undoable.

## Technical Design

Decisions:

- **`a` is the "nest a box" move key.** In MOVE, `a` pushes a new bordered
  box as the last child of the selected node, selects that new child, and
  switches to WRITE. This replaces the old root-level `a`.
- **There is no text guard.** `a` acts on whatever is selected, borderless
  text included. The old "a text does nothing" criterion is dropped (AC 7
  above is reworded accordingly).
- **The new box is a fresh default with an empty text slot.**
  `FlexBox { text: Some(String::new()), ..FlexBox::default() }`: bordered,
  Column, Start, unfilled, no padding. The empty text is what lets WRITE type
  immediately and draw the caret. Leaving WRITE later (`\r`/`\x1b`) drops the
  empty text but keeps the bordered box, because `is_droppable` is false for a
  bordered box.
- **The story's "pressing `a` again" is loose wording.** After the first `a`
  we are in WRITE, so a second `a` types an "a" (AC 6). To nest again you
  leave WRITE first, then press `a` in MOVE.
- **`A` loses its MOVE binding and its history entry.** In MOVE it falls
  through to the no-op arm: it changes neither the diagram nor the undo
  history. In WRITE it is still an ordinary printable character and types
  "A" (AC 5 is about MOVE).
- **Selection and undo come for free.** `Tree::push` returns the new child's
  path, which becomes `state.selected`; `history::recorded` already snapshots
  `a` in MOVE, so the add is undoable in one step.

### `flex::state`: what it knows and does

Add a constructor next to `new_box` / `new_text`:

```rust
pub(crate) fn new_child_box() -> Tree<FlexBox> {
    Tree::leaf(FlexBox {
        text: Some(String::new()),
        ..FlexBox::default()
    })
}
```

`move_key`:

```rust
"a" => {
    state.selected = state.boxes.push(&state.selected, new_child_box());
    state.mode = FlexMode::Write;
}
```

- Delete the `"A"` arm.
- `write_key`, `reduce`, the `FlexState` fields and `Tree` are unchanged.

### `flex::history`

- Remove `"A"` from the `FlexMode::Move` arm of `undoable`. `"a"` stays, so
  the add is recorded and `u` restores the previous tree and selection.

### `flex::view`

- No production change. The new box is an ordinary bordered box with
  `text: Some("")`, already measured and painted, and WRITE already draws the
  typing caret on the selected node.

### Tests

`flex::state`:

- MOVE `a` on a selected box holding "Hello": the new box is at `[0, 0]`,
  bordered, `text == Some("")`, no children; the parent is unchanged; mode is
  WRITE; effect None.
- MOVE `a` on a selected borderless text nests the box inside the text (no
  guard).
- `a` appends as the last child when the selected box already has children.
- `a` selects the new child and typing after it lands in that child.
- In WRITE, `a` types "a" and adds no box.
- `a` no longer changes the outer-box count or adds at the root.
- `A` in MOVE leaves the tree and the history length unchanged; in WRITE it
  types "A".
- Undo after `a` (leave WRITE with `\x1b`, then `u`) restores the tree and
  selection.
- Rework the existing `a_*` and `capital_a_*` tests: the ones that assert a
  root box or that MOVE is kept after `a` now assert an inner box plus WRITE,
  and the ones that exercise `A` as a command become no-op assertions.

`flex::history`:

- `"A"` is no longer undoable in MOVE; `"a"` still is. Update the
  `box_text_and_toggle_keys_are_undoable_in_move_mode` and
  `selection_undo_and_quit_keys_are_not_undoable_in_move_mode` lists.

`flex::view` / `flex::mod`:

- Fixtures that drive `["A"]` or `["A", "A"]` to build an inner box switch to
  `["a", "\x1b"]` (nest, then back to MOVE) or build the tree directly.
- `new_boxes` still marks the `a`-added box as new once: it is the selected
  bordered path. The undo test must leave WRITE before pressing `u`.
