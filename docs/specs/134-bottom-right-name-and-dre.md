## Story

Doug opens `docs/plans.dre` in dre. In the bottom-right corner he reads `plans • dre`. Later he starts a brand-new diagram that has no file yet, and the corner reads `[no name] • dre`. Happy, he always knows which diagram he's in.

## Acceptance Criteria

- With a file, the bottom-right corner reads `<name> • dre`, where `<name>` is the file name without the folder and without `.dre`.
- With no file, the bottom-right corner reads `[no name] • dre`.
- There are no borders in this card.

## Technical Design

### Decisions

- A label borrows its text (`Label.text: &'a str`), so the composed `<name> • dre` needs an owner that outlives the placements. That owner is `State`. `Label` does not change.
- `State` gets a private `footer: String`. It is derived from `save_to`, so it is never written on its own.
- `save_to` becomes private. `State::set_save_to(Option<String>)` is the only writer: it sets `save_to` and recomputes `footer`. The constructors (`open`, `new_file`, `Default`) go through the same computation, so `footer` is right from the start.
  - With a path: `<name> • dre`, where `<name>` is the file name without the folder and without `.dre`.
  - With `None`: `[no name] • dre`.
- A read-only `State::save_to() -> Option<&str>` serves the callers that only read it.
- `diagram_name` becomes a private helper used by `set_save_to`. It is no longer a public getter.
- `State::footer() -> &str` is the getter for the renderers.
- `layout::footer(text: &str)` always returns one label with that text, as wide as the text and one row high. It no longer takes an `Option`, and "nothing when unnamed" goes away.
- `render::editor` stays the one place that composes the screen: `align_right(layout::footer(state.footer()), foot)`. The SVG renderer goes through it too, so the export shows the new text until spec 135 removes it.

### Call sites

- Production writers of `save_to` move to `set_save_to`: `state/command.rs:24`, `state/command.rs:173`, `state/save_prompt.rs:20`.
- `state/command.rs:175` reads through `save_to()`.
- Test writers move to `set_save_to`: the ones in `state/command.rs`, `render/mod.rs` and `render/svg.rs`. Test assertions on `result.save_to` read through `save_to()`.

### Tests

- `state`: `footer()` is `plans • dre` for `docs/plans.dre`, `plans • dre` for `plans.dre`, `plans • dre` for `plans` with no extension, and `[no name] • dre` for a default state. The existing `diagram_name_*` tests become `footer` tests.
- `state`: `set_save_to(None)` after a path goes back to `[no name] • dre`. `set_save_to(Some(..))` after `None` updates the footer.
- `layout`: `footer("plans • dre")` is one label as wide as the text (11), one row high, with no path.
- `render`: `editor` ends with `plans • dre` right-aligned in the bottom row when there is a path, and `[no name] • dre` when there is none.
- `render/svg`: the export shows the new text, and its picture size follows it.
