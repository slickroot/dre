use crate::filesystem;
use std::io;
use std::path::Path;

const DIAGRAM_FILE: &str = "diagram.dre";

#[derive(Clone)]
pub(crate) struct DiagramDir {
    pub(crate) root: String,
}

impl DiagramDir {
    pub(crate) fn for_key(&self, fingerprint: &str) -> io::Result<String> {
        let directory = Path::new(&self.root).join(fingerprint);
        filesystem::create_private_dir(&directory.to_string_lossy())?;
        Ok(directory.join(DIAGRAM_FILE).to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn diagram_dir(name: &str) -> DiagramDir {
        DiagramDir {
            root: std::env::temp_dir()
                .join(format!("dre-diagram-dir-{}-{name}", std::process::id()))
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn cleanup(dir: &DiagramDir) {
        std::fs::remove_dir_all(&dir.root).unwrap();
    }

    #[test]
    fn a_key_gets_a_directory_with_mode_0700() {
        let dir = diagram_dir("mode");
        let path = dir.for_key("SHA256:abc").unwrap();
        let directory = Path::new(&path).parent().unwrap();
        let mode = std::fs::metadata(directory).unwrap().permissions().mode();
        cleanup(&dir);
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn the_path_ends_with_the_diagram_file_name() {
        let dir = diagram_dir("name");
        let path = dir.for_key("SHA256:abc").unwrap();
        cleanup(&dir);
        assert!(path.ends_with(&format!("SHA256:abc/{DIAGRAM_FILE}")));
    }

    #[test]
    fn two_keys_get_different_directories() {
        let dir = diagram_dir("two");
        let first = dir.for_key("SHA256:one").unwrap();
        let second = dir.for_key("SHA256:two").unwrap();
        cleanup(&dir);
        assert_ne!(first, second);
    }

    #[test]
    fn asking_again_gives_the_same_path() {
        let dir = diagram_dir("again");
        let first = dir.for_key("SHA256:abc").unwrap();
        let second = dir.for_key("SHA256:abc").unwrap();
        cleanup(&dir);
        assert_eq!(first, second);
    }
}
