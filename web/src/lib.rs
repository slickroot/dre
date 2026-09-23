use dre::Renderer;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

struct IdleTimer {
    handle: i32,
    _closure: Closure<dyn FnMut()>,
}

fn join(segments: Vec<dre::Segment>) -> String {
    segments.iter().map(|s| s.text.as_str()).collect()
}

#[wasm_bindgen]
pub struct WebSession {
    session: Rc<RefCell<dre::Session>>,
    idle_timer: Option<IdleTimer>,
    on_change: js_sys::Function,
}

#[cfg(target_arch = "wasm32")]
fn schedule_idle(
    session: &Rc<RefCell<dre::Session>>,
    on_change: &js_sys::Function,
) -> Option<IdleTimer> {
    let session = Rc::clone(session);
    let on_change = on_change.clone();
    let closure = Closure::new(move || {
        session.borrow_mut().go_idle();
        let _ = on_change.call0(&JsValue::NULL);
    });
    let handle = web_sys::window()
        .expect("a browser window exists")
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            dre::IDLE_TIMEOUT_MS as i32,
        )
        .expect("setTimeout succeeds");
    Some(IdleTimer {
        handle,
        _closure: closure,
    })
}

#[cfg(target_arch = "wasm32")]
fn clear_timeout(handle: i32) {
    if let Some(window) = web_sys::window() {
        window.clear_timeout_with_handle(handle);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_idle(
    _session: &Rc<RefCell<dre::Session>>,
    _on_change: &js_sys::Function,
) -> Option<IdleTimer> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_timeout(_handle: i32) {}

#[wasm_bindgen]
impl WebSession {
    #[wasm_bindgen(constructor)]
    pub fn new(on_change: js_sys::Function) -> WebSession {
        WebSession {
            session: Rc::new(RefCell::new(dre::Session::new())),
            idle_timer: None,
            on_change,
        }
    }

    pub fn press_key(&mut self, key: &str) {
        if let Some(timer) = self.idle_timer.take() {
            clear_timeout(timer.handle);
        }
        self.session.borrow_mut().press_key(key);
        self.idle_timer = schedule_idle(&self.session, &self.on_change);
    }

    pub fn extent(&self) -> Vec<i32> {
        let (width, height) = self.session.borrow().extent();
        vec![width as i32, height as i32]
    }

    pub fn svg(&self, cols: i32, rows: i32, extent_width: i32, extent_height: i32) -> String {
        let mut out = Vec::new();
        let mut renderer = dre::SvgRenderer::with_canvas(i64::from(cols), i64::from(rows))
            .centered_on(i64::from(extent_width), i64::from(extent_height));
        renderer
            .render(self.session.borrow().state(), &mut out)
            .expect("rendering SVG to an in-memory buffer succeeds");
        String::from_utf8(out).expect("SvgRenderer writes UTF-8")
    }

    pub fn status_line_left(&self) -> String {
        join(dre::status_line(self.session.borrow().state()).left)
    }

    pub fn status_line_right(&self) -> String {
        join(dre::status_line(self.session.borrow().state()).right)
    }
}

#[cfg(test)]
mod tests {
    use super::WebSession;
    use wasm_bindgen::{JsCast, JsValue};

    fn session() -> WebSession {
        WebSession::new(JsValue::UNDEFINED.unchecked_into())
    }

    #[test]
    fn renders_svg_after_key_presses() {
        let mut session = session();

        session.press_key("b");

        let svg = session.svg(160, 50, 10, 3);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn extent_after_a_key_press_is_not_empty() {
        let mut session = session();

        session.press_key("b");

        let extent = session.extent();
        assert_eq!(extent.len(), 2);
        assert!(extent[0] > 0);
        assert!(extent[1] > 0);
    }

    #[test]
    fn status_line_left_starts_in_command_mode() {
        let session = session();

        assert!(session.status_line_left().starts_with(" COMMANDING "));
    }

    #[test]
    fn status_line_left_reflects_insert_mode_after_adding_a_box() {
        let mut session = session();

        session.press_key("b");

        assert!(session.status_line_left().starts_with(" EDITING "));
    }

    #[test]
    fn status_line_right_starts_at_zero_boxes() {
        let session = session();

        assert_eq!(session.status_line_right(), "0 boxes \u{2022} dre");
    }

    #[test]
    fn status_line_right_reflects_added_box() {
        let mut session = session();

        session.press_key("b");

        assert_eq!(session.status_line_right(), "1 boxes . dre");
    }
}
