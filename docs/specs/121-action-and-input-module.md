# Action and input.rs

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

`state.rs` imports `command_mode`, `insert_mode` and `save_prompt_mode` to dispatch keys, and each of them imports `State` back. Each mode module both parses keys and changes state, and `handle_key` holds Command-mode logic inline (digit counting, the colour overlay).

## Acceptance Criteria

- An `Action` type describes every change a key can ask for.
- `input.rs` maps a key and a read-only `&State` to an `Option<Action>`, dispatching on the mode. The keymaps and `parse` functions move there.
- The digit-count and colour-overlay branches of `handle_key` become Command-mode actions.
- `state.rs` no longer imports any mode module or `input`. There are no dependency cycles.
- No behaviour changes.

## Technical Design

Direction is fixed in spec 117 (`input.rs`). The `Action` variants and the `reduce` signature are still to be designed before implementation.
