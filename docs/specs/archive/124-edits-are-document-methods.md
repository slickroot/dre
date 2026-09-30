# Edits are Document methods

Step of spec 117 (one-direction architecture). This is a technical spec, not a
user story: it has no user-facing behaviour.

## Problem

The Command-mode edits (`cycle_colour`, `toggle_fill`, `delete_box`, `paste_box` and others) and the helpers in `state.rs` (`colour_row`, `add_child_box`, `blank_box`, `next_colour`) walk and change the drawing themselves.

## Acceptance Criteria

- Every change to a drawing is a named `Document` method taking a `&[usize]` path.
- The reducers compute the new selection from `Tree` navigation and call those methods.
- The clipboard is `Option<Tree<Node>>`. `Document::remove` returns the detached subtree and `Document::insert` puts a clone back.
- No module outside `diagram.rs` touches `.children` or the boxes.
- `Node`'s fields and `Document.root` are private. `Node` has read methods only: `label()`, `colour()`, `filled()`, `rounded()` (left `pub(crate)` by spec 123).

## Technical Design

### The rule

`Document` stores values. The reducers decide them. `Document` has no editing
rules: no cycling, no toggling, no "no colour means no fill". It is told the
new value and writes it.

### Where the drawing is changed today

All inside `state/`, through `state.doc.root` and `Tree::value_mut`, `push`
and `remove`:

- `command.rs`: `enter_insert`, `new_sibling`, `rename_label`, `cycle_colour`,
  `cycle_siblings_colour`, `toggle_fill`, `toggle_siblings_fill`,
  `toggle_rounded`, `delete_box`, `paste_box`.
- `mod.rs`: `colour_row`, `blank_box`, `add_child_box`, `next_colour`.
- `insert.rs`: holds one `value_mut` for the whole reducer and rewrites
  `node.label`.

Outside `state/`, only tests build `Node` and `Document` literals.

### `Document`'s interface

```rust
pub(crate) enum Scope { Box, Siblings }

impl Document {
    pub(crate) fn tree(&self) -> &Tree<Node>;

    pub(crate) fn set_label(&mut self, path: &[usize], label: String);
    pub(crate) fn set_colour(&mut self, path: &[usize], colour: Option<u8>, scope: Scope);
    pub(crate) fn set_fill(&mut self, path: &[usize], filled: bool, scope: Scope);
    pub(crate) fn set_rounded(&mut self, path: &[usize], rounded: bool, scope: Scope);

    pub(crate) fn insert(&mut self, parent: &[usize], subtree: &Tree<Node>) -> Vec<usize>;
    pub(crate) fn remove(&mut self, path: &[usize]) -> Tree<Node>;
}
```

- `Scope::Siblings` applies the value to every child of `parent_of(path)`,
  the box itself included. No key uses `set_rounded` with `Scope::Siblings`
  yet. The method supports it anyway, to match the other two.
- `insert` clones `subtree` in as the last child of `parent` and returns its
  path, as `Tree::push` does. `parent` is `[]` for the top level. It is the
  only way to add a box: a new box, a new sibling, commit-and-add-child and
  paste all use it.
- `remove` returns the detached subtree, which becomes the clipboard.

### What the reducers do

| action | reducer |
|---|---|
| new box / commit-and-add-child | `insert(&parent, &Tree::leaf(Node::default()))`, then `set_label(&new, PAD)`, selects `new`, Insert mode |
| new sibling | the same with `parent_of(path)` |
| edit / rename label | `set_label(path, label + PAD)` or `set_label(path, PAD)` |
| Insert-mode typing, backspace, commit | read `tree().value(path).label()`, compute the new label, `set_label` |
| cycle colour | `set_colour(path, next_on_palette(current), Scope::Box)` |
| cycle siblings' colour | if the row is all one colour, `next_on_palette` of that colour, else `Some(0)`, then `set_colour(path, c, Scope::Siblings)` |
| toggle fill | if the box has a colour, `set_fill(path, !filled, Scope::Box)`, else nothing |
| toggle siblings' fill | if some sibling has a colour, `set_fill(path, !all_filled, Scope::Siblings)`, else nothing |
| toggle rounded | `set_rounded(path, !rounded, Scope::Box)` |
| delete | `clipboard = Some(remove(path))`, then the new selection from `Tree` navigation, as today |
| paste | `insert(&parent, &branch)` `count` times, selects the last path returned |

- `PAD`, the trailing space that stands in for the Insert-mode cursor, stays in
  `state/`. `Document` never knows a label is being edited.
- A blank box is `Tree::leaf(Node::default())`. `Node::default()` is the only
  way to get a `Node` outside `diagram.rs`, and it can only be blank: nobody
  outside can set its fields. `blank_box()` goes away.
- `add_child_box` keeps only its selection and mode part.

### `palette.rs`

`next_colour` moves to `palette.rs` as
`next_on_palette(Option<u8>) -> Option<u8>`: `None` gives `Some(0)`, and the
last colour gives `None`. It is about the order of the palette, not about boxes.
`diagram.rs` does not import `palette`.

### Privacy

- `Document.root` and `Node`'s fields become private.
- `children` and `parent_of` stay in `diagram.rs` as path helpers.

### Tests

- `#[cfg(test)]` builders in `diagram.rs`: `Document::with_boxes(Vec<Tree<Node>>)`
  and `Node` builders `with_colour`, `with_fill`, `with_rounded` on top of
  the existing `labelled`, `node` and `node_with_children`. They exist in test
  builds only, so production code still cannot build or change a `Node`.
- `new_state`, `editor/store`'s fixture and `layout`'s `laid_out_box` use them
  in place of struct literals.
- Assertions on `state.doc.root` read `state.doc.tree()`.

### Changes from spec 117's method list

- There is no `add_child`, `add_sibling` or `paste`. They are all `insert`.
- There are no `cycle_*` or `toggle_*` methods. They are `set_*` with a `Scope`,
  and the rule that picks the value lives in the reducer.

### Considered and rejected

- `cycle_colour` on `Document`: `diagram` would import `palette`, and the domain
  would know the app's colours.
- `with_siblings: bool`: a bare `true` at the call site does not say what it
  means.
- Label-editing methods on `Document` (`append_char`, `backspace`): the `PAD`
  cursor would move into the domain.
- Building test scenarios only through `insert` and `set_*`: correct, but long
  fixtures for no gain, since the test builders do not exist in release builds.
- Storing hex colours in `Node` now: it changes the file format, every renderer
  and the cycling rule. It is spec 129.
