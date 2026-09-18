**dre** is a keyboard-driven diagram editor that runs in your terminal.
Build a tree of boxes with a few keystrokes, style them, and save it to a
`.dre` file — or export it to a crisp SVG.

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

## Example

`docs/example.dre` shows off rounded corners, border colours, translucent
fills, and arrows:

```xml
<dre>
  <box label="API gateway" colour="2" rounded="true">
    <box label="Auth"/>
    <box label="Orders" fill="1">
      <box label="Postgres">
        <box label="Replica">
          <box label="Backup"/>
        </box>
      </box>
    </box>
  </box>
  <box label="Billing"/>
</dre>
```

`dre --svg docs/example.dre` renders it as `docs/example.svg`:

![The example diagram rendered by dre](docs/example.svg)

## Export to SVG

`dre --svg diagram.dre` writes `diagram.svg` next to your `.dre` file and
exits — a crisp, vector copy with the same boxes, labels, colours, fills,
rounded corners, and arrows, but no cursor or selection.

## Command mode

<!-- keymap:start -->

| Key | Description |
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
| `f` | Toggle the box's fill |
| `F` | Toggle the fill of every sibling |
| `r` | Toggle rounded corners |
| `q` | Save and quit (or choose where to save) |

<!-- keymap:end -->

## Insert mode

<!-- insert-keymap:start -->

| Key | Description |
| --- | --- |
| `Enter` | Finish the box and add a child box |
| `Esc` | Switch to command mode |
| `Backspace` | Remove the last character |

<!-- insert-keymap:end -->

Any printable character appends to the label.

## Other keys

Save prompt: `Enter` saves and quits, `Esc` quits without saving, `Backspace`
removes a character from the filename.

`Ctrl-C` quits without saving.

## How dre is built

Each box below is a source module, and an arrow means the module on its left
drives the module on its right. The figure is itself a dre diagram —
`docs/architecture.dre`:

```xml
<dre>
  <box label="dre" colour="2" rounded="true">
    <box label="cli" colour="4" rounded="true">
      <box label="edit" colour="2">
        <box label="writer" colour="1">
          <box label="command_mode"/>
          <box label="insert_mode"/>
          <box label="save_prompt_mode"/>
        </box>
      </box>
      <box label="export">
        <box label="svg" colour="4"/>
      </box>
    </box>
    <box label="layout" colour="1" fill="1"/>
    <box label="dre_format" colour="3"/>
  </box>
</dre>
```

`cli` splits into an interactive `edit` path (the `writer` terminal loop with
its three modes) and a headless `export` path. Both feed `layout`, which turns
boxes into placements that each renderer draws in its own way — the terminal
sprites in the editor, `<rect>`s and `<text>`s for SVG.

![The architecture of dre rendered by dre](docs/architecture.svg)