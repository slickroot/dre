## Story

Adding the name prompt (spec 137) touched 9 files for one small feature. Almost all of it was copying what `SavePrompt` already does: a mode, five actions, a parser, a reducer file, a history entry, and a footer refresh. Doug won't see any difference, but the next prompt (a search, a rename, a goto) would cost the same again. dre should have one text prompt that the save prompt and the name prompt both use, so a new prompt is a small change in few files.

## Problem

See the diff of PR #155 (https://github.com/slickroot/dre/pull/155, spec 137) for a concrete example: one small feature touched 9 files.

- **Actions are duplicated per prompt.** `Action::mode()` maps each action to one reducer statically, so `Confirm` (which quits, for the save prompt) can't be shared with the name prompt. Each prompt needs its own append, backspace, confirm and cancel actions, its own parser, its own reducer, and its own entries in the non-undoable list in `history.rs`.
- **The footer is stored state that must be refreshed by hand.** The name prompt calls `refresh_footer()` from two reducers. Any new place that changes the mode or `save_to` can forget to call it. The footer is derived from `mode` and `save_to`.
- **One command key touches three places:** the parser, `COMMAND_KEYMAP` and the README keymap table. The sync tests enforce it. Decide whether that is fine or should change.

## Acceptance Criteria

- Nothing Doug can see or do changes. The save prompt (`q`) and the name prompt (`n`) behave exactly as before, and all existing tests for them still pass, adapted only where types moved.
- The save prompt and the name prompt share one prompt mode, one set of prompt actions, one parser and one reducer. What differs between them (what Enter does, the extension, empty Enter, quitting) is expressed in one place per prompt.
- Adding a further text prompt touches no more than about 3 files, not counting tests.
- The footer is computed from the state when it is read, not stored and refreshed.

## Out of scope

- No change to key bindings.
- Behaviour changes agreed in the Technical Design below (no default pre-filled filename; `q`'s prompt becomes visible while typing) are in scope; everything else about `q` and `n` stays as it is.

## Technical Design

### One mode, keyed by what happens when it closes

`Mode::SavePrompt { filename }` is deleted. `Mode::NamePrompt` becomes the only text-prompt mode and gains one field:

```rust
Mode::NamePrompt { name: String, quits: bool }
```

`quits` is the one thing that differs between the two entry points:

- `n` (`open_name_prompt`) opens `Mode::NamePrompt { name: String::new(), quits: false }`. Confirm/Cancel return to `Mode::Command`.
- `q` (`quit`), when there's no `save_to` yet, opens `Mode::NamePrompt { name: String::new(), quits: true }` instead of the old `Mode::SavePrompt`. Confirm/Cancel set `running = false`.

`quit()`'s other two branches (new file with no boxes; already has `save_to`) are unchanged — they still bypass the prompt entirely.

`DEFAULT_FILENAME` is deleted. `q` no longer pre-fills a filename; the user must type one, exactly like `n` already requires a non-empty name. This is a deliberate, visible behaviour change agreed for this refactor.

### One set of actions, one parser, one reducer

- `Action::Confirm`, `Action::Cancel`, `Action::SavePromptBackspace`, `Action::SavePromptAppend` are deleted. `Action::NameAppend`, `NameBackspace`, `NameConfirm`, `NameCancel` are the only prompt actions, used by both flows (names are kept as-is per team preference, even though they now also serve the save flow).
- `ActionMode::SavePrompt` is deleted; `ActionMode::NamePrompt` is the only prompt `ActionMode`.
- `input.rs`: `save_prompt_parse` is deleted. The `Mode::NamePrompt { .. } => name_prompt_parse(key)` arm in `parse()` covers both flows unconditionally.
- `save_prompt.rs` is deleted. `name_prompt.rs`'s `reduce` becomes the single prompt reducer:
  - `NameConfirm` requires `!name.is_empty()` (blocks empty in both flows — forces a name on `q` too, no more saving to `.dre`).
  - On confirm, the extension is appended unconditionally (`format!("{name}{EXTENSION}")`, no "already has it" check) — same rule for both flows.
  - After confirm, and after `NameCancel`: `if quits { state.running = false } else { state.mode = Mode::Command }`.
  - `NameBackspace`/`NameAppend` are unchanged.

### Footer: computed, not stored

The `footer: String` field on `State` and `refresh_footer()` are deleted. `footer()` becomes a plain computed method, read fresh every call:

```rust
pub(crate) fn footer(&self) -> String {
    match &self.mode {
        Mode::NamePrompt { name, .. } if name.is_empty() => format!("{PLACEHOLDER}{FOOTER_SUFFIX}"),
        Mode::NamePrompt { name, .. } => format!("{name}{FOOTER_SUFFIX}"),
        _ => footer_text(self.save_to.as_deref()),
    }
}
```

`set_save_to()` no longer writes to a footer field — it only sets `save_to`. No reducer needs to remember to refresh anything; whichever `mode`/`save_to` is current when `footer()` is called produces the right text.

### Rendering

`render::editor`'s cursor special-case already keys off `Mode::NamePrompt { name, .. }` (now `{ name, .. }` to ignore `quits`) and applies unconditionally to both flows. This means `q` starts showing the live footer text and cursor while typing, where today it shows nothing — agreed as a welcome side effect, not a regression to guard against.

### History

`is_undoable` in `history.rs` collapses to the one existing `Name*` match arm; the now-deleted `Confirm`/`Cancel`/`SavePrompt*` variants are gone so there's nothing left to list for them.

### Net effect on the "touches 3 files" criterion

A further text prompt (search, goto, rename) that only needs "quit or return to Command" on close fits the existing `quits: bool` — no new field needed, only a new call site setting `Mode::NamePrompt { name: ..., quits: ... }` (or a third `Mode` variant if its behaviour differs from a bool split, decided when it's built). Touches: the place that opens it (`command.rs` or similar), `input.rs`'s command parser for its key, and README's keymap table (already enforced by the sync test).
