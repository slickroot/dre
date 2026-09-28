# Footer shows NAME while typing the diagram's name

## User Story

Doug presses `n` to name his diagram. The footer's LED lights up amber next to the word NAME, and the footer gains a border around it, making it obvious he's now typing into it. When he confirms or cancels, the footer returns to its normal MOVE state with no border.

## Acceptance Criteria

- In the Name prompt, the footer shows a lit amber LED followed by "NAME" before the name being typed.
- While naming, the footer has a border around it (it currently has no border).
- Confirming or cancelling the name prompt returns the footer to its normal state (dim lime LED, MOVE, no border).

## Technical Design

This builds on spec 150's `FooterView` (`src/state/mod.rs`), which spec 150 left with `Mode::NamePrompt` defaulting to the same dim lime/MOVE as `Mode::Command`, and `layout::footer`'s corner box hardcoded to `sides: NO_SIDES`. This story fills in both gaps.

### `FooterView` gains `bordered`

```rust
pub(crate) struct FooterView {
    led_colour: u8,
    lit: bool,
    text: String,
    cursor: Option<usize>,
    bordered: bool,   // new: true only for Mode::NamePrompt
}
```

`State::footer()`'s match arms:

- `Mode::Command` → `led_colour: LIME, lit: false, bordered: false`, text prefixed `"MOVE "`.
- `Mode::Insert { .. }` → `led_colour: VIOLET, lit: true, bordered: false`, text prefixed `"WRITE "`.
- `Mode::NamePrompt { name, .. }` → `led_colour: AMBER, lit: true, bordered: true`, text prefixed `"NAME "` (i.e. `format!("NAME {name}{FOOTER_SUFFIX}")`, or `format!("NAME {PLACEHOLDER}{FOOTER_SUFFIX}")` when `name` is empty — same placeholder logic as today, just with the word folded in).

New word constant `const NAME: &str = "NAME";` alongside `MOVE`/`WRITE` (introduced by spec 150) and the existing `PLACEHOLDER`/`FOOTER_SUFFIX`/`NO_NAME`.

### Cursor offset

Today `cursor` is `Some(name.chars().count())`, which lines up because the `NamePrompt` text used to start at the name. Now the text starts with `"NAME "`, so the offset must shift by that prefix's length:

```rust
cursor: Some(NAME.chars().count() + 1 + name.chars().count())
```

(`+ 1` for the space between `NAME` and the typed name.)

### Palette

`src/palette.rs` gains the amber index alongside `LIME`/`VIOLET` (spec 150):

```rust
pub(crate) const AMBER: u8 = 4;
```

### `layout::footer`

The corner box's `sides` comes from `view.bordered` instead of the hardcoded `NO_SIDES`:

```rust
PlacementNode::Box {
    colour: None,   // default foreground, same as every other bordered box — not amber
    fill: Some(FOREGROUND),
    opacity: Some(FOOTER_FILL_OPACITY),
    rounded: false,
    sides: if view.bordered { ALL_SIDES } else { NO_SIDES },
    border: 1,
}
```

No other change to `layout::footer` or `render::editor` is needed — the LED and cursor plumbing already added by spec 150 carries the amber colour and shifted cursor index through unchanged.
