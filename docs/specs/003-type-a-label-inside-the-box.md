# 003 - Type a label inside the box

## Story

Bob presses `b` and a box appears. He starts typing straight away, and his label appears inside the box, which widens to fit as he goes. He mistypes, hits backspace, and the box shrinks back down with the text.

## Acceptance Criteria

- After the box appears, typed characters go straight into the box - no mode to enter first.
- A visible cursor inside the box shows where the next character will land.
- The box widens as the label grows so the text fits.
- The box stays centered on screen as it grows, expanding in both directions.
- Backspace removes the last character.
- The box shrinks back as the label gets shorter, staying centered.
- The box never shrinks below 3x3.

## Technical Design
