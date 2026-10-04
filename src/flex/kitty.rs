use super::canvas::{Frame, TextRun};
use crate::kitty::{place_ext, transmit, ImageId, PlacementId};
use std::io::{self, Write};
use std::num::NonZeroU32;

const BEGIN_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026h";
const END_SYNCHRONIZED_UPDATE: &str = "\x1b[?2026l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const HIDE_CURSOR: &str = "\x1b[?25l";
const RESET: &str = "\x1b[0m";
const IMAGE_Z: i32 = -1;

fn image_id() -> ImageId {
    ImageId::new(NonZeroU32::MIN)
}

fn placement_id() -> PlacementId {
    PlacementId::new(NonZeroU32::MIN)
}

fn move_to(col: i64, row: i64) -> String {
    format!("\x1b[{};{}H", row + 1, col + 1)
}

fn text_run(run: &TextRun) -> String {
    let (r, g, b) = run.colour;
    format!(
        "{}\x1b[38;2;{r};{g};{b}m{}{RESET}",
        move_to(run.col, run.row),
        run.text
    )
}

pub(crate) fn show(frame: &Frame, out: &mut impl Write) -> io::Result<()> {
    write!(out, "{BEGIN_SYNCHRONIZED_UPDATE}")?;
    write!(out, "{}", transmit(&frame.pixels, image_id()))?;
    write!(
        out,
        "{}",
        place_ext(image_id(), placement_id(), 0, 0, IMAGE_Z, None, None)
    )?;
    for run in &frame.texts {
        write!(out, "{}", text_run(run))?;
    }
    match frame.caret {
        Some((col, row)) => write!(out, "{}{SHOW_CURSOR}", move_to(col, row))?,
        None => write!(out, "{HIDE_CURSOR}")?,
    }
    write!(out, "{END_SYNCHRONIZED_UPDATE}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{Canvas, Rgba, Shape};

    struct Solid;
    impl Shape for Solid {
        fn colour_at(&self, _x: i64, _y: i64) -> Option<Rgba> {
            Some([1, 2, 3, 255])
        }
    }

    fn frame(texts: Vec<TextRun>, caret: Option<(i64, i64)>) -> Frame {
        Frame {
            pixels: Canvas::fill(2, 2, &Solid),
            texts,
            caret,
        }
    }

    fn run(col: i64, row: i64, text: &str, colour: (u8, u8, u8)) -> TextRun {
        TextRun {
            col,
            row,
            text: text.to_string(),
            colour,
        }
    }

    fn shown(frame: &Frame) -> String {
        let mut out = Vec::new();
        show(frame, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn everything_is_inside_one_synchronized_update() {
        let s = shown(&frame(vec![run(0, 0, "a", (1, 2, 3))], Some((1, 1))));
        assert!(s.starts_with(BEGIN_SYNCHRONIZED_UPDATE));
        assert!(s.ends_with(END_SYNCHRONIZED_UPDATE));
        assert_eq!(s.matches(BEGIN_SYNCHRONIZED_UPDATE).count(), 1);
        assert_eq!(s.matches(END_SYNCHRONIZED_UPDATE).count(), 1);
    }

    #[test]
    fn the_pixels_are_transmitted_then_placed_at_the_origin_below_text() {
        let f = frame(vec![], None);
        let s = shown(&f);
        let transmitted = transmit(&f.pixels, image_id()).to_string();
        let placed = place_ext(image_id(), placement_id(), 0, 0, -1, None, None).to_string();
        let t = s.find(&transmitted).expect("transmit");
        let p = s.find(&placed).expect("place");
        assert!(t < p);
        assert!(placed.contains("z=-1"));
        assert!(placed.starts_with("\x1b[1;1H"));
    }

    #[test]
    fn successive_shows_reuse_the_same_image_and_placement() {
        let f = frame(vec![], None);
        assert_eq!(shown(&f), shown(&f));
    }

    #[test]
    fn each_text_run_is_moved_to_coloured_and_reset_in_order() {
        let s = shown(&frame(
            vec![run(4, 3, "hi", (10, 20, 30)), run(0, 7, "yo", (40, 50, 60))],
            None,
        ));
        let first = "\x1b[4;5H\x1b[38;2;10;20;30mhi\x1b[0m";
        let second = "\x1b[8;1H\x1b[38;2;40;50;60myo\x1b[0m";
        let a = s.find(first).expect("first run");
        let b = s.find(second).expect("second run");
        assert!(a < b);
    }

    #[test]
    fn text_comes_after_the_image_placement() {
        let s = shown(&frame(vec![run(0, 0, "a", (1, 2, 3))], None));
        let placed = place_ext(image_id(), placement_id(), 0, 0, -1, None, None).to_string();
        assert!(s.find(&placed).unwrap() < s.find("\x1b[38;2;1;2;3m").unwrap());
    }

    #[test]
    fn a_caret_shows_the_cursor_at_its_cell() {
        let s = shown(&frame(vec![], Some((5, 2))));
        assert!(s.contains("\x1b[3;6H\x1b[?25h"));
        assert!(!s.contains(HIDE_CURSOR));
    }

    #[test]
    fn no_caret_hides_the_cursor() {
        let s = shown(&frame(vec![], None));
        assert!(s.contains(HIDE_CURSOR));
        assert!(!s.contains(SHOW_CURSOR));
    }
}
