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
    fn spawn(&self, window: Window, path: &str) -> io::Result<Box<dyn Session>> {
        Ok(Box::new(PtyChild::start(&self.program, window, path)?))
    }
}

struct PtyChild {
    master: OwnedFd,
    pid: Pid,
}

impl PtyChild {
    fn start(program: &str, window: Window, path: &str) -> io::Result<Self> {
        let pty = openpty(&winsize(window), None).map_err(io::Error::from)?;
        fcntl(&pty.master, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).map_err(io::Error::from)?;
        let mut command = Command::new(program);
        command
            .arg(path)
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
