#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Area {
    pub col: i64,
    pub row: i64,
    pub cols: i64,
    pub rows: i64,
}

pub(crate) fn stack<const N: usize>(heights: [Option<i64>; N], window: Area) -> [Area; N] {
    let fixed: i64 = heights.iter().flatten().sum();
    let shared = heights.iter().filter(|height| height.is_none()).count() as i64;
    let left = window.rows - fixed;
    let mut unsized_seen = 0;
    let mut row = window.row;
    heights.map(|height| {
        let rows = height.unwrap_or_else(|| {
            let extra = i64::from(unsized_seen < left % shared);
            unsized_seen += 1;
            left / shared + extra
        });
        let area = Area {
            row,
            rows,
            ..window
        };
        row += rows;
        area
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Area = Area {
        col: 2,
        row: 5,
        cols: 80,
        rows: 24,
    };

    #[test]
    fn a_fixed_row_gets_exactly_its_height() {
        let [_, fixed] = stack([None, Some(3)], WINDOW);
        assert_eq!(fixed.rows, 3);
    }

    #[test]
    fn an_unsized_row_takes_the_rest() {
        let [rest, fixed] = stack([None, Some(3)], WINDOW);
        assert_eq!(rest.rows, WINDOW.rows - fixed.rows);
    }

    #[test]
    fn two_unsized_rows_split_equally_with_the_extra_row_going_to_the_first() {
        let [first, second, fixed] = stack([None, None, Some(3)], WINDOW);
        assert_eq!(first.rows + second.rows, WINDOW.rows - fixed.rows);
        assert_eq!(first.rows, second.rows + 1);
    }

    #[test]
    fn areas_run_top_to_bottom_and_are_full_width() {
        let areas = stack([Some(1), None, Some(3)], WINDOW);
        assert_eq!(areas[0].row, WINDOW.row);
        for pair in areas.windows(2) {
            assert_eq!(pair[1].row, pair[0].row + pair[0].rows);
        }
        let last = areas[areas.len() - 1];
        assert_eq!(last.row + last.rows, WINDOW.row + WINDOW.rows);
        for area in areas {
            assert_eq!((area.col, area.cols), (WINDOW.col, WINDOW.cols));
        }
    }
}
