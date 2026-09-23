# New box palette colours

## User Story

Bob opens dre and colors a box on his diagram. He cycles through the available colors and sees the new palette — Acid lime, Mint, UV violet, Hot magenta, and Amber — instead of the old colors. When he exports his diagram to SVG, the colored boxes keep those same exact colors.

## Acceptance Criteria

- Cycling a box through the 5 colors shows, in order: Acid lime `#C6FF00`, Mint `#39FFB0`, UV violet `#B388FF`, Hot magenta `#FF3DF5`, Amber `#FFB020`.
- These colors are visible in the terminal rendering of a colored box.
- Exporting a diagram with colored boxes to SVG produces the same 5 hex values for the corresponding boxes.
- The old palette colors no longer appear anywhere.

## Technical Design

The palette lives in `src/diagram.rs` as a `const PALETTE: [(u8, u8, u8); 5]`, accessed everywhere (cycling, terminal rendering, SVG export) only through `palette(index: u8) -> Option<(u8, u8, u8)>`. No caller currently reasons about the palette's ordering or count beyond that function, and no test hardcodes the actual RGB values — tests only check indices/bounds via `palette(...)`. This makes the change a contained data swap.

- Change `PALETTE` to `const PALETTE: [(&str, (u8, u8, u8)); 5]`, an ordered array of (short name, RGB) pairs, in this order:
  - `("lime", (0xC6, 0xFF, 0x00))`
  - `("mint", (0x39, 0xFF, 0xB0))`
  - `("violet", (0xB3, 0x88, 0xFF))`
  - `("pink", (0xFF, 0x3D, 0xF5))`
  - `("amber", (0xFF, 0xB0, 0x20))`
- Ordering must stay index-addressable (array, not `HashMap`) since cycling logic (`next_colour` in `src/state.rs`) walks palette indices 0→4→`None` via `palette(i + 1).is_some()`.
- The name is self-documentation only — no accessor is added for it, and nothing in the UI displays it yet (YAGNI: no current caller needs it).
- `palette(index)` keeps its existing signature, `Option<(u8, u8, u8)>`, extracting just the RGB half of the tuple. This means no call sites (terminal rendering, SVG export, `colour()`, `fill_colour()`) need to change.
- No test changes are expected: existing tests reference colors indirectly via `palette(index)` and check bounds, not values.
