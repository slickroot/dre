# Flex: s adds a text beside the last one in MOVE

## User Story

Noor runs `dre-flex`, types "Hello" and presses Enter, so they're in MOVE. They press `s`, and a new empty text appears in the same box, right after "Hello", with a 1-cell space between them. `dre-flex` switches to WRITE, and Noor types "World". The box grows to hold "Hello World", and it stays centred on the screen. Noor presses Enter, then `s` again, and a third text appears after "World". Back in WRITE, pressing `s` just types an "s".

## Acceptance Criteria

- In MOVE, `s` adds a new empty text to the right of the last text in the same box.
- After `s`, `dre-flex` switches to WRITE.
- Typed characters go into the newly added text.
- Each additional `s` adds another text to the right of the last one.
- There is a 1-cell space between texts.
- The box grows to fit all its texts and the spaces between them.
- The box stays centred on the screen.
- Backspace on an empty new text does nothing.
- In WRITE, `s` types an "s" and doesn't add a text.

## Technical Design
