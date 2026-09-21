use dre::Renderer;
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

    pub fn extent(&self) -> Vec<i32> {
        let (width, height) = self.session.extent();
        vec![width as i32, height as i32]
    }

    pub fn svg(&self, cols: i32, rows: i32, extent_width: i32, extent_height: i32) -> String {
        let mut out = Vec::new();
        let mut renderer = dre::SvgRenderer::with_canvas(i64::from(cols), i64::from(rows))
            .centered_on(i64::from(extent_width), i64::from(extent_height));
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

        let svg = session.svg(160, 50, 10, 3);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn extent_after_a_key_press_is_not_empty() {
        let mut session = WebSession::new();

        session.press_key("b");

        let extent = session.extent();
        assert_eq!(extent.len(), 2);
        assert!(extent[0] > 0);
        assert!(extent[1] > 0);
    }
}
