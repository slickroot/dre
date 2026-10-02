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
pub(crate) const TICK: &str = "\x1bTICK";

const ESC: u8 = 0x1b;
const ESCAPE_TIMEOUT_MS: u16 = 25;
const CSI_FINAL_BYTES: std::ops::RangeInclusive<u8> = 0x40..=0x7e;

pub(crate) struct RawMode {
    fd: RawFd,
    saved: Termios,
}

impl RawMode {
    pub(crate) fn enter(fd: RawFd) -> io::Result<Self> {
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
        let mut stdout = io::stdout();
        stdout.write_all(ENTER_ALTERNATE_SCREEN.as_bytes())?;
        stdout.write_all(HIDE_CURSOR.as_bytes())?;
        let (r, g, b) = crate::style::palette(crate::style::BACKGROUND).unwrap();
        write!(stdout, "\x1b]11;rgb:{:02x}/{:02x}/{:02x}\x1b\\", r, g, b)?;
        stdout.flush()?;
        let mut raw = saved.clone();
        cfmakeraw(&mut raw);
        tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;
        Ok(RawMode { fd, saved })
    }
}

impl Drop for RawMode {
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
pub(crate) struct Window {
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

pub(crate) fn probe() -> io::Result<Window> {
    let stdout_fd = io::stdout().as_raw_fd();
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    retry_on_eintr(|| unsafe { terminal_window_size(stdout_fd, &mut winsize) })?;
    Ok(measure(winsize))
}

fn measure(winsize: libc::winsize) -> Window {
    let cols = winsize.ws_col as f64;
    let rows = winsize.ws_row as f64;
    Window {
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

fn timeout(timeout_ms: Option<u32>) -> PollTimeout {
    match timeout_ms {
        Some(ms) => PollTimeout::try_from(ms).unwrap_or(PollTimeout::MAX),
        None => PollTimeout::NONE,
    }
}

pub(crate) fn read_key(fd: RawFd, resize_fd: RawFd, timeout_ms: Option<u32>) -> io::Result<String> {
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let resize_borrowed = unsafe { BorrowedFd::borrow_raw(resize_fd) };
    let mut fds = [
        PollFd::new(borrowed, PollFlags::POLLIN),
        PollFd::new(resize_borrowed, PollFlags::POLLIN),
    ];
    let ready = retry_on_eintr(|| poll(&mut fds, timeout(timeout_ms)))?;
    if fds[1]
        .revents()
        .is_some_and(|events| events.contains(PollFlags::POLLIN))
    {
        let mut byte = [0u8; 1];
        retry_on_eintr(|| read(resize_borrowed, &mut byte))?;
        return Ok(RESIZE.to_string());
    }
    if ready == 0 {
        return Ok(TICK.to_string());
    }
    let mut byte = [0u8; 1];
    let n = retry_on_eintr(|| read(borrowed, &mut byte))?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "input closed"));
    }
    let mut key = (byte[0] as char).to_string();
    if byte[0] == ESC {
        read_escape_sequence(borrowed, &mut key)?;
    }
    Ok(key)
}

fn read_escape_sequence(fd: BorrowedFd, key: &mut String) -> io::Result<()> {
    let Some(introducer) = read_byte_within_escape_timeout(fd)? else {
        return Ok(());
    };
    key.push(introducer as char);
    if introducer != b'[' {
        return Ok(());
    }
    while let Some(byte) = read_byte_within_escape_timeout(fd)? {
        key.push(byte as char);
        if CSI_FINAL_BYTES.contains(&byte) {
            break;
        }
    }
    Ok(())
}

fn read_byte_within_escape_timeout(fd: BorrowedFd) -> io::Result<Option<u8>> {
    let mut fds = [PollFd::new(fd, PollFlags::POLLIN)];
    let ready = retry_on_eintr(|| poll(&mut fds, PollTimeout::from(ESCAPE_TIMEOUT_MS)))?;
    if ready == 0 {
        return Ok(None);
    }
    let mut byte = [0u8; 1];
    let n = retry_on_eintr(|| read(fd, &mut byte))?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "input closed"));
    }
    Ok(Some(byte[0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::OwnedFd;

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
        let window = measure(winsize(80, 24, 800, 480));
        assert_eq!((window.cols, window.rows), (80, 24));
    }

    #[test]
    fn a_cell_is_the_pixel_size_divided_by_the_grid() {
        let window = measure(winsize(80, 24, 800, 480));
        assert_eq!((window.cell_width, window.cell_height), (10, 20));
    }

    #[test]
    fn a_cell_that_does_not_divide_evenly_is_rounded() {
        let window = measure(winsize(3, 3, 8, 7));
        assert_eq!((window.cell_width, window.cell_height), (3, 2));
    }

    // A pipe stands in for the terminal: a real timeout is testable without a tty.
    fn pipes() -> (RawFd, OwnedFd, RawFd, OwnedFd) {
        let (key_read, key_write) = nix::unistd::pipe().unwrap();
        let (resize_read, resize_write) = nix::unistd::pipe().unwrap();
        let key_fd = key_read.as_raw_fd();
        let resize_fd = resize_read.as_raw_fd();
        // `read_key` takes bare `RawFd`s, so the read ends must outlive the call as leaked fds.
        std::mem::forget(key_read);
        std::mem::forget(resize_read);
        (key_fd, key_write, resize_fd, resize_write)
    }

    fn send(fd: &OwnedFd, byte: u8) {
        nix::unistd::write(fd, &[byte]).unwrap();
    }

    #[test]
    fn read_key_returns_a_tick_when_the_timeout_expires_with_no_byte() {
        let (key_read, _key_write, resize_read, _resize_write) = pipes();
        assert_eq!(read_key(key_read, resize_read, Some(10)).unwrap(), TICK);
    }

    #[test]
    fn read_key_returns_the_byte_when_it_arrives_before_the_timeout() {
        let (key_read, key_write, resize_read, _resize_write) = pipes();
        send(&key_write, b'x');
        assert_eq!(read_key(key_read, resize_read, Some(500)).unwrap(), "x");
    }

    #[test]
    fn a_resize_arriving_during_the_timeout_still_wins_over_the_tick() {
        let (key_read, _key_write, resize_read, resize_write) = pipes();
        send(&resize_write, 0);
        assert_eq!(read_key(key_read, resize_read, Some(500)).unwrap(), RESIZE);
    }

    #[test]
    fn read_key_without_a_timeout_never_returns_a_tick() {
        let (key_read, key_write, resize_read, _resize_write) = pipes();
        let (tx, rx) = std::sync::mpsc::channel();
        let reader =
            std::thread::spawn(move || tx.send(read_key(key_read, resize_read, None)).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(rx.try_recv().is_err());
        send(&key_write, b'x');
        reader.join().unwrap();
        assert_eq!(rx.recv().unwrap().unwrap(), "x");
    }
}
