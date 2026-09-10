# 022 - Start a canvas with one keystroke

## Story

Bob opens Dre to an empty canvas. He presses `b` and a box appears, cursor
already inside it, so he can start typing his first label without a second
thought. Once that box exists, `b` goes back to asking him which way.

## Acceptance Criteria

- `b` on an empty canvas creates a box, with no direction key needed.
- That box is selected and Dre is in insert mode, so typing goes straight into its label.
- `b` on a canvas that already has a box waits for a direction, as it does today.

## Technical Design
