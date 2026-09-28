# One keymap table

Refactoring spec — key bindings are currently split between two tables and
three parser matches. Behaviour changes only where noted: idle cursor hiding
is removed.

## Story

There should be one `KEYMAP` table for every action initiated by a user key.
The table should be the source of truth for parsing and for the README
keymap. Adding a new action should require the smallest possible set of
edits, and the compiler should catch an action that has not been considered by
the binding-coverage test.

## Acceptance Criteria

- There is exactly one `KEYMAP` table in `src/state/input.rs`.
- Each user-key action has exactly one `KEYMAP` declaration. Aliases are not
  supported.
- `KEYMAP` drives parsing for exact keys, digits, printable insert text,
  printable name text, and `Ctrl-C`.
- Each `KEYMAP` entry contains its mode, matcher, display label, action
  constructor, and README description. Parser matches and README metadata are
  not duplicated elsewhere.
- The README contains one generated keymap table covering all modes.
- An exhaustive match beside `KEYMAP` checks that every user-key `Action` has
  a corresponding table entry. Adding an `Action` variant fails to compile
  until the coverage match is updated; the test fails until the `KEYMAP` entry
  is added.
- `CancelCount` remains internal parser behaviour for an unrecognised command
  key. It has no `KEYMAP` entry and is the sole exception to the binding
  coverage rule.
- `Interrupt` is a `KEYMAP` command-mode binding for `Ctrl-C`, rather than a
  special case in the outer reducer.
- `Action::Idle` is removed, along with idle cursor hiding and all tests for
  that behaviour. A missing input key no longer creates an action.
- Existing key behaviour remains unchanged apart from idle cursor hiding and
  the consolidated README presentation.

## Out of scope

- No changes to action reducers, undo semantics, count semantics, or key
  choices.
- No aliases or configurable user keymaps.
- No fallback matcher in `KEYMAP`; unknown command keys continue to produce
  `CancelCount` through explicit parser logic outside the table.

## Technical Design

### Typed keymap entries

Replace `KeyBinding` and the separate command/insert tables with one typed
table in `src/state/input.rs`:

```rust
enum KeyMatch {
    Exact(&'static str),
    Digit,
    Printable,
}

struct KeyBinding {
    mode: Mode,
    matcher: KeyMatch,
    display: &'static str,
    action: KeyAction,
    description: &'static str,
}
```

`KeyAction` is an enum of constructors needed by the table. Exact entries
construct fixed actions; `Digit` constructs `Action::Digit(u8)`;
`Printable` constructs either `Action::InsertKey(TextKey::Char(char))` or
`Action::NameAppend(char)`. The table has one row for each fixed or patterned
user-key action, including the name-prompt actions and `Interrupt`.

The parser searches `KEYMAP` for the current mode and input. Exact entries
are checked before pattern entries. A printable or digit match constructs its
parameterised action. If command mode has no table match, the parser returns
`Action::CancelCount`; this fallback is deliberately not a `KeyMatch` and is
not documented as a binding.

`KEYMAP` also supplies the display text and descriptions for one combined
README table. The old command and insert tables, markdown functions, and
separate README sync tests are removed.

### Exhaustive binding coverage

Beside `KEYMAP`, add a test helper with an exhaustive `match` over `Action`.
Each user-key variant asserts that the corresponding action constructor is
represented by exactly one keymap entry. Parameterised variants are checked
against their pattern entry. `CancelCount` has an empty arm because it is
internal parser behaviour. There is no `Idle` arm because `Idle` is deleted.

The match must have no wildcard arm. The test also verifies that no action is
represented by more than one binding and that exact key matches are unique per
mode.

### Input flow

The outer state reducer passes all actual key strings, including `Ctrl-C`, to
the input parser. It no longer converts missing input to `Action::Idle` or
handles `INTERRUPT` separately. A missing key is a no-op. The interrupt
constant may remain as the raw input value used by the keymap row and tests.

### Tests

- Replace command and insert keymap/parser agreement tests with one table
  agreement test covering every exact and patterned entry.
- Add exhaustive action-to-keymap coverage and uniqueness tests.
- Keep tests for digit accumulation, printable boundaries, name prompt keys,
  unknown command keys cancelling a pending count, and `Ctrl-C` quitting.
- Remove idle cursor tests and update tests that inject `None` input to expect
  no state change.
- Keep one README synchronization test for the combined generated table.
