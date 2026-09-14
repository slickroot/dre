import dre_rs
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
        chunks = dre_rs.chunks(dre_rs.encode(sprite.pixels), CHUNK_SIZE)
        header = (
            f"a=T,f=32,s={sprite.width},v={sprite.height},o=z,q=2,z=-1,"
            f"m={dre_rs.more(chunks, 0)}"
        )
        escapes = [dre_rs.escape(header, chunks[0])]
        for index, chunk in enumerate(chunks[1:], start=1):
            escapes.append(dre_rs.escape(f"m={dre_rs.more(chunks, index)}", chunk))
        return "".join(escapes)
