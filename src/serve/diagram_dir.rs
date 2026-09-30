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
