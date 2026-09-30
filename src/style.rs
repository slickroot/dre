use crate::view::Rgb;

const PALETTE: [(&str, (u8, u8, u8)); 8] = [
    ("lime", (0xC6, 0xFF, 0x00)),
    ("mint", (0x39, 0xFF, 0xB0)),
    ("violet", (0xB3, 0x88, 0xFF)),
    ("pink", (0xFF, 0x3D, 0xF5)),
    ("amber", (0xFF, 0xB0, 0x20)),
    ("foreground", (0xE8, 0xEA, 0xED)),
    ("background", (0x0A, 0x0B, 0x0D)),
    ("dim", (0x8C, 0x8E, 0x91)),
];

pub(crate) const FOREGROUND: u8 = 5;
pub(crate) const BACKGROUND: u8 = 6;
pub(crate) const LIME: u8 = 0;
pub(crate) const VIOLET: u8 = 2;
pub(crate) const AMBER: u8 = 4;
pub(crate) const DIM: u8 = 7;

pub(crate) const CELL_WIDTH: i64 = 8;
pub(crate) const CELL_HEIGHT: i64 = 16;
pub(crate) const BOX_FILL_OPACITY: f64 = 0.12;
pub(crate) const FOOTER_FILL_OPACITY: f64 = 0.12;

pub(crate) fn palette(index: u8) -> Option<(u8, u8, u8)> {
    PALETTE.get(index as usize).map(|&(_, rgb)| rgb)
}

pub(crate) fn rgb(colour: Option<u8>) -> Rgb {
    palette(colour.unwrap_or(FOREGROUND)).unwrap()
}

pub(crate) fn next_on_palette(colour: Option<u8>) -> Option<u8> {
    match colour {
        None => Some(0),
        Some(i) if palette(i + 1).is_some() => Some(i + 1),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_of_no_colour_is_the_foreground() {
        assert_eq!(Some(rgb(None)), palette(FOREGROUND));
    }

    #[test]
    fn rgb_of_a_palette_index_is_its_palette_colour() {
        assert_eq!(Some(rgb(Some(DIM))), palette(DIM));
    }

    #[test]
    fn palette_has_a_colour_at_index_zero() {
        assert!(palette(0).is_some());
    }

    #[test]
    fn palette_has_the_foreground_colour_at_its_index() {
        assert_eq!(palette(FOREGROUND), Some((232, 234, 237)));
    }

    #[test]
    fn palette_has_the_background_colour_at_its_index() {
        assert_eq!(palette(BACKGROUND), Some((10, 11, 13)));
    }

    #[test]
    fn palette_has_the_lime_colour_at_its_index() {
        assert_eq!(palette(LIME), Some((198, 255, 0)));
    }

    #[test]
    fn palette_has_the_violet_colour_at_its_index() {
        assert_eq!(palette(VIOLET), Some((179, 136, 255)));
    }

    #[test]
    fn palette_has_the_dim_grey_at_its_index() {
        assert_eq!(palette(DIM), Some((140, 142, 145)));
    }

    #[test]
    fn dim_is_the_last_index_of_the_palette() {
        assert_eq!(last_index(), DIM);
        assert_eq!(palette(DIM + 1), None);
    }

    #[test]
    fn palette_has_no_colour_past_its_last_index() {
        let first_missing = (0..=u8::MAX).find(|&i| palette(i).is_none()).unwrap();
        assert!(first_missing > 0);
        assert_eq!(palette(first_missing), None);
        assert!((first_missing..=u8::MAX).all(|i| palette(i).is_none()));
    }

    fn last_index() -> u8 {
        (0..=u8::MAX)
            .take_while(|&i| palette(i).is_some())
            .last()
            .unwrap()
    }

    #[test]
    fn next_on_palette_starts_plain_boxes_on_the_first_colour() {
        assert_eq!(next_on_palette(None), Some(0));
    }

    #[test]
    fn next_on_palette_advances_to_the_following_colour() {
        assert_eq!(next_on_palette(Some(0)), Some(1));
    }

    #[test]
    fn next_on_palette_turns_the_last_colour_plain() {
        assert_eq!(next_on_palette(Some(last_index())), None);
    }

    #[test]
    fn next_on_palette_cycles_through_the_palette_and_back_to_plain() {
        let mut colour = None;
        for _ in 0..=last_index() {
            colour = next_on_palette(colour);
        }
        assert_eq!(colour, Some(DIM));
        assert_eq!(next_on_palette(colour), None);
    }
}
