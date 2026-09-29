mod host_key;
#[allow(dead_code)]
mod pty_process;
#[allow(dead_code)]
mod session;
#[allow(dead_code)]
mod visitor;

use host_key::HostKey;

use std::io;
use std::process::ExitCode;

pub(crate) fn run(_listen: String, host_key: String) -> io::Result<ExitCode> {
    HostKey::load_or_generate(&host_key)?.into_key();
    Ok(ExitCode::SUCCESS)
}
