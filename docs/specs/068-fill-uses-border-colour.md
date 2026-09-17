# 068: Fill uses border colour

## User Story

As a user, when I press `f` or `F` to fill boxes, the fill always matches the box's border color — toggling fill on or off instead of cycling through colors.

## Acceptance Criteria

- Pressing `f` on a box with a border color toggles fill on (border color at 30% opacity)
- Pressing `f` on a box with fill already on toggles fill off (transparent)
- Pressing `f` on a box with no border color does nothing
- Pressing `F` toggles fill on/off for all siblings, each using its own border color
- Pressing `F` when all siblings have no border color does nothing

## Technical Design

