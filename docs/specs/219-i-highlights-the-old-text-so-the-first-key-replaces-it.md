# Flex: i highlights the old text so the first key replaces it

## User Story

Doug has a box that reads "Hello" and he wants it to read "World". He presses i,
the old word lights up on a filled background so he can see exactly what he is
about to overwrite, and he types W — the whole old word is gone in one
keystroke. No backspacing through five letters, no risk of leaving half the old
word stuck at the end.

## Acceptance Criteria

- Pressing i on a box that has text highlights that entire text with a filled
  background behind it.
- The first printable key Doug types after i replaces the whole highlighted
  text: "Hello" then W leaves the box reading "W".
- Every key he types after that first one appends, exactly as it does today.
- Backspace as that first key leaves the text empty — the old text is already
  gone, so there is nothing left to delete.
- Enter as that first key keeps the old text and moves him on to the next text
  node, exactly as it does today.
- Pressing i on a box that has no text behaves exactly as it does today: no
  highlight, and typing just types.
- Only i produces a highlight. o and the Enter-adds-a-box flow both start on
  empty text, so they never highlight anything.

## Technical Design

This is a `dre-flex` (`src/flex/`) feature.

`FlexBox` (`src/flex/state.rs`) has no cursor field — typing always
appends/pops at the end of `text: Option<String>`. `FlexMode` currently has
two variants, `Move` and `Write`. We add a third: `FlexMode::Replace`.

**Entering Replace (`i` handler, `state.rs:220-223`)**

- If the selected box's text is `Some(s)` with `!s.is_empty()`, set
  `mode = FlexMode::Replace`. The text itself is left untouched so it still
  renders.
- Otherwise (text is `None` or `Some("")`), behave exactly as today: ensure
  `text` is `Some(String::new())` and set `mode = FlexMode::Write`.

**Handling keys in Replace mode**

`write_key` gains a mode check at the top for `FlexMode::Replace`:

- Backspace (`"\x7f"`): set `text = Some(String::new())`, set
  `mode = FlexMode::Write`, and return — wrapped in `history::recorded` so
  undo restores the whole original label in one step. This bypasses the
  normal backspace arm, which would otherwise see empty text on a droppable
  box and delete the box.
- Enter (`"\r"`): no special handling. Fall through unchanged into the
  existing Enter arm, which already preserves the old text, clones the box
  for the next sibling, and sets its own resulting mode — exactly today's
  "moves on to the next text node" behaviour.
- Any printable key: set `text = Some(String::new())`, set
  `mode = FlexMode::Write` (wrapped in `history::recorded`, same undo
  reasoning as Backspace), then fall through into the existing
  printable-key arm so the typed character is pushed onto the now-empty
  text.

**Rendering the highlight**

No new `PlacementNode` variant is needed. In `paint()`, when
`path == state.selected && state.mode == FlexMode::Replace`, emit an extra
`PlacementNode::Box` (reusing the existing filled-box/`solid_fill`
mechanism) using `FLEX_SELECTED_COLOUR`, positioned before the `Label`
placement so it paints behind the text. Its rect is sized to the label
exactly, not the full text rect (which may be wider due to justify/padding):
`{ x: text_rect.x, y: text_rect.y, width: label.chars().count(), height: 1 }`.
