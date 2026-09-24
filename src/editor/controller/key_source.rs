use std::io;
use std::os::fd::RawFd;

use crate::{terminal, IDLE_TIMEOUT_MS};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait KeySource {
    fn next_key(&mut self) -> io::Result<Option<String>>;
}

pub(crate) struct TerminalKeySource {
    pub(crate) fd: RawFd,
    pub(crate) resize_fd: RawFd,
}

impl KeySource for TerminalKeySource {
    fn next_key(&mut self) -> io::Result<Option<String>> {
        terminal::poll_read(self.fd, self.resize_fd, IDLE_TIMEOUT_MS)
    }
}
