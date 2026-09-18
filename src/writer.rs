use nix::libc;
use nix::sys::select::{select, FdSet};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use nix::sys::time::{TimeVal, TimeValLike};
use nix::unistd::read;
use std::fs;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::process::ExitCode;

use crate::dre_format;
use crate::file_document;
use crate::layout::{layout, with_cursor};
use crate::render::{TerminalRenderer, CURSOR};
use crate::state::{handle_key, Mode, State};
use crate::KittyGraphics;

const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const HOME_CURSOR: &str = "\x1b[H";
const INTERRUPT: &str = "\x03";
const KITTY_GRAPHICS_QUERY: &str = "\x1b_Gi=1,a=q;\x1b\\";
const NOT_SUPPORTED_MESSAGE: &str =
    "Dre requires a terminal with Kitty graphics protocol support.";
const KITTY_GRAPHICS_REPLY_TIMEOUT_MICROS: i64 = 500_000;
const CLEAR_LINE: &str = "\r\x1b[K";

nix::ioctl_read_bad!(terminal_window_size, libc::TIOCGWINSZ, libc::winsize);

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

fn supports_kitty_graphics<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<bool> {
    let borrowed = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
    let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
    stream.write_all(KITTY_GRAPHICS_QUERY.as_bytes())?;
    stream.flush()?;
    let mut raw = saved.clone();
    cfmakeraw(&mut raw);
    tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;

    let result = (|| -> io::Result<bool> {
        let mut fds = FdSet::new();
        fds.insert(borrowed);
        let mut timeout = TimeVal::microseconds(KITTY_GRAPHICS_REPLY_TIMEOUT_MICROS);
        let ready = select(None, Some(&mut fds), None, None, Some(&mut timeout))
            .map_err(io::Error::from)?;
        let mut buffer = [0u8; 32];
        let reply: Vec<u8> = if ready > 0 && fds.contains(borrowed) {
            let count = read(borrowed, &mut buffer).map_err(io::Error::from)?;
            buffer[..count].to_vec()
        } else {
            Vec::new()
        };
        Ok(reply.windows(3).any(|window| window == b"i=1"))
    })();

    tcsetattr(borrowed, SetArg::TCSADRAIN, &saved).map_err(io::Error::from)?;
    result
}

fn cell_size() -> io::Result<(i64, i64)> {
    let stdout_fd = io::stdout().as_raw_fd();
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe { terminal_window_size(stdout_fd, &mut winsize) }.map_err(io::Error::from)?;
    let cols = winsize.ws_col as f64;
    let rows = winsize.ws_row as f64;
    let xpixel = winsize.ws_xpixel as f64;
    let ypixel = winsize.ws_ypixel as f64;
    Ok(((xpixel / cols).round() as i64, (ypixel / rows).round() as i64))
}

fn paint<W: Write>(stream: &mut W, lines: &[String]) -> io::Result<()> {
    stream.write_all(HOME_CURSOR.as_bytes())?;
    stream.write_all(lines.join("\r\n").as_bytes())?;
    stream.flush()
}

fn frame<W: Write>(
    state: &State,
    renderer: &mut TerminalRenderer,
    stream: &mut W,
    cols: i64,
    rows: i64,
) -> io::Result<()> {
    let selected = state.doc.selected.clone();
    let placements = with_cursor(layout(&state.doc.boxes), selected);
    let mut lines = renderer.render(&placements, cols, rows);
    if let Mode::SavePrompt { filename } = &state.mode {
        let last = lines.len() - 1;
        lines[last] = prompt_line(filename, cols);
    }
    paint(stream, &lines)
}

fn prompt_line(filename: &str, cols: i64) -> String {
    format!("Save as: {filename}{CURSOR}")
        .chars()
        .chain(std::iter::repeat(' '))
        .take(cols as usize)
        .collect()
}

fn run<W: Write>(stream: &mut W, stdin_fd: RawFd, mut state: State) -> io::Result<()> {
    let (cell_width, cell_height) = cell_size()?;
    let mut renderer = TerminalRenderer::new(KittyGraphics::new(), cell_width, cell_height);
    let guard = RawModeGuard::new(stdin_fd, stream)?;
    let stdin = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
    while state.running {
        let (cols, rows) = terminal_size(stdin_fd)?;
        frame(&state, &mut renderer, &mut *guard.stream, cols, rows)?;
        let mut key_buffer = [0u8; 1];
        read(stdin, &mut key_buffer).map_err(io::Error::from)?;
        let key = std::str::from_utf8(&key_buffer).unwrap_or("").to_string();
        if key == INTERRUPT {
            return Ok(());
        }
        state = handle_key(state, &key);
        if !state.running {
            if let Some(path) = &state.save_to {
                fs::write(path, dre_format::write(&file_document::from_state(&state)))?;
            }
        }
    }
    Ok(())
}

fn terminal_size(stdin_fd: RawFd) -> io::Result<(i64, i64)> {
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe { terminal_window_size(stdin_fd, &mut winsize) }.map_err(io::Error::from)?;
    Ok((winsize.ws_col as i64, winsize.ws_row as i64))
}

fn load_state(arg: Option<String>) -> io::Result<State> {
    let Some(path) = arg else {
        return Ok(State::default());
    };
    let text = match fs::read_to_string(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut state = State::default();
            state.save_to = Some(path);
            state.new_file = true;
            return Ok(state);
        }
        result => result?,
    };
    let doc = dre_format::read(&text).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, format!("{path}: not a valid diagram"))
    })?;
    let mut state = file_document::to_state(doc);
    state.save_to = Some(path);
    Ok(state)
}

pub fn write(file: Option<String>) -> io::Result<ExitCode> {
    let state = load_state(file)?;
    let mut stdout = io::stdout();
    let stdin_fd = io::stdin().as_raw_fd();
    let supported = supports_kitty_graphics(&mut stdout, stdin_fd)?;
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
    use crate::state::new_state;
    use std::io::Cursor;

    fn written(buffer: &Cursor<Vec<u8>>) -> String {
        String::from_utf8(buffer.get_ref().clone()).expect("test output is always ASCII")
    }

    #[test]
    fn the_cursor_goes_home_before_the_lines() {
        let mut stream = Cursor::new(Vec::new());
        paint(&mut stream, &["ab".to_string(), "cd".to_string()]).unwrap();
        assert_eq!(written(&stream), format!("{HOME_CURSOR}ab\r\ncd"));
    }

    #[test]
    fn no_newline_follows_the_last_line() {
        let mut stream = Cursor::new(Vec::new());
        paint(&mut stream, &["ab".to_string(), "cd".to_string()]).unwrap();
        assert!(!written(&stream).ends_with('\n'));
    }

    #[test]
    fn the_stream_is_flushed() {
        let mut stream = Cursor::new(Vec::new());
        assert!(paint(&mut stream, &["ab".to_string()]).is_ok());
    }

    #[test]
    fn the_renderer_is_given_the_terminal_size() {
        let state = State::default();
        let mut renderer = TerminalRenderer::new(KittyGraphics::new(), 1, 1);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream, 3, 2).unwrap();
        assert_eq!(
            written(&stream),
            format!("{HOME_CURSOR}   \r\n   {}", crate::DELETE_ALL)
        );
    }

    #[test]
    fn what_the_renderer_returned_is_painted() {
        let state = State::default();
        let mut renderer = TerminalRenderer::new(KittyGraphics::new(), 1, 1);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream, 3, 2).unwrap();
        assert!(written(&stream).starts_with(HOME_CURSOR));
    }

    #[test]
    fn the_prompt_shows_the_filename_followed_by_the_cursor() {
        assert_eq!(prompt_line("a.dre", 15), format!("Save as: a.dre{CURSOR}"));
    }

    #[test]
    fn the_prompt_is_padded_to_the_terminal_width() {
        assert_eq!(prompt_line("a", 14), format!("Save as: a{CURSOR}   "));
    }

    #[test]
    fn the_prompt_is_cut_to_the_terminal_width() {
        assert_eq!(prompt_line("abc", 10), "Save as: a");
    }

    #[test]
    fn the_prompt_replaces_the_last_row_in_save_prompt_mode() {
        let state = new_state(vec![], Mode::SavePrompt { filename: "a".to_string() }, None);
        let mut renderer = TerminalRenderer::new(KittyGraphics::new(), 1, 1);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream, 11, 2).unwrap();
        assert_eq!(
            written(&stream),
            format!("{HOME_CURSOR}{}\r\n{}", " ".repeat(11), prompt_line("a", 11))
        );
    }

    #[test]
    fn no_prompt_is_shown_in_command_mode() {
        let state = new_state(vec![], Mode::Command, None);
        let mut renderer = TerminalRenderer::new(KittyGraphics::new(), 1, 1);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream, 11, 2).unwrap();
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
        assert_eq!(state.doc.selected, Some(crate::state::Path { ancestors: vec![], index: 0 }));
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
