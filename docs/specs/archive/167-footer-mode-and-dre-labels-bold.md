# Footer mode and "dre" labels are bold

## User Story

Maya opens dre and looks at the footer. The current mode name (MOVE, WRITE, or NAME) and the trailing "dre" label are rendered in a bold font weight, while everything else in the footer stays as it is today. Maya can immediately pick out the mode and app name at a glance.

## Acceptance Criteria

- In every mode (Move, Write, Name), the mode word (MOVE / WRITE / NAME) is rendered in bold font weight.
- The trailing "dre" label is rendered in bold font weight.
- The filename between them keeps its current appearance (regular weight, dimmed color) — unchanged.
- The LED indicator and overall footer layout/spacing are unchanged.

## Technical Design

dre's footer is not a text-attribute terminal UI — it's a custom pixel renderer. `src/view.rs::footer()` builds `Label` structs (`text`, `colour`) that flow through `src/render/terminal.rs` (`LabelStyle` → `draw_label` → `GlyphSource::glyph`) which rasterizes glyphs via `fontdue` from a single bundled `IosevkaRegular.ttf`, plus a parallel `src/render/svg.rs` exporter. There is currently no bold/weight concept anywhere, so this feature needs a second font weight threaded through both rendering paths.

### Font asset

Bundle a real Bold TTF rather than faking bold via double-rendering, so glyph shapes stay correct. Copy `Iosevka-Bold.ttf` (from the system's Iosevka 34.7.0 install) into `assets/IosevkaBold.ttf`, alongside the existing `assets/IosevkaRegular.ttf`, and load it via `include_bytes!` the same way.

### `Label` struct (`src/view.rs`)

Add a `bold: bool` field (mirroring the existing `colour: Option<u8>` style — a simple flag is enough since only two weights exist today, no need for a `Weight` enum).

```rust
pub struct Label<'a> {
    pub text: Cow<'a, str>,
    pub colour: Option<u8>,
    pub bold: bool,
}
```

### `footer()` (`src/view.rs`)

Set `bold: true` on the mode-word `Label` and the `FOOTER_SUFFIX` ("dre") `Label`. The filename `Label` gets `bold: false`, keeping its existing `colour: Some(style::DIM)` unchanged. LED indicator and column layout/spacing/padding are untouched.

### `GlyphCache` / `GlyphSource` (`src/render/font.rs`)

Keep a single `GlyphCache` struct (so `Renderer` still holds one glyph-source field and call sites still make one `glyph()` call), but give it two internal `fontdue::Font` instances (`regular`, `bold`) and two internal `HashMap` caches (`regular_cache`, `bold_cache`) instead of one shared `HashMap<(char, Option<u8>), Canvas>`. Extend the signature to `glyph(&mut self, ch: char, colour: Option<u8>, bold: bool) -> &Canvas`, which picks the matching font + cache internally.

### `LabelStyle` / `draw_label` (`src/render/terminal.rs`)

Add `bold: bool` to `LabelStyle` (constructed from `Label.bold`), and pass it through `draw_label` into the `glyph()` call.

### SVG export (`src/render/svg.rs`)

`label_text()` reads `Label.bold` and, when true, adds `font-weight="bold"` to the emitted `<text font-family="Iosevka, monospace">` element — no separate bold font-family reference needed for the SVG path.