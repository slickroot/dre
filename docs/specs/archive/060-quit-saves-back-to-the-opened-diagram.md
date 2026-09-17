# 060 - Quit saves back to the opened diagram

## Story

Bob opens `plans.dre`, adds a box and presses `q`. The change is saved straight
back to `plans.dre` and `dre` quits without asking for a filename. The next time
he runs `dre plans.dre`, the new box is there.

## Acceptance Criteria

- After opening an existing diagram with `dre plans.dre`, pressing `q` saves the
  diagram back to `plans.dre` and quits, without showing the "Save as:" prompt.
- `q` saves back to the file even if nothing was changed.
- Running `dre plans.dre` again shows the diagram as it was when Bob quit.
- Ctrl-C still quits without saving.
- A diagram started without a file still shows the "Save as:" prompt on `q`.

## Technical Design

This builds on spec 059 as merged. `writer::load_state(arg)` reads the file,
`dre_format::read` parses the XML into a `FileDoc`, and
`file_document::to_state` builds the starting state with `save_to: None`.
Nothing changes in `dre_format` or `file_document`.

### No new state

`State::save_to` already holds the file to write on quit. It now has two
sources:

- the "Save as:" prompt's Confirm, as today (a new diagram), or
- startup, when a diagram was opened from a file.

A diagram opened from a file carries `save_to = Some(path)` for the whole
session.

### `writer`

- `load_state(arg)`: in the path branch, right after
  `file_document::to_state(doc)`, set `state.save_to = Some(path)` and return
  the state. The no-argument branch still returns `State::default()` with
  `save_to: None`, so a new diagram still prompts. `file_document` knows
  nothing about paths.
- `run()`: the save now depends on quitting, not just on `save_to` being set.
  Otherwise every keystroke would write the file:

  ```rust
  state = handle_key(state, &key);
  if !state.running {
      if let Some(path) = &state.save_to {
          fs::write(path, dre_format::write(&file_document::from_state(&state)))?;
      }
  }
  ```

- The path is saved exactly as given. `with_extension` only applies to names
  typed at the prompt.
- Saving writes canonical XML through `dre_format::write`. A hand-formatted
  file is reformatted but keeps the same diagram, because
  `read(&write(&doc)) == Some(doc)`.

- Ctrl-C needs no change. `INTERRUPT` returns before `handle_key` and before
  the save, so it still quits without saving, even when `save_to` is set.

### `command_mode::quit`

It branches on `save_to`:

- `Some(_)`: set `running = false` and leave `save_to` and `mode` as they are.
  No prompt is shown.
- `None`: set `mode = SavePrompt { filename: DEFAULT_FILENAME }`, as today.

There is no dirty check. `q` always saves back, even if nothing changed. `Quit`
stays out of `is_undoable` and keeps `min_depth` 0. `reduce` already
reselects before calling `quit`, so the selection is kept.

### Tests

- `state::new_state(boxes, mode, selected: Option<Path>, save_to: Option<String>)`
  gets a new last parameter. Existing callers pass `None`.
- `command_mode`: with `save_to = Some("plans.dre")`, `q` stops running, keeps
  `save_to`, stays in Command mode and keeps boxes and selection. With
  `save_to = None`, `q` still opens the prompt (the existing test).
- `save_prompt_mode`: existing tests only change to pass `None`.
- `writer::load_state`: a valid file loads with `save_to` set to the given
  path. No argument gives `save_to: None`.
