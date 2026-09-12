import fcntl
import os
import struct
import unittest
from contextlib import contextmanager

from sketch import writer
from sketch.kitty import KittyGraphics
from sketch.render import GraphicsRenderer, TerminalRenderer
from sketch.writer import (
    HOME_CURSOR,
    INTERRUPT,
    WINSIZE,
    cell_size,
    frame,
    paint,
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

    def test_the_text_renderer_is_wrapped_in_a_graphics_renderer(self):
        renderer = self.renderer()
        self.assertIsInstance(renderer, GraphicsRenderer)
        self.assertIsInstance(renderer.text, TerminalRenderer)

    def test_the_graphics_are_drawn_by_kitty(self):
        self.assertIsInstance(self.renderer().graphics, KittyGraphics)

    def test_the_renderer_carries_the_cell_size(self):
        renderer = self.renderer()
        self.assertEqual((renderer.cell_width, renderer.cell_height), (8, 32))
