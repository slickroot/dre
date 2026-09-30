use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

pub(crate) fn read(path: &str) -> io::Result<String> {
    fs::read_to_string(path)
}

pub(crate) fn write(path: &str, contents: &str) -> io::Result<()> {
    fs::write(path, contents)
}

pub(crate) fn write_private(path: &str, contents: &str) -> io::Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?
        .write_all(contents.as_bytes())
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn create_private_dir(path: &str) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

pub(crate) fn missing(path: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("no such file: {path}"))
}

pub(crate) fn invalid(path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{path}: not a valid diagram"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_says_there_is_no_such_file() {
        assert_eq!(missing("x.dre").to_string(), "no such file: x.dre");
    }

    #[test]
    fn invalid_says_the_file_is_not_a_diagram() {
        assert_eq!(invalid("x.dre").to_string(), "x.dre: not a valid diagram");
    }
}
