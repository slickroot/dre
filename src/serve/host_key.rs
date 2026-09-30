use crate::filesystem;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use std::io;

pub(crate) struct HostKey(PrivateKey);

impl HostKey {
    pub(crate) fn load_or_generate(path: &str) -> io::Result<Self> {
        match filesystem::read(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::generate(path),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn into_key(self) -> PrivateKey {
        self.0
    }

    fn parse(text: &str) -> io::Result<Self> {
        PrivateKey::from_openssh(text)
            .map(Self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn generate(path: &str) -> io::Result<Self> {
        let key =
            PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).map_err(io::Error::other)?;
        let text = key.to_openssh(LineEnding::LF).map_err(io::Error::other)?;
        filesystem::write_private(path, &text)?;
        Ok(Self(key))
    }
}
