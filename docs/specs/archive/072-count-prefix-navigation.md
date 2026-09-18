# 072: Count Prefix Navigation

## User Story

I can type a number before `j`, `k`, or `h` to move that many steps at once,
instead of tapping the key over and over.

## Acceptance Criteria

- `j` on its own still moves to the next sibling — it's the same as `1j`. Same for `k` and `h`
- `3j` moves the selection 3 siblings forward; if fewer than 3 siblings remain below, it stops on the last sibling
- `3k` moves the selection 3 siblings back; if fewer than 3 remain above, it stops on the first sibling
- `3h` moves the selection up 3 levels; if there aren't 3 levels, it stops at the top
- Multi-digit counts work: `12j` moves 12 siblings forward
- A count prefix followed by any other command (e.g. `2s`) ignores the count and behaves exactly as it does today

## Technical Design

### Count state (`src/state.rs`)
- Add a private `pending_count: Option<usize>` field on `State` (default `None`), like the private `history` field: never serialised, lives only across command-mode keystrokes.
- In `state::handle_key`'s `Mode::Command` branch (`src/state.rs:145`), before `command_mode::parse`:
  - a single digit (`"0"`..`"9"`) accumulates saturatingly — `Some(pending.unwrap_or(0).saturating_mul(10).saturating_add(digit))` — and returns early. `parse` is never called and no snapshot is taken.
  - any other key takes the parse path as today; when `parse` returns `None` the pending count is still cleared (reset-on-every-non-digit-key).
- `command_mode::parse` is untouched: digits still map to `None`, so `command_keymap_agrees_with_parse` (`src/command_mode.rs:393`) stays green.

### Application (`src/command_mode.rs`)
- `reduce` reads `pending_count` at the top — `let count = pending.unwrap_or(1)` — and immediately clears the field. Every parsed command therefore consumes the prefix (`2s` behaves exactly as `s`), including commands below their `min_depth` (e.g. `3j` on an empty canvas discards the count and changes nothing).
- The `SelectNext`/`SelectPrevious`/`SelectParent` arms run their existing 1-step reducers `count` times. Each already clamps at the boundary (`select_next` `src/command_mode.rs:174`, `select_previous` `:183`, `select_parent` `:160`), so `3j` with fewer than 3 siblings below stops on the last, and `3h` stops at the top.
- Zero count = no-op: the loop runs zero times and the arm re-selects the original path, so selection is preserved.
- Movement stays non-undoable; the digit branch never snapshots, so undo history is unaffected.

### Mode isolation
- Digits accumulate only in command mode. Insert mode keeps typing them into labels and `SavePrompt` into the filename — no change to those modules.

### UI / README
- Silent: no status echo, no render/layout/writer changes. The README keymap table and its sync test are unchanged (no new keys).

### Known limitation (follow-up story, out of scope)
- The movement loop runs `count` times without a cap and the accumulator saturates to `usize::MAX`, so an absurd prefix (e.g. `999…9j`) can iterate `usize::MAX` times. Selection stops changing at the boundary, but the app can hang. A follow-up story should cap the loop at the reachable bound (`min(count, siblings remaining)` / `min(count, ancestors.len())`).

### Test criteria
- digits accumulate across keystrokes: `3`, `2`,`j` moves 32 siblings (clamped at the end)
- repeated digits saturate at `usize::MAX` without panic
- `3j` moves 3 forward, clamped to the last sibling; `3k` clamped to the first; `3h` climbs at most to the top-level box
- `j` == `1j`; multi-digit `12j` works
- `0j`/`0k`/`0h` leave selection unchanged
- `2s` behaves exactly as `s` and the prefix is consumed (next `j` moves one step)
- `3` then an unknown key (`x`) resets the count; next `j` moves one step
- `3j` with no selection leaves the document unchanged and consumes the count
- insert mode still types digits as label text (no regression)
- `new_state` test helper (and any struct literal) gains `pending_count`