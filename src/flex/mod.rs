use crate::tty::{probe, RawMode, Window};
use nix::unistd::read;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::process::ExitCode;

const BACKGROUND_RGBA: [u8; 4] = [0x0A, 0x0B, 0x0D, 0xFF];
const SQUARE_RGBA: [u8; 4] = [0x3F, 0x3F, 0x46, 0xFF];

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

    draw_background(&mut stdout, width, height)?;

    let mut squares = 0;

    loop {
        let mut byte = [0u8; 1];
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        read(borrowed, &mut byte).map_err(io::Error::from)?;
        match byte[0] {
            0x03 => return Ok(()),
            b'a' => {
                let (width, height, col, row) = layout_rect(squares, &window);
                draw_rect(
                    &mut stdout,
                    SQUARE_RGBA,
                    width,
                    height,
                    col,
                    row,
                    2 + squares,
                )?;
                squares += 1;
            }
            _ => {}
        }
    }
}

fn layout_rect(idx: i64, window: &Window) -> (i64, i64, i64, i64) {
    let width = 2 * window.cell_width;
    let height = window.cell_height;
    let col = 2 * idx;
    let row = 0;
    (width, height, col, row)
}

fn draw_background<W: Write>(stdout: &mut W, width: i64, height: i64) -> io::Result<()> {
    draw_rect(stdout, BACKGROUND_RGBA, width, height, 0, 0, 1)
}

fn draw_rect<W: Write>(
    stdout: &mut W,
    color: [u8; 4],
    width: i64,
    height: i64,
    col: i64,
    row: i64,
    id: i64,
) -> io::Result<()> {
    let pixels: Vec<u8> = color
        .iter()
        .copied()
        .cycle()
        .take((width * height * 4) as usize)
        .collect();
    stdout.write_all(kitty::draw(&pixels, width, height, col, row, id).as_bytes())?;
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_background_writes_draw() {
        let mut stdout = Vec::new();

        draw_background(&mut stdout, 1, 1).unwrap();

        let pixels: Vec<u8> = BACKGROUND_RGBA.iter().copied().cycle().take(4).collect();
        let expected = kitty::draw(&pixels, 1, 1, 0, 0, 1);

        assert_eq!(stdout, expected.as_bytes());
    }

    #[test]
    fn draw_rect_writes_draw() {
        let mut stdout = Vec::new();
        let color = [0x12, 0x34, 0x56, 0x78];

        draw_rect(&mut stdout, color, 2, 3, 5, 7, 9).unwrap();

        let pixels: Vec<u8> = color.iter().copied().cycle().take(2 * 3 * 4).collect();
        let expected = kitty::draw(&pixels, 2, 3, 5, 7, 9);

        assert_eq!(stdout, expected.as_bytes());
    }

    #[test]
    fn layout_rect_first_square_sits_at_top_left() {
        let window = Window {
            cols: 40,
            rows: 20,
            cell_width: 10,
            cell_height: 20,
        };

        assert_eq!(layout_rect(0, &window), (20, 20, 0, 0));
    }

    #[test]
    fn layout_rect_third_square_sits_to_the_right() {
        let window = Window {
            cols: 40,
            rows: 20,
            cell_width: 10,
            cell_height: 20,
        };

        assert_eq!(layout_rect(2, &window), (20, 20, 4, 0));
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

    fn send<W: Write>(stream: &mut W, bytes: &[u8]) -> io::Result<()> {
        stream.write_all(bytes)?;
        stream.flush()
    }

    pub(super) fn require<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<()> {
        send(stream, QUERY.as_bytes())?;

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
        send(stream, CLEAR_LINE.as_bytes())?;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            NOT_SUPPORTED_MESSAGE,
        ))
    }

    fn is_supported(reply: &[u8]) -> bool {
        reply.windows(3).any(|window| window == b"i=1")
    }

    pub(super) fn draw(
        pixels: &[u8],
        width: i64,
        height: i64,
        col: i64,
        row: i64,
        id: i64,
    ) -> String {
        let transmit = chunked(
            &format!("a=t,f=32,s={width},v={height},o=z,q=2,i={id}"),
            pixels,
        );
        let place = format!(
            "\x1b[{};{}H\x1b_Ga=p,i={id},p={id},q=2,z={id};\x1b\\",
            row + 1,
            col + 1
        );
        transmit + &place
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
