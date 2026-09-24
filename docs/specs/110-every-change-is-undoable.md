# 110: Every change is undoable

## Story

Doug is sketching a diagram. He adds a sibling with `s`, edits a label with
`i`, and renames another with `I`. Then he sees that his last change was a
mistake. He presses `u` and the diagram goes back to how it was before that
change. He keeps pressing `u` to walk back further, and every change he made
can be undone. Doug never loses more than the one change he wanted to take
back.

## Acceptance Criteria

- Every change to the diagram can be undone with `u`. This covers adding a
  sibling (`s`), editing a label (`i`) and renaming a label (`I`), as well as
  the changes that can already be undone.
- Each `u` undoes one step, and repeated `u` walks back through the whole
  history.
- One edit session, from `i` to `Esc`, is one step, however many letters Doug
  typed or deleted.
- Pressing `Enter` in an edit session ends the current step. It adds a child
  box, which is its own step, and the text typed in the child is another step.
  Example: `i`, "Cache", `Enter`, "Redis", `Esc`. The first `u` clears "Redis"
  and leaves the empty child. The second `u` removes the child. The third `u`
  puts the old label back on "Cache".
- Adding a sibling with `s` works the same way. The sibling is one step and the
  text typed into it is another step. Example: `s`, "Queue", `Esc`. The first
  `u` clears "Queue" and leaves the empty sibling. The second `u` removes the
  sibling.
- Renaming with `I` works the same way. Clearing the old label is one step and
  the new text is another step. Example: `I` on "Cache", "Redis", `Esc`. The
  first `u` clears "Redis". The second `u` brings "Cache" back.
- An edit session that ends with the same text it started with is not a step,
  whether Doug pressed `Esc` straight away or typed and then backspaced back to
  the original. The next `u` undoes the change before it.

## Technical Design
