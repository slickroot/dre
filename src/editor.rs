use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::diagram::Document;
use crate::layout::{layout, PlacementNode};
use crate::render::{GlyphCache, Renderer, TerminalRenderer, CACHE_LIMIT};
use crate::state::{handle_key, State};
use crate::terminal::{RawScreen, Terminal};
use crate::{dre_format, file_document, filesystem, kitty, state, terminal, IDLE_TIMEOUT_MS};

const INTERRUPT: &str = "\x03";

pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode> {
    let state = load(file)?;
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    kitty::require(&mut stdout, stdin.as_raw_fd())?;
    let terminal = terminal::probe()?;
    let glyph_source = Box::new(GlyphCache::new(terminal.cell_width, terminal.cell_height));
    let mut renderer = TerminalRenderer::new(terminal, glyph_source, CACHE_LIMIT);
    let _screen = RawScreen::open(stdin.as_raw_fd())?;

    let fd = stdin.as_raw_fd();
    let resize_fd = terminal::install_resize_pipe()?;
    let state = edit(
        state,
        || terminal::poll_read(fd, resize_fd, IDLE_TIMEOUT_MS),
        terminal::probe,
        &mut stdout,
        &mut renderer,
    )?;

    if let Some(path) = &state.save_to {
        filesystem::write(
            path,
            &dre_format::write(&file_document::from_document(&state.doc)),
        )?;
    }
    Ok(ExitCode::SUCCESS)
}

fn edit(
    state: State,
    mut next_key: impl FnMut() -> io::Result<Option<String>>,
    mut probe: impl FnMut() -> io::Result<Terminal>,
    output: &mut impl Write,
    renderer: &mut TerminalRenderer,
) -> io::Result<State> {
    let mut state = state;
    while state.running {
        renderer.render(&state, output)?;
        output.flush()?;

        match next_key()? {
            Some(key) if key == INTERRUPT => {
                state.save_to = None;
                break;
            }
            Some(key) if key == terminal::RESIZE => renderer.on_resize(probe()?),
            Some(key) => {
                state = handle_key(state, &key);
                if let Some((left, right)) = selected_box_edges(&state.doc) {
                    if let Some(delta) =
                        overflow_delta(left, right, state.scroll_x, renderer.columns())
                    {
                        state = handle_key(state, &format!("\x1bSCROLL{delta}"));
                    }
                }
            }
            None => state = state::hide_idle_cursor(state),
        }
    }
    Ok(state)
}

fn overflow_delta(_box_left: i64, box_right: i64, scroll_x: i64, cols: i64) -> Option<i64> {
    if box_right - scroll_x > cols {
        Some(box_right - scroll_x - (cols - 1))
    } else {
        None
    }
}

fn selected_box_edges(doc: &Document) -> Option<(i64, i64)> {
    let selected = doc.selected.as_ref()?;
    let placements = layout(&doc.boxes);
    let nodes = placements
        .iter()
        .filter(|placement| matches!(placement.node, PlacementNode::Node(_)));
    let labels = placements
        .iter()
        .filter(|placement| matches!(placement.node, PlacementNode::Label(_)));
    nodes
        .zip(labels)
        .find_map(|(node, label)| match &label.node {
            PlacementNode::Label(label) if &label.path == selected => {
                Some((node.x, node.x + node.width))
            }
            _ => None,
        })
}

fn load(file: Option<String>) -> io::Result<State> {
    let Some(path) = file else {
        return Ok(State::default());
    };
    let text = match filesystem::read(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(state::new_file(path)),
        result => result?,
    };
    let doc = dre_format::read(&text)
        .map(file_document::to_document)
        .ok_or_else(|| filesystem::invalid(&path))?;
    Ok(state::load(doc, Some(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{self, Path};
    use crate::render::FakeGlyphSource;
    use crate::state::{new_state, Mode};
    use crate::terminal::Terminal;
    use std::fs;

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("dre-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn temp_file(name: &str, contents: &str) -> String {
        let path = temp_path(name);
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn no_argument_starts_from_an_empty_diagram() {
        let state = load(None).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, None);
    }

    #[test]
    fn a_valid_file_loads_with_the_first_box_selected() {
        let path = temp_file(
            "valid.dre",
            &dre_format::write(&dre_format::FileDoc {
                boxes: vec![dre_format::FileBox {
                    label: "API".to_string(),
                    colour: None,
                    fill: None,
                    rounded: false,
                    children: vec![],
                }],
            }),
        );
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert_eq!(state.doc.boxes.len(), 1);
        assert_eq!(state.doc.boxes[0].label, "API");
        assert_eq!(
            state.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn a_valid_file_loads_saving_back_to_the_given_path() {
        let path = temp_file("save-to.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(state.unwrap().save_to, Some(path));
    }

    #[test]
    fn a_file_with_no_boxes_loads_an_empty_canvas_with_nothing_selected() {
        let path = temp_file("empty-dre.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn a_zero_byte_file_is_invalid_data() {
        let path = temp_file("zero.dre", "");
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(
            result.err().map(|e| e.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    }

    #[test]
    fn a_malformed_file_is_invalid_data() {
        let path = temp_file("malformed.dre", "<dre><box");
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(
            result.err().map(|e| e.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    }

    #[test]
    fn an_invalid_file_has_an_exact_error_message() {
        let path = temp_file(
            "bad-colour.dre",
            "<dre><box label=\"A\" colour=\"99\"/></dre>",
        );
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let err = result.err().unwrap();
        assert_eq!(err.to_string(), format!("{path}: not a valid diagram"));
    }

    #[test]
    fn a_valid_file_is_not_a_new_file() {
        let path = temp_file("not-new.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert!(!state.unwrap().new_file);
    }

    #[test]
    fn a_missing_file_loads_an_empty_canvas_saved_to_that_path() {
        let path = temp_path("missing.dre");
        let state = load(Some(path.clone())).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some(path));
        assert!(state.new_file);
    }

    fn renderer() -> TerminalRenderer {
        let terminal = Terminal {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        };
        let source = Box::new(FakeGlyphSource::new(
            terminal.cell_width,
            terminal.cell_height,
        ));
        TerminalRenderer::new(terminal, source, CACHE_LIMIT)
    }

    fn state_saving_to(path: &str) -> State {
        state::load(Default::default(), Some(path.to_string()))
    }

    fn run_script_with_probe(
        state: State,
        script: Vec<Option<&str>>,
        probe: impl FnMut() -> io::Result<Terminal>,
    ) -> (io::Result<State>, Vec<u8>) {
        let mut script = script.into_iter();
        let next_key = || match script.next() {
            Some(key) => Ok(key.map(str::to_string)),
            None => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "script ended")),
        };
        let mut output = Vec::new();
        let result = edit(state, next_key, probe, &mut output, &mut renderer());
        (result, output)
    }

    fn run_script(state: State, script: Vec<Option<&str>>) -> (io::Result<State>, Vec<u8>) {
        run_script_with_probe(state, script, || {
            panic!(
                "unexpected terminal probe: only a RESIZE key makes edit() re-probe the terminal, \
                 and this test's script has none. Use run_script_with_probe to supply a probe stub \
                 for tests that send RESIZE."
            )
        })
    }

    fn run(state: State, key: &str) -> (io::Result<State>, Vec<u8>) {
        run_script(state, vec![Some(key)])
    }

    #[test]
    fn an_interrupt_clears_where_to_save() {
        let (result, _) = run(state_saving_to("a.dre"), INTERRUPT);
        assert_eq!(result.unwrap().save_to, None);
    }

    #[test]
    fn a_resize_key_propagates_a_failed_probe() {
        let (result, _) = run_script_with_probe(
            state_saving_to("a.dre"),
            vec![Some(terminal::RESIZE), Some("q")],
            || Err(io::Error::other("probe failed")),
        );
        let error = result.err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), "probe failed");
    }

    #[test]
    fn a_resize_key_re_probes_the_terminal_once() {
        let mut probe_calls = 0;
        let (result, _) = run_script_with_probe(
            state_saving_to("a.dre"),
            vec![Some(terminal::RESIZE), Some("q")],
            || {
                probe_calls += 1;
                Ok(Terminal {
                    cols: 30,
                    rows: 12,
                    cell_width: 1,
                    cell_height: 1,
                })
            },
        );
        assert!(!result.unwrap().running);
        assert_eq!(probe_calls, 1);
    }

    #[test]
    fn a_quit_stops_the_loop_and_keeps_where_to_save() {
        let (result, _) = run(state_saving_to("a.dre"), "q");
        let state = result.unwrap();
        assert!(!state.running);
        assert_eq!(state.save_to, Some("a.dre".to_string()));
    }

    #[test]
    fn a_frame_is_painted_before_the_first_key_is_read() {
        let (_, output) = run(state_saving_to("a.dre"), INTERRUPT);
        assert!(!output.is_empty());
    }

    #[test]
    fn an_idle_second_hides_the_cursor_and_the_next_key_restores_it() {
        let selected = Path {
            ancestors: vec![],
            index: 0,
        };
        let mut state = new_state(
            vec![diagram::node("a"), diagram::node("b")],
            Mode::Command,
            Some(selected.clone()),
        );
        state.save_to = Some("a.dre".to_string());
        let (result, output) = run_script(state, vec![None, Some("q")]);
        let state = result.unwrap();
        assert_eq!(state.doc.selected, Some(selected));
        assert!(!state.running);
        let output = String::from_utf8(output).unwrap();
        let frames: Vec<&str> = output.split("\x1b[H").skip(1).collect();
        let sprite_count = |frame: &str| frame.matches("a=T,f=32").count();
        assert_eq!(
            sprite_count(frames[0]),
            sprite_count(frames[1]) + 1,
            "the cursor sprite is visible before the idle second, and hidden after it"
        );
    }

    #[test]
    fn a_box_that_fits_within_the_columns_does_not_overflow() {
        assert_eq!(overflow_delta(0, 20, 0, 20), None);
    }

    #[test]
    fn a_box_whose_right_edge_lands_exactly_on_the_last_column_does_not_overflow() {
        assert_eq!(overflow_delta(0, 19, 0, 20), None);
    }

    #[test]
    fn a_box_one_column_past_the_edge_overflows_by_the_exact_amount_needed() {
        assert_eq!(overflow_delta(0, 21, 0, 20), Some(2));
    }

    #[test]
    fn a_box_far_past_the_edge_overflows_by_the_exact_amount_needed_to_land_on_the_last_column() {
        assert_eq!(overflow_delta(11, 25, 0, 20), Some(6));
    }

    #[test]
    fn an_existing_scroll_offset_is_taken_into_account() {
        assert_eq!(overflow_delta(11, 25, 3, 20), Some(3));
    }

    #[test]
    fn selected_box_edges_is_none_when_nothing_is_selected() {
        let doc = diagram::Document {
            boxes: vec![diagram::node("a")],
            selected: None,
        };
        assert_eq!(selected_box_edges(&doc), None);
    }

    #[test]
    fn selected_box_edges_returns_the_edges_of_the_selected_box() {
        let doc = diagram::Document {
            boxes: vec![diagram::node("a"), diagram::node("bb")],
            selected: Some(Path {
                ancestors: vec![],
                index: 1,
            }),
        };
        let placements = layout(&doc.boxes);
        let bb_placement = placements
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Node(node) if node.label == "bb"))
            .unwrap();
        assert_eq!(
            selected_box_edges(&doc),
            Some((bb_placement.x, bb_placement.x + bb_placement.width))
        );
    }

    #[test]
    fn selected_box_edges_matches_a_nested_selection() {
        let doc = diagram::Document {
            boxes: vec![diagram::node_with_children(
                "parent",
                vec![diagram::node("child")],
            )],
            selected: Some(Path {
                ancestors: vec![0],
                index: 0,
            }),
        };
        let placements = layout(&doc.boxes);
        let child_placement = placements
            .iter()
            .find(|placement| matches!(&placement.node, PlacementNode::Node(node) if node.label == "child"))
            .unwrap();
        assert_eq!(
            selected_box_edges(&doc),
            Some((child_placement.x, child_placement.x + child_placement.width))
        );
    }

    #[test]
    fn a_normal_key_sequence_that_never_overflows_leaves_scroll_x_at_zero() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.save_to = Some("a.dre".to_string());
        let (result, _) = run_script(state, vec![Some("b"), Some("\x1b"), Some("q")]);
        assert_eq!(result.unwrap().scroll_x, 0);
    }

    #[test]
    fn adding_a_box_that_overflows_the_right_edge_scrolls_it_fully_into_view() {
        let mut state = new_state(vec![], Mode::Command, None);
        state.save_to = Some("a.dre".to_string());
        let (result, _) = run_script(
            state,
            vec![Some("b"), Some("\r"), Some("\r"), Some("\x1b"), Some("q")],
        );
        let state = result.unwrap();
        assert_ne!(state.scroll_x, 0);
        let (_, right) = selected_box_edges(&state.doc).unwrap();
        let visible_right_edge = right - state.scroll_x;
        assert_eq!(visible_right_edge, renderer().columns() - 1);
    }
}
