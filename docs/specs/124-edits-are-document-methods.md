# Edits are Document methods

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

The Command-mode edits (`cycle_colour`, `toggle_fill`, `delete_box`, `paste_box` and others) and the helpers in `state.rs` (`colour_row`, `add_child_box`, `blank_box`, `next_colour`) walk and change the drawing themselves.

## Acceptance Criteria

- Every change to a drawing is a named `Document` method taking a `&[usize]` path.
- The reducers compute the new selection from `Tree` navigation and call those methods.
- The clipboard is `Option<Tree<Node>>`. `Document::remove` returns the detached subtree and `Document::paste` puts a clone back.
- No module outside `diagram.rs` touches `.children` or the boxes.

## Technical Design

Method list and clipboard type are fixed in spec 117 (`diagram.rs`). The exact method signatures are still to be designed before implementation.
