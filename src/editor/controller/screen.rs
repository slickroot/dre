use std::io::{self, Stdout, Write};

use crate::render::{Renderer, TerminalRenderer};
use crate::state::State;
use crate::tty;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Screen {
    fn render(&mut self, state: &State) -> io::Result<()>;
    fn resize(&mut self) -> io::Result<()>;
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
        self.renderer.on_resize(tty::probe()?);
        Ok(())
    }
}
