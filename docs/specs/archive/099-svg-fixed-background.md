# SVG fixed background

## User Story

Doug exports his diagram from dre to SVG. Instead of the SVG switching between light and dark backgrounds depending on the viewer's system preference, it always shows with the fixed background color `#0A0B0D`, matching the terminal exactly.

## Acceptance Criteria

- Exported SVG files have a background of `#0A0B0D` regardless of the viewer's OS/browser light or dark mode setting.
- The `prefers-color-scheme` media query and its alternate color values are removed from the exported SVG.
- Viewing the SVG in both a light-mode and dark-mode browser produces the identical background color.

## Technical Design

`src/render/svg.rs` currently gives every exported SVG a fixed white background: `style_block()` emits `<style>svg { --bg: rgb(255,255,255) }</style>` and `background_rect()` fills using `fill="var(--bg)"`. There is no `prefers-color-scheme` media query left in the code (a prior change already dropped the dark-mode `--ink` override and inlined `colour(None)` — i.e. `palette(FOREGROUND)` — as literal `rgb(...)` values in `rect()`, `label_text()`, `arrow_paths()`, `marker_defs()`, and `cursor_rect()`). The only remaining piece is the background, which is still hardcoded to white via a CSS custom property.

Changes:
- Remove `style_block()` entirely and its call site in `SvgRenderer::draw()` — no `<style>` block is needed once the background is a fixed literal color.
- Change `background_rect()` to resolve the fill color by calling `palette(BACKGROUND).unwrap()` directly (matching the existing `(10, 11, 13)` / `#0A0B0D` entry added in spec 098), and emit a literal `fill="rgb({r},{g},{b})"` instead of `fill="var(--bg)"`.
- Update the co-located tests in `svg.rs`: rewrite/remove the style-block tests (`the_style_light_default_is_built_from_the_ink_constant`, `the_style_dark_override_uses_a_media_query_with_white_ink_and_black_bg`, and the two tests asserting the style block precedes/follows the `<svg>` tag) since there is no more style block, and update `expected_background()` and the background-rect tests to assert the literal `#0A0B0D` fill.

No new collaborators are introduced — `background_rect()` calls `palette` from `crate::diagram`, the same module `colour()` already depends on for foreground.
