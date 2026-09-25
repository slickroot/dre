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

- No new behavior, and no change to key bindings.
- The technical design is left open for a later session.
