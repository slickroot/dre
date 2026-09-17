use nix::libc;
use nix::sys::select::{select, FdSet};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use nix::sys::time::{TimeVal, TimeValLike};
use nix::unistd::read;
use std::fs;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::process::ExitCode;

use crate::dre_format::v0;
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
    let placements = with_cursor(layout(&state.doc.boxes, cols, rows), selected);
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

fn load_state(path: Option<String>) -> io::Result<State> {
    match path {
        None => Ok(State::default()),
        Some(path) => {
            let contents = fs::read_to_string(&path)?;
            match v0::read(&contents) {
                Some(doc) => Ok(file_document::to_state(doc)),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{path} is not a valid diagram"),
                )),
            }
        }
    }
}

fn run<W: Write>(stream: &mut W, stdin_fd: RawFd, state: State) -> io::Result<()> {
    let (cell_width, cell_height) = cell_size()?;
    let mut renderer = TerminalRenderer::new(KittyGraphics::new(), cell_width, cell_height);
    let mut state = state;
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
        if let Some(path) = &state.save_to {
            fs::write(path, v0::write(&file_document::from_state(&state)))?;
        }
    }
    Ok(())
}

fn terminal_size(stdin_fd: RawFd) -> io::Result<(i64, i64)> {
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe { terminal_window_size(stdin_fd, &mut winsize) }.map_err(io::Error::from)?;
    Ok((winsize.ws_col as i64, winsize.ws_row as i64))
}

pub fn write() -> io::Result<ExitCode> {
    let mut stdout = io::stdout();
    let stdin_fd = io::stdin().as_raw_fd();
    let supported = supports_kitty_graphics(&mut stdout, stdin_fd)?;
    if !supported {
        let _ = stdout.write_all(CLEAR_LINE.as_bytes());
        println!("{NOT_SUPPORTED_MESSAGE}");
        return Ok(ExitCode::FAILURE);
    }
    let state = load_state(std::env::args().nth(1))?;
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
        let state = new_state(vec![], Mode::SavePrompt { filename: "a".to_string() }, vec![]);
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
        let state = new_state(vec![], Mode::Command, vec![]);
        let mut renderer = TerminalRenderer::new(KittyGraphics::new(), 1, 1);
        let mut stream = Cursor::new(Vec::new());
        frame(&state, &mut renderer, &mut stream, 11, 2).unwrap();
        assert!(!written(&stream).contains("Save as:"));
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "dre-writer-test-{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn no_path_gives_the_default_state() {
        let state = load_state(None).unwrap();
        assert_eq!(state.doc.boxes, Vec::new());
        assert_eq!(state.doc.selected, Vec::<usize>::new());
        assert_eq!(state.mode, Mode::Command);
        assert_eq!(state.save_to, None);
    }

    #[test]
    fn a_valid_file_is_loaded_with_the_first_box_selected() {
        let path = unique_temp_path("valid");
        let text = "\"a\" colour=2 rounded\n  \"b\"\n";
        fs::write(&path, text).unwrap();

        let state = load_state(Some(path.to_str().unwrap().to_string())).unwrap();

        let expected = file_document::to_state(v0::read(text).unwrap());
        assert_eq!(state.doc, expected.doc);
        assert_eq!(state.doc.selected, vec![0]);

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_invalid_file_is_a_data_error() {
        let path = unique_temp_path("invalid");
        fs::write(&path, "not canonical text").unwrap();

        let result = load_state(Some(path.to_str().unwrap().to_string()));
        match result {
            Err(err) => assert_eq!(err.kind(), io::ErrorKind::InvalidData),
            Ok(_) => panic!("expected an error"),
        }

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_empty_file_gives_no_boxes_and_no_selection() {
        let path = unique_temp_path("empty");
        fs::write(&path, "").unwrap();

        let state = load_state(Some(path.to_str().unwrap().to_string())).unwrap();
        assert_eq!(state.doc.boxes, Vec::new());
        assert_eq!(state.doc.selected, Vec::<usize>::new());

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_nonexistent_path_is_an_error() {
        let path = unique_temp_path("missing");
        let result = load_state(Some(path.to_str().unwrap().to_string()));
        assert!(result.is_err());
    }
}
