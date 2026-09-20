use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use nix::unistd::read;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::process::ExitCode;

use crate::dre_format;
use crate::file_document;
use crate::filesystem;
use crate::kitty;
use crate::render::{Renderer, TerminalRenderer};
use crate::state::{self, handle_key, Mode, State};
use crate::terminal;

const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const CURSOR: char = '\u{2588}';
const INTERRUPT: &str = "\x03";
const NOT_SUPPORTED_MESSAGE: &str =
    "Dre requires a terminal with Kitty graphics protocol support.";
const CLEAR_LINE: &str = "\r\x1b[K";

struct RawModeGuard<'a, W: Write> {
    fd: RawFd,
    saved: Termios,
    stream: &'a mut W,
}

impl<'a, W: Write> RawModeGuard<'a, W> {
    fn new(fd: RawFd, stream: &'a mut W) -> io::Result<Self> {
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
        stream.write_all(ENTER_ALTERNATE_SCREEN.as_bytes())?;
        stream.write_all(HIDE_CURSOR.as_bytes())?;
        stream.flush()?;
        let mut raw = saved.clone();
        cfmakeraw(&mut raw);
        tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;
        Ok(RawModeGuard { fd, saved, stream })
    }
}

impl<'a, W: Write> Drop for RawModeGuard<'a, W> {
    fn drop(&mut self) {
        let borrowed = unsafe { BorrowedFd::borrow_raw(self.fd) };
        let _ = tcsetattr(borrowed, SetArg::TCSADRAIN, &self.saved);
        let _ = self.stream.write_all(SHOW_CURSOR.as_bytes());
        let _ = self.stream.write_all(LEAVE_ALTERNATE_SCREEN.as_bytes());
        let _ = self.stream.flush();
    }
}

fn frame<W: Write>(state: &State, renderer: &mut TerminalRenderer, stream: &mut W) -> io::Result<()> {
    renderer.render(&state.doc, stream)?;
    let prompt = match &state.mode {
        Mode::SavePrompt { filename } => Some(format!("Save as: {filename}{CURSOR}")),
        _ => None,
    };
    renderer.status_line(prompt.as_deref(), stream)?;
    stream.flush()
}

fn run<W: Write>(stream: &mut W, stdin_fd: RawFd, mut state: State) -> io::Result<()> {
    let terminal = terminal::probe()?;
    let mut renderer = TerminalRenderer::new(terminal);
    let guard = RawModeGuard::new(stdin_fd, stream)?;
    let stdin = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
    while state.running {
        frame(&state, &mut renderer, &mut *guard.stream)?;
        let mut key_buffer = [0u8; 1];
        read(stdin, &mut key_buffer).map_err(io::Error::from)?;
        let key = std::str::from_utf8(&key_buffer).unwrap_or("").to_string();
        if key == INTERRUPT {
            return Ok(());
        }
        state = handle_key(state, &key);
        if !state.running {
            if let Some(path) = &state.save_to {
                filesystem::write(path, &dre_format::write(&file_document::from_document(&state.doc)))?;
            }
        }
    }
    Ok(())
}

fn load_state(arg: Option<String>) -> io::Result<State> {
    let Some(path) = arg else {
        return Ok(State::default());
    };
    let text = match filesystem::read(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(state::new_file(path)),
        result => result?,
    };
    let doc = dre_format::read(&text).ok_or_else(|| filesystem::invalid(&path))?;
    Ok(state::load(file_document::to_document(doc), Some(path)))
}

pub fn write(file: Option<String>) -> io::Result<ExitCode> {
    let state = load_state(file)?;
    let mut stdout = io::stdout();
    let stdin_fd = io::stdin().as_raw_fd();
    let supported = kitty::supported(&mut stdout, stdin_fd)?;
    if !supported {
        let _ = stdout.write_all(CLEAR_LINE.as_bytes());
        println!("{NOT_SUPPORTED_MESSAGE}");
        return Ok(ExitCode::FAILURE);
    }
    run(&mut stdout, stdin_fd, state)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::Path;
    use crate::state::new_state;
    use crate::terminal::Terminal;
    use std::fs;
    use std::io::Cursor;

    fn terminal(cols: i64, rows: i64) -> Terminal {
        Terminal { cols, rows, cell_width: 1, cell_height: 1 }
    }

    fn written(buffer: &Cursor<Vec<u8>>) -> String {
        String::from_utf8(buffer.get_ref().clone()).expect("test output is always ASCII")
    }

    #[test]
    fn the_renderer_is_given_the_terminal_size() {
        let state = State::default();
        let terminal = terminal(3, 2);
        let mut renderer = TerminalRenderer::new(terminal);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream).unwrap();
        assert_eq!(
            written(&stream),
            format!("\x1b[H   \r\n   {}", crate::kitty::clear())
        );
    }

    #[test]
    fn what_the_renderer_returned_is_painted() {
        let state = State::default();
        let terminal = terminal(3, 2);
        let mut renderer = TerminalRenderer::new(terminal);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream).unwrap();
        assert!(written(&stream).starts_with("\x1b[H"));
    }

    #[test]
    fn the_prompt_is_drawn_on_the_last_row_after_the_frame_in_save_prompt_mode() {
        let state = new_state(vec![], Mode::SavePrompt { filename: "a".to_string() }, None);
        let terminal = terminal(11, 2);
        let mut renderer = TerminalRenderer::new(terminal);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream).unwrap();
        let Terminal { rows, .. } = terminal;
        assert!(written(&stream).contains(&format!("\x1b[{rows};1HSave as: a{CURSOR}")));
    }

    #[test]
    fn no_prompt_is_shown_in_command_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let terminal = terminal(11, 2);
        let mut renderer = TerminalRenderer::new(terminal);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream).unwrap();
        assert!(!written(&stream).contains("Save as:"));
    }

    fn temp_file(name: &str, contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("dre-{}-{name}", std::process::id()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn no_argument_starts_from_an_empty_diagram() {
        let state = load_state(None).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, None);
    }

    #[test]
    fn a_valid_file_loads_with_the_first_box_selected() {
        let path = temp_file("valid.dre", &dre_format::write(&dre_format::FileDoc {
            boxes: vec![dre_format::FileBox { label: "API".to_string(), colour: None, fill: None, rounded: false, children: vec![] }],
        }));
        let state = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert_eq!(state.doc.boxes.len(), 1);
        assert_eq!(state.doc.boxes[0].label, "API");
        assert_eq!(state.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
    }

    #[test]
    fn a_valid_file_loads_saving_back_to_the_given_path() {
        let path = temp_file("save-to.dre", "<dre/>");
        let state = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(state.unwrap().save_to, Some(path));
    }

    #[test]
    fn a_file_with_no_boxes_loads_an_empty_canvas_with_nothing_selected() {
        let path = temp_file("empty-dre.dre", "<dre/>");
        let state = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn a_zero_byte_file_is_invalid_data() {
        let path = temp_file("zero.dre", "");
        let result = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(result.err().map(|e| e.kind()), Some(io::ErrorKind::InvalidData));
    }

    #[test]
    fn a_malformed_file_is_invalid_data() {
        let path = temp_file("malformed.dre", "<dre><box");
        let result = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(result.err().map(|e| e.kind()), Some(io::ErrorKind::InvalidData));
    }

    #[test]
    fn an_invalid_file_has_an_exact_error_message() {
        let path = temp_file("bad-colour.dre", "<dre><box label=\"A\" colour=\"99\"/></dre>");
        let result = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let err = result.err().unwrap();
        assert_eq!(err.to_string(), format!("{path}: not a valid diagram"));
    }

    #[test]
    fn a_valid_file_is_not_a_new_file() {
        let path = temp_file("not-new.dre", "<dre/>");
        let state = load_state(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert!(!state.unwrap().new_file);
    }

    #[test]
    fn a_missing_file_loads_an_empty_canvas_saved_to_that_path() {
        let path = std::env::temp_dir()
            .join(format!("dre-{}-missing.dre", std::process::id()))
            .to_string_lossy()
            .into_owned();
        let state = load_state(Some(path.clone())).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some(path));
        assert!(state.new_file);
    }
}
