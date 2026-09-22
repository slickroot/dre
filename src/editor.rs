use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::render::{Renderer, TerminalRenderer};
use crate::state::{handle_key, Mode, State};
use crate::terminal::RawScreen;
use crate::{dre_format, file_document, filesystem, kitty, state, terminal, IDLE_TIMEOUT_MS};

const CURSOR: char = '\u{2588}';
const INTERRUPT: &str = "\x03";

pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode> {
    let state = load(file)?;
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    kitty::require(&mut stdout, stdin.as_raw_fd())?;
    let mut renderer = TerminalRenderer::new(terminal::probe()?);
    let _screen = RawScreen::open(stdin.as_raw_fd())?;

    let fd = stdin.as_raw_fd();
    let resize_fd = terminal::install_resize_pipe()?;
    let state = edit(
        state,
        || terminal::poll_read(fd, resize_fd, IDLE_TIMEOUT_MS),
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
    output: &mut impl Write,
    renderer: &mut TerminalRenderer,
) -> io::Result<State> {
    let mut state = state;
    while state.running {
        let status = status(&state);
        renderer.render(&state, output)?;
        renderer.status_line(status.as_deref(), output)?;
        output.flush()?;

        match next_key()? {
            Some(key) if key == INTERRUPT => {
                state.save_to = None;
                break;
            }
            Some(key) if key == terminal::RESIZE => renderer.on_resize(terminal::probe()?),
            Some(key) => state = handle_key(state, &key),
            None => state = state::hide_idle_cursor(state),
        }
    }
    Ok(state)
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

fn status(state: &State) -> Option<String> {
    match &state.mode {
        Mode::SavePrompt { filename } => Some(format!("Save as: {filename}{CURSOR}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{self, Path};
    use crate::state::{new_state, Mode};
    use crate::terminal::Terminal;
    use std::fs;

    #[test]
    fn a_save_prompt_shows_the_filename_being_typed() {
        let state = new_state(
            vec![],
            Mode::SavePrompt {
                filename: "a".to_string(),
            },
            None,
        );
        assert_eq!(status(&state), Some(format!("Save as: a{CURSOR}")));
    }

    #[test]
    fn command_mode_has_no_status_line() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(status(&state), None);
    }

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
        TerminalRenderer::new(Terminal {
            cols: 20,
            rows: 10,
            cell_width: 1,
            cell_height: 1,
        })
    }

    fn state_saving_to(path: &str) -> State {
        state::load(Default::default(), Some(path.to_string()))
    }

    fn run_script(state: State, script: Vec<Option<&str>>) -> (io::Result<State>, Vec<u8>) {
        let mut script = script.into_iter();
        let next_key = || match script.next() {
            Some(key) => Ok(key.map(str::to_string)),
            None => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "script ended")),
        };
        let mut output = Vec::new();
        let result = edit(state, next_key, &mut output, &mut renderer());
        (result, output)
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
    fn a_resize_key_re_probes_the_terminal_and_propagates_a_failed_probe() {
        let (result, _) = run_script(
            state_saving_to("a.dre"),
            vec![Some(terminal::RESIZE), Some("q")],
        );
        assert!(
            result.is_err(),
            "terminal::probe performs a real ioctl against stdout, which is not a TTY \
             in the test process, so the RESIZE arm's re-probe is expected to fail here"
        );
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
        assert!(
            frames[0].contains(CURSOR),
            "the cursor is visible before the idle second"
        );
        assert!(
            !frames[1].contains(CURSOR),
            "the cursor is hidden after the idle second"
        );
    }
}
