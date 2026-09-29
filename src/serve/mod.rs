use std::io;
use std::process::ExitCode;

pub(crate) fn run(_listen: String, _host_key: String) -> io::Result<ExitCode> {
    Ok(ExitCode::SUCCESS)
}
