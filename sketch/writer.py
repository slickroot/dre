import fcntl
import os
import struct
import sys
import termios
import tty
from contextlib import contextmanager
from typing import Iterator, List, TextIO, Tuple

from .kitty import KittyGraphics
from .layout import layout, with_cursor
from .render import GraphicsRenderer, Renderer, TerminalRenderer
from .state import State, handle_key

ENTER_ALTERNATE_SCREEN = "\x1b[?1049h"
LEAVE_ALTERNATE_SCREEN = "\x1b[?1049l"
HIDE_CURSOR = "\x1b[?25l"
SHOW_CURSOR = "\x1b[?25h"
HOME_CURSOR = "\x1b[H"
INTERRUPT = "\x03"
WINSIZE = "HHHH"
KITTY_GRAPHICS_QUERY = "\x1b_Gi=1,a=q;\x1b\\"
NOT_SUPPORTED_MESSAGE = (
    "sketch requires a terminal with Kitty graphics protocol support."
)


@contextmanager
def terminal_session(stream: TextIO, stdin: TextIO) -> Iterator[None]:
    saved = termios.tcgetattr(stdin)
    stream.write(ENTER_ALTERNATE_SCREEN)
    stream.write(HIDE_CURSOR)
    stream.flush()
    tty.setraw(stdin)
    try:
        yield
    finally:
        termios.tcsetattr(stdin, termios.TCSADRAIN, saved)
        stream.write(SHOW_CURSOR)
        stream.write(LEAVE_ALTERNATE_SCREEN)
        stream.flush()


def supports_kitty_graphics(stream: TextIO, stdin: TextIO) -> bool:
    saved = termios.tcgetattr(stdin)
    stream.write(KITTY_GRAPHICS_QUERY)
    stream.flush()
    tty.setraw(stdin)
    try:
        reply = stdin.read(32)
    finally:
        termios.tcsetattr(stdin, termios.TCSADRAIN, saved)
    return "i=1" in reply


def cell_size() -> Tuple[int, int]:
    packed = fcntl.ioctl(
        sys.stdout, termios.TIOCGWINSZ, struct.pack(WINSIZE, 0, 0, 0, 0)
    )
    rows, cols, xpixel, ypixel = struct.unpack(WINSIZE, packed)
    return round(xpixel / cols), round(ypixel / rows)


def paint(stream: TextIO, lines: List[str]) -> None:
    stream.write(HOME_CURSOR)
    stream.write("\r\n".join(lines))
    stream.flush()


def frame(state: State, renderer: Renderer, stream: TextIO) -> None:
    cols, rows = os.get_terminal_size()
    placements = with_cursor(layout(state.boxes, cols, rows), state.selected)
    paint(stream, renderer.render(placements, cols, rows))


def run(stream: TextIO, stdin: TextIO) -> None:
    renderer = GraphicsRenderer(
        TerminalRenderer(), KittyGraphics(), *cell_size()
    )
    state = State()
    with terminal_session(stream, stdin):
        while state.running:
            frame(state, renderer, stream)
            key = stdin.read(1)
            if key == INTERRUPT:
                return
            state = handle_key(state, key)


def main() -> None:
    if not supports_kitty_graphics(sys.stdout, sys.stdin):
        print(NOT_SUPPORTED_MESSAGE)
        sys.exit(1)
        return
    run(sys.stdout, sys.stdin)
