# Nadia's new box grows into place

## User Story

Nadia opens dre-flex and is in move mode. She presses `a` to add a box below. Instead of popping in, the new box unrolls from its top-left corner down and to the right until it reaches full size. Pleased with how smooth that felt, she carries on drawing!

## Acceptance Criteria

- In move mode, pressing `a` makes the new box grow from nothing to its full size.
- The box grows from its top-left corner, down and to the right.
- The grow takes 150 ms.
- The grow eases out: a fast start and a gentle landing.
- Inner boxes added with `A` grow the same way.
- The other boxes jump straight to their new places. Only the new box grows.
- A box that existed before doesn't grow again when the screen is redrawn.

## Technical Design
All changes are in `src/flex/mod.rs`, `src/flex/view.rs`, `src/view.rs`, `src/render/terminal.rs` and `src/kitty.rs`. **The terminal plays the grow, not dre.** When a box is new, dre sends it as one image with animation frames, and the terminal plays them by itself. There are no threads, timers or clocks, and the run loop is unchanged. `reduce` and `FlexState` are unchanged too: as far as the state knows, a box was simply added.

The grow is a one-shot. The render right after the box appears sends the animation. Every later render draws that box with normal tiles, which look the same as the last frame. A key pressed during the 150 ms snaps the box to full size, which is acceptable.

### Screen (`flex/mod.rs`)

- **`TerminalFlexScreen` remembers the box paths it drew last time** (`drawn: Option<HashSet<Vec<usize>>>`, `None` before the first render).
- On `render`, it collects the box paths in `state.boxes`. The new ones are those that are not in `drawn`. It passes them to `view::scene`, then stores all the paths in `drawn`.
- **First render:** `drawn` is `None`, so nothing grows.
- This works because `Tree::push` always appends, so a path that was already drawn still means the same box. That covers `a` (outer boxes) and `A` (inner boxes). Undo followed by `a` gives a path that wasn't drawn last time, so that box grows again.
- A resize doesn't change the boxes, so no box is new and nothing grows.
- **`start` tells the renderer whether it runs in WezTerm:** it sets `renderer.wezterm` when the `TERM_PROGRAM` environment variable is `WezTerm`.

### View (`view.rs`, `flex/view.rs`)

- **`PlacementNode::Box` gains `grow: bool`.** Every existing constructor passes `false`. That includes dre's main editor (`layout/tree.rs`, `view.rs`), `svg.rs`, the tests and the other flex boxes.
- **`scene(state, window, new: &HashSet<Vec<usize>>)`.** `paint` sets `grow: true` when the box's path is in `new`.
- The SVG renderer ignores `grow`.

### Renderer (`render/terminal.rs`)

- **`draw_box` for a `grow: true` box** sends a grow image instead of tiles, but only when the whole box fits in its area (`crop` cuts nothing off). Otherwise it falls back to tiles and the box pops in.
- **The grow image uses a transient id** (`allocate_transient`). The next frame already deletes transient images, so on the next render the grow image is removed and the box is drawn with tiles. That's how the one-shot works, with no tracking.
- **Frames are shrunk boxes:** `grow_frames(width_px, height_px, style) -> Vec<Canvas>`. All frames are full-size transparent canvases. Frame `k` of `GROW_FRAMES = 9` draws a complete outline with `box_shape` at `eased(k / 9)` of the full width and height, anchored at the top-left. The last frame is exactly the full-size box.
- **Ease-out cubic:** `eased(t) = 1 - (1 - t)^3`, in a pure function.
- **Timing:** `GROW_MS = 150`. Each growing frame's gap is `GROW_MS / (GROW_FRAMES - 1)`, rounded, which is 19 ms. The last frame gets `GROW_HOLD_MS = 2_000_000_000`, about 23 days, so it stays on screen.
- **The root frame is the full-size box,** the same as the last frame. A terminal that doesn't play the frames shows the root, so the box pops in instead of staying invisible until the next render. Terminals that play the frames skip it, because the root is gapless.
- **The root frame is unique per grow.** `TerminalRenderer` keeps a `grow_stamp: u64` counter, seeded at startup from the system time so that runs in the same terminal window differ. The 48 low bits of the stamp are written into the lowest bit of the R, G and B bytes of the root's first 16 pixels. A change of 1 in a colour channel can't be seen, and this works whether the box is filled or not.
- **`TerminalRenderer` gains `wezterm: bool`,** `false` from `new`. It is passed to `kitty::grow`.
- `Image` gains a `Growing { id, root: Canvas, frames: Vec<Canvas> }` variant. Its `command` uses `kitty::grow`.

### Kitty (`kitty.rs`)

- **`grow(root, frames, id, col, row, z, gap_ms, hold_ms, wezterm) -> Command`**: moves the cursor, transmits and places the root (`a=T`, as `transmission` does), then sends one `a=f` per frame. Each frame is `f=32`, `o=z`, chunked like `transmission`, and has its gap in `z=`. Only when `wezterm` is true does each frame also get `Z=` with the same value. Finally it sends `a=a,s=3,v=2` so that kitty plays the frames once.

### Terminal quirks

Write these down in comments where they apply, because they aren't obvious:

1. **WezTerm reads a frame's gap from `Z`, kitty from `z`.** That's `wezterm-escape-parser/src/apc.rs`, `KittyImageFrame::from_keys`. Without `Z`, WezTerm uses 40 ms for every frame.
2. **WezTerm ignores `a=a`, so it loops.** That's why the last frame has a gap of about 23 days instead of relying on `v=2`.
3. **WezTerm keys its playback state by the hash of the root frame.** The hash is computed once at `a=T` (`ImageData::with_data`, `wezterm-cell/src/image.rs`) and the GUI caches the animation state under it (`wezterm-gui/src/glyphcache.rs`, `cached_image`). Two grows with the same root share one state, and the second grow shows up already at full size. Hence the unique stamp in the root's lowest colour bits.
4. **WezTerm skips a root frame with a gap of 0,** so the root never shows during the grow. Ghostty creates the root gapless too (`controlAnimation`, `src/terminal/kitty/graphics_exec.zig`).
5. **Kitty rejects a command with a key it doesn't know, and `Z` is one of them.** It reports `invalid key character` and drops the whole command (`kitty/parse-graphics-command.h`), and `q=2` hides the error. That's why `Z` is only sent to WezTerm.
6. **Some terminals don't play animation frames.** Ghostty only gained them in August 2026. Those terminals show only the root, which is why the root is the full box.

Quirks 1 to 4 were verified in WezTerm with the prototype below: shrunk frames, ease-out, 150 ms, repeated `a` presses in one window. Quirks 5 and 6 come from kitty's and Ghostty's sources.

### Tests

Screen:
- The first render marks no box as new.
- After `a`, the next render marks exactly the new path. The render after that marks none.
- `A` marks the new inner box's path.
- `a`, `u`, `a` marks the re-added path again.

View:
- The placement whose path is in `new` has `grow: true`. All the others have `false`.

Renderer:
- `eased(0) == 0`, `eased(1) == 1`, and it never decreases. `eased(0.5) == 0.875`.
- `grow_frames` returns 9 canvases of the full size. The drawn width and height never decrease. The last frame equals `box_canvas` for the same size and style. Pixels outside each frame's box are transparent.
- A `grow: true` box that fits produces one `a=T` followed by 9 `a=f` commands, and no tiles for that box. A `grow: true` box that is cropped produces tiles.
- Two grows of the same size have different root payloads.
- The root differs from the full-size box only in the lowest bit of its colour bytes.
- `wezterm` is passed through to the grow command: with it, frames carry `Z`. Without it, they don't.
- The frame after a grow deletes the grow image and draws the box with tiles.

Kitty:
- `grow` writes the gap in `z` on every frame, writes the hold on the last frame, and ends with `a=a,i=<id>,s=3,v=2`.
- Without `wezterm`, no frame has a `Z` key. With `wezterm`, every frame has `Z` equal to its `z`.

### Prototype

A standalone Python script (stdlib only) that was used to try the grow in WezTerm before this design. Use it as a reference for the escape-code sequence: a unique root, the frames with `z` and `Z`, the long hold, then `a=a`. Its `shrunk` mode is the one chosen. Its `clip` mode and the `--gap`/`--hold` flags are only for experiments. It doesn't follow dre's layout: boxes just stack downwards.

Run it in WezTerm with `python3 grow.py` (add `--slow` to watch the easing, or `--selftest` to print one grow's escape codes without a terminal).

<details>
<summary><code>grow.py</code></summary>

```python
#!/usr/bin/env python3
"""Prototype: a box "grows into place" using only Kitty graphics animation frames.

The terminal plays the animation; this script sends everything up front and
then just waits for keys. No threads, no redraw loop.

Keys:  a  add a growing box below the last one
       m  toggle mode: shrunk (complete outline at eased size) / clip (full box revealed)
       q / Ctrl-C  quit

Flags: --slow       durations x10, to inspect the easing
       --gap N      gap of each growing frame in ms
       --hold N     gap of the last (full size) frame in ms
       --selftest   headless: check frame sizes and print one grow's escape codes
"""
import base64
import fcntl
import os
import random
import struct
import sys
import termios
import tty
import zlib

# Look of dre-flex (src/flex/view.rs). FLEX_BORDER = 1 is in *pixels*
# (render/terminal.rs box_shape -> BoxShape.border, colour_at compares pixel offsets).
FLEX_BORDER = 1
FLEX_BORDER_COLOUR = (0x2A, 0x2A, 0x2E)
FLEX_SELECTED_COLOUR = (0x8A, 0xB4, 0xF8)
MARGIN_COLS = 2       # empty cells left and right of each box
BOX_ROWS = 3
GAP_ROWS = 1          # FLEX_SPACE.height
TOP_ROW = 1           # 0-based row of the first box

FRAMES = 9
TOTAL_MS = 150
HOLD_MS = 2_000_000_000
CHUNK = 4096

ESC = "\x1b"


# ---------------------------------------------------------------- pixels

def ease_out_cubic(t):
    return 1 - (1 - t) ** 3


def outline_rows(w, h, colour, border=FLEX_BORDER):
    """Return (edge_row, body_row) bytes for a w-px wide outline box, transparent inside."""
    r, g, b = colour
    edge_px = bytes((r, g, b, 255))
    clear_px = b"\0\0\0\0"
    edge_row = edge_px * w
    if w <= 2 * border:
        body_row = edge_row
    else:
        body_row = edge_px * border + clear_px * (w - 2 * border) + edge_px * border
    return edge_row, body_row


def outline_box(w, h, colour, border=FLEX_BORDER):
    """RGBA bytes of a complete w x h outline box."""
    if w <= 0 or h <= 0:
        return b""
    edge_row, body_row = outline_rows(w, h, colour, border)
    if h <= 2 * border:
        return edge_row * h
    return edge_row * border + body_row * (h - 2 * border) + edge_row * border


def canvas(full_w, full_h, w, h, row_source):
    """Full-size transparent canvas with a w x h region at the top-left taken from
    row_source(y) (which returns exactly w*4 bytes)."""
    pad = b"\0\0\0\0" * (full_w - w)
    out = bytearray()
    for y in range(h):
        out += row_source(y)
        out += pad
    out += b"\0\0\0\0" * full_w * (full_h - h)
    return bytes(out)


def eased_sizes(full_w, full_h, frames=FRAMES):
    """Grown (w, h) for frames 1..frames; the last is always the full size."""
    sizes = []
    for k in range(1, frames + 1):
        e = ease_out_cubic(k / frames)
        w = max(1, min(full_w, round(full_w * e)))
        h = max(1, min(full_h, round(full_h * e)))
        sizes.append((w, h))
    sizes[-1] = (full_w, full_h)
    return sizes


def grow_frames(full_w, full_h, colour, mode, frames=FRAMES):
    """List of ((w, h), rgba_bytes) for the animation frames (not the root frame).
    Every rgba buffer is full_w x full_h."""
    result = []
    if mode == "clip":
        full = outline_box(full_w, full_h, colour)
        stride = full_w * 4
        for w, h in eased_sizes(full_w, full_h, frames):
            data = canvas(full_w, full_h, w, h,
                          lambda y, w=w: full[y * stride: y * stride + w * 4])
            result.append(((w, h), data))
    else:  # shrunk: a complete smaller box each frame
        for w, h in eased_sizes(full_w, full_h, frames):
            small = outline_box(w, h, colour)
            stride = w * 4
            data = canvas(full_w, full_h, w, h,
                          lambda y, small=small, stride=stride: small[y * stride:(y + 1) * stride])
            result.append(((w, h), data))
    return result


# ---------------------------------------------------------------- protocol

def gfx(keys, payload=b"", compress=True):
    """Build one (possibly chunked) graphics command. keys: dict, order kept."""
    keys = dict(keys)
    keys["q"] = 2
    if payload and compress:
        payload = zlib.compress(payload)
        keys["o"] = "z"
    data = base64.standard_b64encode(payload).decode("ascii") if payload else ""
    head = ",".join(f"{k}={v}" for k, v in keys.items())
    if len(data) <= CHUNK:
        return f"{ESC}_G{head};{data}{ESC}\\" if data else f"{ESC}_G{head}{ESC}\\"
    parts = []
    chunks = [data[i:i + CHUNK] for i in range(0, len(data), CHUNK)]
    for n, chunk in enumerate(chunks):
        m = 0 if n == len(chunks) - 1 else 1
        if n == 0:
            parts.append(f"{ESC}_G{head},m={m};{chunk}{ESC}\\")
        else:
            parts.append(f"{ESC}_Gm={m},q=2;{chunk}{ESC}\\")
    return "".join(parts)


def move(row, col):
    return f"{ESC}[{row + 1};{col + 1}H"


def static_box_cmds(image_id, row, col, w, h, colour):
    return move(row, col) + gfx(
        {"a": "T", "f": 32, "s": w, "v": h, "i": image_id, "p": 1, "C": 1},
        outline_box(w, h, colour))


def flag(name, default):
    """Value of `--name N` on the command line, for experiments."""
    if name in sys.argv:
        return int(sys.argv[sys.argv.index(name) + 1])
    return default


def hold_ms():
    return flag("--hold", HOLD_MS)


# WezTerm keeps each animation's playback state in a per-window cache keyed by
# the hash of the image as first transmitted (the root frame), and never
# rehashes when frames are added. Identical roots would share one state that is
# already parked on the last frame, so each root carries a unique id in the RGB
# bytes of fully transparent pixels. The random start keeps runs apart too.
grows = random.randrange(1 << 32)


def unique_root(w, h):
    global grows
    grows += 1
    root = bytearray(w * h * 4)
    stamp = grows.to_bytes(6, "big")
    root[0:3] = stamp[0:3]  # pixel 0, alpha stays 0
    root[4:7] = stamp[3:6]  # pixel 1, alpha stays 0
    return bytes(root)


def grow_cmds(image_id, row, col, w, h, colour, mode, scale=1):
    """Escape codes for one growing box: transparent root + FRAMES frames + start."""
    out = [move(row, col)]
    # Root frame: fully transparent, full size (frames must fit inside the root).
    out.append(gfx({"a": "T", "f": 32, "s": w, "v": h, "i": image_id, "p": 1, "C": 1},
                   unique_root(w, h)))
    gap = flag("--gap", max(1, round(TOTAL_MS * scale / (FRAMES - 1))))
    frames = grow_frames(w, h, colour, mode)
    for n, (_, data) in enumerate(frames):
        z = hold_ms() if n == len(frames) - 1 else gap
        out.append(gfx({"a": "f", "i": image_id, "f": 32, "s": w, "v": h, "z": z, "Z": z}, data))  # WezTerm reads the gap from Z, kitty from z
    # Root frame gap (kitty; WezTerm uses 0 for the root anyway), then play once.
    out.append(gfx({"a": "a", "i": image_id, "r": 1, "z": 1}))
    out.append(gfx({"a": "a", "i": image_id, "s": 3, "v": 2}))
    return "".join(out)


# ---------------------------------------------------------------- terminal

def window_size():
    raw = fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b"\0" * 8)
    rows, cols, xpix, ypix = struct.unpack("HHHH", raw)
    return rows, cols, xpix, ypix


def write(s):
    data = s.encode("utf-8") if isinstance(s, str) else s
    fd = sys.stdout.fileno()
    while data:
        n = os.write(fd, data)
        data = data[n:]


def status(rows, cols, mode, scale, count):
    text = f" mode: {mode}  (m toggles)   a: add box   q: quit   boxes: {count}" \
           f"{'   SLOW x10' if scale != 1 else ''}"
    write(move(rows - 1, 0) + f"{ESC}[2K" + text[:cols])


def main():
    scale = 10 if "--slow" in sys.argv else 1
    if "--selftest" in sys.argv:
        return selftest(scale)

    rows, cols, xpix, ypix = window_size()
    if not xpix or not ypix:
        sys.exit("terminal did not report pixel size (TIOCGWINSZ ws_xpixel/ws_ypixel = 0)")
    cw, ch = xpix / cols, ypix / rows
    box_w = round((cols - 2 * MARGIN_COLS) * cw)
    box_h = round(BOX_ROWS * ch)

    fd = sys.stdin.fileno()
    saved = termios.tcgetattr(fd)
    mode = "shrunk"
    next_id = 1000
    row = TOP_ROW
    count = 0
    try:
        tty.setraw(fd)
        write(f"{ESC}[?25l{ESC}[2J")
        write(static_box_cmds(next_id, row, MARGIN_COLS, box_w, box_h, FLEX_BORDER_COLOUR))
        next_id += 1
        count += 1
        row += BOX_ROWS + GAP_ROWS
        status(rows, cols, mode, scale, count)
        while True:
            key = os.read(fd, 1)
            if key in (b"q", b"\x03", b""):
                break
            if key == b"m":
                mode = "clip" if mode == "shrunk" else "shrunk"
            elif key == b"a":
                if row + BOX_ROWS > rows - 1:
                    write("\a")
                    continue
                write(grow_cmds(next_id, row, MARGIN_COLS, box_w, box_h,
                                FLEX_SELECTED_COLOUR, mode, scale))
                next_id += 1
                count += 1
                row += BOX_ROWS + GAP_ROWS
            status(rows, cols, mode, scale, count)
    finally:
        write(gfx({"a": "d", "d": "A"}) + f"{ESC}[2J{ESC}[H{ESC}[?25h")
        termios.tcsetattr(fd, termios.TCSADRAIN, saved)


# ---------------------------------------------------------------- headless check

def abbreviate(seq):
    import re
    seq = re.sub(r";([A-Za-z0-9+/=]{12})[A-Za-z0-9+/=]*(\x1b\\)",
                 lambda m: f";{m.group(1)}...<payload>" + m.group(2), seq)
    return seq.replace("\x1b", "ESC")


def selftest(scale):
    for full_w, full_h in [(3000, 96), (500, 60), (7, 5)]:
        for mode in ("shrunk", "clip"):
            frames = grow_frames(full_w, full_h, FLEX_SELECTED_COLOUR, mode)
            sizes = [s for s, _ in frames]
            assert len(frames) == FRAMES
            for (w0, h0), (w1, h1) in zip(sizes, sizes[1:]):
                assert w1 >= w0 and h1 >= h0, sizes
            assert sizes[-1] == (full_w, full_h), sizes
            for (w, h), data in frames:
                assert len(data) == full_w * full_h * 4
                # pixel (w-1, h-1) is edge in shrunk mode, and outside is transparent
                if w < full_w:
                    assert data[(0 * full_w + w) * 4 + 3] == 0
                if mode == "shrunk":
                    assert data[((h - 1) * full_w + (w - 1)) * 4 + 3] == 255
            print(f"{full_w}x{full_h} {mode}: {sizes}")
    seq = grow_cmds(1001, 5, 2, 500, 60, FLEX_SELECTED_COLOUR, "shrunk", scale)
    print("\nOne grow (500x60 px, shrunk), payloads abbreviated:\n")
    for part in abbreviate(seq).split("ESC\\"):
        if part:
            print(part + "ESC\\")
    print("\nok")


if __name__ == "__main__":
    main()
```

</details>
