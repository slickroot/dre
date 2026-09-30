# Dim joins the palette

This is a technical spec, not a user story: it has no user-facing behaviour.

## Problem

`DIM` (index 7) is a colour that lives outside `PALETTE`. `style::colour()` special-cases it, so there are two lookups with different domains: `palette()` knows 7 colours, `colour()` knows 8. The file reader validates against `palette()`, so `colour="7"` is rejected although the renderer draws it fine.

## Acceptance Criteria

- `PALETTE` has 8 entries: 5 accents (0-4), foreground (5), background (6) and dim (7).
- There is one lookup, `style::palette()`. `style::colour()` is gone.
- The footer draws the filename in the same grey as before.
- A `.dre` file with `colour="7"` opens. `colour="8"` is still rejected.
- The box colour cycle walks all 8 entries, then returns to plain.
- The file format does not change. Existing files read and save exactly as before.

## Technical Design

### Approach

The index stays the colour. The file stores a slot number and the code turns it into RGB, like an Xresources theme. Hex in the file was considered and dropped: the slot number is the stable contract, and hex would let a file name a colour the palette does not have. A typed `Colour` enum was considered and dropped too: the file reader is the only place an arbitrary index appears and it already validates, so the enum would only guard against a mistake the code does not make, at the cost of about 40 edited tests and of the `LIME`/`VIOLET`/`AMBER` names the footer uses. `Node.colour` stays `Option<u8>`.

The change is to move dim into the table and delete the special case.

### Components

- **`style::PALETTE`** gains `("dim", (0x8C, 0x8E, 0x91))` as its 8th entry. `DIM` stays `7`.
- **`style::palette(index)`** is the only lookup. **`style::colour()`** is deleted.
- **`render::colour(Option<u8>)`** calls `palette(colour.unwrap_or(FOREGROUND))` instead of `style::colour`.
- **`dre_format::in_palette`** and **`next_on_palette`** are unchanged. They already ask `palette()`, so they follow the table: 7 is valid and the cycle has one more stop.

### Collaborations

No new ones. `render`, `dre_format`, `tty` and `view` all go through `style`, and `style` now has one lookup.

### Tests

- `style`: `palette(DIM)` returns the grey; the palette has no colour past index 7; the cycle reaches `Some(7)` and then `None`.
- `dre_format`: `colour="7"` reads; the "outside the palette" test moves to `colour="8"`, on a top-level and a nested box; the edges test covers 7.
- `render`: `Some(DIM)` draws the grey through `palette()`.
- Manual: the footer filename looks the same as before, and cycling a box's colour passes through the grey.

### Out of scope

- Hex colours in the file.
- A typed `Colour` enum or `Slot` newtype.
- User-configurable themes.
