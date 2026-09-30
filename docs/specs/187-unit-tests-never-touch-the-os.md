# Unit tests never touch the OS

## Refactoring Goal

`make test` finishes in under 0.04s. Today it takes 0.51s. Tests run in parallel, so the run lasts as long as its slowest test, and two tests account for almost all of it:

| Test | Time | Cause |
|---|---|---|
| `render::tiles::composing_the_tiles_gives_exactly_the_whole_sprite` | 0.57s | Loops over every palette colour × 6 styles × 2 cell sizes × 6 sizes, rasterising and comparing whole sprites |
| `serve::pty_process::a_read_with_no_output_returns_an_empty_chunk` | 0.10s | Waits out the real `READ_POLL_TIMEOUT_MS` (100ms) |

Behind them are tests that spawn processes, write files, open pipes, send signals and sleep. They cost 5 to 55ms each, depending on load. There is no user story. This spec only changes tests.

## Technical Design

Decisions:

- **Rule: no unit test touches the OS.** That means no files, directories, permissions, pipes, PTYs, child processes, signals, sleeps or real timeouts. OS-boundary code (`filesystem`, `tty::read_key`, `serve::pty_process`, `serve::host_key`, `serve::diagram_dir`) is not tested at all. It is **not** moved behind `#[ignore]` or a separate `make` target, and no seams are extracted to rescue the logic inside it. Tests that are deleted are gone for good.
- **Only tests change.** No production code changes. `READ_POLL_TIMEOUT_MS`, `PtyChild`, `tty::read_key` and `DiagramDir` stay as they are.
- **The tiles test is reduced to one case.** Tiling is geometry. `tiles()`, `Band::of` and `reference_index` are the same code for every style, and colour only changes pixel values. One shape at one size that has a *repeated* middle band in both axes covers corners, edges and middles.
- **Brackets leave the tiles test.** Each bracket is a fixed L of about 2×2 cells, and the rest of its placement is always empty, so brackets shouldn't be tiled in the first place. Taking brackets out of the tiler is a production change and belongs in a separate spec.
- **Pure tests in the same modules are kept.** These include `tty::measure` (window and cell maths), `filesystem::missing` and `filesystem::invalid` (error messages), `cli` argument parsing and `output_path`, and the visitor tests that only use `MockSpawner`/`MockSession`.

### `render::tiles` tests: what changes

`composing_the_tiles_gives_exactly_the_whole_sprite` checks one case:

- shape: `TileStyle::Box(outlined(Some(0), true, ALL_SIDES))` at `ODD_CELL` (11×23, which catches rounding and off-by-one errors that the even `CELL` hides)
- size: `cells_with_middle(column_band) + 1` × `cells_with_middle(row_band) + 1`, so the middle band repeats in both axes
- assertion: `composed(...).pixels == whole_sprite(...).pixels`, as before

Remove any helpers this leaves unused (`clippy -D warnings` flags them). These are likely `styles`, `colours`, `borderless`, `CELLS`, `EXTRA_CELLS`, and the `Brackets` arm of `whole_sprite` if it becomes dead.

### Tests deleted

| Module | Tests |
|---|---|
| `serve::pty_process` | the whole `tests` module (6 tests) |
| `serve::host_key` | the whole `tests` module (2) |
| `serve::diagram_dir` | the whole `tests` module (4) |
| `serve` (`mod.rs`) | `the_config_offers_publickey_and_keyboard_interactive_with_the_host_key` |
| `filesystem` | `reading_gives_back_what_was_written`, `a_private_write_is_readable_only_by_its_owner`, `a_private_dir_is_accessible_only_to_its_owner_and_may_already_exist`, `reading_a_missing_path_is_not_found` |
| `tty` | all 8 `read_key_*` tests |
| `cli` | `export_writes_an_svg_document_beside_the_input`, `a_missing_file_reports_no_such_file`, `a_corrupt_file_reports_not_a_valid_diagram` |
| `serve::visitor` | `a_key_spawns_with_the_path_of_its_diagram_dir`, `the_same_key_connecting_twice_closes_the_first_and_spawns_a_second`, `different_keys_never_close_each_other`, `an_old_connection_ending_late_does_not_evict_its_replacement` |
| `state::input` | `readme_keymap_table_stays_in_sync` (reads and writes `README.md`) |

That's 33 tests deleted and 1 rewritten. Remove any test helpers and imports this leaves unused, for example `filesystem::tests::temp_path`, the `cli` temp-path helper, the `tty` signal and pipe helpers, and `read_until` and `window_size_of` in `pty_process`. If a module's `tests` block ends up empty, delete it.

### Verification

- `make test` passes, and `cargo test -p dre --lib` reports `finished in` ≤ 0.04s on a warm run. A trial run with exactly these tests skipped and the tiles test removed took 0.03s (726 tests).
- `make clippy` is clean.
- `grep -rn "temp_dir\|std::fs::\|pipe()\|kill(\|thread::sleep\|PtyChild::start" src --include=*.rs` finds no matches inside `#[cfg(test)]` modules.
