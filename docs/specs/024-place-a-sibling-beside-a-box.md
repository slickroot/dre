# 024 - Place a sibling beside a box

## Story

Bob has a box selected with nothing pointing into it. He presses `sl` and a new
box appears beside it on the right — unconnected, no arrow, just sitting there
waiting for a label. `sj` puts one below instead.

## Acceptance Criteria

- `sl` on a box with no parent creates a new box to its right, joined by a space
  rather than an arrow.
- `sj` on a box with no parent creates a new box below it, joined by a space
  rather than an arrow.
- The new box is appended at the end of the canvas, as `b` does today, even when
  the selected box is not the last one.
- `sh` and `sk` create nothing.
- `s` on an empty canvas creates nothing.
- The new box becomes the selection and the app enters insert mode, as `b` does.

## Technical Design
