# Status line shows file and box info

Marouane is editing a diagram in dre. He glances at the status line at the bottom of the screen and sees everything at a glance: on the left, the current mode, the filename, and a `[+]` if he has unsaved changes; on the right, how many boxes are in the diagram and the app name "dre". He makes an edit, sees the `[+]` appear, saves, and watches it disappear — all without leaving the keyboard.

## Acceptance Criteria

- Status line shows, left-aligned: `mode | filename`
- If there are unsaved changes, `[+]` appears right after the filename; otherwise it's omitted
- Before the first save, filename shows the default `diagram.dre`
- Status line shows, right-aligned: `N boxes . dre` (always plural, e.g. "1 boxes", "2 boxes")
- Left and right groups are separated by blank space filling the remaining width

## Technical Design
