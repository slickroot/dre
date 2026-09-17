# 062 - Refuse to open a file that isn't a valid diagram

## Story

Bob edits `plans.dre` by hand and makes a mistake. When he runs
`dre plans.dre`, `dre` tells him "plans.dre: not a valid diagram" and exits,
instead of opening a diagram that's wrong.

## Acceptance Criteria

- Running `dre plans.dre` on a file that isn't a valid diagram shows
  `plans.dre: not a valid diagram` and exits.
- Anything unexpected in the file counts as not valid, including:
  - a colour or fill that isn't in the palette (e.g. `colour=99`)
  - a setting `dre` doesn't know (e.g. `shadow`)
  - an element other than `<box>` inside the diagram (e.g. `<arrow>`), or text
    between elements
  - a root element other than `<dre>`

## Technical Design
Builds on spec 060. `writer::load_state` already turns `dre_format::read`
returning `None` into an `InvalidData` error, which `main` prints before
exiting with failure. That happens before the terminal check, so the message
shows on a plain terminal too. This spec makes `read` stricter and changes the
wording of the message.

What `read` lets through today:

- serde ignores unknown attributes (`shadow="1"`), unknown elements (`<arrow>`)
  and stray text.
- quick-xml does not check the root name (`<plans>` loads) and ignores anything
  after the root element (`<dre/><dre/>`).
- `colour` and `fill` are `u8`, so `colour="99"` loads and then panics in
  `render` on `PALETTE[99]`.

Files that aren't UTF-8 (e.g. `dre photo.png`) are out of scope. They keep
today's I/O error.

### `dre_format`

All the "is this a valid file" rules live here. `read` still returns
`Option<FileDoc>`, and a `Some` is always a diagram that can be drawn.

- `#[serde(deny_unknown_fields)]` on `FileDoc` and `FileBox`. This rejects
  unknown attributes, unknown child elements and text content.
- `read` returns `Some` only when all three checks pass:
  1. **`only_a_dre_root(text) -> bool`**: walks `quick_xml::Reader` events.
     Before the root, only the XML declaration, comments and whitespace are
     allowed. The first start or empty tag must be named `dre`. After the root
     closes (tracked by depth), only comments and whitespace are allowed. Any
     reader error means invalid.
  2. serde deserialises the text into a `FileDoc`.
  3. **`in_palette(&FileDoc) -> bool`**: walks every box and its children
     recursively. Every `colour` and `fill` that is `Some(i)` must have
     `i < state::PALETTE_SIZE`.
- We chose a check after parsing over a `PaletteIndex` serde newtype. The root
  check needs a post-parse step anyway, and the newtype would change `FileBox`
  construction in every test and in `file_document`, while `Node` stays a
  plain `u8`.

### `writer::load_state`

- Change the message to `format!("{path}: not a valid diagram")`. Nothing else
  changes. It stays `ErrorKind::InvalidData`, and `main` prints it and exits
  with failure.

### Tests

- `dre_format`, where `read` gives `None` for:
  - `colour="5"` (the first value outside the palette) and `fill="99"`
  - a bad colour on a nested child box, not only on a top-level box
  - an unknown attribute: `<box label="A" shadow="true"/>`
  - an unknown element: `<dre><arrow/></dre>`, and inside a box:
    `<box label="A"><arrow/></box>`
  - text content: `<dre>hi</dre>`, `<box label="A">hi</box>`
  - a different root: `<plans><box label="A"/></plans>`
  - content after the root: `<dre/><dre/>`, `<dre/>junk`
- `dre_format`, where `read` still gives `Some` for:
  - `colour="4"` and `fill="0"` (the edges of the palette)
  - a leading `<?xml version="1.0"?>`, plus comments and whitespace before
    and after the root
  - the existing round-trip and reformatted-XML tests, unchanged
- `writer`: an invalid file's error message is exactly
  `<path>: not a valid diagram`, for example with `colour="99"`.
