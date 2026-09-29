# Each frame is shown whole

This is a rendering spec. It changes when the terminal shows a frame, not what the frame contains.

## Problem

With a busy diagram on screen (for example a root box with ten children full of text), the screen blinks while the user is idle.

The controller renders a full frame every time the key poll times out (`IDLE_TIMEOUT_MS`), about once a second, even when nothing changed. Each frame, built by `Frame::into_bytes`, writes the text rows, then deletes every image placement (`kitty::soft_clear`), then places every image again. A busy diagram has hundreds of placements: one per label character (spec 169) and one per box and glow cell (spec 170).

The terminal paints on its own schedule, not at frame boundaries. When it paints after the soft clear and before the last placement, the user sees a screen with images missing or half placed for one refresh. The more placements a frame has, the wider that window is and the more often it is caught.

## Acceptance Criteria

- The terminal never shows a partly written frame: every paint shows either the previous frame complete or the new frame complete.
- The screen does not blink while the user is idle, whatever the size of the diagram.
- Typing, moving and resizing do not flicker.
- The bytes of each frame are unchanged apart from the wrapping below. The same text, deletes and image commands are sent in the same order.
- Terminals that do not support synchronized output behave exactly as today.

## Technical Design

### Synchronized output

Every frame is wrapped in the synchronized output mode (DEC private mode 2026), which WezTerm and Kitty support:

- `BEGIN_SYNCHRONIZED_UPDATE = "\x1b[?2026h"` is written first, before `HOME_CURSOR`.
- `END_SYNCHRONIZED_UPDATE = "\x1b[?2026l"` is written last, after the final image command.

While the mode is set, the terminal keeps showing the last complete screen and applies everything it receives without painting. It paints once when the mode is reset. Terminals that do not recognise mode 2026 ignore both sequences, as they ignore any unknown private mode.

The wrapping lives in `Frame::into_bytes`, so every render path gets it: the normal render, the command-mode flash render, and the render after a timed command. Each frame is written with a single `write_all`, so exactly one begin and one end are sent per frame and they always pair.

### Out of scope

- Skipping the render when the key poll times out and nothing changed.
- Diffing text or placements against the previous frame, or no longer deleting every placement each frame.
- Wrapping anything outside a frame: entering or leaving the alternate screen, hiding or showing the cursor, the Kitty support query in `kitty::require`, and the transient-image deletes already sent inside a frame.
- Detecting whether the terminal supports mode 2026.

### Tests

`render/terminal.rs`:

- A rendered frame starts with `BEGIN_SYNCHRONIZED_UPDATE` followed by `HOME_CURSOR`, and ends with `END_SYNCHRONIZED_UPDATE`.
- A frame contains exactly one begin and one end, and the end comes after the last image command, both for an empty scene and for a scene with fresh, cached and transient images.
- With the wrapping stripped, the frame bytes are exactly those of today: rows, then soft clear, then deletes of the previous transient images, then image commands in scene order.
- Existing tests that strip `HOME_CURSOR` or compare whole frame prefixes or suffixes are updated to account for the wrapping. They keep asserting the same content and order.
