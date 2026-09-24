use std::io::{self, Stdout, Write};
use std::os::fd::RawFd;

use super::controller::{KeySource, Screen};
use super::store::Files;
use crate::render::{Renderer, TerminalRenderer};
use crate::state::State;
use crate::{filesystem, IDLE_TIMEOUT_MS};

pub(crate) struct DiskFiles;

impl Files for DiskFiles {
    fn read(&self, path: &str) -> io::Result<String> {
        filesystem::read(path)
    }

    fn write(&self, path: &str, contents: &str) -> io::Result<()> {
        filesystem::write(path, contents)
    }
}

pub(crate) struct TerminalKeys {
    pub(crate) fd: RawFd,
    pub(crate) resize_fd: RawFd,
}

impl KeySource for TerminalKeys {
    fn next_key(&mut self) -> io::Result<Option<String>> {
        crate::terminal::poll_read(self.fd, self.resize_fd, IDLE_TIMEOUT_MS)
    }
}

pub(crate) struct TerminalScreen {
    pub(crate) renderer: TerminalRenderer,
    pub(crate) out: Stdout,
}

impl Screen for TerminalScreen {
    fn render(&mut self, state: &State) -> io::Result<()> {
        self.renderer.render(state, &mut self.out)?;
        self.out.flush()
    }

    fn resize(&mut self) -> io::Result<()> {
        self.renderer.on_resize(crate::terminal::probe()?);
        Ok(())
    }
}
