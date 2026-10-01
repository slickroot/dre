# Doug undoes his last change with u

## User Story

Doug is building a diagram in dre-flex. He adds a box he didn't want, so he presses `u` in move mode and it's gone. He presses `u` a few more times and watches his earlier changes come off one at a time, until he's back where he wanted to be. Happy, he carries on drawing!

## Acceptance Criteria

- In move mode, pressing `u` undoes the last change to the diagram: adding a box (`a`, `A`), adding a text (`s`, or `i` on an empty box), editing a text (`i`), or toggling width, spread, direction or fill (`w`, `g`, `d`, `f`).
- Pressing `u` again keeps going back one change at a time, all the way to how the diagram was when Doug opened dre-flex.
- Adding a text and typing into it count as one change, so after `s`, "Hello" and Enter, one `u` removes the whole text.
- Editing a text counts as one change, so after `i`, " World" and Enter on "Hello", one `u` brings back "Hello".
- Moving the selection (`h`, `j`, `k`, `l`) isn't a change, so `u` skips past it and undoes the last real change.
- Pressing `u` when there's nothing to undo does nothing.
- In write mode, `u` types the letter "u" like any other letter.

## Technical Design
