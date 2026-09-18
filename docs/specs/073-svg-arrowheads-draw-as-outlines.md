# 073: SVG Arrowheads Draw as Outlines

## User Story

I can export my diagram to SVG and see arrowheads that look exactly like they do
in the Terminal — hollow outlines, not solid filled triangles.

## Acceptance Criteria

- The SVG arrowhead is drawn as two edge strokes only, with no solid fill inside
- Those two edge strokes are the same stroke thickness as the box borders currently have in the SVG
- The hollow arrowhead applies to every arrow, pointing left or right
- I can run `dre --svg` and open the result in the browser, and the arrowheads look just like they do in the Terminal render

## Technical Design