const PALETTE: [(&str, (u8, u8, u8)); 7] = [
    ("lime", (0xC6, 0xFF, 0x00)),
    ("mint", (0x39, 0xFF, 0xB0)),
    ("violet", (0xB3, 0x88, 0xFF)),
    ("pink", (0xFF, 0x3D, 0xF5)),
    ("amber", (0xFF, 0xB0, 0x20)),
    ("foreground", (0xE8, 0xEA, 0xED)),
    ("background", (0x0A, 0x0B, 0x0D)),
];

pub(crate) const FOREGROUND: u8 = 5;
pub(crate) const BACKGROUND: u8 = 6;

pub(crate) fn palette(index: u8) -> Option<(u8, u8, u8)> {
    PALETTE.get(index as usize).map(|&(_, rgb)| rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn palette_has_no_colour_past_its_last_index() {
        let first_missing = (0..=u8::MAX).find(|&i| palette(i).is_none()).unwrap();
        assert!(first_missing > 0);
        assert_eq!(palette(first_missing), None);
        assert!((first_missing..=u8::MAX).all(|i| palette(i).is_none()));
    }
}
