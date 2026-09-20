use std::fs;
use std::io;

pub(crate) fn read(path: &str) -> io::Result<String> {
    fs::read_to_string(path)
}

pub(crate) fn write(path: &str, contents: &str) -> io::Result<()> {
    fs::write(path, contents)
}

pub(crate) fn missing(path: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("no such file: {path}"))
}

pub(crate) fn invalid(path: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{path}: not a valid diagram"))
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
    fn reading_a_missing_path_is_not_found() {
        let path = temp_path("absent.dre");
        assert_eq!(read(&path).err().map(|e| e.kind()), Some(io::ErrorKind::NotFound));
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
