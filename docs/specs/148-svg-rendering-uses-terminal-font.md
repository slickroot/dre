# SVG rendering uses same font as terminal

Hamid draws a diagram in dre and exports it to SVG. Later, he opens that SVG file on any computer — his own, a colleague's, or in a browser that doesn't have Iosevka installed. The text renders in Iosevka, exactly like it looks in the dre terminal.

## Acceptance Criteria

- The exported SVG references Iosevka via `@font-face`, pointing at a hosted copy of the font (not just a bare `font-family` name).
- Opening the SVG with network access loads Iosevka and renders text to match the terminal's Iosevka rendering.
- Opening the SVG without network access (or if the font URL is unreachable) falls back to the system's monospace font rather than failing to render.

## Technical Design

The SVG's `font-family` currently reads `"monospace"` with no `@font-face` declaration at all — text renders in whatever monospace font the viewer has, not Iosevka. Embedding the font's bytes directly was considered but rejected: the bundled `IosevkaRegular.ttf` is ~10.8 MB, and base64-encoding it into every exported SVG would balloon each file to ~14+ MB. Instead, we link to the font file already tracked in this repo.

- `SvgRenderer::document()` (`src/render/svg.rs`) gains a `<defs><style>@font-face{...}</style></defs>` block, emitted for both `Export` and `Editor`/canvas mode output:
  ```css
  @font-face {
    font-family: "Iosevka";
    src: url("https://raw.githubusercontent.com/slickroot/dre/main/assets/IosevkaRegular.ttf") format("truetype");
  }
  ```
- The URL tracks `main` rather than pinning to a commit SHA — simpler to construct, and the monospace fallback covers the (rare) risk of the file moving or the branch changing underneath it.
- `label_text()` and its test-helper duplicates change `font-family="monospace"` to `font-family="Iosevka, monospace"`, so Iosevka is preferred when it loads and the existing monospace behavior is the fallback.
- No new dependencies or subsetting/compression step needed — this only touches string generation in `svg.rs`.
