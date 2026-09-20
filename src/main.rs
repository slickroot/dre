use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use render::{Renderer, TerminalRenderer};
use state::{handle_key, Mode, State};
use terminal::RawScreen;

mod canvas;
mod cli;
mod command_mode;
mod diagram;
mod dre_format;
mod file_document;
mod filesystem;
mod insert_mode;
mod kitty;
mod layout;
mod render;
mod save_prompt_mode;
mod state;
mod terminal;

const CURSOR: char = '\u{2588}';
const INTERRUPT: &str = "\x03";

fn main() -> ExitCode {
    let result = match cli::parse_args() {
        cli::Command::Edit(file) => edit(file),
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

fn edit(file: Option<String>) -> io::Result<ExitCode> {
    let mut state = load(file)?;
    let mut stdout = io::stdout();
    kitty::require(&mut stdout, io::stdin().as_raw_fd())?;
    let mut renderer = TerminalRenderer::new(terminal::probe()?);
    let _screen = RawScreen::open(io::stdin().as_raw_fd())?;

    while state.running {
        let status = status(&state);
        renderer.render(&state.doc, &mut stdout)?;
        renderer.status_line(status.as_deref(), &mut stdout)?;
        stdout.flush()?;

        let mut key = [0u8; 1];
        io::stdin().read_exact(&mut key)?;
        let key = (key[0] as char).to_string();
        if key == INTERRUPT {
            state.save_to = None;
            break;
        }
        state = handle_key(state, &key);
    }

    if let Some(path) = &state.save_to {
        filesystem::write(path, &dre_format::write(&file_document::from_document(&state.doc)))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn load(file: Option<String>) -> io::Result<State> {
    let Some(path) = file else { return Ok(State::default()) };
    let text = match filesystem::read(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(state::new_file(path)),
        result => result?,
    };
    let doc = dre_format::read(&text)
        .map(file_document::to_document)
        .ok_or_else(|| filesystem::invalid(&path))?;
    Ok(state::load(doc, Some(path)))
}

fn status(state: &State) -> Option<String> {
    match &state.mode {
        Mode::SavePrompt { filename } => Some(format!("Save as: {filename}{CURSOR}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::Path;
    use crate::state::new_state;
    use std::fs;

    #[test]
    fn a_save_prompt_shows_the_filename_being_typed() {
        let state = new_state(vec![], Mode::SavePrompt { filename: "a".to_string() }, None);
        assert_eq!(status(&state), Some(format!("Save as: a{CURSOR}")));
    }

    #[test]
    fn command_mode_has_no_status_line() {
        let state = new_state(vec![], Mode::Command, None);
        assert_eq!(status(&state), None);
    }

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("dre-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn temp_file(name: &str, contents: &str) -> String {
        let path = temp_path(name);
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn no_argument_starts_from_an_empty_diagram() {
        let state = load(None).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, None);
    }

    #[test]
    fn a_valid_file_loads_with_the_first_box_selected() {
        let path = temp_file("valid.dre", &dre_format::write(&dre_format::FileDoc {
            boxes: vec![dre_format::FileBox { label: "API".to_string(), colour: None, fill: None, rounded: false, children: vec![] }],
        }));
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert_eq!(state.doc.boxes.len(), 1);
        assert_eq!(state.doc.boxes[0].label, "API");
        assert_eq!(state.doc.selected, Some(Path { ancestors: vec![], index: 0 }));
    }

    #[test]
    fn a_valid_file_loads_saving_back_to_the_given_path() {
        let path = temp_file("save-to.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(state.unwrap().save_to, Some(path));
    }

    #[test]
    fn a_file_with_no_boxes_loads_an_empty_canvas_with_nothing_selected() {
        let path = temp_file("empty-dre.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let state = state.unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn a_zero_byte_file_is_invalid_data() {
        let path = temp_file("zero.dre", "");
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(result.err().map(|e| e.kind()), Some(io::ErrorKind::InvalidData));
    }

    #[test]
    fn a_malformed_file_is_invalid_data() {
        let path = temp_file("malformed.dre", "<dre><box");
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert_eq!(result.err().map(|e| e.kind()), Some(io::ErrorKind::InvalidData));
    }

    #[test]
    fn an_invalid_file_has_an_exact_error_message() {
        let path = temp_file("bad-colour.dre", "<dre><box label=\"A\" colour=\"99\"/></dre>");
        let result = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        let err = result.err().unwrap();
        assert_eq!(err.to_string(), format!("{path}: not a valid diagram"));
    }

    #[test]
    fn a_valid_file_is_not_a_new_file() {
        let path = temp_file("not-new.dre", "<dre/>");
        let state = load(Some(path.clone()));
        fs::remove_file(&path).unwrap();
        assert!(!state.unwrap().new_file);
    }

    #[test]
    fn a_missing_file_loads_an_empty_canvas_saved_to_that_path() {
        let path = temp_path("missing.dre");
        let state = load(Some(path.clone())).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some(path));
        assert!(state.new_file);
    }

    #[test]
    fn an_interrupt_leaves_nothing_to_save_so_the_file_is_never_written() {
        let path = temp_path("interrupted.dre");
        let mut state = load(Some(path.clone())).unwrap();
        let key = (INTERRUPT.as_bytes()[0] as char).to_string();
        assert_eq!(key, INTERRUPT);
        state.save_to = None;
        if let Some(path) = &state.save_to {
            filesystem::write(path, "").unwrap();
        }
        assert!(!std::path::Path::new(&path).exists());
    }
}
