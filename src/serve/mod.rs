mod diagram_dir;
mod host_key;
mod pty_process;
mod session;
mod sessions;
mod visitor;

use diagram_dir::DiagramDir;
use host_key::HostKey;
use pty_process::PtyProcess;
use russh::server::{Config, Server};
use russh::MethodKind;
use russh::MethodSet;
use std::io;
use std::process::ExitCode;
use std::sync::Arc;
use visitor::SshServer;

fn config(host_key: HostKey) -> Config {
    Config {
        methods: MethodSet::from(&[MethodKind::PublicKey, MethodKind::KeyboardInteractive][..]),
        keys: vec![host_key.into_key()],
        ..Config::default()
    }
}

pub(crate) fn run(listen: String, host_key: String, data_dir: String) -> io::Result<ExitCode> {
    let config = Arc::new(config(HostKey::load_or_generate(&host_key)?));
    let mut server = SshServer::new(Arc::new(PtyProcess::dre()?), DiagramDir { root: data_dir });
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(server.run_on_address(config, listen))?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_offers_publickey_and_keyboard_interactive_with_the_host_key() {
        let path = std::env::temp_dir().join(format!("dre-config-{}", std::process::id()));
        let path = path.to_str().unwrap();
        let config = config(HostKey::load_or_generate(path).unwrap());
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            &*config.methods,
            &[MethodKind::PublicKey, MethodKind::KeyboardInteractive]
        );
        assert_eq!(config.keys.len(), 1);
    }
}
