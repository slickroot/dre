use nix::libc;
use nix::sys::select::{select, FdSet};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use nix::sys::time::{TimeVal, TimeValLike};
use nix::unistd::read;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::process::ExitCode;

use crate::layout::{layout, with_cursor};
use crate::render::TerminalRenderer;
use crate::state::{handle_key, State};
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
    let placements = with_cursor(layout(state.boxes.clone(), cols, rows), state.selected.clone());
    let lines = renderer.render(&placements, cols, rows);
    paint(stream, &lines)
}

fn run<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<()> {
    let (cell_width, cell_height) = cell_size()?;
    let mut renderer = TerminalRenderer::new(KittyGraphics::new(), cell_width, cell_height);
    let mut state = State::default();
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
        state = handle_key(&state, &key);
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
    run(&mut stdout, stdin_fd)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
