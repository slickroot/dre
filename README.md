<!-- LOGO -->
<h1 align="center">
  <img src="docs/assets/dre-logo.png" alt="DRE logo" width="128">
  <br>
  dre
</h1>

<p align="center">
  Keyboard-driven diagrams, directly in your terminal.
  <br>
  Build with a few keystrokes. Save as <code>.dre</code>. Export crisp SVG.
  <br>
  <a href="#install">Install</a>
  ·
  <a href="#getting-started">Getting started</a>
  ·
  <a href="#command-mode">Keymap</a>
  ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

<p align="center">
  <img src="docs/assets/demo.gif" alt="Demo of dre: building and styling a diagram with a few keystrokes" width="720">
</p>

## Try it without installing

```
ssh dre.elaich.com
```

Visitors get their own canvas, remembered between sessions.

## About

**dre** is a keyboard-driven diagram editor that runs in your terminal. Build
a tree of boxes, style it as you work, and keep it in a compact `.dre` file—or
export it to a crisp SVG when it is ready to share.

## Requirements

- macOS (Apple Silicon or Intel) or Linux (x86_64 or arm64)
- a terminal with Kitty graphics support — WezTerm works, Terminal.app doesn't

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
a filename, `q` asks for one, and `Enter` saves and quits, `Esc` quits
without saving. `Ctrl-C` quits without saving.

## Example

`docs/example.dre` shows off rounded corners, border colours, translucent
fills, and arrows:

```xml
<dre>
  <box label="API gateway" colour="2" fill="1" rounded="true">
    <box label="Auth" colour="1" fill="1"/>
    <box label="Orders" colour="1" fill="1">
      <box label="Postgres" colour="4" fill="1">
        <box label="Replica" colour="4" fill="1"/>
        <box label="Archive" colour="4" fill="1"/>
      </box>
    </box>
    <box label="Payments" colour="1" fill="1"/>
  </box>
</dre>
```

Each colour names a layer — pink the edge, orange the services, blue the
data — and the rounded corners mark the single entry point.

`dre --svg docs/example.dre` renders it as `docs/example.svg`:

![The example diagram rendered by dre](docs/example.svg)

## Export to SVG

`dre --svg diagram.dre` writes `diagram.svg` next to your `.dre` file and
exits — a crisp, vector copy with the same boxes, labels, colours, fills,
rounded corners, and arrows, but no cursor or selection.

## Command mode

<!-- keymap:start -->

| Mode | Key | Description |
| --- | --- | --- |
| Command | `u` | Undo the last change |
| Command | `b` | Add a child box |
| Command | `s` | Add a sibling box |
| Command | `d` | Delete the selected box and its descendants |
| Command | `p` | Paste the cut box and its descendants as the last child of the selected box |
| Command | `h` | Select the parent box |
| Command | `l` | Select the first child box |
| Command | `j` | Select the next sibling |
| Command | `k` | Select the previous sibling |
| Command | `i` | Edit the selected box's label |
| Command | `I` | Rename the selected box's label |
| Command | `c` | Cycle the box's colour |
| Command | `C` | Cycle the colour of every sibling |
| Command | `F` | Toggle the fill of every sibling |
| Command | `f` | Toggle the box's fill |
| Command | `r` | Toggle rounded corners |
| Command | `R` | Toggle rounded corners of every sibling |
| Command | `q` | Save and quit (or choose where to save) |
| Command | `n` | Name the diagram |
| Command | `0–9` | Build a count prefix |
| Command | `Ctrl-C` | Quit without saving |
| Insert | `Enter` | Finish the box and add a child box |
| Insert | `Esc` | Switch to command mode |
| Insert | `Backspace` | Remove the last character |
| Insert | `←` | Move the cursor left |
| Insert | `→` | Move the cursor right |
| Insert | `Printable` | Append a printable character |
| Name prompt | `Enter` | Save the name |
| Name prompt | `Esc` | Cancel naming |
| Name prompt | `Backspace` | Remove a character from the name |
| Name prompt | `Printable` | Append a character to the name |

<!-- keymap:end -->

Any printable character appends to the label.

## Other keys

Save prompt: `Enter` saves and quits, `Esc` quits without saving, `Backspace`
removes a character from the name.

`Ctrl-C` quits without saving.

## How dre is built

An arrow means the box on its left feeds the box on its right. The figure is
itself a dre diagram — `docs/architecture.dre`:

```xml
<dre>
  <box label="state" rounded="true">
    <box label="layout">
      <box label="tui" colour="3"/>
      <box label="svg" colour="3"/>
    </box>
  </box>
</dre>
```

`state` holds the tree of boxes and is where everything starts. `layout` turns
that tree into placements, and two renderers draw the same placements in their
own way: `tui` (`src/render.rs`) paints sprites in the editor, and `svg`
(`src/svg.rs`) writes `<rect>`s and `<text>`s. Purple marks the renderers.

![The architecture of dre rendered by dre](docs/architecture.svg)
