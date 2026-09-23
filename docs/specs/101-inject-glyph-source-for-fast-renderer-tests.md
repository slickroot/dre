# Inject a glyph source so renderer tests skip real font parsing

## Description

`cargo test` runs 445 tests in 47.82s — nowhere near the sub-second feedback
loop TDD depends on. The suite has no end-to-end layer; what's slow is a
single shared cost paid by unit tests that shouldn't need it.

`render/terminal.rs` holds 115 tests, and nearly all of them build a
`TerminalRenderer` via a `renderer()` helper. `TerminalRenderer::new`
constructs a `GlyphCache`, and `GlyphCache::new` calls
`fontdue::Font::from_bytes` on the bundled Iosevka TTF — a 10.8MB font file
embedded via `include_bytes!`. That's a full font parse (glyph table, cmap,
metrics) on every one of those 115+ test calls (`editor.rs`'s renderer tests
pay the same cost), even though none of them assert anything about actual
glyph shapes — they check box placement, cursor position, status-line text,
and escape sequences. The font parse is incidental setup cost, not something
under test.

## Technical Design

**`GlyphSource` trait** (`render/font.rs`), the seam between `TerminalRenderer`
and glyph rasterization:

```rust
pub(super) trait GlyphSource {
    fn glyph(&mut self, ch: char, hint: bool) -> &Canvas;
}
```

`GlyphCache` implements it with its existing behavior, unchanged: parse the
real font once at construction, rasterize and cache glyphs on demand.

**`TerminalRenderer` takes the source as a constructor argument** instead of
building one internally:

```rust
impl TerminalRenderer {
    pub(crate) fn new(terminal: Terminal, glyph_source: Box<dyn GlyphSource>) -> Self {
        TerminalRenderer { terminal, cache: HashMap::new(), glyph_source }
    }
}
```

The `glyph_cache: GlyphCache` field becomes `glyph_source: Box<dyn GlyphSource>`.
No other method on `TerminalRenderer` changes — `draw_label` already only
calls `.glyph(character, label.hint)`, which is exactly the trait method.

**Production** (`editor.rs::open`, the one real call site) switches to:

```rust
let glyph_source = Box::new(GlyphCache::new(terminal.cell_width, terminal.cell_height));
let mut renderer = TerminalRenderer::new(terminal, glyph_source);
```

Font parsing still happens exactly once per process — no behavior change for
the running app.

**Tests get a `FakeGlyphSource`** (`render/font.rs`, `#[cfg(test)]`) that never
touches `fontdue` or the TTF bytes:

```rust
#[cfg(test)]
pub(super) struct FakeGlyphSource {
    cell_width: i64,
    cell_height: i64,
    blank: Canvas,
}

#[cfg(test)]
impl FakeGlyphSource {
    pub(super) fn new(cell_width: i64, cell_height: i64) -> Self {
        let pixels = vec![0u8; (cell_width * cell_height) as usize * 4];
        FakeGlyphSource {
            cell_width,
            cell_height,
            blank: Canvas { pixels, width: cell_width, height: cell_height },
        }
    }
}

#[cfg(test)]
impl GlyphSource for FakeGlyphSource {
    fn glyph(&mut self, _ch: char, _hint: bool) -> &Canvas {
        &self.blank
    }
}
```

It returns one fixed, fully-transparent `cell_width × cell_height` canvas for
every character — matching the real `GlyphCache`'s output *shape* (same
canvas dimensions), just with no coverage data computed and no font involved.
Tests that care about glyph placement/count (e.g. the `is_glyph` filter in
`render/terminal.rs`) keep working unchanged; tests that cared about actual
rendered glyph pixels don't exist today and aren't a goal here.

**Both `renderer()` test helpers switch to the fake:**

```rust
// render/terminal.rs
fn renderer_on(terminal: Terminal) -> TerminalRenderer {
    let source = Box::new(FakeGlyphSource::new(terminal.cell_width, terminal.cell_height));
    TerminalRenderer::new(terminal, source)
}

// editor.rs
fn renderer() -> TerminalRenderer {
    let terminal = Terminal { cols: 20, rows: 10, cell_width: 1, cell_height: 1 };
    let source = Box::new(FakeGlyphSource::new(terminal.cell_width, terminal.cell_height));
    TerminalRenderer::new(terminal, source)
}
```

### Decisions and trade-offs

- **Trait object (`Box<dyn GlyphSource>`), not a generic parameter.** A
  generic `TerminalRenderer<G: GlyphSource>` would push the type parameter
  into every signature that names `TerminalRenderer` (`editor.rs::open`,
  `edit`, struct fields). The trait object keeps `TerminalRenderer` and the
  `Renderer` trait exactly as they are today; the extra vtable indirection on
  glyph lookup is immaterial next to the cost it replaces.
- **One constructor, not two.** `TerminalRenderer::new(terminal, glyph_source)`
  is the only way to build one — no separate `with_glyph_source` test-only
  constructor. Production and tests both pass a source explicitly; there's no
  implicit "default" behavior to keep in sync.
- **Fake returns a fixed blank canvas, not a tiny real font.** A minimal
  one-glyph TTF fixture was considered and rejected: tests render arbitrary
  status-line text and user-typed labels, so most characters would still hit
  a `.notdef` fallback glyph, and authoring/maintaining a custom font fixture
  is new tooling for no real fidelity gain. The fake sidesteps `fontdue`
  entirely.
- **No change to `GlyphCache`'s own construction.** An earlier direction had
  `GlyphCache::new` take an already-parsed `Font` so it could be parsed once
  and cloned across tests. That's no longer needed — tests don't construct a
  real `GlyphCache` at all anymore, so `GlyphCache::new(cell_width,
  cell_height)` keeps parsing `FONT_BYTES` internally, called exactly once by
  production.
