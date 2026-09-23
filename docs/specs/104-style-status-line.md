# 104: Style the status line

## User Story

Doug is working in dre and switches into COMMANDING or EDITING mode. He glances
at the status line and immediately spots the mode label in a bold lime cell
with a cell of padding on each side, while the rest of the line (filename and
box count) sits on a dim background with clean vertical-bar separators and a
filled circle before "dre." Satisfied that the status line is easy to read at
a glance, he keeps working.

## Acceptance Criteria

- In COMMANDING/EDITING modes, the mode label is rendered bold, with a lime
  background, text colored using the app's background color, and one cell of
  padding on each side.
- The rest of the status line (filename`[+]` and box count) is rendered with
  a dim/darker shade of the app's background color, with normal
  (non-reverse-video) foreground text.
- The separator between the mode cell and the filename is a vertical bar with
  one space of padding on each side (`" │ "`).
- The separator between the box count and "dre" is a filled circle (`●`)
  instead of `.`.
- The "Save as:" prompt keeps its current plain rendering — unaffected by
  this story.

## Technical Design
