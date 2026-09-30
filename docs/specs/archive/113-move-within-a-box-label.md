## Story

Doug is editing a box and has typed "Chche". He notices the missing "a". He
presses ← three times, so the cursor sits between "C" and "h". He types "a" and
the label reads "Cache". Happy, he presses Esc and moves on. If he had typed
"Cxache" instead, he would have pressed Backspace to remove the "x" before the
cursor.

## Acceptance Criteria

- In edit mode, ← moves the cursor one letter to the left and → moves it one
  letter to the right. The cursor is drawn where it is.
- Typing a letter puts it in at the cursor.
- Backspace deletes the letter before the cursor.
- ← at the start of the label and → at the end of the label do nothing. Doug
  stays in edit mode.
- When Doug starts an edit with `i` on a box that already has text, the cursor
  starts at the end of the label.
- Enter and Esc keep the whole label, not just the text before the cursor.
  Enter still adds a child box and Esc still leaves edit mode.

## Technical Design

### Decisions

1. **The cursor is an index in the mode.** `Mode::Insert` becomes
   `Mode::Insert { cursor: usize }`, counted in chars. The label in the doc
   stays the single source of truth for the text. Undo does not change: the doc
   is still mutated on every key and `Commit` is still the step boundary.
2. **One Insert action for typing.** `InsertAppend(char)` and `InsertBackspace`
   are replaced by `InsertKey(TextKey)`. `Commit` (Esc) and `CommitAndAddChild`
   (Enter) stay, because they leave the mode or change the tree. `InsertKey` is
   not undoable, like the two actions it replaces.
3. **`TextKey` is a small `Copy` enum**: `Left`, `Right`, `Backspace`,
   `Char(char)`. `Action` is `Copy`, so it cannot carry the raw key string.
   `insert_parse` decodes the raw key once: `"\x1b[D"` is `Left`, `"\x1b[C"` is
   `Right`, `"\x7f"` is `Backspace`, a printable key is `Char(c)`. Any other key
   is `None`, so unknown sequences such as `"\x1b[A"` never reach the reducer.
4. **Text editing is a pure function**, `edit(text, cursor, key) -> (String,
   usize)`, in its own module with no dependency on `State`. `insert::reduce`
   calls it, writes the new label to the doc and stores the new cursor. Adding
   a later key (Home, End, Delete) is a new `TextKey` variant and a case in
   `edit`.
5. **Escape sequences are decoded in the tty layer.** `poll_read` in
   `src/tty.rs` returns whole keys instead of single bytes. After an `ESC` byte
   it polls a short timeout (about 25 ms). If a `[` follows, it reads up to the
   final byte and returns the sequence as one string (`"\x1b[D"`). If nothing
   follows, it returns the lone `"\x1b"` (Esc). Without this, ← arrives as
   `ESC`, `[`, `D` and the `ESC` commits the edit.
6. **`PAD` goes away.** It was a reserved trailing cell that gave the box room
   for the cursor after the last letter, and its position also marked the
   cursor. The doc label now holds only what Doug typed.
   - `layout::diagram` takes `editing: Option<&[usize]>`, the path of the label
     in Insert mode, and widens that label by one cell. The box does not jitter
     as the cursor moves.
   - `with_cursor` takes `Option<usize>`, the edit index, and draws at
     `placement.x + index` in Insert mode. In Command mode it still draws on
     the last letter.
   - `Commit` and `CommitAndAddChild` no longer strip a char. They keep the
     whole label, wherever the cursor is.
   - `enter_insert` and `add_child_box` stop adding `PAD`.

### Behaviour

- `i` on a box with text starts Insert with `cursor` at the end of the label.
  `I` (rename) empties the label and starts at 0. A new box starts at 0.
- `Left` at 0 and `Right` at the end return the same text and cursor. Doug
  stays in Insert.
- `Char(c)` inserts at the cursor and moves it one right. `Backspace` removes
  the char before the cursor and moves it one left. At 0 it does nothing.

### Keymap

`INSERT_KEYMAP` gets `←` and `→` rows mapped to `InsertKey(Left)` and
`InsertKey(Right)`. The README table is regenerated with
`UPDATE_README=1 cargo test`. The existing agreement test between
`INSERT_KEYMAP` and `insert_parse` covers the new rows.

### Slices

1. `TextKey` and `edit`, unit tested (insert in the middle, backspace in the
   middle, edges, empty text).
2. `Mode::Insert { cursor }` and `InsertKey`, replacing `InsertAppend` and
   `InsertBackspace`. Typing and Backspace behave as before with the cursor at
   the end. `PAD` is removed from the state code and the tests that used it.
3. Layout and render: `editing` widens the label, `with_cursor` draws at the
   index.
4. tty: whole-key decoding with the Esc timeout, tested with a pipe that
   delivers the bytes in two steps.
5. `←` and `→` in `insert_parse` and `INSERT_KEYMAP`, README table, and the
   story test from the spec ("Chche" to "Cache", then Esc keeps the whole
   label).
