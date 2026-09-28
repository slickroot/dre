# Footer shows MOVE or WRITE

## User Story

Doug opens dre and lands in Command mode. Before the diagram name, the footer shows a dim lime LED next to the word MOVE. He starts editing a box's label, and the footer's LED lights up violet next to the word WRITE. He presses Escape to leave editing, and the footer goes back to the dim lime LED and MOVE.

## Acceptance Criteria

- In Command mode, the footer shows a dim (unlit) lime LED followed by "MOVE" before the diagram name.
- In Insert mode, the footer shows a lit violet LED followed by "WRITE" before the diagram name.
- Switching between the two modes updates the LED and word immediately.

## Technical Design

Today `State::footer()` returns a plain `String`, and `layout::footer(text: &str)` lays out a single tinted `Box` plus one `Label`. This story needs a colour that varies independently of the text (the LED), so the footer becomes a small view model instead of a string.

### `FooterView`

`State::footer()` returns a new `pub(crate) struct FooterView` (in `src/state/mod.rs`):

```rust
pub(crate) struct FooterView {
    led_colour: u8,      // palette::LIME or palette::VIOLET
    lit: bool,           // false = dim (0.12 opacity), true = lit (1.0 opacity)
    text: String,        // "MOVE plans • dre" / "WRITE plans • dre" / "type a name • dre" — same composition as today, with the mode word folded in
    cursor: Option<usize>, // Some(edit_index) while NamePrompt is being typed, else None
}
```

- `Mode::Command` → `led_colour: LIME, lit: false`, text prefixed with `"MOVE "`.
- `Mode::Insert { .. }` → `led_colour: VIOLET, lit: true`, text prefixed with `"WRITE "`.
- `Mode::NamePrompt { .. }` → defaults to the same dim lime/MOVE as Command for now (spec 151 will redefine this with amber). `cursor` is `Some(name.chars().count())`; all other modes leave `cursor: None`.
- New word constants `"MOVE"` / `"WRITE"` live alongside the existing `NO_NAME`, `PLACEHOLDER`, `FOOTER_SUFFIX` constants.

### Palette

`src/palette.rs` gains named constants for the two indices this story needs, matching the existing `FOREGROUND`/`BACKGROUND` convention:

```rust
pub(crate) const LIME: u8 = 0;
pub(crate) const VIOLET: u8 = 2;
```

### `layout::footer`

`layout::footer` changes signature from `fn footer(text: &str)` to `fn footer(view: &FooterView)`. It builds the existing background `Box` and `Label` (label shifted right to make room for the LED plus one column of gap), plus a new LED placement:

```rust
PlacementNode::Box {
    colour: Some(view.led_colour),
    fill: Some(view.led_colour),
    opacity: Some(if view.lit { 1.0 } else { FOOTER_FILL_OPACITY }),
    rounded: true,
    sides: ALL_SIDES,
    border: 0,
}
```

sized `1x1`, positioned before the label with one column of padding. The label's `path` stays `vec![]` as it is today.

Cursor placement moves inside `layout::footer` itself, reusing the existing `with_cursor` helper (`src/layout.rs:300`) instead of index-based lookups:

```rust
pub(crate) fn footer(view: &FooterView) -> Vec<Placement<'static>> {
    let placements = vec![/* box, led, label */];
    with_cursor(placements, view.cursor.map(|_| Vec::new()), view.cursor)
}
```

This matches how `render::body` already places the diagram's cursor by matching a label's `path`, and means adding the LED placement can't silently break cursor positioning through a stale index.

### `render::editor`

`render/mod.rs:25-38` simplifies — the manual `Mode::NamePrompt` cursor-pushing block is deleted entirely, since `layout::footer` now owns that:

```rust
pub(crate) fn editor(state: &State, window: Area) -> Vec<(Area, Vec<Placement<'_>>)> {
    let [body, foot] = composer::stack([None, Some(FOOTER_ROWS)], window);
    let footer = align_right(layout::footer(&state.footer()), foot);
    vec![(body, self::body(state, body)), (foot, footer)]
}
```
