# 066 - README explains how to start and quit dre

## Story

Bob has installed `dre`. He reads the README and learns how to start `dre`,
open or create a diagram, and quit, with or without saving.

## Acceptance Criteria

The README explains:

- `dre` opens an empty canvas.
- `dre plans.dre` opens `plans.dre`, or starts a new diagram under that name
  if the file doesn't exist.
- With a filename, `q` saves to that file and quits.
- Without a filename, `q` asks "Save as:" (pre-filled with `diagram.dre`).
  `Enter` saves and `Esc` quits without saving.
- `Ctrl-C` quits without saving.

## Technical Design
