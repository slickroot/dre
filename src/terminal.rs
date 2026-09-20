use nix::libc;
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};

nix::ioctl_read_bad!(terminal_window_size, libc::TIOCGWINSZ, libc::winsize);

const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";

pub(crate) struct RawScreen {
    fd: RawFd,
    saved: Termios,
}

impl RawScreen {
    pub(crate) fn open(fd: RawFd) -> io::Result<Self> {
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
        let mut stdout = io::stdout();
        stdout.write_all(ENTER_ALTERNATE_SCREEN.as_bytes())?;
        stdout.write_all(HIDE_CURSOR.as_bytes())?;
        stdout.flush()?;
        let mut raw = saved.clone();
        cfmakeraw(&mut raw);
        tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;
        Ok(RawScreen { fd, saved })
    }
}

impl Drop for RawScreen {
    fn drop(&mut self) {
        let borrowed = unsafe { BorrowedFd::borrow_raw(self.fd) };
        let _ = tcsetattr(borrowed, SetArg::TCSADRAIN, &self.saved);
        let mut stdout = io::stdout();
        let _ = stdout.write_all(SHOW_CURSOR.as_bytes());
        let _ = stdout.write_all(LEAVE_ALTERNATE_SCREEN.as_bytes());
        let _ = stdout.flush();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Terminal {
    pub(crate) cols: i64,
    pub(crate) rows: i64,
    pub(crate) cell_width: i64,
    pub(crate) cell_height: i64,
}

pub(crate) fn probe() -> io::Result<Terminal> {
    let stdout_fd = io::stdout().as_raw_fd();
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe { terminal_window_size(stdout_fd, &mut winsize) }.map_err(io::Error::from)?;
    Ok(measure(winsize))
}

fn measure(winsize: libc::winsize) -> Terminal {
    let cols = winsize.ws_col as f64;
    let rows = winsize.ws_row as f64;
    Terminal {
        cols: winsize.ws_col as i64,
        rows: winsize.ws_row as i64,
        cell_width: (winsize.ws_xpixel as f64 / cols).round() as i64,
        cell_height: (winsize.ws_ypixel as f64 / rows).round() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn winsize(cols: u16, rows: u16, xpixel: u16, ypixel: u16) -> libc::winsize {
        libc::winsize { ws_col: cols, ws_row: rows, ws_xpixel: xpixel, ws_ypixel: ypixel }
    }

    #[test]
    fn the_window_size_gives_the_columns_and_rows() {
        let terminal = measure(winsize(80, 24, 800, 480));
        assert_eq!((terminal.cols, terminal.rows), (80, 24));
    }

    #[test]
    fn a_cell_is_the_pixel_size_divided_by_the_grid() {
        let terminal = measure(winsize(80, 24, 800, 480));
        assert_eq!((terminal.cell_width, terminal.cell_height), (10, 20));
    }

    #[test]
    fn a_cell_that_does_not_divide_evenly_is_rounded() {
        let terminal = measure(winsize(3, 3, 8, 7));
        assert_eq!((terminal.cell_width, terminal.cell_height), (3, 2));
    }
}
