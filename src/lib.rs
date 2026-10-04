#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub mod bench;
#[cfg(not(target_arch = "wasm32"))]
mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod composer;
mod diagram;
#[cfg(not(target_arch = "wasm32"))]
mod dre_format;
#[cfg(not(target_arch = "wasm32"))]
mod editor;
#[cfg(not(target_arch = "wasm32"))]
mod filesystem;
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub mod flex;
#[cfg(not(target_arch = "wasm32"))]
mod key_source;
#[cfg(not(target_arch = "wasm32"))]
mod kitty;
mod layout;
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod serve;
mod state;
mod style;
#[cfg(test)]
mod test_support;
#[cfg(not(target_arch = "wasm32"))]
mod tty;
pub mod view;

#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

pub use diagram::Document;
pub use render::{Renderer, SvgRenderer};
pub use state::State;

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
        self.state = state::reduce(std::mem::take(&mut self.state), key).0;
    }

    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }

    pub fn document(&self) -> &Document {
        self.state.doc()
    }

    pub fn state(&self) -> &State {
        &self.state
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
        cli::Command::Edit(file) => editor::bootstrap::run(file),
        cli::Command::Export { input } => cli::export(input),
        cli::Command::Serve {
            listen,
            host_key,
            data_dir,
        } => serve::run(listen, host_key, data_dir),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
