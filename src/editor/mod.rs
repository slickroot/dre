mod controller;
mod store;
mod terminal;

use std::io;
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::kitty;
use crate::render::{GlyphCache, TerminalRenderer, CACHE_LIMIT};
use crate::terminal::RawScreen;
use controller::DreController;
use store::{FileStateStore, StateStore};
use terminal::{DiskFiles, StateReducer, TerminalKeys, TerminalScreen};

pub(crate) struct Editor {
    store: Box<dyn StateStore>,
    controller: DreController,
}

impl Editor {
    pub(crate) fn new(store: Box<dyn StateStore>, controller: DreController) -> Self {
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
    let terminal = crate::terminal::probe()?;
    let glyph_source = Box::new(GlyphCache::new(terminal.cell_width, terminal.cell_height));
    let renderer = TerminalRenderer::new(terminal, glyph_source, CACHE_LIMIT);
    let _raw = RawScreen::open(fd)?;
    let resize_fd = crate::terminal::install_resize_pipe()?;

    let store = FileStateStore::new(Box::new(DiskFiles));
    let controller = DreController::new(
        Box::new(TerminalKeys { fd, resize_fd }),
        Box::new(TerminalScreen {
            renderer,
            out: stdout,
        }),
        Box::new(StateReducer),
    );
    let mut editor = Editor::new(Box::new(store), controller);
    editor.run(file.as_deref())?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::controller::{MockKeySource, MockReducer, MockScreen};
    use super::*;
    use crate::state::State;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct FakeStore {
        loaded: State,
        fail_load: bool,
        saved: Rc<RefCell<Vec<State>>>,
    }

    impl StateStore for FakeStore {
        fn load(&self, _path: Option<&str>) -> io::Result<State> {
            if self.fail_load {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "bad file"));
            }
            Ok(self.loaded.clone())
        }

        fn save(&self, state: &State) -> io::Result<()> {
            self.saved.borrow_mut().push(state.clone());
            Ok(())
        }
    }

    fn controller_rendering(frames: usize) -> DreController {
        let mut keys = MockKeySource::new();
        keys.expect_next_key()
            .times(frames)
            .returning(|| Ok(Some("q".to_string())));
        let mut screen = MockScreen::new();
        screen.expect_render().times(frames).returning(|_| Ok(()));
        let mut reducer = MockReducer::new();
        reducer
            .expect_reduce()
            .times(frames)
            .returning(|mut state, _| {
                state.running = false;
                state
            });
        DreController::new(Box::new(keys), Box::new(screen), Box::new(reducer))
    }

    fn editor_with(
        fail_load: bool,
        controller: DreController,
    ) -> (Editor, Rc<RefCell<Vec<State>>>) {
        let saved = Rc::new(RefCell::new(Vec::new()));
        let store = FakeStore {
            loaded: State::open(Default::default(), Some("a.dre".to_string())),
            fail_load,
            saved: saved.clone(),
        };
        (Editor::new(Box::new(store), controller), saved)
    }

    #[test]
    fn running_loads_then_runs_then_saves_the_final_state() {
        let (mut editor, saved) = editor_with(false, controller_rendering(1));
        editor.run(Some("a.dre")).unwrap();
        let saved = saved.borrow();
        assert_eq!(saved.len(), 1);
        assert!(!saved[0].running);
        assert_eq!(saved[0].save_to, Some("a.dre".to_string()));
    }

    #[test]
    fn a_failed_load_neither_renders_nor_saves() {
        let (mut editor, saved) = editor_with(true, controller_rendering(0));
        let error = editor.run(Some("a.dre")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(saved.borrow().is_empty());
    }
}
