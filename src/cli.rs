use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::diagram;
use crate::dre_format;
use crate::filesystem;
use crate::render::{Renderer, SvgRenderer};
use crate::state::State;
use crate::view;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Edit(Option<String>),
    Export { input: String },
    Serve { listen: String, host_key: String },
}

const DEFAULT_LISTEN: &str = "0.0.0.0:2222";
const DEFAULT_HOST_KEY_UNDER_HOME: &str = ".local/share/dre/host_key";

fn default_host_key() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/{DEFAULT_HOST_KEY_UNDER_HOME}")
}

fn parse_serve<I: Iterator<Item = String>>(mut args: I) -> Command {
    let mut listen = DEFAULT_LISTEN.to_string();
    let mut host_key = default_host_key();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--listen" => listen = args.next().unwrap_or(listen),
            "--host-key" => host_key = args.next().unwrap_or(host_key),
            _ => {}
        }
    }
    Command::Serve { listen, host_key }
}

pub(crate) fn parse_args() -> Command {
    parse_args_from(std::env::args().skip(1))
}

fn parse_args_from<I: Iterator<Item = String>>(mut args: I) -> Command {
    match args.next() {
        Some(arg) if arg == "--svg" => Command::Export {
            input: args.next().unwrap_or_default(),
        },
        Some(arg) if arg == "serve" => parse_serve(args),
        Some(arg) => Command::Edit(Some(arg)),
        None => Command::Edit(None),
    }
}

fn output_path(input: &str) -> PathBuf {
    Path::new(input).with_extension("svg")
}

pub(crate) fn export(input: String) -> io::Result<ExitCode> {
    let text = match filesystem::read(&input) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(filesystem::missing(&input)),
        Err(e) => return Err(e),
    };
    let doc = dre_format::read(&text).ok_or_else(|| filesystem::invalid(&input))?;
    let doc = diagram::to_document(doc);
    let state = State::open(doc, None);
    let window = view::Area {
        col: 0,
        row: 0,
        cols: crate::render::FULL_HD_WIDTH / crate::style::CELL_WIDTH,
        rows: crate::render::FULL_HD_HEIGHT / crate::style::CELL_HEIGHT,
    };
    SvgRenderer::default().render(
        &view::body(&state, window),
        &mut File::create(output_path(&input))?,
    )?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Command {
        parse_args_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_arguments_yields_edit_without_a_file() {
        assert_eq!(parse(&[]), Command::Edit(None));
    }

    #[test]
    fn a_plain_argument_yields_edit_for_that_file() {
        assert_eq!(parse(&["x.dre"]), Command::Edit(Some("x.dre".to_string())));
    }

    #[test]
    fn an_svg_flag_yields_export_with_the_input() {
        assert_eq!(
            parse(&["--svg", "diagram.dre"]),
            Command::Export {
                input: "diagram.dre".to_string()
            }
        );
    }

    #[test]
    fn serve_without_flags_applies_the_defaults() {
        assert_eq!(
            parse(&["serve"]),
            Command::Serve {
                listen: DEFAULT_LISTEN.to_string(),
                host_key: default_host_key()
            }
        );
    }

    #[test]
    fn the_default_host_key_lives_under_home() {
        assert!(default_host_key().ends_with(DEFAULT_HOST_KEY_UNDER_HOME));
    }

    #[test]
    fn serve_with_listen_overrides_only_the_address() {
        assert_eq!(
            parse(&["serve", "--listen", "127.0.0.1:9000"]),
            Command::Serve {
                listen: "127.0.0.1:9000".to_string(),
                host_key: default_host_key()
            }
        );
    }

    #[test]
    fn serve_with_host_key_overrides_only_the_key_path() {
        assert_eq!(
            parse(&["serve", "--host-key", "./tmp/host_key"]),
            Command::Serve {
                listen: DEFAULT_LISTEN.to_string(),
                host_key: "./tmp/host_key".to_string()
            }
        );
    }

    #[test]
    fn serve_accepts_both_flags_in_either_order() {
        assert_eq!(
            parse(&["serve", "--host-key", "k", "--listen", "l"]),
            Command::Serve {
                listen: "l".to_string(),
                host_key: "k".to_string()
            }
        );
    }

    #[test]
    fn output_path_swaps_the_dre_extension_for_svg() {
        assert_eq!(output_path("x.dre"), PathBuf::from("x.svg"));
    }

    #[test]
    fn output_path_appends_svg_to_an_extensionless_path() {
        assert_eq!(output_path("report"), PathBuf::from("report.svg"));
    }

    fn temp_file(name: &str, contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("dre-cli-{}-{name}", std::process::id()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn export_writes_an_svg_document_beside_the_input() {
        let input = temp_file("valid.dre", "<dre><box label=\"API\"/></dre>");
        let output = std::path::Path::new(&input).with_extension("svg");
        let _ = fs::remove_file(&output);
        let result = export(input.clone());
        let svg = fs::read_to_string(&output).expect("the output svg exists");
        fs::remove_file(&input).unwrap();
        fs::remove_file(&output).unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
        assert!(!svg.contains("• dre"));
    }

    #[test]
    fn a_missing_file_reports_no_such_file() {
        let path = std::env::temp_dir()
            .join(format!("dre-cli-{}-missing.dre", std::process::id()))
            .to_string_lossy()
            .into_owned();
        let err = export(path.clone()).err().unwrap();
        assert_eq!(err.to_string(), format!("no such file: {path}"));
    }

    #[test]
    fn a_corrupt_file_reports_not_a_valid_diagram() {
        let path = temp_file("corrupt.dre", "this is not xml");
        let err = export(path.clone()).err();
        fs::remove_file(&path).unwrap();
        assert_eq!(
            err.unwrap().to_string(),
            format!("{path}: not a valid diagram")
        );
    }
}
