use dre::{Renderer, Session, SvgRenderer};

#[test]
fn pressing_b_creates_a_renderable_box_through_the_session() {
    let mut session = Session::new();

    session.press_key("b");

    let mut svg = Vec::new();
    SvgRenderer::default().render(session.document(), &mut svg).unwrap();
    let svg = String::from_utf8(svg).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
}

#[test]
fn a_session_document_can_be_rendered_as_svg() {
    let session = Session::new();

    let mut svg = Vec::new();
    SvgRenderer::default().render(session.document(), &mut svg).unwrap();

    let svg = String::from_utf8(svg).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
}
