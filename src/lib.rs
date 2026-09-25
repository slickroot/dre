#[cfg(not(target_arch = "wasm32"))]
mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod diagram;
#[cfg(not(target_arch = "wasm32"))]
mod dre_format;
#[cfg(not(target_arch = "wasm32"))]
mod editor;
#[cfg(not(target_arch = "wasm32"))]
mod filesystem;
#[cfg(not(target_arch = "wasm32"))]
mod kitty;
mod layout;
mod palette;
mod render;
mod state;
mod status_line;
#[cfg(test)]
mod test_support;
#[cfg(not(target_arch = "wasm32"))]
mod tty;

#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

pub use diagram::Document;
pub use render::{Renderer, SvgRenderer};
pub use state::State;

pub const IDLE_TIMEOUT_MS: u16 = 1000;

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
        self.state = state::reduce(std::mem::take(&mut self.state), Some(key));
    }

    #[doc(hidden)]
    pub fn go_idle(&mut self) {
        self.state = state::reduce(std::mem::take(&mut self.state), None);
    }

    pub fn is_running(&self) -> bool {
        self.state.running
    }

    pub fn document(&self) -> &Document {
        &self.state.doc
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn extent(&self) -> (i64, i64) {
        let placements = layout::layout(&self.state.doc.boxes);
        let width = placements
            .iter()
            .map(|placement| placement.x + placement.width)
            .max()
            .unwrap_or(0);
        let height = placements
            .iter()
            .map(|placement| placement.y + placement.height)
            .max()
            .unwrap_or(0);
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
        cli::Command::Edit(file) => editor::bootstrap::run(file),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with_a_box_selected() -> Session {
        let mut session = Session::new();
        session.press_key("b");
        session.press_key("\x1b");
        session
    }

    #[test]
    fn going_idle_hides_the_cursor_and_the_next_key_restores_it_on_the_same_box() {
        let mut session = session_with_a_box_selected();
        let selected = session.state().selected.clone();
        assert!(selected.is_some());
        session.go_idle();
        assert_eq!(session.state().selected, None);
        session.press_key("z");
        assert_eq!(session.state().selected, selected);
    }

    #[test]
    fn going_idle_in_insert_mode_leaves_the_caret() {
        let mut session = Session::new();
        session.press_key("b");
        let selected = session.state().selected.clone();
        assert!(selected.is_some());
        session.go_idle();
        assert_eq!(session.state().selected, selected);
    }
}
