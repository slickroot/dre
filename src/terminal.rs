use nix::libc;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::sys::signal::{self, SaFlags, SigHandler, SigSet, Signal};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
use nix::unistd::read;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};
use std::sync::atomic::{AtomicI32, Ordering};

nix::ioctl_read_bad!(terminal_window_size, libc::TIOCGWINSZ, libc::winsize);

const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
const HIDE_CURSOR: &str = "\x1b[?25l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const RESET_BACKGROUND_COLOUR: &str = "\x1b]111\x1b\\";

pub(crate) const RESIZE: &str = "\x1bRESIZE";

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
        let (r, g, b) = crate::palette::palette(crate::palette::BACKGROUND).unwrap();
        write!(stdout, "\x1b]11;rgb:{:02x}/{:02x}/{:02x}\x1b\\", r, g, b)?;
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
        let _ = stdout.write_all(RESET_BACKGROUND_COLOUR.as_bytes());
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

fn retry_on_eintr<T>(mut syscall: impl FnMut() -> nix::Result<T>) -> io::Result<T> {
    loop {
        match syscall() {
            Err(nix::errno::Errno::EINTR) => continue,
            result => return result.map_err(io::Error::from),
        }
    }
}

pub(crate) fn probe() -> io::Result<Terminal> {
    let stdout_fd = io::stdout().as_raw_fd();
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    retry_on_eintr(|| unsafe { terminal_window_size(stdout_fd, &mut winsize) })?;
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

static RESIZE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

// Signal-safe: only touches the static fd and makes a raw async-signal-safe write(2) call.
#[allow(dead_code)]
extern "C" fn handle_sigwinch(_: libc::c_int) {
    let fd = RESIZE_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let byte = [0u8; 1];
        unsafe {
            libc::write(fd, byte.as_ptr() as *const libc::c_void, 1);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn install_resize_pipe() -> io::Result<RawFd> {
    let (read_fd, write_fd) = nix::unistd::pipe().map_err(io::Error::from)?;
    let write_raw_fd = write_fd.as_raw_fd();
    RESIZE_WRITE_FD.store(write_raw_fd, Ordering::Relaxed);
    std::mem::forget(write_fd);
    let action = signal::SigAction::new(
        SigHandler::Handler(handle_sigwinch),
        SaFlags::empty(),
        SigSet::empty(),
    );
    unsafe { signal::sigaction(Signal::SIGWINCH, &action) }.map_err(io::Error::from)?;
    let read_raw_fd = read_fd.as_raw_fd();
    std::mem::forget(read_fd);
    Ok(read_raw_fd)
}

pub(crate) fn poll_read(
    fd: RawFd,
    resize_fd: RawFd,
    timeout_ms: u16,
) -> io::Result<Option<String>> {
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let resize_borrowed = unsafe { BorrowedFd::borrow_raw(resize_fd) };
    let mut fds = [
        PollFd::new(borrowed, PollFlags::POLLIN),
        PollFd::new(resize_borrowed, PollFlags::POLLIN),
    ];
    let ready = retry_on_eintr(|| poll(&mut fds, PollTimeout::from(timeout_ms)))?;
    if ready == 0 {
        return Ok(None);
    }
    if fds[1]
        .revents()
        .is_some_and(|events| events.contains(PollFlags::POLLIN))
    {
        let mut byte = [0u8; 1];
        retry_on_eintr(|| read(resize_borrowed, &mut byte))?;
        return Ok(Some(RESIZE.to_string()));
    }
    let mut byte = [0u8; 1];
    let n = retry_on_eintr(|| read(borrowed, &mut byte))?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "input closed"));
    }
    Ok(Some((byte[0] as char).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn winsize(cols: u16, rows: u16, xpixel: u16, ypixel: u16) -> libc::winsize {
        libc::winsize {
            ws_col: cols,
            ws_row: rows,
            ws_xpixel: xpixel,
            ws_ypixel: ypixel,
        }
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

    #[test]
    fn poll_read_returns_none_when_no_byte_arrives_within_the_timeout() {
        use std::os::fd::AsRawFd;
        let (read, _write) = nix::unistd::pipe().unwrap();
        let (resize_read, _resize_write) = nix::unistd::pipe().unwrap();
        let result = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn poll_read_survives_a_signal_interrupting_the_poll() {
        use std::os::fd::AsRawFd;
        use std::time::Duration;

        extern "C" fn ignore(_: libc::c_int) {}
        let action = signal::SigAction::new(
            SigHandler::Handler(ignore),
            SaFlags::empty(),
            SigSet::empty(),
        );
        unsafe { signal::sigaction(Signal::SIGWINCH, &action) }.unwrap();

        let (read, write) = nix::unistd::pipe().unwrap();
        let (resize_read, _resize_write) = nix::unistd::pipe().unwrap();
        let pid = nix::unistd::getpid();
        let interrupter = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            nix::sys::signal::kill(pid, Signal::SIGWINCH).unwrap();
            nix::unistd::write(&write, b"x").unwrap();
        });
        let result = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1000);
        interrupter.join().unwrap();
        assert_eq!(result.unwrap(), Some("x".to_string()));
    }

    #[test]
    fn poll_read_returns_the_byte_that_is_ready() {
        use std::os::fd::AsRawFd;
        let (read, write) = nix::unistd::pipe().unwrap();
        nix::unistd::write(&write, b"x").unwrap();
        let (resize_read, _resize_write) = nix::unistd::pipe().unwrap();
        let result = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1000);
        assert_eq!(result.unwrap(), Some("x".to_string()));
    }

    #[test]
    fn poll_read_error_when_the_input_is_closed() {
        use std::os::fd::AsRawFd;
        let (read, write) = nix::unistd::pipe().unwrap();
        drop(write);
        let (resize_read, _resize_write) = nix::unistd::pipe().unwrap();
        let result = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1000);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn poll_read_returns_resize_when_the_resize_fd_becomes_readable() {
        use std::os::fd::AsRawFd;
        let (read, _write) = nix::unistd::pipe().unwrap();
        let (resize_read, resize_write) = nix::unistd::pipe().unwrap();
        nix::unistd::write(&resize_write, b"\0").unwrap();
        let result = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1000);
        assert_eq!(result.unwrap(), Some(RESIZE.to_string()));
    }

    #[test]
    fn poll_read_drains_the_resize_byte_so_it_is_not_reported_twice() {
        use std::os::fd::AsRawFd;
        let (read, _write) = nix::unistd::pipe().unwrap();
        let (resize_read, resize_write) = nix::unistd::pipe().unwrap();
        nix::unistd::write(&resize_write, b"\0").unwrap();
        let first = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1000);
        assert_eq!(first.unwrap(), Some(RESIZE.to_string()));

        let second = poll_read(read.as_raw_fd(), resize_read.as_raw_fd(), 1);
        assert!(matches!(second, Ok(None)));
    }
}
