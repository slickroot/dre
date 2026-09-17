# 059 - Open a saved diagram

## Story

Bob saved a diagram to `plans.dre` yesterday. Today he runs `dre plans.dre` and
the diagram is back exactly as he left it, with the first box selected so he
can pick up where he left off.

## Acceptance Criteria

- Running `dre plans.dre` on an existing, valid `.dre` file shows the diagram
  saved in that file.
- Every box comes back with its label, its place in the nesting, its border
  colour, its fill colour, and its rounded corners exactly as saved.
- The first top-level box is selected when the diagram opens.
- If the file has no boxes in it, the canvas is empty and nothing is selected.
- Quitting still shows the usual "Save as:" prompt (quitting straight back to
  the file is a separate story).

## Technical Design

### Format

A `.dre` file is XML. The root is `<dre>`, and each box is a `<box>` element.
Child boxes nest inside their parent at any depth. There are no `@n` groups.

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

- **Header**: the `label` attribute. It is short and single-line.
- **Settings**: the `colour`, `fill` and `rounded` attributes. An attribute is
  left out when it is at its default (no colour, no fill, square corners).
- **No boxes**: `<dre/>`. A zero-byte file is not valid XML, so it is not a
  valid diagram (spec 062). Spec 061 never writes one.
- **No version attribute.** A plain `<dre>` is the first format. If the format
  ever changes incompatibly, the new format adds `version`, and a file without
  it is read as this one.
- **Future body** (not in this story): free-text notes about a box go in a
  `<body>` element inside `<box>`, next to the child boxes. Attributes turn
  line breaks into spaces, so notes can't be an attribute. Adding `<body>`
  doesn't break existing files.

### Layers

A file model sits between the text and state, so refactoring state can't
silently change the format:

```
XML text ⇄ dre_format (FileDoc, serde) ⇄ file_document ⇄ state (free to change)
```

`dre_format` does not import `state`. Only `file_document` knows both sides.

### Dependencies

- `serde` with the `derive` feature.
- `quick-xml` with the `serialize` feature.

These replace the hand-written parser. Only `serde`, `quick-xml` and `memchr`
end up in the binary. `serde_derive`, `syn`, `quote`, `proc-macro2` and
`unicode-ident` run only at build time.

### `dre_format` (`src/dre_format.rs`)

The `v0` submodule goes away. The structs are the format:

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename = "dre")]
pub(crate) struct FileDoc {
    #[serde(rename = "box", default)]
    pub(crate) boxes: Vec<FileBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileBox {
    #[serde(rename = "@label")]
    pub(crate) label: String,
    #[serde(rename = "@colour", default, skip_serializing_if = "Option::is_none")]
    pub(crate) colour: Option<u8>,
    #[serde(rename = "@fill", default, skip_serializing_if = "Option::is_none")]
    pub(crate) fill: Option<u8>,
    #[serde(rename = "@rounded", default, skip_serializing_if = "is_false")]
    pub(crate) rounded: bool,
    #[serde(rename = "box", default)]
    pub(crate) children: Vec<FileBox>,
}
```

- **`write(&FileDoc) -> String`**: serialises with quick-xml's `Serializer`,
  indented by two spaces, and adds a trailing newline. quick-xml escapes
  labels.
- **`read(&str) -> Option<FileDoc>`**: `quick_xml::de::from_str(text).ok()`.
  It accepts any well-formed XML with the right structure, whatever the
  attribute order, quoting, whitespace, or self-closing versus explicit
  closing tags.
- **Invariant**: `read(&write(&doc)) == Some(doc)`. Saving keeps the diagram
  but may reformat a hand-edited file.
- Removed from the worktree's version: `quote`, `line`, `Groups`, `Draft`,
  `ParsedBlock`, reference resolution, and the exact-text check
  `write(&doc) == text`.
- Left to spec 062: rules serde doesn't enforce, such as a colour outside the
  palette, unknown attributes or elements, or a wrong root element name. They
  go in a validation step after `read`, not in a parser.

### `file_document` (`src/file_document.rs`)

Unchanged apart from importing from `crate::dre_format`:

- `to_state(FileDoc) -> State`: converts each `FileBox` to a `Node`
  (`None ⇄ PLAIN`, `Some(n) ⇄ n as i64`) and builds
  `State { doc: Document { boxes, selected }, ..State::default() }`, where
  `selected` is `[0]` if there are boxes and `[]` otherwise. Mode stays
  Command and `save_to` stays `None`, so quitting still shows "Save as:".
- `from_state(&State) -> FileDoc`: converts `state.doc.boxes` back. Selection
  is not saved.

### `writer`

- `load_state(std::env::args().nth(1))`:
  - No argument: `State::default()`.
  - A path: `fs::read_to_string(path)?` → `dre_format::read` →
    `file_document::to_state`. `None` becomes
    `io::Error::new(InvalidData, …)`. `main` prints it and exits with failure.
    Specs 061 and 062 replace these paths with their own behaviour.
- `run(stream, stdin_fd, state: State)` receives its starting state.
- Saving becomes
  `fs::write(path, dre_format::write(&file_document::from_state(&state)))`.

### Tests

- `dre_format`:
  - `write` pins the exact text for the spec example, a single plain box, and
    an empty diagram (`<dre/>` plus a newline).
  - Settings at their defaults are left out. Labels containing `"`, `<` and
    `&` are escaped.
  - `read(&write(&doc)) == Some(doc)` holds for those examples, including deep
    nesting.
  - `read` accepts reformatted XML: attributes in another order, single quotes,
    extra whitespace, and `<box …></box>` instead of `<box …/>`.
  - `read` returns `None` for a zero-byte file, malformed XML, a box without a
    label, and a non-numeric colour.
- `file_document`: an empty `FileDoc` gives no boxes and nothing selected. A
  non-empty one selects `[0]`. Colours and fills map correctly in both
  directions.
- `writer::load_state`: a valid XML file loads with the first box selected.
  `<dre/>` gives no boxes and nothing selected. A zero-byte file and a
  malformed file are `InvalidData`. A missing path is an error.
