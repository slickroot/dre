use std::io;

use super::store::Files;
use crate::filesystem;

pub(crate) struct DiskFiles;

impl Files for DiskFiles {
    fn read(&self, path: &str) -> io::Result<String> {
        filesystem::read(path)
    }

    fn write(&self, path: &str, contents: &str) -> io::Result<()> {
        filesystem::write(path, contents)
    }
}
