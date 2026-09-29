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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("dre-host-key-{}-{name}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn a_missing_key_is_generated_with_mode_0600() {
        let path = temp_path("generated");
        let key = HostKey::load_or_generate(&path);
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(key.unwrap().into_key().algorithm(), Algorithm::Ed25519);
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn an_existing_key_is_loaded_unchanged() {
        let path = temp_path("reloaded");
        let first = HostKey::load_or_generate(&path);
        let second = HostKey::load_or_generate(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(first.unwrap().into_key(), second.unwrap().into_key());
    }
}
