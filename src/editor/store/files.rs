use std::io;

use crate::filesystem;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Files {
    fn read(&self, path: &str) -> io::Result<String>;
    fn write(&self, path: &str, contents: &str) -> io::Result<()>;
}

pub(crate) struct DiskFiles;

impl Files for DiskFiles {
    fn read(&self, path: &str) -> io::Result<String> {
        filesystem::read(path)
    }

    fn write(&self, path: &str, contents: &str) -> io::Result<()> {
        filesystem::write(path, contents)
    }
}
