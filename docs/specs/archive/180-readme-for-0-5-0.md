# README for the 0.5.0 release

## Refactoring Goal

The README is reorganised for the 0.5.0 release. A visitor sees dre working before reading anything, the diagrams match the code as it is today, and the terminal has real screenshots. There is no user story; this is a documentation refactoring.

## Technical Design

### Layout (top to bottom)

1. Logo, tagline, nav links (unchanged)
2. **Demo GIF**, directly under the tagline
3. **Try it without installing**: `ssh dre.elaich.com`, plus one sentence: visitors get their own canvas, remembered between sessions
4. Requirements and Install
5. Getting started, with three terminal screenshots (first box, selection brackets, colour and fill styling)
6. Command mode: the keymap table, with the "Other keys" section folded in (it is dropped as a separate heading)
7. **Files and export**: the merged "Example" and "Export to SVG" sections
8. How dre is built: one refreshed architecture diagram
9. Contributing link

### Demo GIF

- `~/Downloads/out.gif` (800x477, 10 fps, 2.4 MB) is committed unchanged as `docs/assets/demo.gif`. No re-encoding; text legibility matters more than the ~1 MB saved.
- Embedded as `<img src="docs/assets/demo.gif" width="720">` inside a centred `<p align="center">`, matching the logo header.
- Committed once; re-recording it adds another copy to history.

### Screenshots

- Taken by hand from WezTerm by Marouane, saved as `docs/assets/first-box.png`, `docs/assets/selection.png` and `docs/assets/styling.png`.
- CONTRIBUTING.md gains a short "refreshing the README assets" section listing the keystrokes and window size used for each, so a later release can reproduce them. No script, no `make screenshots` target.

### Try it without installing

- Placed right after the GIF. The host comes from `PUBLIC_HOST` in `src/serve/visitor.rs` (`dre.elaich.com`); the README repeats it as text.
- `dre serve` and the `dre-web.zip` wasm build are documented in CONTRIBUTING.md (for people hosting dre), not in the README.

### Files and export (merged section)

- One paragraph on the `.dre` format, with the XML of `docs/example.dre` shown once.
- The `dre --svg docs/example.dre` command and the rendered `docs/example.svg`.
- The duplicated prose of the old "Export to SVG" section is removed. `docs/example.dre` and `docs/example.svg` are unchanged.

### Architecture diagram

- One diagram, drawn in dre itself: `docs/architecture.dre`, exported to `docs/architecture.svg` with `dre --svg`. About 8 boxes following the data:
  - Entry points: the terminal (`editor`), `serve` (SSH) and `web` (wasm), all feeding the same core.
  - Core: `state` -> `layout` -> `render`, ending in the `tui` and `svg` renderers.
  - `store` (the `.dre` files) hangs off `editor`.
- Colour marks the layers: entry points, core, renderers.
- Left out on purpose: `kitty`, `tty`, `composer`. The module tree belongs in CONTRIBUTING.md.
- The prose under the diagram is rewritten to match, with the file references (`src/render/terminal.rs`, `src/render/svg.rs`) checked against the tree.

### Constraints

- The `<!-- keymap:start -->` / `<!-- keymap:end -->` markers stay, with the table between them exactly as generated. `readme_keymap_table_stays_in_sync` (`src/state/input.rs`) reads them. Regenerate with `UPDATE_README=1 make test`, never by hand.
- Image paths are relative (`docs/assets/...`), as today, so they render on GitHub.

### Sequencing: the screenshots come last

- The screenshots are supplied by Marouane, so the slice that embeds them is the final one. Every other slice (GIF, layout, merged section, architecture diagram, CONTRIBUTING.md) is independent of the PNGs.
- The implementer builds everything else first and adds the three `<img>` lines last.
- The pull request is not opened until `first-box.png`, `selection.png` and `styling.png` exist in `docs/assets/`, so `main` never has broken image links. If they are missing when the other slices are done, the implementer asks Marouane for them.

### Out of scope

- The 0.5.0 version bump in `Cargo.toml` and the release itself.
- A script that regenerates the screenshots (decided against for 0.5.0).
- Re-encoding the GIF.
- Any code change.
