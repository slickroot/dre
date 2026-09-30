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
- The separator between the box count and "dre" is a bullet (`•`) instead of
  `.`.
- While the "Save as:" prompt is active, the mode cell keeps showing
  whichever mode was active before the prompt opened (e.g. `COMMANDING`),
  styled the same as always, and the filename segment is replaced with
  `Save as: {filename}` (with the existing cursor block). The rest of the
  line (box count, separators, "dre") is unaffected and keeps the dim
  styling.

## Technical Design

### Data model (`src/state.rs`)

`StatusLine` changes from two plain `String`s to two sequences of styled
segments:

```rust
pub(crate) struct StatusLine {
    pub(crate) left: Vec<Segment>,
    pub(crate) right: Vec<Segment>,
}

pub(crate) struct Segment {
    pub(crate) text: String,
    pub(crate) style: Style,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct Style {
    pub(crate) bold: bool,
    pub(crate) background: Option<(u8, u8, u8, u8)>, // RGBA; alpha is composited against BACKGROUND at render time
    pub(crate) foreground: Option<(u8, u8, u8)>,      // RGB
}
```

`status_line()` always builds the same segment shape, regardless of mode:

- Mode segment (`left[0]`): text is the mode label with one literal space
  of padding on each side (e.g. `" COMMANDING "`), so the padding inherits
  the segment's own style. Style: `bold: true`, `background:` lime RGBA
  (opaque, alpha `0xFF`, from the existing `palette(0)` "lime" entry),
  `foreground:` the app's `BACKGROUND` RGB.
- `" │ "` separator (`left[1]`): dim style (see below).
- Filename segment (`left[2]`): `"{filename}{marker}"` normally, or
  `"Save as: {filename}{CURSOR}"` when `state.mode` is `SavePrompt`. Dim
  style.
- Box count segment (`right[0]`): `"{box_count} boxes"`. Dim style.
- `" • "` separator (`right[1]`): dim style.
- `"dre"` segment (`right[2]`): dim style.

"Dim style" means `bold: false`, `background:` black composited at a local
alpha constant over `BACKGROUND`, `foreground: None` (renderer emits no FG
escape, so it falls back to the terminal's default foreground — satisfying
"normal (non-reverse-video) foreground text").

### Rendering (`src/render/terminal.rs`)

`render_status_line`:

1. Sum the `text.len()` of every segment in `left` and `right` to compute
   remaining width, same as today's padding calculation.
2. Build a filler `Segment` of repeated spaces with the same dim style,
   sized to fill the remaining columns (or truncate the last segment if
   content overflows `cols`, same edge case handling as today).
3. Move the cursor once: `\x1b[{rows};1H`.
4. For each segment in `left`, filler, `right` (in order): emit `\x1b[0m`,
   then `\x1b[1m` if `bold`, then `\x1b[48;2;r;g;bm` if `background` is
   `Some` (after alpha-compositing against `BACKGROUND`), then
   `\x1b[38;2;r;g;bm` if `foreground` is `Some`, then the segment's text.
5. Emit a final `\x1b[0m` after the last segment.

A local helper `fn composite(overlay: (u8, u8, u8, u8), backdrop: (u8, u8,
u8)) -> (u8, u8, u8)` in `render/terminal.rs` does the alpha blend. The dim
alpha value is a local `const DIM_ALPHA: u8` in the same file (not in
`diagram.rs`'s palette, since it's a status-line-specific rendering
detail).

### Colors

- Lime: reuse `diagram::palette(0)` RGB, alpha `0xFF`.
- Mode-cell foreground: `diagram::BACKGROUND`'s RGB.
- Dim background: `(0, 0, 0, DIM_ALPHA)` composited over `diagram::BACKGROUND`'s
  RGB.

### Save prompt

No separate code path or `StatusLine` variant — `status_line()` branches
only on the filename segment's text (`"{filename}{marker}"` vs.
`"Save as: {filename}{CURSOR}"`), reusing the same mode segment, separators,
and right-hand segments as Command/Insert. This removes the old
`SavePrompt` early-return branch in favor of one unified builder.

### Tests affected

The exact-string assertions in `render/terminal.rs` (`status_line_output`,
`status_line_text`, ~lines 2075-2180) that check for the blanket
`\x1b[7m...\x1b[0m` reverse-video wrap need rewriting to assert the new
per-segment ANSI sequences (bold/background/foreground codes per segment)
instead.
