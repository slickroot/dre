## Story

Doug opens dre with a brand-new diagram and draws a few boxes. The corner reads `[no name] • dre`. He presses `n` in normal mode. A prompt appears with a gray placeholder, "type a name", and the cursor at its start, so he knows to type the name there. He types `plans` and presses Enter. The corner now reads `plans • dre`, and `plans.dre` already exists on disk. Happy, he keeps drawing, knowing every change is saved.

## Acceptance Criteria

- Pressing `n` in normal mode opens a name prompt. It shows a gray placeholder reading "type a name", with the cursor at the start of it.
- Typing hides the placeholder and shows what Doug types.
- Enter with a name creates `<name>.dre` right away. The corner reads `<name> • dre`, and the diagram saves after every change from then on, like any diagram opened with a file name.
- dre always adds `.dre` to the typed name, so `plans.dre` becomes `plans.dre.dre`.
- If `<name>.dre` already exists, it is overwritten.
- Esc closes the prompt. The diagram stays as it was, and nothing is written.
- Enter with nothing typed does nothing. The prompt stays open.
- If the diagram already has a name, `n` works the same way and renames it. On Enter, the existing file is renamed to `<new name>.dre`, and no copy is left under the old name.

## Technical Design
