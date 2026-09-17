# 061 - Start a new diagram under a filename

## Story

Bob runs `dre ideas.dre`. There's no such file yet, so he gets an empty canvas.
He draws a few boxes and presses `q`. His diagram is saved to `ideas.dre` and
`dre` quits without asking for a filename.

## Acceptance Criteria

- Running `dre ideas.dre` when no file with that name exists shows an empty
  canvas.
- After drawing, pressing `q` saves the diagram to `ideas.dre` and quits,
  without showing the "Save as:" prompt.
- Running `dre ideas.dre` again shows the diagram as it was when Bob quit.
- Pressing `q` on a canvas with no boxes quits without creating `ideas.dre`.

## Technical Design
Builds on spec 060. There, `load_state` sets `save_to = Some(path)` for an
opened file, `writer::run` writes `save_to` once after the loop ends (Ctrl-C
returns before it), and `command_mode::quit` stops without the prompt when
`save_to` is `Some`.

### `State`

- New field `new_file: bool`, default `false`. It is `true` only when the path
  given on the command line did not exist at startup.

### `writer::load_state`

- A missing file is no longer an error:

  ```rust
  let text = match fs::read_to_string(&path) {
      Err(e) if e.kind() == io::ErrorKind::NotFound => {
          return Ok(State { save_to: Some(path), new_file: true, ..State::default() });
      }
      result => result?,
  };
  ```

- An existing file keeps `new_file = false`. Any other I/O error still exits
  with the error.

### `command_mode::quit`

Checked in order:

1. `new_file` and `doc.boxes` is empty → `save_to = None`, `running = false`.
   Nothing is written, so no empty file is created. This covers both "did
   nothing" and "undid everything".
2. `save_to` is `Some` → `running = false`. The write after the loop saves the
   diagram (spec 060).
3. Otherwise → open the "Save as:" prompt, as today.

### Tests

Depends on spec 064: tests set `new_file` and `save_to` on the state
returned by `new_state`, so adding `new_file` changes no existing test.

- `writer`: `a_missing_file_is_an_error` becomes "a missing file loads an
  empty canvas saved to that path": no boxes, nothing selected,
  `save_to == Some(path)`, `new_file`. Loading an existing file leaves
  `new_file` false.
- `command_mode`:
  - `q` on a new file with no boxes stops with `save_to == None`.
  - `q` on a new file after adding a box and undoing it stops with
    `save_to == None`.
  - `q` on a new file with boxes stops with `save_to == Some(path)` and no
    prompt.
  - `q` on an existing file with no boxes (`new_file == false`) still stops
    with `save_to == Some(path)`.
