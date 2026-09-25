## Story

Doug has `plans.dre` open. He runs `dre --svg plans.dre` and opens `plans.svg`. He sees his diagram and nothing else: no name, no `• dre`, no empty strip at the bottom. Happy, he can drop the picture straight into a document.

## Acceptance Criteria

- The exported SVG contains only the diagram. It has no name and no `• dre`.
- The picture is exactly the size of the diagram, with no extra row at the bottom.
- The live editor's bottom-right corner is unchanged. It still reads `<name> • dre`, or `[no name] • dre`.

## Technical Design
