#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TextKey {
    Left,
    Right,
    Backspace,
    Char(char),
}

pub(crate) fn edit(text: &str, cursor: usize, key: TextKey) -> (String, usize) {
    let mut chars: Vec<char> = text.chars().collect();
    let cursor = match key {
        TextKey::Left => cursor.saturating_sub(1),
        TextKey::Right => (cursor + 1).min(chars.len()),
        TextKey::Backspace if cursor > 0 => {
            chars.remove(cursor - 1);
            cursor - 1
        }
        TextKey::Backspace => cursor,
        TextKey::Char(c) => {
            chars.insert(cursor, c);
            cursor + 1
        }
    };
    (chars.into_iter().collect(), cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_inserts_in_the_middle_and_advances() {
        assert_eq!(edit("ac", 1, TextKey::Char('b')), ("abc".into(), 2));
    }

    #[test]
    fn char_inserts_at_the_start_and_end() {
        assert_eq!(edit("bc", 0, TextKey::Char('a')), ("abc".into(), 1));
        assert_eq!(edit("ab", 2, TextKey::Char('c')), ("abc".into(), 3));
    }

    #[test]
    fn char_inserts_into_empty_text() {
        assert_eq!(edit("", 0, TextKey::Char('a')), ("a".into(), 1));
    }

    #[test]
    fn backspace_removes_the_char_before_the_cursor() {
        assert_eq!(edit("abc", 2, TextKey::Backspace), ("ac".into(), 1));
        assert_eq!(edit("abc", 3, TextKey::Backspace), ("ab".into(), 2));
    }

    #[test]
    fn backspace_at_the_start_does_nothing() {
        assert_eq!(edit("abc", 0, TextKey::Backspace), ("abc".into(), 0));
        assert_eq!(edit("", 0, TextKey::Backspace), ("".into(), 0));
    }

    #[test]
    fn left_and_right_move_the_cursor() {
        assert_eq!(edit("abc", 2, TextKey::Left), ("abc".into(), 1));
        assert_eq!(edit("abc", 1, TextKey::Right), ("abc".into(), 2));
    }

    #[test]
    fn left_at_the_start_and_right_at_the_end_do_nothing() {
        assert_eq!(edit("abc", 0, TextKey::Left), ("abc".into(), 0));
        assert_eq!(edit("abc", 3, TextKey::Right), ("abc".into(), 3));
        assert_eq!(edit("", 0, TextKey::Left), ("".into(), 0));
        assert_eq!(edit("", 0, TextKey::Right), ("".into(), 0));
    }

    #[test]
    fn cursor_counts_chars_not_bytes() {
        assert_eq!(edit("éà", 1, TextKey::Char('ü')), ("éüà".into(), 2));
        assert_eq!(edit("éà", 1, TextKey::Backspace), ("à".into(), 0));
        assert_eq!(edit("éà", 1, TextKey::Right), ("éà".into(), 2));
    }
}
