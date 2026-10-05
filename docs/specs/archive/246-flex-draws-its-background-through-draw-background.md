# flex draws its background through draw_background

## Problem

`start` in `src/flex/mod.rs` builds the background pixel buffer
inline, by hand: it cycles `BACKGROUND_RGBA` into a `Vec<u8>`, then
immediately calls `transmit`/`place` and writes the result to
`stdout`. That inline block mixes "what to draw" with "how `start`
sequences startup," and `BACKGROUND_RGBA`'s bytes are decimal, which
hides the colour it actually is.

This is a refactoring spec: no behaviour the user can see changes.

## Technical Design

### `BACKGROUND_RGBA` shows its colour in hex

Stays a `[u8; 4]` (RGBA, alpha included) — kept at 32-bit depth
(`f=32`) rather than dropped to 24-bit, because alpha is reserved for
future layers to blend against this one, even though the background
itself is always fully opaque today. Only the literal's formatting
changes, from decimal to hex bytes:

```rust
const BACKGROUND_RGBA: [u8; 4] = [0x0A, 0x0B, 0x0D, 0xFF];
```

No new type, no packed `u32`, no conversion step — same array, same
type, hex digits instead of decimal.

### `draw_background` groups the drawing

A new top-level function in `flex/mod.rs`, next to `start`/`run`,
replaces the inline block in `start` (building the pixel buffer,
calling `kitty::transmit` and `kitty::place`, writing, and flushing):

```rust
fn draw_background<W: Write>(stdout: &mut W, width: i64, height: i64) -> io::Result<()> {
    let pixels: Vec<u8> = BACKGROUND_RGBA
        .iter()
        .copied()
        .cycle()
        .take((width * height * 4) as usize)
        .collect();
    stdout.write_all(kitty::transmit(&pixels, width, height).as_bytes())?;
    stdout.write_all(kitty::place(0, 0).as_bytes())?;
    stdout.flush()
}
```

`start` calls `draw_background(&mut stdout, width, height)?` instead
of building and sending the buffer itself.

`draw_background` stays outside `mod kitty` on purpose: `mod kitty`
only knows the Kitty graphics protocol (how bytes reach the
terminal), while `draw_background` knows what to draw and currently
chooses Kitty as the transport. Keeping it top-level means swapping
the transport later only changes what `draw_background` calls
internally — `start` and `BACKGROUND_RGBA` don't need to know.

### Migration

One slice, no behaviour change:

1. Reformat `BACKGROUND_RGBA`'s bytes as hex literals.
2. Extract `draw_background` from `start`'s inline block; `start`
   calls it instead.
3. Confirm `dre-flex` still starts, paints the same background, and
   Ctrl-C exits cleanly.

## Acceptance Criteria

- `BACKGROUND_RGBA` is written with hex byte literals, still
  `[u8; 4]`, still 32-bit (alpha included).
- `draw_background` exists as a top-level function in `flex/mod.rs`,
  performs the pixel-buffer build, `transmit`, `place`, write, and
  flush, and is the only place that does so.
- `start` contains no inline pixel-buffer or write/flush logic for
  the background — it only calls `draw_background`.
- `draw_background` is not part of `mod kitty`; it calls into
  `mod kitty`'s public functions.

## Out of scope

- Any change to the actual background colour, the Kitty protocol
  parameters (`f=32`, chunking, etc.), or `start`/`run`'s visible
  behaviour, timing, or Ctrl-C exit path.
- Building a general layer system (tracked separately, spec 243) —
  this spec only extracts one function for one layer.
- Dropping alpha or moving to 24-bit pixels.
