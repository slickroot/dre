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

fn main() -> ExitCode {
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
