## Story

Doug opens dre with no file name and starts drawing. In the bottom-right corner he reads `[no name — press n to name it] • dre`. He didn't know he could name the diagram before, but now he presses `n`, types `plans` and presses Enter. The corner reads `plans • dre`, and the hint is gone. Happy, he no longer wonders how to name his diagram.

## Acceptance Criteria

- With no file name, the bottom-right corner reads `[no name — press n to name it] • dre`.
- The hint stays in every mode where the name prompt is closed: normal and typing a box label. While the name prompt is open, the corner shows the prompt and ` • dre`, with no hint.
- Once the diagram has a name, the corner reads `<name> • dre`, and the hint is gone.

## Technical Design
This builds on spec 137 (the `n` name prompt), so it is implemented after 137 is merged.

- The hint is only a change of text. `NO_NAME` in `state/mod.rs` becomes `[no name — press n to name it]`. `footer_text` is unchanged: with no `save_to` it gives `<hint> • dre`, and with a path it gives `<name> • dre`. Once `set_save_to(Some(..))` runs, the hint is gone.
- Every mode shows the hint because the footer doesn't depend on the mode. The exception is `NamePrompt`: 137 replaces the whole name slot with the prompt, hint included, so `prompt • dre` shows. Nothing extra is needed for that.
- `layout::footer` and `render::align_right` are unchanged. The corner is now 36 columns wide. On a terminal narrower than that, the text is clipped on the left, as it is today for a long name. No fallback text.
- SVG export is unaffected, since it draws no footer (spec 135).

### Tests

- `state`: `footer_of_a_default_state_has_no_name` and `set_save_to_none_after_a_path_goes_back_to_no_name` keep using `NO_NAME` and pass on the new text. Add one test pinning the literal `[no name — press n to name it] • dre` for a default state.
- `state`: naming a diagram through the prompt (137's `NameConfirm`) gives `plans • dre`, without the hint.
- `render`: the corner test at `render/mod.rs` that hardcodes `[no name] • dre` now expects the hint text, right-aligned in the bottom row.
- `render`: with a name prompt open, the corner has no hint text (covered together with 137's prompt tests).
