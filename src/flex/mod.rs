use crate::tty::{probe, RawMode};
use nix::unistd::read;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::process::ExitCode;

const BACKGROUND_RGBA: [u8; 4] = [10, 11, 13, 255];

pub fn run() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> io::Result<()> {
    let fd = io::stdin().as_raw_fd();
    let mut stdout = io::stdout();
    kitty::require(&mut stdout, fd)?;

    let window = probe()?;
    let width = window.cols * window.cell_width;
    let height = window.rows * window.cell_height;

    let _raw = RawMode::enter(fd)?;

    let pixels: Vec<u8> = BACKGROUND_RGBA
        .iter()
        .copied()
        .cycle()
        .take((width * height * 4) as usize)
        .collect();

    stdout.write_all(kitty::transmit(&pixels, width, height).as_bytes())?;
    stdout.write_all(kitty::place(0, 0).as_bytes())?;
    stdout.flush()?;

    loop {
        let mut byte = [0u8; 1];
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        read(borrowed, &mut byte).map_err(io::Error::from)?;
        if byte[0] == 0x03 {
            return Ok(());
        }
    }
}

mod kitty {
    use base64::Engine as _;
    use nix::sys::select::{select, FdSet};
    use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg, Termios};
    use nix::sys::time::{TimeVal, TimeValLike};
    use nix::unistd::read;
    use std::io::{self, Write};
    use std::os::fd::{BorrowedFd, RawFd};

    const QUERY: &str = "\x1b_Gi=1,a=q;\x1b\\";
    const CLEAR_LINE: &str = "\r\x1b[K";
    const NOT_SUPPORTED_MESSAGE: &str =
        "Dre requires a terminal with Kitty graphics protocol support.";
    const REPLY_TIMEOUT_MICROS: i64 = 500_000;
    const CHUNK_SIZE: usize = 4096;

    struct RawMode {
        fd: RawFd,
        saved: Termios,
    }

    impl RawMode {
        fn enter(fd: RawFd) -> io::Result<Self> {
            let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
            let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
            let mut raw = saved.clone();
            cfmakeraw(&mut raw);
            tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;
            Ok(RawMode { fd, saved })
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            let borrowed = unsafe { BorrowedFd::borrow_raw(self.fd) };
            let _ = tcsetattr(borrowed, SetArg::TCSADRAIN, &self.saved);
        }
    }

    pub(super) fn require<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<()> {
        stream.write_all(QUERY.as_bytes())?;
        stream.flush()?;

        let supported = {
            let raw = RawMode::enter(stdin_fd)?;
            let borrowed = unsafe { BorrowedFd::borrow_raw(raw.fd) };
            let mut fds = FdSet::new();
            fds.insert(borrowed);
            let mut timeout = TimeVal::microseconds(REPLY_TIMEOUT_MICROS);
            let ready = select(None, Some(&mut fds), None, None, Some(&mut timeout))
                .map_err(io::Error::from)?;
            let mut buffer = [0u8; 32];
            let reply: Vec<u8> = if ready > 0 && fds.contains(borrowed) {
                let count = read(borrowed, &mut buffer).map_err(io::Error::from)?;
                buffer[..count].to_vec()
            } else {
                Vec::new()
            };
            is_supported(&reply)
        };

        if supported {
            return Ok(());
        }
        stream.write_all(CLEAR_LINE.as_bytes())?;
        stream.flush()?;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            NOT_SUPPORTED_MESSAGE,
        ))
    }

    fn is_supported(reply: &[u8]) -> bool {
        reply.windows(3).any(|window| window == b"i=1")
    }

    pub(super) fn transmit(pixels: &[u8], width: i64, height: i64) -> String {
        chunked(
            &format!("a=t,f=32,s={width},v={height},o=z,q=2,i=1"),
            pixels,
        )
    }

    pub(super) fn place(col: i64, row: i64) -> String {
        format!(
            "\x1b[{};{}H\x1b_Ga=p,i=1,p=1,q=2,z=0;\x1b\\",
            row + 1,
            col + 1
        )
    }

    fn chunked(keys: &str, pixels: &[u8]) -> String {
        let payload = encode(pixels);
        let chunk_list = chunks(&payload, CHUNK_SIZE);
        let header = format!("{keys},m={}", more(&chunk_list, 0));
        let mut escapes = vec![escape(&header, &chunk_list[0])];
        for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
            let keys = format!("m={}", more(&chunk_list, index));
            escapes.push(escape(&keys, chunk));
        }
        escapes.join("")
    }

    fn encode(pixels: &[u8]) -> String {
        base64(zlib(pixels))
    }

    fn zlib(pixels: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, pixels)
            .and_then(|_| encoder.finish())
            .expect("writes to an in-memory Vec<u8> can't fail")
    }

    fn base64(bytes: Vec<u8>) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
        payload
            .as_bytes()
            .chunks(chunk_size)
            .map(|chunk| {
                std::str::from_utf8(chunk)
                    .expect(
                        "base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8",
                    )
                    .to_owned()
            })
            .collect()
    }

    fn more(chunks: &[String], index: usize) -> i32 {
        if index == chunks.len() - 1 {
            0
        } else {
            1
        }
    }

    fn escape(keys: &str, payload: &str) -> String {
        format!("\x1b_G{keys};{payload}\x1b\\")
    }
}
