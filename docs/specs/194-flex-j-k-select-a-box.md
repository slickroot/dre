# Flex: j and k select a box to style in MOVE

## User Story

Noor has three boxes stacked in `dre-flex`: "Hello", "World" and "Again". They're in MOVE, and the last box they added, "Again", has a `#8AB4F8` border. Noor presses `k` twice, and the blue border moves up to "Hello". They press `w`, and "Hello" goes full width. Then they press `f`, and "Hello" gets filled. The other boxes stay as they were. Noor presses `j`, and the border moves down to "World". They press `a`, and a new empty box appears at the bottom of the stack with the blue border. When Noor presses `i` to type, the blue border goes away.

## Acceptance Criteria

- In MOVE, the selected box has a `#8AB4F8` border.
- In WRITE, no box has the `#8AB4F8` border.
- In MOVE, `j` selects the box below and `k` selects the box above.
- With the bottom box selected, `j` keeps the selection there. With the top box selected, `k` keeps the selection there.
- `w`, `f`, `g`, `s` and `i` act on the selected box. The other boxes don't change.
- `a` adds the new box at the bottom of the stack, and the new box becomes the selected one.
- In WRITE, `j` and `k` type a "j" and a "k" and don't change the selection.

## Technical Design
