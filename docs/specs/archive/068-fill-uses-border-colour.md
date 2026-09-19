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

### Data model (`state.rs`)

Replace `Node.fill: Option<u8>` with `Node.filled: bool` (default `false`). A box's fill
colour is always *derived*: the 30%-opacity composite of the box's own border colour,
shown only when `filled && colour.is_some()`. There is no stored relation from fill to a
colour anymore; fill never drifts from the border because it is not stored.

- `Node.filled` is sticky through colour cycles: `c`/`C` moving a filled box's colour to
  `None` leaves `filled == true`, but renders no fill (no border colour to derive from).
  Cycling a colour back on makes the fill reappear.

### Commands (`command_mode.rs`)

Rename and re-behavior the two commands:

- `Command::CycleFill` → `Command::ToggleFill` (`f`), min_depth 1, undoable:
  - `colour` is `None` → no-op.
  - else `filled = !filled`.
- `Command::CycleSiblingsFill` → `Command::ToggleSiblingsFill` (`F`), min_depth 2, undoable:
  - every sibling has `colour == None` → no-op.
  - else if all siblings are filled → set every sibling `filled = false`.
  - else → set every sibling `filled = true` (colourless siblings get `filled = true`
    too but render transparent).
- Update `COMMAND_KEYMAP` descriptions (and regenerate README table via
  `UPDATE_README=1 cargo test`); the keymap sync test stays as-is.

### Rendering (`render.rs`)

Replace `fill_colour(fill: Option<u8>)` with a derived fill computed from
`(colour: Option<u8>, filled: bool)`:

- `!filled || colour.is_none()` → `TRANSPARENT`.
- else → `PALETTE[colour]` alpha-composited at `FILL_ALPHA` (77 ≈ 30%), unchanged pipeline.

`outline_box` builds its fill from the node's border colour + `filled`. The sprite cache
must still distinguish `filled` states for the same border colour (a filled and an
unfilled box of one colour are different sprites); keep `fill` in `SpriteKey` populated
from the derived value.

### Serialization (`dre_format.rs`, `file_document.rs`)

Presence of the `@fill` attribute *is* the flag — no `true`/`false` values:

- Write: emit `fill="<border colour index>"` only when `Node.filled && colour.is_some()`.
  Omit `@fill` when unfilled or when filled-but-colourless (sticky intent is not persisted;
  such a box reloads as unfilled; the visual round-trip is exact).
- Read (`to_state`): `filled = file_box.fill.is_some()`; any present `@fill`, including
  legacy numeric values such as `fill="0"`, now means filled.
- Drop the `fill` palette-index validation in `in_palette` (only `colour` is still
  constrained); a legacy `@fill` value that once meant "filled with palette index N"
  now simply means "filled".

### Undo/snapshot

No special handling. Snapshotting stays per-command for undoable commands; a no-op
`ToggleFill`/`ToggleSiblingsFill` press pushes a clone of the unchanged doc, and `u`
simply restores that identical doc (appears as a no-op).

