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

Direction is fixed in spec 117: key → `Action` → `reduce(State, Action) -> State`
→ view. This spec builds the first two arrows and takes the mode imports out of
`state.rs`.

### Modules

| Module | Is | Imports |
|---|---|---|
| `action.rs` (new) | the vocabulary: everything a key can ask for | nothing |
| `input.rs` (new) | which key asks for what: `parse(&State, key) -> Option<Action>` | `action`, `state` (read only) |
| `reduce.rs` (new) | `reduce(State, Action) -> State`, a thin dispatcher | `action`, the three mode modules |
| `command_mode.rs`, `insert_mode.rs`, `save_prompt_mode.rs` | what each action does: `reduce` only, no keymap, no `parse` | `action`, `state` |
| `state.rs` | the session: `State`, `Mode`, helpers | `diagram`, `palette`. No mode module, no `input`, no `reduce` |
| `editor.rs` | the loop: the only caller of `input::parse` and `reduce` | everything |

Arrows: `editor` → `input`, `reduce`, `action`. `input` → `action`, `state`.
`reduce` → `action`, the mode modules → `state`. Nothing points back at `state`,
so there are no cycles.

### `Action`

```rust
pub(crate) enum Action {
    Command(CommandAction),
    Insert(InsertAction),
    SavePrompt(SavePromptAction),
}
```

Nested per mode, not flat: `Backspace` and `Append(char)` exist in both Insert
and SavePrompt with different meanings, and the existing per-mode reducers
already take one enum each. The three enums move from the mode modules into
`action.rs` and are renamed `CommandAction`, `InsertAction`, `SavePromptAction`.
Their variants are unchanged, except for two new `CommandAction` variants:

- `Digit(u8)`: a count digit `0`-`9`. `reduce` accumulates `pending_count`
  (saturating, as today). If `colour_overlay` is set, it then takes the count,
  clears the overlay and, for `1..=7`, snapshots and sets the colour. This is
  the digit and colour-overlay logic that `handle_key` holds inline today.
- `CancelCount`: any key Command mode does not recognise. `reduce` clears
  `pending_count`. It keeps today's behaviour, where `3` then `x` drops the
  count. Without it, `parse` returning `None` would leave the count set.

`ScrollBy(i64)` stays in `CommandAction`.

### `input.rs`

- `parse(&State, key) -> Option<Action>` matches on `state.mode()` and calls
  `command_parse`, `insert_parse` or `save_prompt_parse`. These are the old
  `parse` functions and keymaps, moved.
- `command_parse` always returns `Some`: a keymap hit, `Digit(u8)` for a digit,
  otherwise `CancelCount`. `insert_parse` and `save_prompt_parse` return `None`
  for keys they ignore, as today.
- The `\x1bSCROLL` prefix check is deleted. No key string is synthesized any
  more.
- It reads `State` only for the mode. It never writes it.

### `reduce.rs`

- `reduce(mut state, action) -> State`. First it restores `last_selected` (the
  idle-cursor restore that `handle_key` does today), then dispatches on the outer
  `Action` variant to the mode's `reduce`.
- Dispatch follows the `Action`, not `state.mode`. This keeps `ScrollBy` working
  in every mode, as the special case at the top of `handle_key` does today,
  without a special case.
- The mode reducers keep their existing guards (for example
  `let Mode::SavePrompt { .. } = .. else { return state }`).

### `editor.rs`

```rust
Some(key) => {
    if let Some(action) = input::parse(&state, &key) {
        state = reduce(state, action);
    }
    // scroll follow-up
    if let Some(delta) = overflow_delta(..) {
        state = reduce(state, Action::Command(CommandAction::ScrollBy(delta)));
    }
}
```

- `editor.rs` builds `ScrollBy` directly. It no longer formats
  `"\x1bSCROLL{delta}"` and sends it back through key handling.
- `handle_key` is deleted. `state::hide_idle_cursor` stays where it is.
- `editor.rs` still writes `save_to = None` on interrupt. Turning that into an
  `Action` belongs to the `reduce`/`State` work in spec 117, not to this step.

### Tests

- Tests of the `parse` functions and keymaps move with them to `input.rs`.
- Tests that go through `handle_key` (counts, scroll, idle hide and restore)
  call `input::parse` then `reduce`, through a small test helper.
- New tests: `parse` returns `Digit(u8)` for a digit and `CancelCount` for an
  unknown Command-mode key. `reduce` applies the colour overlay through `Digit`.
  `ScrollBy` takes effect in Insert mode.
- The existing tests are the no-behaviour-change check. They must pass with
  their assertions unchanged.
