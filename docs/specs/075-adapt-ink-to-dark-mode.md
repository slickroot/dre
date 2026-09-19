# 075: Adapt ink to dark mode

## User Story

When I open my exported diagram on a dark-themed page (like GitHub dark mode), the diagram's
plain color — the labels, the arrows, and the plain box borders — turns white, so the diagram
stays readable. The diagram also paints its own background — pure white on light pages, pure
black on dark pages — so it never blends into the page around it.

## Acceptance Criteria

- On a light-mode page, the exported SVG looks exactly as it does today
- On a dark-mode page, the labels turn white
- On a dark-mode page, the arrows turn white
- On a dark-mode page, the plain (uncoloured) box borders turn white
- Coloured box borders and fills stay the same in both modes
- A pure-white background is painted behind the diagram on light mode and a pure-black background on dark mode, so the page's own background never shows through

## Technical Design

The exported SVG self-adapts: every exported document carries a `<style>` block, defined via two CSS custom properties, so the same file renders black ink on a white background on light pages and white ink on a black background on dark pages.

**Mechanism**
- A `<style>` block is emitted on every SVG, immediately after the opening `<svg ...>` tag and before any `<defs>`.
- The block defines two custom properties on the `svg` root: `--ink` for foreground ink and `--bg` for the background.
  - Light default: `svg { --ink: rgb(0,0,0); --bg: rgb(255,255,255) }`.
  - Dark override: `@media (prefers-color-scheme: dark) { svg { --ink: rgb(255,255,255); --bg: rgb(0,0,0) } }`.
- The variables inherit from the `svg` root, so the arrowhead marker inside `<defs>` picks up `--ink` too, and the background `<rect>` picks up `--bg`.
- A background `<rect>` is emitted immediately after the `<style>` block and before any `<defs>`. It spans the full `viewBox` (its `x`, `y`, `width`, and `height` are the viewBox origin and span) and is filled with `var(--bg)`. Because it is the first paint in the document, boxes, arrows, and labels sit on top of it.

**Ink elements**
- `SvgRenderer` swaps its hard-coded `INK` tuple for the `var(--ink)` string at every ink site:
  - arrow shaft/trunk/stop strokes (`arrow_paths`)
  - the arrowhead marker stroke (`marker_defs`)
  - plain (uncoloured) box borders (`rect`, where `node.colour` is `None`)
  - label text fills (`label_text`)
- Coloured box borders continue to use their literal palette `rgb`, so they are structurally immune to the dark-mode override — no selector can retarget them.
- The background is painted by the single background `<rect>`; `--bg` only ever fills it and no other background element is added.

**Responsibility**
- The entire change lives in `src/svg.rs`: the added `<style>` emission, the background `<rect>`, and the replacement of `INK` with `var(--ink)` at the four ink sites. No CLI, layout, or state changes.
- The `INK` black tuple remains the value for black in both the light `--ink` default and the dark `--bg` override, both declared inside the emitted style block.