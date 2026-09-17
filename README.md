**dre** is a keyboard-driven diagram editor that runs in your terminal.
Build a tree of boxes with a few keystrokes, style them, and save it to a
`.dre` file.

## Requirements

- an Apple Silicon Mac
- tested terminals: WezTerm works, Terminal.app doesn't

## Install

```
curl -fsSL https://raw.githubusercontent.com/slickroot/dre/main/install.sh | bash
```

## Getting started

Run `dre` to open an empty canvas. `dre plans.dre` opens `plans.dre`, or
starts a new diagram under that name if the file doesn't exist.

`dre` has three modes: command mode (the default), insert mode, and a save
prompt. In command mode, every key runs a command (see the table below);
`i` edits the selected box's label and `I` renames it, which enters insert
mode.

To quit, press `q`. With a filename, `q` saves to that file and quits. Without
a filename, `q` asks "Save as:" — pre-filled with `diagram.dre` — and `Enter`
saves, `Esc` quits without saving. `Ctrl-C` quits without saving.

## Command mode

<!-- keymap:start -->| Key | Description |
| --- | --- |
| `u` | Undo the last change |
| `b` | Add a child box |
| `s` | Add a sibling box |
| `h` | Select the parent box |
| `l` | Select the first child box |
| `j` | Select the next sibling |
| `k` | Select the previous sibling |
| `i` | Edit the selected box's label |
| `I` | Rename the selected box's label |
| `c` | Cycle the box's colour |
| `C` | Cycle the colour of every sibling |
| `f` | Cycle the box's fill |
| `F` | Cycle the fill of every sibling |
| `r` | Toggle rounded corners |
| `q` | Save and quit (or choose where to save) |
<!-- keymap:end -->

## Other keys

Insert mode: `Esc` commits or cancels the edit, `Backspace` removes a
character, any printable character appends to the label.

Save prompt: `Enter` saves and quits, `Esc` quits without saving, `Backspace`
removes a character from the filename.

`Ctrl-C` quits without saving.