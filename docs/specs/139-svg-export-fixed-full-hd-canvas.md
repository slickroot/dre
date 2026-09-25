## Story

Doug has `plans.dre` open. He runs `dre --svg plans.dre` and opens `plans.svg`. Whatever the size of his diagram, the picture is a Full HD canvas, 1920 × 1080, with the diagram centred on it and the text at the same size every time. He exports a tiny diagram and a huge one, and the text looks identical in both. Happy, he drops them side by side in his slides.

## Acceptance Criteria

- Every exported SVG is exactly 1920 × 1080, whatever the size of the diagram.
- The whole canvas is filled with the dark background (`#0A0B0D`), including the empty space around the diagram.
- The diagram is centred on the canvas.
- The text and the boxes are the same size as today's export. Nothing inside the diagram is scaled.
- If the diagram is larger than the canvas, it is cut off evenly at the canvas edges, like in the terminal.
- The live editor and its bottom-right `<name> • dre` are unchanged.

## Technical Design
