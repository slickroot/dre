use std::io;
use std::os::fd::RawFd;

use crate::tty;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait KeySource {
    fn next_key(&self, timeout_ms: Option<u32>) -> io::Result<String>;
}

pub(crate) struct TtyKeySource {
    pub(crate) fd: RawFd,
    pub(crate) resize_fd: RawFd,
}

impl KeySource for TtyKeySource {
    fn next_key(&self, timeout_ms: Option<u32>) -> io::Result<String> {
        tty::read_key(self.fd, self.resize_fd, timeout_ms)
    }
}
