import base64
import zlib
from typing import List

from .render import Sprite

CHUNK_SIZE = 4096
DELETE_ALL = "\x1b_Ga=d,d=A,q=2;\x1b\\"


class KittyGraphics:
    def draw(self, sprites: List[Sprite]) -> str:
        return DELETE_ALL + "".join(self._sprite(sprite) for sprite in sprites)

    def _sprite(self, sprite: Sprite) -> str:
        return f"\x1b[{sprite.row + 1};{sprite.col + 1}H" + self._transmission(
            sprite
        )

    def _transmission(self, sprite: Sprite) -> str:
        chunks = _chunks(_encode(sprite.pixels))
        header = (
            f"a=T,f=32,s={sprite.width},v={sprite.height},o=z,q=2,z=-1,"
            f"m={_more(chunks, 0)}"
        )
        escapes = [_escape(header, chunks[0])]
        for index, chunk in enumerate(chunks[1:], start=1):
            escapes.append(_escape(f"m={_more(chunks, index)}", chunk))
        return "".join(escapes)


def _encode(pixels: bytes) -> str:
    return base64.b64encode(zlib.compress(pixels)).decode()


def _chunks(payload: str) -> List[str]:
    return [
        payload[start : start + CHUNK_SIZE]
        for start in range(0, len(payload), CHUNK_SIZE)
    ]


def _more(chunks: List[str], index: int) -> int:
    return 0 if index == len(chunks) - 1 else 1


def _escape(keys: str, payload: str) -> str:
    return f"\x1b_G{keys};{payload}\x1b\\"
