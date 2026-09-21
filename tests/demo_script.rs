use dre::{Renderer, Session, SvgRenderer};

const KEYS: &[&str] = &[
    "b", "P", "l", "a", "n", "\x1b", "s", "B", "u", "i", "l", "d", "\x1b", "s", "S", "h", "i", "p",
    "\x1b",
];

fn svg(session: &Session) -> String {
    let mut out = Vec::new();
    SvgRenderer::default().render(session.document(), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn landing_page_script_draws_labelled_boxes_and_changes_colour() {
    let mut session = Session::new();
    for key in KEYS {
        session.press_key(key);
    }
    let before = svg(&session);
    for label in ["Plan", "Build", "Ship"] {
        assert!(before.contains(label), "missing {label}");
    }

    session.press_key("c");
    let recoloured = svg(&session);
    assert_ne!(before, recoloured);
    assert!(recoloured.contains("Ship"));

    session.press_key("f");
    assert_ne!(recoloured, svg(&session));
}
