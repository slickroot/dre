# 033 - Pack siblings into the next free row

## Story

Bob has three boxes hanging off one parent, and gives the middle one two
children of its own. Today that shoves the outer two siblings far apart, leaving
a big empty margin above and below. Instead, the first and third child slide in
as close as they can get to the middle child without bumping into its children,
so the picture stays tight and no row is left empty.

## Acceptance Criteria

- Each gap between two adjacent siblings is sized on its own, from just those
  two neighbours' subtrees — one tall child no longer pushes all of its siblings
  apart.
- Siblings sit as close together as they can without any two boxes colliding.
- The vertical gap between boxes is never less than 3 rows, anywhere boxes are
  stacked.
- With an odd number of children, the parent sits level with the middle child,
  whatever the gaps around it look like.
- With an even number of children, the parent sits one normal half-gap below the
  first of the two middle children.
- A parent with plain children looks exactly as it does today.

## Technical Design
