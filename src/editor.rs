use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::action::Action;
use crate::input::parse;
use crate::reduce::reduce;
use crate::render::{GlyphCache, Renderer, TerminalRenderer, CACHE_LIMIT};
use crate::state::State;
use crate::terminal::{RawScreen, Terminal};
use crate::{dre_format, file_document, filesystem, kitty, terminal, IDLE_TIMEOUT_MS};

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
            Some(key) if key == INTERRUPT => state = reduce(state, Action::Interrupt),
            Some(key) if key == terminal::RESIZE => renderer.on_resize(probe()?),
            Some(key) => {
                if let Some(action) = parse(&state, &key) {
                    state = reduce(state, action);
                }
            }
            None => state = reduce(state, Action::Idle),
        }
    }
    Ok(state)
}

fn load(file: Option<String>) -> io::Result<State> {
    let Some(path) = file else {
        return Ok(State::default());
    };
    let text = match filesystem::read(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(State::new_file(path)),
        result => result?,
    };
    let doc = dre_format::read(&text)
        .map(file_document::to_document)
        .ok_or_else(|| filesystem::invalid(&path))?;
    Ok(State::open(doc, Some(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{self, Path};
    use crate::render::FakeGlyphSource;
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
        State::open(Default::default(), Some(path.to_string()))
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
        let state = State::open(
            diagram::Document {
                boxes: vec![diagram::node("a"), diagram::node("b")],
                selected: None,
            },
            Some("a.dre".to_string()),
        );
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
}
