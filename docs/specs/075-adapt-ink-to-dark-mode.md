# 075: Adapt ink to dark mode

## User Story

When I open my exported diagram on a dark-themed page (like GitHub dark mode), the diagram's
plain color — the labels, the arrows, and the plain box borders — turns white, so the diagram
stays readable with no background added. On light mode it looks exactly like today.

## Acceptance Criteria

- On a light-mode page, the exported SVG looks exactly as it does today
- On a dark-mode page, the labels turn white
- On a dark-mode page, the arrows turn white
- On a dark-mode page, the plain (uncoloured) box borders turn white
- Coloured box borders and fills stay the same in both modes
- No background is added — the page's own background still shows through

## Technical Design

The exported SVG self-adapts: every exported document carries a `<style>` block, defined via a single CSS custom property, so the same file renders black on light pages and white on dark pages.

**Mechanism**
- A `<style>` block is emitted on every SVG, immediately after the opening `<svg ...>` tag and before any `<defs>`.
- The block defines `svg { --ink: rgb(0,0,0) }` as the light default and `@media (prefers-color-scheme: dark) { svg { --ink: rgb(255,255,255) } }` as the dark override.
- The variable inherits from the `svg` root, so the arrowhead marker inside `<defs>` picks it up too.

**Ink elements**
- `SvgRenderer` swaps its hard-coded `INK` tuple for the `var(--ink)` string at every ink site:
  - arrow shaft/trunk/stop strokes (`arrow_paths`)
  - the arrowhead marker stroke (`marker_defs`)
  - plain (uncoloured) box borders (`rect`, where `node.colour` is `None`)
  - label text fills (`label_text`)
- Coloured box borders continue to use their literal palette `rgb`, so they are structurally immune to the dark-mode override — no selector can retarget them.
- No background element is added; the variable only re-paints foreground ink.

**Responsibility**
- The entire change lives in `src/svg.rs`: the added `<style>` emission and the replacement of `INK` with `var(--ink)` at the four ink sites. No CLI, layout, or state changes.
- The `INK` black tuple remains the default value declared inside the emitted style block.