import base64
import random
import re
import unittest
import zlib

from sketch.kitty import CHUNK_SIZE, DELETE_ALL, KittyGraphics
from sketch.render import Sprite


def sprite(width=2, height=2, col=0, row=0, byte=b"\xff"):
    return Sprite(
        pixels=byte * (width * height * 4),
        width=width,
        height=height,
        col=col,
        row=row,
    )


def escapes(payload):
    return re.findall(
        r"\x1b_G(.*?);(.*?)\x1b\\",
        payload[len(DELETE_ALL) :],
        re.DOTALL,
    )


class KittyGraphicsTest(unittest.TestCase):
    def setUp(self):
        self.graphics = KittyGraphics()

    def test_an_empty_frame_deletes_every_placement(self):
        self.assertEqual(self.graphics.draw([]), DELETE_ALL)

    def test_a_frame_starts_by_deleting_every_placement(self):
        self.assertTrue(self.graphics.draw([sprite()]).startswith(DELETE_ALL))

    def test_the_cursor_is_positioned_before_the_image(self):
        payload = self.graphics.draw([sprite(col=3, row=5)])
        self.assertIn("\x1b[6;4H\x1b_G", payload)

    def test_the_header_describes_the_sprite(self):
        keys = escapes(self.graphics.draw([sprite(width=4, height=6)]))[0][0]
        self.assertEqual(
            set(keys.split(",")),
            {"a=T", "f=32", "s=4", "v=6", "o=z", "q=2", "z=-1", "m=0"},
        )

    def test_the_payload_round_trips(self):
        drawn = sprite(width=8, height=8)
        chunks = escapes(self.graphics.draw([drawn]))
        joined = "".join(chunk for _, chunk in chunks)
        self.assertEqual(
            zlib.decompress(base64.b64decode(joined)), drawn.pixels
        )

    def test_a_small_sprite_is_sent_as_one_chunk(self):
        chunks = escapes(self.graphics.draw([sprite()]))
        self.assertEqual(len(chunks), 1)
        self.assertIn("m=0", chunks[0][0].split(","))

    def test_a_large_sprite_is_split_into_chunks(self):
        drawn = large_sprite()
        chunks = escapes(self.graphics.draw([drawn]))
        self.assertGreater(len(chunks), 1)
        self.assertTrue(all(len(chunk) <= CHUNK_SIZE for _, chunk in chunks))
        self.assertTrue(all("m=1" in keys.split(",") for keys, _ in chunks[:-1]))
        self.assertIn("m=0", chunks[-1][0].split(","))

    def test_continuation_chunks_carry_only_the_more_flag(self):
        chunks = escapes(self.graphics.draw([large_sprite()]))
        for keys, _ in chunks[1:]:
            self.assertIn(keys, ("m=1", "m=0"))

    def test_the_payload_of_a_large_sprite_round_trips(self):
        drawn = large_sprite()
        chunks = escapes(self.graphics.draw([drawn]))
        joined = "".join(chunk for _, chunk in chunks)
        self.assertEqual(
            zlib.decompress(base64.b64decode(joined)), drawn.pixels
        )

    def test_each_sprite_is_positioned_and_transmitted_in_order(self):
        payload = self.graphics.draw(
            [sprite(col=1, row=1), sprite(width=3, height=3, col=7, row=2)]
        )
        first = payload.index("\x1b[2;2H")
        second = payload.index("\x1b[3;8H")
        self.assertLess(first, second)
        self.assertEqual(len(escapes(payload)), 2)
        self.assertIn("s=3", escapes(payload)[1][0].split(","))


def large_sprite():
    width, height = 64, 64
    noise = random.Random(0)
    pixels = bytes(noise.getrandbits(8) for _ in range(width * height * 4))
    return Sprite(pixels=pixels, width=width, height=height, col=0, row=0)


if __name__ == "__main__":
    unittest.main()
