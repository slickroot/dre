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
use terminal::{DiskFiles, TerminalKeys, TerminalScreen};

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
    );
    let mut editor = Editor::new(Box::new(store), controller);
    editor.run(file.as_deref())?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::controller::tests::{RecordingScreen, Screenings, ScriptedKeys};
    use super::*;
    use crate::state::State;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct SavedState {
        state: State,
        frames_rendered_before: usize,
    }

    struct FakeStore {
        loaded: State,
        fail_load: bool,
        saved: Rc<RefCell<Vec<SavedState>>>,
        seen: Rc<RefCell<Screenings>>,
    }

    impl StateStore for FakeStore {
        fn load(&self, _path: Option<&str>) -> io::Result<State> {
            if self.fail_load {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "bad file"));
            }
            Ok(self.loaded.clone())
        }

        fn save(&self, state: &State) -> io::Result<()> {
            self.saved.borrow_mut().push(SavedState {
                state: state.clone(),
                frames_rendered_before: self.seen.borrow().frames.len(),
            });
            Ok(())
        }
    }

    fn editor_with(
        fail_load: bool,
        keys: Vec<Option<String>>,
    ) -> (
        Editor,
        Rc<RefCell<Vec<SavedState>>>,
        Rc<RefCell<Screenings>>,
    ) {
        let seen = Rc::new(RefCell::new(Screenings::default()));
        let saved = Rc::new(RefCell::new(Vec::new()));
        let store = FakeStore {
            loaded: State::open(Default::default(), Some("a.dre".to_string())),
            fail_load,
            saved: saved.clone(),
            seen: seen.clone(),
        };
        let controller = DreController::new(
            Box::new(ScriptedKeys {
                script: keys.into_iter(),
            }),
            Box::new(RecordingScreen { seen: seen.clone() }),
        );
        (Editor::new(Box::new(store), controller), saved, seen)
    }

    #[test]
    fn running_loads_then_runs_then_saves_the_final_state() {
        let (mut editor, saved, seen) = editor_with(false, vec![Some("q".to_string())]);
        editor.run(Some("a.dre")).unwrap();
        let saved = saved.borrow();
        assert_eq!(saved.len(), 1);
        assert!(!saved[0].state.running);
        assert_eq!(saved[0].state.save_to, Some("a.dre".to_string()));
        assert_eq!(saved[0].frames_rendered_before, seen.borrow().frames.len());
        assert!(saved[0].frames_rendered_before > 0);
    }

    #[test]
    fn a_failed_load_neither_renders_nor_saves() {
        let (mut editor, saved, seen) = editor_with(true, vec![Some("q".to_string())]);
        let error = editor.run(Some("a.dre")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(seen.borrow().frames.is_empty());
        assert!(saved.borrow().is_empty());
    }
}
