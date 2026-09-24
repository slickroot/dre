mod controller;
mod store;

use std::io;
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::kitty;
use crate::render::{GlyphCache, TerminalRenderer, CACHE_LIMIT};
use crate::tty::{self, RawMode};
use controller::key_source::TtyKeySource;
use controller::reducer::StateReducer;
use controller::screen::TerminalScreen;
use controller::{Controller, DreController};
use store::files::DiskFiles;
use store::{FileStateStore, StateStore};

pub(crate) struct Editor {
    store: Box<dyn StateStore>,
    controller: Box<dyn Controller>,
}

impl Editor {
    pub(crate) fn new(store: Box<dyn StateStore>, controller: Box<dyn Controller>) -> Self {
        Self { store, controller }
    }

    pub(crate) fn run(&mut self, path: Option<&str>) -> io::Result<()> {
        let state = self.store.load(path)?;
        let state = self.controller.run(state)?;
        self.store.save(&state)
    }
}

pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode> {
    let mut stdout = io::stdout();
    let fd = io::stdin().as_raw_fd();
    kitty::require(&mut stdout, fd)?;
    let window = tty::probe()?;
    let glyph_source = Box::new(GlyphCache::new(window.cell_width, window.cell_height));
    let renderer = TerminalRenderer::new(window, glyph_source, CACHE_LIMIT);
    let _raw = RawMode::enter(fd)?;
    let resize_fd = tty::install_resize_pipe()?;

    let store = FileStateStore::new(Box::new(DiskFiles));
    let controller = DreController::new(
        Box::new(TtyKeySource { fd, resize_fd }),
        Box::new(TerminalScreen {
            renderer,
            out: stdout,
        }),
        Box::new(StateReducer),
    );
    let mut editor = Editor::new(Box::new(store), Box::new(controller));
    editor.run(file.as_deref())?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::controller::MockController;
    use super::store::MockStateStore;
    use super::*;
    use crate::state::State;
    use mockall::Sequence;

    fn marked(count: usize) -> State {
        let mut state = State::default();
        state.pending_count = Some(count);
        state
    }

    #[test]
    fn running_loads_then_runs_the_controller_then_saves_the_state_it_returns() {
        let mut seq = Sequence::new();
        let mut store = MockStateStore::new();
        store
            .expect_load()
            .withf(|path| *path == Some("a.dre"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(marked(1)));
        let mut controller = MockController::new();
        controller
            .expect_run()
            .withf(|state| state.pending_count == Some(1))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(marked(2)));
        store
            .expect_save()
            .withf(|state| state.pending_count == Some(2))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        Editor::new(Box::new(store), Box::new(controller))
            .run(Some("a.dre"))
            .unwrap();
    }

    #[test]
    fn a_failed_load_neither_runs_the_controller_nor_saves() {
        let mut store = MockStateStore::new();
        store
            .expect_load()
            .times(1)
            .returning(|_| Err(io::Error::new(io::ErrorKind::InvalidData, "bad file")));
        store.expect_save().never();
        let mut controller = MockController::new();
        controller.expect_run().never();

        let error = Editor::new(Box::new(store), Box::new(controller))
            .run(Some("a.dre"))
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_failed_controller_run_propagates_and_nothing_is_saved() {
        let mut store = MockStateStore::new();
        store.expect_load().times(1).returning(|_| Ok(marked(1)));
        store.expect_save().never();
        let mut controller = MockController::new();
        controller
            .expect_run()
            .times(1)
            .returning(|_| Err(io::Error::other("controller failed")));

        let error = Editor::new(Box::new(store), Box::new(controller))
            .run(Some("a.dre"))
            .unwrap_err();

        assert_eq!(error.to_string(), "controller failed");
    }
}
