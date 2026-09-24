## Story

Bob opens `plans.dre`, adds a box called "Orders" and presses Enter. The file on disk is updated right away. He renames a box, deletes another and undoes the delete, and the file follows each time. Then his terminal crashes. He runs `dre plans.dre` again and everything is there. Later he presses `q` and DRE quits, saving one last time.

## Acceptance Criteria

- After I open an existing diagram (`dre plans.dre`), the file is saved after every action that alters the diagram: adding, renaming, deleting or moving a box, changing colour or fill, and undo or redo.
- While I'm typing a label, nothing is saved until I confirm it with Enter.
- Moving the selection doesn't save anything.
- If DRE or the terminal crashes, running `dre plans.dre` again shows the diagram as of my last change.
- `q` still saves and quits, without a prompt.
- Ctrl-C quits and leaves the file as my last change left it.
- A diagram started without a file still shows the "Save as:" prompt on `q`, and isn't autosaved. That's a later card.

## Technical Design
