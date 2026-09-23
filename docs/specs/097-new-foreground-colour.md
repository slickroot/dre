# New foreground colour

## User Story

Doug opens dre and draws a plain, uncolored box with some text and an arrow. Instead of the old grey/black look, the borders, text, arrows, and cursor all appear in the new foreground color. When he exports to SVG, those same elements use the same foreground color there too.

## Acceptance Criteria

- Uncoloured box borders render in `#E8EAED` in the terminal.
- Text render in `#E8EAED` in the terminal.
- Arrows render in `#E8EAED` in the terminal.
- The cursor renders in `#E8EAED` in the terminal.
- Exporting to SVG produces `#E8EAED` for all of the above (borders, text, arrows, cursor).
- The old grey and black colors no longer appear for these elements.

## Technical Design

Today there are two separate, uncoordinated "default foreground" constants: `PLAIN_COLOUR` (grey `(128,128,128)` at `src/render/mod.rs:38`, used by the terminal renderer for uncoloured box borders/edges/arrows) and `INK` (black `(0,0,0)` at `src/render/svg.rs:9`, used by SVG via a `--ink` CSS custom property, with a `prefers-color-scheme: dark` flip to white). Label glyph ink is separately hard-coded to `PLAIN_COLOUR` in `src/render/font.rs:85`. The terminal cursor (`draw_cursor` in `src/render/terminal.rs:225-227`) currently has no explicit color at all — it writes a plain character into the text grid and inherits whatever foreground color the real terminal happens to use.

### Single source of truth

- Add a 6th entry to the existing `PALETTE` array in `src/diagram.rs` for `#E8EAED`, exposed via a named constant `const FOREGROUND: u8 = 5;`.
- Change the shared dispatcher `colour()` in `src/render/mod.rs` from `None => PLAIN_COLOUR, Some(i) => palette(i).unwrap()` to `palette(colour.unwrap_or(FOREGROUND)).unwrap()`. `PALETTE`/`palette()` becomes the single source of truth for every color, including the default foreground — there is no more special-cased default RGB literal.
- Delete `PLAIN_COLOUR` (`src/render/mod.rs:38`) and `INK` (`src/render/svg.rs:9`); both are replaced by the one shared lookup.

### Terminal renderer

- `src/render/font.rs`: replace the `PLAIN_COLOUR` import with a call to `colour(None)` for glyph ink, so label text ink comes from the same dispatcher as everything else.
- `src/render/terminal.rs`: turn `draw_cursor` from a free function that writes a plain char into the text grid into a `TerminalRenderer` method, matching `draw_box`/`draw_label`. It builds a small solid-`colour(None)`-filled canvas (one cell) and places it via `screen.place`, so the cursor is an explicitly colored pixel sprite rather than relying on the terminal's inherited default foreground.

### SVG renderer

- Drop the `prefers-color-scheme: dark` media query and the `--ink`/white-ink flip in `style_block()` (`src/render/svg.rs:104-109`) entirely — SVG output no longer varies by light/dark mode for foreground elements.
- Remove the `--ink` CSS custom property and its `var(--ink)` usages in `rect`, `label_text`, `cursor_rect`, `arrow_paths`, and `marker_defs`. Each of these calls `colour(None)` directly and inlines the resulting `rgb(...)` value, the same way `rect()` already does for palette colors via `colour(node.colour)`.
- `--bg` (background) is untouched — it stays a fixed-white CSS var; this spec only changes foreground elements.

### Result

Borders, arrows, and the cursor (terminal + SVG) and label text (terminal; SVG text already routed through `colour()`) all resolve to `#E8EAED` through one shared `colour()`/`PALETTE` lookup, eliminating the separate grey/black defaults.
