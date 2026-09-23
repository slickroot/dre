# Terminal background fill

## User Story

Bob opens dre in his terminal. Instead of seeing his terminal's own background showing through, the entire dre canvas is filled with the new solid background color, `#0A0B0D`, giving the app a consistent look regardless of his terminal theme.

## Acceptance Criteria

- When dre opens, the entire canvas/viewport area is filled with `#0A0B0D`.
- This background fill remains in place regardless of what boxes, text, or arrows are drawn on top of it.
- The fill covers the full viewport, not just the area behind existing shapes.
- No part of the underlying terminal's own background color is visible through the dre canvas.

## Technical Design

dre currently has no concept of an opaque canvas backdrop: boxes, arrows, and text are rasterized as transparent RGBA sprites (`src/canvas.rs`, `src/render/shapes.rs`, `src/render/font.rs`) and composited via the Kitty graphics protocol (`src/kitty.rs`) over whatever the terminal's own background happens to be. There is no full-viewport fill layer today.

Rather than adding a full-viewport Kitty image sprite (which would require re-encoding/compressing and re-transmitting a solid-colour RGBA bitmap every frame), we set the terminal's actual background colour once via the `OSC 11` escape sequence when dre takes over the terminal. This is native to the terminal emulator, costs nothing per frame, and needs no changes to the existing sprite-compositing pipeline — every transparent pixel in every box/arrow/text sprite will now reveal `#0A0B0D` instead of the user's terminal theme.

**Lifecycle hook:** `src/terminal.rs`'s `RawScreen` guard (`RawScreen::open`/`impl Drop for RawScreen`) already brackets the terminal session (entering/leaving the alternate screen, hiding/showing the cursor) and runs on panic unwind. We extend it symmetrically:
- `RawScreen::open` additionally writes the `OSC 11` sequence to set the background to `#0A0B0D`.
- `Drop for RawScreen` additionally writes `OSC 111` ("reset background to default") before leaving the alternate screen, restoring whatever background the user's terminal had before dre started. No querying/storing of the original colour is needed.

**Colour definition:** Following the pattern established for `FOREGROUND` on the in-progress `097-new-foreground-colour` branch (`src/diagram.rs`: `pub(crate) const FOREGROUND: u8 = 5;`, resolved via `palette(FOREGROUND)`), we add:
- `pub(crate) const BACKGROUND: u8 = 6;` in `src/diagram.rs`
- a corresponding `(10, 11, 13)` entry appended to `PALETTE`

`src/terminal.rs` resolves `palette(BACKGROUND)` to build the `OSC 11` hex string, keeping `#0A0B0D` defined in exactly one place rather than duplicated as a raw escape-code constant.

**Dependency:** This spec depends on the `BACKGROUND`/`FOREGROUND` palette pattern from spec 096/097. Implementation branches off `097-new-foreground-colour` rather than `main`.
