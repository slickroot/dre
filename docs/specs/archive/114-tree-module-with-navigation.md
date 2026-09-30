# Tree module with navigation

## Problem

The tree is read and edited all over the code. `state.rs`, `diagram.rs`,
`command_mode.rs`, `insert_mode.rs` and others reach into `doc.boxes`,
`.children` and `.selected` directly (about 320 places), and every caller
walks paths by hand. `Path { ancestors, index }` forces callers to pop and
rebuild it for every step, and `at` and `children_at` are the same walk under
two unclear names. This is a technical spec, not a user story: it has no
user-facing behaviour. It is the first step of a series that moves every read
and edit of the tree into one module.

## Acceptance Criteria

- A new module `src/tree.rs` defines `Tree<T>` and nothing else uses it yet.
  No existing module changes.
- A path is a `Vec<usize>` (taken as `&[usize]`): the child indices from the
  invisible root down to a box. `[0]` is the first top-level box and `[0, 2]`
  is its third child.
- `Tree::leaf(value)`, `Tree::new(value, children)` and `Tree::root(children)`
  build trees. `Tree::root` needs `T: Default` and makes the invisible root
  that holds the top-level boxes.
- `parent(&path)` is a free function that needs no tree. `tree.child(&path)`,
  `tree.next(&path)` and `tree.previous(&path)` are methods. Each returns a
  path.
- When there is nowhere to go, they return the same path: `parent` of a
  top-level box, `child` of a box with no children, `next` of the last sibling
  and `previous` of the first sibling. They never return `[]`.
- `child` goes to the first child.
- `parent` panics on `[]`. `child`, `next` and `previous` panic when the path
  does not address an existing box, and on `[]`.
- The unit tests in `src/tree.rs` are the specification of this interface.

## Technical Design

### Decisions

- One module owns the tree. `tree.rs` is the only place that reads or edits
  nodes. Later specs move `state.rs`, `command_mode.rs`, `insert_mode.rs`,
  `layout.rs` and the file formats onto it and remove `Path`, `at`,
  `children_at`, `append` and `remove` from `diagram.rs`.
- `Tree<T>` is generic: a `value` and a list of `children`. `Document.boxes`
  is a list, so the tree has an invisible root that holds the top-level boxes.
  Its value is `T::default()`, so `Node` and test values such as `&str` work.
  The path `[]` addresses that root and is valid for `get`, but navigation never
  returns it.
- The fields of `Tree` are private. Callers use the interface, not
  `tree.children[2]`. Read accessors for layout and the file formats come with
  the specs that need them.
- A path is a plain `Vec<usize>`, not a struct. It is one flat address from the
  root, so "the parent" is the path without its last step and "the index" is
  its last step. There are no `ancestors` and `index` fields to pop and
  rebuild.
- The caller owns the selection. `Tree` knows nothing about it. The caller
  works out where the selection will be before it edits, from the navigation
  functions and from paths it names itself.
- Basic operations use one word: `parent`, `child`, `next`, `previous`. No
  `first_child`.
- Navigation returns a path, not an `Option`. The tree owns the stay-put rule,
  so the reducer never writes `unwrap_or(path)`.
- A path that does not address a box is a caller bug, so it panics, as `at()`
  does today on a bad index. Stay-put is for a real box with no neighbour.
- `parent` is only path arithmetic: it drops the last step and never reads a
  box. So it is a free function on slices, `parent(&[1, 2, 3])` is `&[1, 2]`
  and `parent(&[1])` is `&[1]`. It returns a sub-slice, allocates nothing and
  cannot check that the path exists. Its only panic is on `[]`, which has no
  parent.
- `get` and `get_mut` are private. They are the only functions that walk the
  tree. `child`, `next` and `previous` are short and built on them, for example
  `next` calls `get` on the parent to count siblings.
- The public interface is only the constructors and the four navigation
  functions in this spec. Each one is used by a select command in
  `command_mode.rs`: `SelectParent`, `SelectChild`, `SelectNext` and
  `SelectPrevious`. Edits, such as set, append, insert and remove, come in
  later specs, one at a time, each with its unit tests.

### Interface

```rust
pub struct Tree<T> { /* private: value, children */ }

pub fn parent(path: &[usize]) -> &[usize];

impl<T> Tree<T> {
    pub fn leaf(value: T) -> Self;
    pub fn new(value: T, children: Vec<Tree<T>>) -> Self;
    pub fn root(children: Vec<Tree<T>>) -> Self where T: Default;

    pub fn child(&self, path: &[usize]) -> Vec<usize>;
    pub fn next(&self, path: &[usize]) -> Vec<usize>;
    pub fn previous(&self, path: &[usize]) -> Vec<usize>;

    fn get(&self, path: &[usize]) -> &Tree<T>;
    fn get_mut(&mut self, path: &[usize]) -> &mut Tree<T>;
}
```

### Test list

Each is a unit test in `src/tree.rs`, on `Tree::root(vec![...])` of `&str`
leaves.

- `parent` of a nested path is the path without its last step.
- `parent` of a top-level path is the same path.
- `parent` panics on `[]`.
- `child` of a box with children is its first child.
- `child` of a box with no children is the same path.
- `next` of a box with a next sibling is that sibling, at any depth.
- `next` of the last sibling is the same path.
- `previous` of a box with a previous sibling is that sibling, at any depth.
- `previous` of the first sibling is the same path.
- `child`, `next` and `previous` each panic on `[]`, on an index past the last
  sibling and on a path that goes through a box with too few children.
- `get` returns the box at a path, including the invisible root at `[]`.
- `get_mut` changes the box at a path and nothing else.
