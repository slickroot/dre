import fcntl
import io
import os
import select
import struct
import sys
import termios
import tty
import unittest
from contextlib import contextmanager, redirect_stdout

from dre import writer
from dre.kitty import KittyGraphics
from dre.render import TerminalRenderer
from dre.writer import (
    HOME_CURSOR,
    INTERRUPT,
    WINSIZE,
    cell_size,
    frame,
    main,
    paint,
    supports_kitty_graphics,
)


class Stream:
    def __init__(self):
        self.written = []
        self.flushes = 0

    def write(self, text):
        self.written.append(text)

    def flush(self):
        self.flushes += 1

    def text(self):
        return "".join(self.written)


class StubRenderer:
    def __init__(self, lines):
        self.lines = lines
        self.sizes = []

    def render(self, placements, cols, rows):
        self.sizes.append((cols, rows))
        return self.lines


class PaintTest(unittest.TestCase):
    def test_the_cursor_goes_home_before_the_lines(self):
        stream = Stream()
        paint(stream, ["ab", "cd"])
        self.assertEqual(stream.text(), HOME_CURSOR + "ab\r\ncd")

    def test_no_newline_follows_the_last_line(self):
        stream = Stream()
        paint(stream, ["ab", "cd"])
        self.assertFalse(stream.text().endswith("\n"))

    def test_the_stream_is_flushed(self):
        stream = Stream()
        paint(stream, ["ab"])
        self.assertEqual(stream.flushes, 1)


class FrameTest(unittest.TestCase):
    def setUp(self):
        self.terminal_size = os.get_terminal_size
        os.get_terminal_size = lambda: os.terminal_size((9, 4))

    def tearDown(self):
        os.get_terminal_size = self.terminal_size

    def test_the_renderer_is_given_the_terminal_size(self):
        renderer = StubRenderer(["x"])
        frame(writer.State([]), renderer, Stream())
        self.assertEqual(renderer.sizes, [(9, 4)])

    def test_what_the_renderer_returned_is_painted(self):
        stream = Stream()
        frame(writer.State([]), StubRenderer(["ab", "cd"]), stream)
        self.assertEqual(stream.text(), HOME_CURSOR + "ab\r\ncd")


class WindowSize:
    def __init__(self, rows, cols, xpixel, ypixel):
        self.packed = struct.pack(WINSIZE, rows, cols, xpixel, ypixel)
        self.ioctl = fcntl.ioctl

    def __enter__(self):
        fcntl.ioctl = lambda *arguments: self.packed
        return self

    def __exit__(self, *details):
        fcntl.ioctl = self.ioctl


class CellSizeTest(unittest.TestCase):
    def test_the_pixel_size_is_divided_by_the_terminal_size(self):
        with WindowSize(rows=20, cols=100, xpixel=800, ypixel=400):
            self.assertEqual(cell_size(), (8, 20))

    def test_a_partial_cell_is_rounded_to_the_nearest_pixel(self):
        with WindowSize(rows=10, cols=10, xpixel=96, ypixel=104):
            self.assertEqual(cell_size(), (10, 10))


class Stdin:
    def __init__(self, keys):
        self.keys = list(keys)

    def read(self, count):
        return self.keys.pop(0)


class RunTest(unittest.TestCase):
    def setUp(self):
        self.frame = writer.frame
        self.terminal_session = writer.terminal_session
        self.renderers = []
        writer.frame = lambda state, renderer, stream: self.renderers.append(
            renderer
        )
        writer.terminal_session = contextmanager(
            lambda stream, stdin: iter([None])
        )

    def tearDown(self):
        writer.frame = self.frame
        writer.terminal_session = self.terminal_session

    def renderer(self):
        with WindowSize(rows=25, cols=80, xpixel=640, ypixel=800):
            writer.run(Stream(), Stdin([INTERRUPT]))
        return self.renderers[0]

    def test_the_renderer_is_a_terminal_renderer(self):
        renderer = self.renderer()
        self.assertIsInstance(renderer, TerminalRenderer)

    def test_the_graphics_are_drawn_by_kitty(self):
        self.assertIsInstance(self.renderer().graphics, KittyGraphics)

    def test_the_renderer_carries_the_cell_size(self):
        renderer = self.renderer()
        self.assertEqual((renderer.cell_width, renderer.cell_height), (8, 32))


class Termios:
    def __init__(self):
        self.saved = object()
        self.tcgetattr = termios.tcgetattr
        self.tcsetattr = termios.tcsetattr
        self.setraw = tty.setraw
        self.raw = []
        self.restored = []

    def __enter__(self):
        termios.tcgetattr = lambda stdin: self.saved
        termios.tcsetattr = lambda stdin, when, settings: self.restored.append(
            settings
        )
        tty.setraw = lambda stdin: self.raw.append(stdin)
        return self

    def __exit__(self, *details):
        termios.tcgetattr = self.tcgetattr
        termios.tcsetattr = self.tcsetattr
        tty.setraw = self.setraw


class RawStdin:
    def __init__(self, reply):
        self.reply = reply.encode()
        self.reads = []
        self.os_read = os.read
        self.select = select.select

    def fileno(self):
        return id(self)

    def __enter__(self):
        os.read = self._read
        select.select = self._select
        return self

    def __exit__(self, *details):
        os.read = self.os_read
        select.select = self.select

    def _select(self, readers, writers, errors, timeout):
        return ((readers if self.reply else []), [], [])

    def _read(self, fd, count):
        assert fd == self.fileno()
        self.reads.append(count)
        return self.reply


class SupportsKittyGraphicsTest(unittest.TestCase):
    def test_the_query_is_written_to_the_stream(self):
        stream = Stream()
        with Termios(), RawStdin("i=1") as stdin:
            supports_kitty_graphics(stream, stdin)
        self.assertEqual(stream.text(), "\x1b_Gi=1,a=q;\x1b\\")

    def test_stdin_is_put_into_raw_mode(self):
        with Termios() as fake, RawStdin("i=1") as stdin:
            supports_kitty_graphics(Stream(), stdin)
        self.assertEqual(fake.raw, [stdin])

    def test_stdin_is_read_exactly_once(self):
        with Termios(), RawStdin("i=1") as stdin:
            supports_kitty_graphics(Stream(), stdin)
        self.assertEqual(len(stdin.reads), 1)

    def test_the_original_termios_settings_are_restored(self):
        with Termios() as fake, RawStdin("i=1") as stdin:
            supports_kitty_graphics(Stream(), stdin)
        self.assertEqual(fake.restored, [fake.saved])

    def test_a_reply_containing_i_1_is_supported(self):
        with Termios(), RawStdin("\x1b_Gi=1;OK\x1b\\") as stdin:
            result = supports_kitty_graphics(Stream(), stdin)
        self.assertTrue(result)

    def test_no_reply_is_not_supported(self):
        with Termios(), RawStdin("") as stdin:
            result = supports_kitty_graphics(Stream(), stdin)
        self.assertFalse(result)

    def test_stdin_is_not_read_when_nothing_arrives(self):
        with Termios(), RawStdin("") as stdin:
            supports_kitty_graphics(Stream(), stdin)
        self.assertEqual(stdin.reads, [])

    def test_a_reply_without_i_1_is_not_supported(self):
        with Termios(), RawStdin("garbage") as stdin:
            result = supports_kitty_graphics(Stream(), stdin)
        self.assertFalse(result)


class MainTest(unittest.TestCase):
    def setUp(self):
        self.supports_kitty_graphics = writer.supports_kitty_graphics
        self.run = writer.run
        self.runs = []
        writer.run = lambda stream, stdin: self.runs.append((stream, stdin))

    def tearDown(self):
        writer.supports_kitty_graphics = self.supports_kitty_graphics
        writer.run = self.run

    def test_run_is_called_with_graphics_support(self):
        writer.supports_kitty_graphics = lambda stream, stdin: True
        main()
        self.assertEqual(self.runs, [(sys.stdout, sys.stdin)])

    def test_run_is_not_called_without_graphics_support(self):
        writer.supports_kitty_graphics = lambda stream, stdin: False
        with self.assertRaises(SystemExit):
            main()
        self.assertEqual(self.runs, [])

    def test_the_message_is_printed_without_graphics_support(self):
        writer.supports_kitty_graphics = lambda stream, stdin: False
        output = io.StringIO()
        with redirect_stdout(output), self.assertRaises(SystemExit):
            main()
        self.assertIn(
            "Dre requires a terminal with Kitty graphics protocol support.",
            output.getvalue(),
        )

    def test_the_line_is_cleared_before_the_message(self):
        writer.supports_kitty_graphics = lambda stream, stdin: False
        output = io.StringIO()
        with redirect_stdout(output), self.assertRaises(SystemExit):
            main()
        self.assertTrue(output.getvalue().startswith(writer.CLEAR_LINE))

    def test_exit_1_without_graphics_support(self):
        writer.supports_kitty_graphics = lambda stream, stdin: False
        with self.assertRaises(SystemExit) as raised:
            main()
        self.assertEqual(raised.exception.code, 1)
