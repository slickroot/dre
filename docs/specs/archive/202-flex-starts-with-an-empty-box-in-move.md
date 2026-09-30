# Flex: dre-flex starts with an empty box in MOVE

## User Story

Lina runs `dre-flex` and sees one empty box, centred on the screen, with no text inside. She's in MOVE. She presses `A`, and a new empty box appears inside it, exactly in the middle, with 1 cell of space between it and the outer box's border on every side. She presses `A` again, and a second inner box appears to the right of the first, with a 1-cell gap between them. There's still 1 cell of space around the pair. She presses `i`, and the outer box gains an empty text beside the inner boxes, and she's in WRITE, typing into it.

## Acceptance Criteria

- When `dre-flex` starts, there is one empty box with no text in it, centred on the screen.
- When `dre-flex` starts, it is in MOVE.
- In MOVE, `A` adds an empty box inside the selected box.
- There is exactly 1 cell of space between the inner box and the outer box's border on the top, bottom, left and right.
- The outer box grows to fit the inner box and stays centred on the screen.
- Each additional `A` adds another inner box to the right of the last one, with a 1-cell gap between them.
- There is still 1 cell of space between the row of inner boxes and the outer box's border on every side.
- In MOVE, `i` on a box with no text adds an empty text to it and switches to WRITE. On a box that already has a text, `i` just switches to WRITE.

## Technical Design

Decisions:

- **Every new box is empty.** `new_box()` returns `Tree::new(FlexNode::Box(FlexBox::default()), vec![])`, with no text child. The starting state, `a` (outer box below) and `A` (inner box) all use it, so there's one kind of new box.
- **`dre-flex` starts in MOVE.** `FlexState::default()` has `mode: FlexMode::Move`. It still holds one box, selected at `[0]`.
- **No padding.** An empty box is only its border: 2 × 2, which is what `measure` already returns for a box with no children (`2 × FLEX_BORDER` each way). The border is thin and drawn at the cell's edge, so the border cells themselves are the "1 cell of space" around the inner boxes. No space is added that isn't taken by a node, so there is no `FLEX_PADDING`, and `measure`, `distribute`, `arrange` and `lay_out_window` don't change. The rest of the criteria already follow from spec 201's layout:
  - One `A`: the outer box measures 4 × 4, and the inner 2 × 2 box sits at `rect.x + FLEX_BORDER`, `rect.y + FLEX_BORDER`.
  - Each further `A`: `distribute` (`Start`) puts the next inner box `FLEX_GAP` to the right of the last one.
  - The outer box grows to fit its children, and `lay_out_window` keeps it centred.
- **`i` makes sure there's a text to type into** (`move_key`). If the selected box has no `Text` child, `i` pushes `Tree::leaf(FlexNode::Text(String::new()))` into it. Either way it switches to WRITE. On a box that already has a text, `i` behaves as it does today. WRITE can only be reached through `i` or `s`, and both leave a text in the selected box, so `selected_text()`'s `expect` still holds.
- **`A` leaves the selection on the outer box**, as it does today. That's why a second `A` adds the next inner box beside the first rather than inside it.

### Collaborators

- `src/flex/state.rs`: `new_box`, `FlexState::default` and `move_key`'s `"i"` change.
- `src/flex/view.rs`: no code change. Only test expectations change.
- `types::Tree`, `view::interior`, `Placement` and `src/flex/mod.rs`: no change.

### Testing plan (TDD, thin slices)

Existing tests that start from `FlexState::default()` and expect WRITE with an empty text **replay the real keys**: `typed(&["i", "H", "e", ...])`. They go through `i` the same way the user does, so there's no fixture for a state the app can only reach through `i`.

1. State: `starts_in_write_mode_with_exactly_one_empty_box` becomes `starts_in_move_with_one_box_and_no_text`. `a_default_box_has_exactly_one_empty_text` becomes `a_new_box_has_no_children`. Replay `"i"` in the typing tests that start from the default.
2. State: `i` on a box with no text adds one empty text and switches to WRITE. `i` on a box with a text adds nothing and switches to WRITE. After `i`, typing goes into the new text.
3. State: `a` and `A` add boxes with no children.
4. View: `an_empty_box_is_three_cells_wide` becomes `an_empty_box_is_two_cells_wide_and_two_high`.
5. View: one `A` makes the outer box 4 × 4, with the inner box at the outer box's `x + 1`, `y + 1`. Two `A`s make it 7 × 4, with the second inner box at the first one's `x + 3`, and the outer box stays centred on the window.
6. View: `a_new_box_measures_around_its_empty_text` becomes `a_new_box_measures_2_by_2`. The spread and column tests that build `new_box()` inner boxes (the `outer(..)` fixture, `spread(..)`, `in_column(..)`) get their numbers updated for a 2 × 2 inner box instead of 3 × 3.

### Out of scope

- Padding inside boxes. It's rejected for now, because no space is added that isn't taken by a node.
- Spec 199's selectable texts, and what `i` does when a text is selected.
