use std::io;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Window {
    pub(crate) cols: u32,
    pub(crate) rows: u32,
    pub(crate) pixel_width: u32,
    pub(crate) pixel_height: u32,
}

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Spawner {
    fn spawn(&self, window: Window) -> io::Result<Box<dyn Session>>;
}

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Session: Send + Sync {
    fn write(&self, bytes: &[u8]);
    fn read(&self) -> Option<Vec<u8>>;
    fn resize(&self, window: Window);
    #[allow(dead_code)]
    fn wait(&self);
}
