## User Story

Doug opens the app and looks at the footer bar. Instead of a plain outlined box, it has a light foreground-coloured tint (12% opacity) filling it, making it stand out from the body while the footer text stays perfectly readable on top.

## Acceptance Criteria

- The footer box is filled with the foreground colour at 12% opacity (distinct from the existing 30% fill used elsewhere).
- This applies consistently in both the terminal renderer and the SVG renderer.
- The footer text remains fully legible over the tinted fill.

## Technical Design

Opacity becomes a per-box property instead of the single hardcoded `FILL_ALPHA` constant it is today.

- `PlacementNode::Box` (`src/layout.rs:204`) gains a new field `opacity: Option<f64>`, alongside the existing `fill: Option<u8>` (colour). `opacity` is `0.0` (fully transparent) to `1.0` (fully opaque), and is only ever read when `fill` is `Some`; when `fill` is `None`, `opacity` is `None` too and is never checked.
- Two new constants live in `src/render/mod.rs` next to the existing `ARROW_OPACITY: f64 = 0.5`:
  - `BOX_FILL_OPACITY: f64 = 0.3` — the existing 30% fill used for regular diagram boxes.
  - `FOOTER_FILL_OPACITY: f64 = 0.12` — the new 12% fill for the footer.
- The old `FILL_ALPHA: u16 = 77` constant (`src/render/mod.rs:114`) is removed.
- `layout::diagram`'s box construction (`src/layout.rs:87`, currently `fill: node.filled().then_some(node.colour()).flatten()`) sets `opacity: Some(BOX_FILL_OPACITY)` whenever `fill` is `Some`.
- `layout::footer` (`src/layout.rs:228-254`) is updated to:
  - `fill: Some(FOREGROUND)` (using the existing `pub(crate) const FOREGROUND: u8 = 5` from `src/palette.rs:11`) and `opacity: Some(FOOTER_FILL_OPACITY)`.
  - `sides: NO_SIDES`, a new constant `pub const NO_SIDES: Sides = (false, false, false, false);` added next to `ALL_SIDES` (`src/layout.rs:13`), so the border is no longer drawn. `colour` and `border` are left unchanged since they have no visible effect once all sides are disabled.
- Terminal renderer: `fill_colour` (`src/render/terminal.rs:28-38`) takes the box's `opacity` and converts it to a byte alpha (`(opacity * OPAQUE as f64) as u16`) instead of using the removed `FILL_ALPHA` constant.
- SVG renderer: `rect` (`src/render/svg.rs:243-279`) uses the box's `opacity` f64 directly as the `fill-opacity` attribute instead of computing it from the removed constant.
