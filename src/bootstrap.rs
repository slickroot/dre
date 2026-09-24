use std::io;
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::editor::controller::key_source::TtyKeySource;
use crate::editor::controller::reducer::StateReducer;
use crate::editor::controller::screen::TerminalScreen;
use crate::editor::controller::DreController;
use crate::editor::store::files::DiskFiles;
use crate::editor::store::FileStateStore;
use crate::editor::Editor;
use crate::kitty;
use crate::render::{GlyphCache, TerminalRenderer, CACHE_LIMIT};
use crate::tty;

pub(crate) fn run(file: Option<String>) -> io::Result<ExitCode> {
    let mut stdout = io::stdout();
    let fd = io::stdin().as_raw_fd();
    kitty::require(&mut stdout, fd)?;
    let window = tty::probe()?;
    let glyph_source = Box::new(GlyphCache::new(window.cell_width, window.cell_height));
    let renderer = TerminalRenderer::new(window, glyph_source, CACHE_LIMIT);
    let _raw = tty::RawMode::enter(fd)?;
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
    Editor::new(Box::new(store), Box::new(controller)).run(file.as_deref())?;
    Ok(ExitCode::SUCCESS)
}
