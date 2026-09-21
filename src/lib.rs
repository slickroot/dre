#[cfg(not(target_arch = "wasm32"))]
mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod command_mode;
mod diagram;
#[cfg(not(target_arch = "wasm32"))]
mod dre_format;
#[cfg(not(target_arch = "wasm32"))]
mod editor;
#[cfg(not(target_arch = "wasm32"))]
mod file_document;
#[cfg(not(target_arch = "wasm32"))]
mod filesystem;
mod insert_mode;
#[cfg(not(target_arch = "wasm32"))]
mod kitty;
mod layout;
mod render;
mod save_prompt_mode;
mod state;
#[cfg(not(target_arch = "wasm32"))]
mod terminal;

#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

pub use diagram::Document;
pub use render::{Renderer, SvgRenderer};

pub struct Session {
    state: state::State,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: state::State::default(),
        }
    }

    pub fn press_key(&mut self, key: &str) {
        self.state = state::handle_key(std::mem::take(&mut self.state), key);
    }

    pub fn is_running(&self) -> bool {
        self.state.running
    }

    pub fn document(&self) -> &Document {
        &self.state.doc
    }

    pub fn extent(&self) -> (i64, i64) {
        let placements = layout::layout(&self.state.doc.boxes);
        let width = placements.iter().map(|placement| placement.x + placement.width).max().unwrap_or(0);
        let height = placements.iter().map(|placement| placement.y + placement.height).max().unwrap_or(0);
        (width, height)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> ExitCode {
    let result = match cli::parse_args() {
        cli::Command::Edit(file) => editor::open(file),
        cli::Command::Export { input } => cli::export(input),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
