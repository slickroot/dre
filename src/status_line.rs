use crate::palette::{palette, BACKGROUND};

#[derive(Clone, Copy)]
pub(crate) enum ModeLabel {
    Commanding,
    Editing,
}

pub(crate) struct StatusInput {
    pub(crate) mode: ModeLabel,
    pub(crate) filename: String,
    pub(crate) box_count: usize,
}

pub(crate) struct StatusLine {
    pub(crate) left: Vec<Segment>,
    pub(crate) right: Vec<Segment>,
}

pub(crate) struct Segment {
    pub(crate) text: String,
    pub(crate) style: Style,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct Style {
    pub(crate) bold: bool,
    pub(crate) background: Option<(u8, u8, u8, u8)>,
    pub(crate) foreground: Option<(u8, u8, u8)>,
}

pub(crate) const DIM_ALPHA: u8 = 0x40;

fn dim(text: impl Into<String>) -> Segment {
    Segment {
        text: text.into(),
        style: Style {
            bold: false,
            background: Some((0, 0, 0, DIM_ALPHA)),
            foreground: None,
        },
    }
}

pub(crate) fn status_line(input: &StatusInput) -> StatusLine {
    let mode_text = match input.mode {
        ModeLabel::Commanding => "COMMANDING",
        ModeLabel::Editing => "EDITING",
    };
    let (r, g, b) = palette(0).unwrap();
    let mode = Segment {
        text: format!(" {mode_text} "),
        style: Style {
            bold: true,
            background: Some((r, g, b, 0xFF)),
            foreground: palette(BACKGROUND),
        },
    };
    StatusLine {
        left: vec![mode, dim(" \u{2502} "), dim(input.filename.clone())],
        right: vec![
            dim(format!("{} boxes", input.box_count)),
            dim(" \u{2022} "),
            dim("dre"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(mode: ModeLabel) -> StatusInput {
        StatusInput {
            mode,
            filename: "diagram.dre[+]".to_string(),
            box_count: 3,
        }
    }

    #[test]
    fn pads_the_mode_label_with_a_space_on_each_side() {
        let commanding = status_line(&input(ModeLabel::Commanding));
        let editing = status_line(&input(ModeLabel::Editing));
        assert_eq!(commanding.left[0].text, " COMMANDING ");
        assert_eq!(editing.left[0].text, " EDITING ");
    }

    #[test]
    fn styles_the_mode_segment_bold_on_the_first_palette_colour() {
        let style = status_line(&input(ModeLabel::Commanding)).left[0].style;
        let (r, g, b) = palette(0).unwrap();
        assert!(style.bold);
        assert_eq!(style.background, Some((r, g, b, 0xFF)));
        assert_eq!(style.foreground, palette(BACKGROUND));
    }

    #[test]
    fn styles_every_other_segment_dim() {
        let line = status_line(&input(ModeLabel::Editing));
        let others: Vec<&Segment> = line.left[1..].iter().chain(line.right.iter()).collect();
        assert_eq!(others.len(), 5);
        for segment in others {
            assert!(!segment.style.bold);
            assert_eq!(segment.style.background, Some((0, 0, 0, DIM_ALPHA)));
            assert_eq!(segment.style.foreground, None);
        }
    }

    #[test]
    fn separates_the_mode_from_the_filename_with_a_vertical_bar() {
        let line = status_line(&input(ModeLabel::Commanding));
        assert_eq!(line.left[1].text, " \u{2502} ");
    }

    #[test]
    fn shows_the_filename_as_given() {
        let line = status_line(&input(ModeLabel::Commanding));
        assert_eq!(line.left[2].text, "diagram.dre[+]");
    }

    #[test]
    fn shows_the_box_count_then_the_app_name_on_the_right() {
        let line = status_line(&input(ModeLabel::Commanding));
        let texts: Vec<&str> = line.right.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["3 boxes", " \u{2022} ", "dre"]);
    }

    #[test]
    fn says_boxes_even_for_a_single_box() {
        let mut single = input(ModeLabel::Commanding);
        single.box_count = 1;
        assert_eq!(status_line(&single).right[0].text, "1 boxes");
    }
}
