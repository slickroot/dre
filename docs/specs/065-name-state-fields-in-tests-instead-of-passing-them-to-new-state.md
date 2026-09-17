# 065 - Name state fields in tests instead of passing them to `new_state`

## Technical Design

A refactoring with no change in behaviour.

### Problem

Spec 060 added `save_to` to the test helper
`state::new_state(boxes, mode, selected, save_to)`. That changed every caller,
and most of them now end in a `None` that doesn't say what it's for:

```rust
let state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }), None);
```

Spec 061 then added `State::new_file` without a parameter. Tests set it on
the returned state (`command_mode::new_file_state`). Two styles now exist, and
the parameter style would make every new field change every test again.

### `state::new_state`

- Goes back to `new_state(boxes: Vec<Node>, mode: Mode, selected: Option<Path>) -> State`,
  with `save_to: None` and `new_file: false`.
- A new `State` field never becomes a parameter. `new_state` fills it with its
  default.
- `history` stays private. The helper stays in `state.rs` because other modules
  can't write `..State::default()` while a field is private.

### Tests that need another field

They set it by name on the returned state, as `new_file` already is:

```rust
let mut state = new_state(vec![node("a")], Mode::Command, Some(Path { ancestors: vec![], index: 0 }));
state.save_to = Some("plans.dre".to_string());
```

### Changes

- `command_mode`, `insert_mode`, `save_prompt_mode`, `state`, `writer`: callers
  drop the trailing `None`.
- `command_mode` callers that pass `Some(..)` set `save_to` on the returned
  state instead:
  - `q_with_a_file_to_save_to_stops_without_prompting`
  - `new_file_state`
  - `q_on_an_existing_file_with_no_boxes_keeps_its_file_to_save_to`
- No test is added, removed or weakened. `cargo test` passes before and after.
