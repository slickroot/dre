# dre draws its terminal text in JetBrains Mono

## User Story

Marouane opens dre and draws a box with a label. The label shows up in JetBrains Mono, wider and easier to read than before. A glance at the footer shows the mode name and "dre" in JetBrains Mono Bold. The whole screen now has one consistent, more readable font.

## Acceptance Criteria

- Box labels in the terminal render in JetBrains Mono Regular.
- Arrow labels in the terminal render in JetBrains Mono Regular.
- The footer's mode name and "dre" render in JetBrains Mono Bold.
- JetBrains Mono is bundled with dre (no separate install required).
- Iosevka is no longer shipped with dre. Exported SVGs losing their Iosevka font until the SVG story lands is an accepted side effect.

## Technical Design
### Fonts bundled

- Add the static `JetBrainsMono-Regular.ttf` and `JetBrainsMono-Bold.ttf` from the official JetBrains Mono v2.304 release to `assets/`. Don't use the variable `JetBrainsMono[wght].ttf`: fontdue can't pick a weight from it. The plain and NL (no ligatures) builds are equivalent here because fontdue draws one glyph per cell and never applies ligatures.
- Delete `assets/IosevkaRegular.ttf` and `assets/IosevkaBold.ttf`. The bundled fonts shrink from about 21.6 MB to about 0.55 MB.
- Add the release's `OFL.txt` as `assets/JetBrainsMono-OFL.txt`. The SIL Open Font License 1.1 requires the copyright notice and licence text to ship with the font. It also covers the test subsets.

### `GlyphCache` (`src/render/font.rs`)

- `FONT_BYTES` and `BOLD_FONT_BYTES` load the JetBrains Mono files with `include_bytes!`. The `GlyphSource` trait, `GlyphCache`'s state and the callers (`editor/bootstrap.rs`, `flex/mod.rs`) don't change. Box labels, arrow labels and the bold footer already go through `GlyphCache`, so all three switch fonts together.
- The sizing algorithm stays the same. It fits the regular weight's advance to `cell_width` and caps it so ascent plus descent fits `cell_height`. Both weights share those metrics. Only the comments and the `expect` messages that mention Iosevka change.
- The algorithm already produces the "wider" outcome. Measured on an 8×16 cell (advance 0.60 em and line height 1.32 em, compared with 0.50 and 1.25 for Iosevka), both fonts are limited by height: the pixel size is about 12.1 px instead of 12.8 px, and the ascent stays about 12.4 px. Each glyph's advance grows from 6.4 px (80% of the cell) to about 7.3 px (91% of the cell).

### Tests

- Keep the subset fonts so that parsing in tests stays fast. A full JetBrains Mono file takes about 40 ms to parse in a debug build, and a subset about 0.1 ms.
- Replace `assets/test/IosevkaSubset.ttf` and `assets/test/IosevkaBoldSubset.ttf` with `assets/test/JetBrainsMonoSubset.ttf` and `assets/test/JetBrainsMonoBoldSubset.ttf`. Cut them from the bundled TTFs with `pyftsubset`, covering the same characters as today (M, B, i, g). `TEST_FONT_BYTES` and `TEST_BOLD_FONT_BYTES` point to the new subsets.
- The existing `GlyphCache` tests carry over unchanged: the glyph is one cell, descenders aren't clipped, the ink colour is right, and bold and regular are cached separately.
- New test: in an 8×16 cell, the regular `M`'s advance width at the cache's `px_size` is at least 7 px. This pins the wider text, so a later change of font or sizing can't quietly make labels narrow again.

### Out of scope

- `src/render/svg.rs` and its tests stay untouched. Its `@font-face` URL still points to `assets/IosevkaRegular.ttf` on `main`, so after this story it returns 404 and SVG viewers fall back to `monospace`. The spec accepts this until the SVG story lands. `MONOSPACE_ADVANCE_RATIO = 0.6` already matches JetBrains Mono.
