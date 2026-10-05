# Capital A prepends a square

## Problem

Dana is editing her flex layout. She presses capital `A`, and a new square appears at the front of the row in a darker color (`#2A2A2E`), sliding every square she's already placed one step to the right to make room.

## Technical Design

## Acceptance Criteria

- Pressing `A` (uppercase) inserts a new square at the leftmost position.
- All previously placed squares shift one slot to the right.
- The new square's color is `#2A2A2E`.
- Pressing lowercase `a` still appends to the end as before, unaffected.
- Pressing `A` repeatedly keeps inserting at the front each time, pushing the whole row right.

## Out of scope
