use dre::{view, Renderer};
use wasm_bindgen::prelude::*;

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

    pub fn svg(&self, cols: i32, rows: i32) -> String {
        let mut out = Vec::new();
        let mut renderer = dre::SvgRenderer::with_canvas(i64::from(cols), i64::from(rows));
        let window = view::Area {
            col: 0,
            row: 0,
            cols: i64::from(cols),
            rows: i64::from(rows),
        };
        let scene = view::editor(self.session.state(), window);
        renderer
            .render(&scene, &mut out)
            .expect("rendering SVG to an in-memory buffer succeeds");
        String::from_utf8(out).expect("SvgRenderer writes UTF-8")
    }
}

impl Default for WebSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::WebSession;

    #[test]
    fn renders_svg_after_key_presses() {
        let mut session = WebSession::new();

        session.press_key("b");

        let svg = session.svg(160, 50);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
    }
}
