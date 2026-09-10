# Fill uses border palette with transparency

## User Story

As a user, when I fill a box in the kitty graphics view, I want the fill to use the same colour palette as the border (at 30% opacity), so my diagram has a consistent, faded fill that matches its border colour scheme.

## Acceptance Criteria

- In the kitty graphics renderer, pressing `f` on a selected box cycles its fill through the same 5 palette colours used for borders.
- Cycling starts from "no fill" (transparent interior), then colour 1 → colour 2 → ... → colour 5 → back to "no fill."
- When a fill colour is set, the box interior renders as that palette colour blended at 30% opacity against the background.
- This applies only to the kitty graphics renderer (terminal ANSI rendering is unaffected).

## Technical Design
