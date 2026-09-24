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

pub(crate) fn open(file: Option<String>) -> io::Result<ExitCode> {
    let store = FileStateStore::new(Box::new(DiskFiles));
    let state = store.load(file.as_deref())?;
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    kitty::require(&mut stdout, stdin.as_raw_fd())?;
    let terminal = crate::terminal::probe()?;
    let glyph_source = Box::new(GlyphCache::new(terminal.cell_width, terminal.cell_height));
    let renderer = TerminalRenderer::new(terminal, glyph_source, CACHE_LIMIT);
    let _screen = RawScreen::open(stdin.as_raw_fd())?;

    let fd = stdin.as_raw_fd();
    let resize_fd = crate::terminal::install_resize_pipe()?;
    let mut controller = DreController::new(
        Box::new(TerminalKeys { fd, resize_fd }),
        Box::new(TerminalScreen {
            renderer,
            out: stdout,
        }),
    );
    let state = controller.run(state)?;

    store.save(&state)?;
    Ok(ExitCode::SUCCESS)
}
