# 105: Arrows don't steal attention

## User Story

Doug opens a diagram in dre. The arrows are drawn as soft, thin lines instead
of bright, thick ones, so the boxes and their labels are the first things Doug
sees. Doug can follow the arrows from box to box without them pulling the eye
away, and gets on with the diagram.

## Acceptance Criteria

- Arrows are drawn in the foreground colour at 50% opacity, both in the
  terminal and in the exported SVG.
- Arrows are 2px thick in the terminal, down from 4px. The exported SVG stays
  at 2px.
- The colour and thickness apply to the whole arrow: the shaft, the branching
  lines to several children, and the arrowhead.

## Technical Design
