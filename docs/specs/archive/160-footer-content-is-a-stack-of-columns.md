# Footer content is a stack of columns

This is a technical spec, not a user story. Its one visible change is that
the footer's filename renders in its own colour, distinct from the mode
word next to it.

## Problem

`layout::footer` (`src/layout.rs:276`) builds the footer by hand:

- The mode word, filename/name and suffix are pasted into one `String`
  (`FooterView::text`), then wrapped in a single `Label` — so the whole
  thing is always one colour, because `Label` has no colour field at all.
- Element positions (`led_x`, `label_x`) are computed from ad hoc constants
  (`SIDE_PADDING`, `LED_WIDTH`, `LED_LABEL_GAP`) and `centre()`/`interior()`,
  rather than composed from each element's own spacing.
- `with_cursor` (`src/layout.rs:406`) finds the cursor's target label by
  matching `Label::path == path`, reusing a field whose real job elsewhere
  is identifying a diagram tree node. In the footer there is no tree node,
  so `path: vec![]` is used as a sentinel. This only works because there is
  exactly one label; splitting the text into several labels would make the
  sentinel ambiguous.
- `state::footer()` mixes domain knowledge (what mode we're in) with
  presentation (which palette colour, which word, which placeholder string)
  because `FooterView` is a flat, already-formatted bag of strings.

Every one of these is a symptom of the same gap: there's no way to lay out
a row of independent, self-contained pieces. This spec adds one, scoped to
the footer.

## Acceptance Criteria

- The footer's filename (or the name being typed, or the "no name"/"type a
  name" placeholder) renders in a different colour from the mode word and
  LED, in both the SVG and terminal renderers.
- No other visible change: layout, spacing and behaviour of the footer are
  otherwise the same as today.

## Technical Design

### `FooterModel` replaces `FooterView`

`state/mod.rs` exposes domain facts, not presentation:

```rust
pub(crate) enum FooterMode {
    Move,
    Write,
    Naming,
}

pub(crate) struct FooterModel {
    pub(crate) mode: FooterMode,
    pub(crate) filename: Option<String>,
    pub(crate) cursor: Option<usize>,
}
```

`FooterMode` is footer-specific and deliberately decoupled from `state::Mode`
— a change to `Mode`'s internals for unrelated reasons shouldn't ripple into
the footer.

`State::footer()` maps `Mode` → `FooterModel`:

- `Mode::Command` → `Move`, `filename: self.save_to.as_deref().map(file_stem_without_dre)`, `cursor: None`.
- `Mode::Insert { .. }` → `Write`, same `filename`, `cursor: None`.
- `Mode::NamePrompt { name, .. }` → `Naming`, `filename: (!name.is_empty()).then(|| name.clone())`, `cursor: Some(name.chars().count())`.

`filename: None` means "nothing to show yet" — what word fills that gap
(`NO_NAME` vs `PLACEHOLDER`) is a presentation decision, not a domain one,
so it moves to `layout`. `NO_NAME`, `PLACEHOLDER`, `FOOTER_SUFFIX`, `MOVE`,
`WRITE` and the `NAME` word (from spec 151, not yet built) move from
`state/mod.rs` to `layout.rs` alongside the code that consumes them.
`palette::LIME`/`VIOLET`/`AMBER` move the same way — `state` no longer picks
colours.

### `Column`

A column is a placeholder that can hold any placement:

```rust
struct Column<'a> {
    node: PlacementNode<'a>,
    width: i64,
    padding: i64,
}
```

`width` is supplied by the caller, same as every `Placement` already
requires today (e.g. `text_width`, `LED_WIDTH`) — no new width-inference
logic. `padding` is symmetric (one value, both sides) and applied to every
column, including the first and last (there is no separate outer margin).

### Stacking columns

A private function in `layout.rs` turns `Vec<Column>` into positioned
placements:

```rust
fn stack_columns(columns: Vec<Column>, y: i64) -> (Vec<Placement<'static>>, i64) // returns placements, total width
```

- Columns are placed left to right. Each column's own padding is added on
  both its sides; adjacent columns' padding is **additive** (not
  CSS-style collapsing) — two columns with padding 2 each leave a 4-wide
  gap between them.
- Total width is the sum of every column's `width` plus all padding,
  including before the first and after the last column. The footer's
  bordered box is sized to hug this total exactly — `centre()`/`interior()`
  are no longer used for the footer (they stay as-is for the diagram).
- All footer content sits on one row (`y = BOX_HEIGHT / 2`, height 1), same
  as today — the stack doesn't need per-column height until something needs
  more than one row, which is out of scope here.

### Cursor, without `with_cursor`

`layout::footer` already knows, while building the column list, which
column (if any) is the one being typed into. It emits the `Cursor`
placement inline at that column's assigned `x` plus `FooterModel::cursor`'s
offset, while it still has the column's position in hand — no lookup by
`path`, no sentinel, no `with_cursor` call for the footer. `with_cursor`
itself is untouched and keeps serving the diagram, where `path` really does
identify a tree node.

### `Label` gains a colour

```rust
pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub path: Vec<usize>,
    pub colour: Option<u8>,  // new; None = default foreground, as today
}
```

Every existing call site sets `colour: None` (no behaviour change outside
the footer). `render/svg.rs` and `render/terminal.rs` use `label.colour`
instead of always drawing in the default foreground.

### `layout::footer`

```rust
pub(crate) fn footer(model: &FooterModel) -> Vec<Placement<'static>>
```

Builds, in order: the LED column (colour/lit from `model.mode`), the mode
word column (`"MOVE"`/`"WRITE"`/`"NAME"`, default colour), and the
filename column (`model.filename` or the mode-appropriate placeholder,
its own colour, plus `FOOTER_SUFFIX`) — then calls `stack_columns`, then
wraps the result in the bordered corner box sized to the returned total
width. `bordered` (spec 151) is derived from `model.mode == FooterMode::Naming`
directly, rather than being a separate field threaded through from `state`.

### Considered and rejected

- **A general-purpose layout module (`Row`/`Stack`/`Column`) usable beyond
  the footer.** No second caller exists yet; scoped to the footer, as
  spec 131 already decided for the screen-level stack. Revisit if/when a
  second use case shows up.
- **A closed enum of footer element kinds** (`enum FooterColumn { Led {..}, Text {..} }`).
  Rejected in favour of a generic wrapper around `PlacementNode`, so the
  stack doesn't need to know what an LED or a label is.
- **Asymmetric (`left`, `right`) padding.** Not needed yet; symmetric
  padding is enough for the footer's current elements.
- **Tracking each placement's originating node with an id/path, and looking
  the cursor's target up afterwards.** This is the same "build, then search"
  shape as `with_cursor`, just with a different key, and it would add a
  field to the shared `Placement`/`Label` types that's meaningless outside
  the footer. Emitting the cursor inline while building the columns avoids
  the lookup entirely.
- **Moving this into its own `src/footer.rs` module.** Left in `layout.rs`
  for now; can split out later if it grows.

### Tests

- `stack_columns`: two columns' widths and paddings sum correctly; padding
  is additive between adjacent columns and counted at both outer edges;
  columns are placed left to right at the given `y`.
- `layout::footer`: filename column carries `model.filename`'s colour and
  text (or the right placeholder for `Move`/`Write` vs `Naming`); the
  cursor placement lands at the filename column's `x` plus the model's
  cursor offset, only when `model.cursor` is `Some`; the bordered box's
  width matches the stack's total width exactly.
- `Label` with `colour: Some(_)` renders in that palette colour in both
  `render/svg.rs` and `render/terminal.rs`; `colour: None` keeps rendering
  in the default foreground (regression coverage for every existing
  diagram-label call site).
