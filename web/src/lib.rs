use dre::Renderer;
use wasm_bindgen::prelude::*;

const CANVAS_COLUMNS: i64 = 160;
const CANVAS_ROWS: i64 = 50;

#[wasm_bindgen]
pub struct WebSession {
    session: dre::Session,
}

#[wasm_bindgen]
impl WebSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebSession {
        WebSession {
            session: dre::Session::new(),
        }
    }

    pub fn press_key(&mut self, key: &str) {
        self.session.press_key(key);
    }

    pub fn svg(&self) -> String {
        let mut out = Vec::new();
        let mut renderer = dre::SvgRenderer::with_canvas(CANVAS_COLUMNS, CANVAS_ROWS);
        renderer
            .render(self.session.document(), &mut out)
            .expect("rendering SVG to an in-memory buffer succeeds");
        String::from_utf8(out).expect("SvgRenderer writes UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::WebSession;

    #[test]
    fn renders_svg_after_key_presses() {
        let mut session = WebSession::new();

        session.press_key("b");

        let svg = session.svg();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
    }
}
