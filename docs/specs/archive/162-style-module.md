# Split `palette` into a `style` module for shared visual constants

This is a technical refactoring spec. It changes module ownership and file
boundaries without changing the rendered diagram or its behaviour.

## Problem

`src/layout.rs` imports `BOX_FILL_OPACITY` and `FOOTER_FILL_OPACITY` from
`crate::render`, and callers of cell geometry (`src/cli.rs`) import
`CELL_WIDTH` and `CELL_HEIGHT` from `crate::render` too. `render` is meant to
consume `layout`'s placements, not the other way around, so `layout`
depending on `render` for these constants is a dependency in the wrong
direction. `palette.rs` already plays the role of a shared, dependency-free
home for colour constants (`FOREGROUND`, `BACKGROUND`, `LIME`, `VIOLET`,
`AMBER`, `palette`, `next_on_palette`) used by `layout`, `render`, `tty`,
`dre_format`, and `state`. There is no reason to have two shared-constants
modules with two different rules.

## Acceptance Criteria

- `src/palette.rs` is renamed to `src/style.rs`; `mod palette;` in `lib.rs`
  becomes `mod style;`.
- `style` additionally owns `CELL_WIDTH`, `CELL_HEIGHT`, `BOX_FILL_OPACITY`,
  and `FOOTER_FILL_OPACITY`, moved out of `render/mod.rs`.
- `layout.rs` no longer imports anything from `crate::render`.
- Every existing `crate::palette::*` reference across the crate
  (`layout.rs`, `render/mod.rs`, `render/svg.rs`, `render/terminal.rs`,
  `render/font.rs`, `tty.rs`, `dre_format.rs`, `state/mod.rs`,
  `state/command.rs`, `cli.rs`) is updated to `crate::style::*`.
- Render-only visual constants that don't cross the layout/render boundary
  (`ROUNDED_RADIUS`, `ARROW_OPACITY`, `LED_DOT_RATIO`, `LED_HALO_ALPHA`,
  `LED_DIM_ALPHA`) stay in `render/mod.rs`.
- No behaviour changes. Rendered terminal and SVG output, and all existing
  tests, are unchanged except for import paths.
- The full Rust test suite passes.

## Technical Design

Decided in the design session.

### Scope

`style` is scoped strictly to constants and functions that cross the
`layout` ↔ `render` boundary today: `palette`, `next_on_palette`,
`FOREGROUND`, `BACKGROUND`, `LIME`, `VIOLET`, `AMBER`, `CELL_WIDTH`,
`CELL_HEIGHT`, `BOX_FILL_OPACITY`, `FOOTER_FILL_OPACITY`. Visual constants
used only inside `render` are left there; moving them would not fix a
dependency direction problem, only rename it.

### Move, not rewrite

This is a pure move:

- `git mv src/palette.rs src/style.rs`.
- Cut `CELL_WIDTH`, `CELL_HEIGHT`, `BOX_FILL_OPACITY`, and
  `FOOTER_FILL_OPACITY` from `render/mod.rs` and paste them into `style.rs`
  as `pub(crate) const`, unchanged.
- Keep every existing name, including the function `palette`. Call sites
  read `crate::style::palette(...)` — the repetition of "style" is
  accepted; it doesn't warrant a rename.
- Existing `palette.rs` tests (palette lookup, `next_on_palette` cycling)
  move to `style.rs` unchanged. No new tests are added for `CELL_WIDTH`,
  `CELL_HEIGHT`, `BOX_FILL_OPACITY`, or `FOOTER_FILL_OPACITY`: they are bare
  constants with no behaviour of their own, and are already exercised
  indirectly by the `layout`, `render/svg.rs`, `render/terminal.rs`, and
  `render/font.rs` tests that consume them.

### Import updates

Update every `use crate::palette::...` and `use crate::render::{CELL_WIDTH,
CELL_HEIGHT, BOX_FILL_OPACITY, FOOTER_FILL_OPACITY}` (and any
`crate::render::CELL_WIDTH`-style qualified path, e.g. in `cli.rs`) to
`crate::style::...`. `render/mod.rs`, `render/svg.rs`, and
`render/terminal.rs` keep re-importing these constants from `style` wherever
they're used, the same way they import from `palette` today.

### Result

`layout` depends only on `style`, `diagram`, `state::FooterView`, and
`types`. `render` depends on `style` and `layout`. The dependency graph
between `layout` and `render` becomes one-directional.
