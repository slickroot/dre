use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;

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

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("dre-fs-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn reading_gives_back_what_was_written() {
        let path = temp_path("round-trip.dre");
        write(&path, "<dre/>").unwrap();
        let text = read(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(text.unwrap(), "<dre/>");
    }

    #[test]
    fn a_private_write_is_readable_only_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_path("private.key");
        write_private(&path, "secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        let text = read(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(text.unwrap(), "secret");
    }

    #[test]
    fn reading_a_missing_path_is_not_found() {
        let path = temp_path("absent.dre");
        assert_eq!(
            read(&path).err().map(|e| e.kind()),
            Some(io::ErrorKind::NotFound)
        );
    }

    #[test]
    fn missing_says_there_is_no_such_file() {
        assert_eq!(missing("x.dre").to_string(), "no such file: x.dre");
    }

    #[test]
    fn invalid_says_the_file_is_not_a_diagram() {
        assert_eq!(invalid("x.dre").to_string(), "x.dre: not a valid diagram");
    }
}
