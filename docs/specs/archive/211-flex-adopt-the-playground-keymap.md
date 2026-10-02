# Flex: adopt the playground keymap

## Refactoring

Move-mode keys in `dre-flex` change to match the playground keymap. This frees `d`, `y` and `p` for the clipboard later:

| Action                     | Old key | New key |
|----------------------------|---------|---------|
| Add a text beside the last | `s`     | `o`     |
| Spread texts (justify)     | `g`     | `s`     |
| Toggle row/column          | `d`     | `r`     |
| Add side padding           | `p`     | `]`     |
| (nothing yet: clipboard)   | —       | `d` `y` `p` |

Nothing else changes in behaviour. Write mode still types every printable key, including `o`, `s`, `r`, `]`, `d`, `y` and `p`.

## Technical Design

All changes are in `src/flex/state.rs`, `src/flex/history.rs` and the tests in `src/flex/view.rs`. Only the keys change, not the layout or the state shape.

### State (`state.rs`)

- In `move_key`, these match arms get new keys and keep their bodies:
  - `"s"` (push an empty text, enter write) → `"o"`
  - `"g"` (toggle `justify`) → `"s"`
  - `"d"` (toggle `direction`) → `"r"`
  - `"p"` (add `padding`) → `"]"`
- `d`, `y` and `p` get no arm and fall into `_ => {}`, so in move mode they do nothing.
- `write_key` doesn't change.

### History (`history.rs`)

- `undoable` becomes `matches!(key, "a" | "A" | "o" | "i" | "s" | "r" | "f" | "]")`.
- `d`, `y` and `p` are not undoable, so pressing them leaves no empty undo step.

### Tests

Rename and re-key the existing tests. Their assertions don't change.

- `state.rs`:
  - `s_*` tests become `o_*`, pressing `"o"`.
  - `g_*` tests become `s_*`, pressing `"s"`.
  - `d_*` tests become `r_*`, pressing `"r"`.
  - `p_*` tests become `close_bracket_*`, pressing `"]"`.
  - Do this in that order (s→o first, then g→s) so the two `s` sets don't get mixed up.
  - Also re-key any other test whose key sequence uses these keys in move mode, for example the `moved(..., &["s", "!"])` helper calls.
- `view.rs`:
  - `g_spreads_hello_…` becomes `s_spreads_hello_…`.
  - `d_on_the_parent_…` becomes `r_on_the_parent_…`.
  - The `p_*` padding tests become `close_bracket_*`.
  - Re-key the key sequences too: in move mode, `"s"` → `"o"`, `"g"` → `"s"`, `"d"` → `"r"`, `"p"` → `"]"`. Keys typed in write mode stay as they are (for example the `"W", "o", "r", "l", "d"` after an `o`).
- `history.rs`:
  - The undoable list becomes `["a", "A", "o", "i", "s", "r", "f", "]"]`.
  - `d`, `y` and `p` join the not-undoable list in move mode.
  - The write-mode list gains `"o"`, `"r"`, `"]"` and `"y"`.

New tests:
- `state.rs`: in move mode, `d`, `y` and `p` each leave the state unchanged and return no effect. This test is meant to break when the clipboard spec gives them a job.
- `state.rs`: in write mode, `]` is typed into the box and leaves `padding` at 0. This replaces the old `p`-in-write test, and a `p`-typed-in-write-mode check stays too.
