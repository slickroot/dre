# 079: Render module

Refactoring spec — no user story. Behaviour doesn't change.

## Goal

Spec 076 created the single-method `Renderer` trait, and both renderers implement it. What's left is moving the rendering code into a `src/render/` directory: the interface and the visual rules both renderers share in `mod.rs`, and each renderer with its own helpers in its own file.

```
src/render/
  mod.rs       the Renderer trait + what both renderers share
  terminal.rs  TerminalRenderer and its terminal-only helpers
  svg.rs       SvgRenderer (moved from src/svg.rs)
```

## Technical Design

### Principle

`render` is one module with one public face: `Renderer`, `TerminalRenderer` and `SvgRenderer`. Everything else — the shared visual rules and each backend's helpers — is private to `render/`, and the compiler enforces it. Nothing outside `render/` uses any of it today (checked: only `writer.rs` and `cli.rs` import from `render`/`svg`).

### `src/render/mod.rs` — the interface and the shared rules

- `mod terminal; mod svg;` — private submodules.
- `pub(crate) use terminal::TerminalRenderer;` and `pub(crate) use svg::SvgRenderer;` — the only way callers reach a concrete renderer.
- `pub(crate) trait Renderer` — moved as-is.
- Shared visual rules, **private** (no visibility modifier; child modules see them through `use super::…`):
  - Cell size in pixels: `CELL_WIDTH`, `CELL_HEIGHT`.
  - Arrowhead shape: `ARROWHEAD_ANGLE_DEG`, `ARROWHEAD_EDGE_LENGTH`, `arrowhead_depth`, `arrowhead_slope`.
  - Box look: `BORDER`, `ROUNDED_RADIUS`.
  - Colours: `colour`, `PLAIN_COLOUR` (it is what `colour(None)` returns), `FILL_ALPHA`, `OPAQUE`.
- Tests: the shared ones — `colour` and arrowhead shape.

### `src/render/terminal.rs` — the terminal renderer

The rest of today's `src/render.rs`, private unless noted:

- `TerminalRenderer` — `pub(crate)` so `mod.rs` can re-export it; its `new`, `resize` and `Renderer` impl keep their current visibility.
- Pixel drawing: `Canvas`, `RoundedBox`, `centered_span`, `python_round`, `body_row`, `square_pixels`, `fill_colour`, `TRANSPARENT`.
  - `fill_colour` stays terminal-only: it is how the terminal applies the shared `FILL_ALPHA` rule (composite over black, make opaque). The SVG applies the same rule as a `fill` + `opacity` attribute.
- Sprite cache: `SpriteKey`, `sprite_key`, `CACHE_LIMIT`.
- Text-cell output: `cell`, `RESET`, `BLANK`, `BLANK_CELL`, `CURSOR`, `HOME_CURSOR`.
- `ARROW_STROKE = 4` — the terminal's own stroke.
- Tests: all of today's `render.rs` tests except the shared ones.

### `src/render/svg.rs` — the SVG renderer

- `src/svg.rs` moved as-is. `SvgRenderer` is `pub(crate)` so `mod.rs` can re-export it.
- Keeps its own `ARROW_STROKE = 2`, `ARROW_JOIN_OVERLAP`, `INK`.
- Imports the shared items from `super` instead of `crate::render`, including the inline paths in `rect` (`crate::render::FILL_ALPHA`, the local `use crate::render::{BORDER, OPAQUE, ROUNDED_RADIUS}`) and in its tests.

### Callers

- `main.rs` drops `mod svg;`; `mod render;` now resolves to `src/render/mod.rs`.
- `cli.rs`: `use crate::render::{Renderer, SvgRenderer};`.
- `writer.rs`: `use crate::render::{Renderer, TerminalRenderer};`.
  - `CURSOR` is no longer reachable, so `writer.rs` defines its own private cursor character for `prompt_line`.
  - Its tests stop importing `HOME_CURSOR` and spell out the expected frame start themselves.
  - Nothing else in `writer.rs` changes. Slimming it down to a plain render-per-keystroke loop (moving the save prompt and `load_state` out) is a separate spec.

### `#[allow(dead_code)]`

The `#[allow(dead_code)]` attributes on `CELL_WIDTH`, `CELL_HEIGHT` and `CACHE_LIMIT` are removed wherever the item is actually used, so real dead code isn't hidden. The build stays warning-free.

### Dependencies after this spec

```
diagram      ← (nothing)
layout       → diagram
render       → diagram, layout, kitty
dre_format   → diagram
file_document→ diagram, dre_format
state, command_mode, insert_mode, save_prompt_mode → diagram
cli          → render, file_document, dre_format
writer       → state, diagram, render, file_document
```

Inside `render`: `terminal → mod`, `svg → mod`; `terminal` and `svg` never import each other.
