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