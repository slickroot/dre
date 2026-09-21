use std::process::ExitCode;

mod canvas;
mod cli;
mod command_mode;
mod diagram;
mod dre_format;
mod editor;
mod file_document;
mod filesystem;
mod insert_mode;
mod kitty;
mod layout;
mod render;
mod save_prompt_mode;
mod state;
mod terminal;

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
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
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
