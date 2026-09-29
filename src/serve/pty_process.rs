use super::session::{Session, Spawner, Window};
use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg, FdFlag};
use nix::libc;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::pty::{openpty, Winsize};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::{read, setsid, write, Pid};
use std::io;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

const READ_BUFFER_SIZE: usize = 4096;
const READ_POLL_TIMEOUT_MS: u16 = 100;

nix::ioctl_write_ptr_bad!(set_window_size, libc::TIOCSWINSZ, libc::winsize);
nix::ioctl_none_bad!(take_controlling_terminal, libc::TIOCSCTTY);

pub(crate) struct PtyProcess {
    program: String,
}

impl PtyProcess {
    pub(crate) fn dre() -> io::Result<Self> {
        let program = std::env::current_exe()?.to_string_lossy().into_owned();
        Ok(Self { program })
    }
}

impl Spawner for PtyProcess {
    fn spawn(&self, window: Window) -> io::Result<Box<dyn Session>> {
        Ok(Box::new(PtyChild::start(&self.program, window)?))
    }
}

struct PtyChild {
    master: OwnedFd,
    pid: Pid,
}

impl PtyChild {
    fn start(program: &str, window: Window) -> io::Result<Self> {
        let pty = openpty(&winsize(window), None).map_err(io::Error::from)?;
        fcntl(&pty.master, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).map_err(io::Error::from)?;
        let mut command = Command::new(program);
        command
            .stdin(Stdio::from(pty.slave.try_clone()?))
            .stdout(Stdio::from(pty.slave.try_clone()?))
            .stderr(Stdio::from(pty.slave));
        unsafe {
            command.pre_exec(|| {
                setsid().map_err(io::Error::from)?;
                take_controlling_terminal(libc::STDIN_FILENO).map_err(io::Error::from)?;
                Ok(())
            });
        }
        let child = command.spawn()?;
        Ok(Self {
            master: pty.master,
            pid: Pid::from_raw(child.id() as i32),
        })
    }
}

impl Session for PtyChild {
    fn write(&self, bytes: &[u8]) {
        let mut remaining = bytes;
        while !remaining.is_empty() {
            match write(&self.master, remaining) {
                Ok(written) => remaining = &remaining[written..],
                Err(Errno::EINTR) => {}
                Err(_) => return,
            }
        }
    }

    fn read(&self) -> Option<Vec<u8>> {
        let mut fds = [PollFd::new(self.master.as_fd(), PollFlags::POLLIN)];
        match poll(&mut fds, PollTimeout::from(READ_POLL_TIMEOUT_MS)) {
            Ok(0) | Err(Errno::EINTR) => return Some(Vec::new()),
            Ok(_) => {}
            Err(_) => return None,
        }
        let mut buffer = [0u8; READ_BUFFER_SIZE];
        match read(&self.master, &mut buffer) {
            Ok(0) | Err(Errno::EIO) => None,
            Ok(count) => Some(buffer[..count].to_vec()),
            Err(Errno::EINTR) => Some(Vec::new()),
            Err(_) => None,
        }
    }

    fn resize(&self, window: Window) {
        let size = winsize(window);
        let _ = unsafe { set_window_size(self.master.as_raw_fd(), &size) };
        let _ = kill(self.pid, Signal::SIGWINCH);
    }

    fn wait(&self) {
        let _ = waitpid(self.pid, None);
    }
}

impl Drop for PtyChild {
    fn drop(&mut self) {
        let pid = self.pid;
        std::thread::spawn(move || {
            let _ = waitpid(pid, None);
        });
    }
}

fn winsize(window: Window) -> Winsize {
    Winsize {
        ws_col: clamp(window.cols),
        ws_row: clamp(window.rows),
        ws_xpixel: clamp(window.pixel_width),
        ws_ypixel: clamp(window.pixel_height),
    }
}

fn clamp(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::signal::kill;
    use std::time::{Duration, Instant};

    const WINDOW: Window = Window {
        cols: 80,
        rows: 24,
        pixel_width: 800,
        pixel_height: 480,
    };
    const RESIZED: Window = Window {
        cols: 100,
        rows: 30,
        pixel_width: 1000,
        pixel_height: 600,
    };
    const DEADLINE: Duration = Duration::from_secs(5);

    fn spawn_child(window: Window) -> PtyChild {
        PtyChild::start("cat", window).unwrap()
    }

    fn read_until(session: &PtyChild, expected: &str) -> String {
        let start = Instant::now();
        let mut seen = String::new();
        while !seen.contains(expected) && start.elapsed() < DEADLINE {
            if let Some(bytes) = session.read() {
                seen.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
        seen
    }

    fn window_size_of(session: &PtyChild) -> libc::winsize {
        nix::ioctl_read_bad!(get_window_size, libc::TIOCGWINSZ, libc::winsize);
        let mut size: libc::winsize = unsafe { std::mem::zeroed() };
        unsafe { get_window_size(session.master.as_raw_fd(), &mut size) }.unwrap();
        size
    }

    #[test]
    fn written_bytes_come_back_from_the_child() {
        let session = spawn_child(WINDOW);
        session.write(b"hello\n");
        assert!(read_until(&session, "hello").contains("hello"));
    }

    #[test]
    fn a_read_with_no_output_returns_an_empty_chunk() {
        let session = spawn_child(WINDOW);
        assert_eq!(session.read(), Some(Vec::new()));
    }

    #[test]
    fn the_pty_starts_with_the_requested_window() {
        let size = window_size_of(&spawn_child(WINDOW));
        assert_eq!(
            (size.ws_col, size.ws_row, size.ws_xpixel, size.ws_ypixel),
            (80, 24, 800, 480)
        );
    }

    #[test]
    fn resize_changes_the_pty_window() {
        let session = spawn_child(WINDOW);
        session.resize(RESIZED);
        let size = window_size_of(&session);
        assert_eq!(
            (size.ws_col, size.ws_row, size.ws_xpixel, size.ws_ypixel),
            (100, 30, 1000, 600)
        );
    }

    #[test]
    fn dropping_the_session_hangs_up_and_reaps_the_child() {
        let session = spawn_child(WINDOW);
        let pid = session.pid;
        drop(session);
        let start = Instant::now();
        while kill(pid, None).is_ok() && start.elapsed() < DEADLINE {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(kill(pid, None), Err(Errno::ESRCH));
    }
}
