pub(crate) const BLANK: char = ' ';
pub(crate) const CURSOR: char = '\u{2588}';

pub(crate) const ARROW_STROKE: i64 = 4;
pub(crate) const ARROWHEAD_ANGLE_DEG: f64 = 30.0;
pub(crate) const ARROWHEAD_EDGE_LENGTH: f64 = 15.0;
pub(crate) fn arrowhead_depth() -> f64 {
    ARROWHEAD_EDGE_LENGTH * ARROWHEAD_ANGLE_DEG.to_radians().cos()
}
pub(crate) fn arrowhead_slope() -> f64 {
    ARROWHEAD_ANGLE_DEG.to_radians().tan()
}
pub(crate) const RESET: &str = "\x1b[0m";

#[allow(dead_code)]
pub(crate) const CACHE_LIMIT: usize = 512;

pub(crate) const ROUNDED_RADIUS: i64 = 20;
pub(crate) const BORDER: i64 = 4;

pub(crate) const OPAQUE: u8 = 255;
pub(crate) const FILL_ALPHA: u16 = 77;
pub(crate) const TRANSPARENT: (u8, u8, u8, u8) = (0, 0, 0, 0);
pub(crate) const PLAIN_COLOUR: (u8, u8, u8) = (128, 128, 128);
pub(crate) const PALETTE: [(u8, u8, u8); 5] = [
    (255, 190, 11),
    (251, 86, 7),
    (255, 0, 110),
    (131, 56, 236),
    (58, 134, 255),
];

pub(crate) fn centered_span(c: i64, width: i64) -> std::ops::Range<i64> {
    let start = c - (width - 1).div_euclid(2);
    start..(start + width)
}

pub(crate) fn colour(colour: i64) -> (u8, u8, u8) {
    if colour == crate::state::PLAIN {
        PLAIN_COLOUR
    } else {
        PALETTE[colour as usize]
    }
}

pub(crate) fn fill_colour(fill: i64) -> (u8, u8, u8, u8) {
    if fill == crate::state::PLAIN {
        TRANSPARENT
    } else {
        let (r, g, b) = PALETTE[fill as usize];
        let composite =
            |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        (composite(r), composite(g), composite(b), OPAQUE)
    }
}

pub(crate) fn cell(character: char, colour: i64, fill: i64) -> String {
    let mut codes = Vec::new();
    if colour != crate::state::PLAIN {
        codes.push(30 + colour);
    }
    if fill != crate::state::PLAIN {
        codes.push(40 + fill);
    }
    if codes.is_empty() {
        return character.to_string();
    }
    let joined = codes
        .iter()
        .map(|code| code.to_string())
        .collect::<Vec<_>>()
        .join(";");
    format!("\x1b[{joined}m{character}{RESET}")
}

pub(crate) struct Canvas {
    pub(crate) first_x: i64,
    pub(crate) last_x: i64,
    pub(crate) first_y: i64,
    pub(crate) last_y: i64,
    ink: [u8; 4],
    span: i64,
    buffer: Vec<u8>,
}

impl Canvas {
    pub(crate) fn new(first_x: i64, last_x: i64, first_y: i64, last_y: i64, ink: [u8; 4]) -> Self {
        let span = last_x - first_x;
        let buffer = vec![0u8; (span * (last_y - first_y) * 4) as usize];
        Canvas {
            first_x,
            last_x,
            first_y,
            last_y,
            ink,
            span,
            buffer,
        }
    }

    pub(crate) fn point(&mut self, x: i64, y: i64, width: i64) {
        for px in centered_span(x, width) {
            for py in centered_span(y, width) {
                if self.first_x <= px && px < self.last_x && self.first_y <= py && py < self.last_y
                {
                    let start = self.offset(px, py);
                    self.buffer[start..start + 4].copy_from_slice(&self.ink);
                }
            }
        }
    }

    pub(crate) fn horizontal(&mut self, y: i64, x0: i64, x1: i64, width: i64) {
        let start_x = x0.max(self.first_x);
        let stop_x = (x1 + 1).min(self.last_x);
        if start_x >= stop_x {
            return;
        }
        for py in centered_span(y, width) {
            if !(self.first_y <= py && py < self.last_y) {
                continue;
            }
            let start = self.offset(start_x, py);
            for i in 0..(stop_x - start_x) as usize {
                self.buffer[start + i * 4..start + i * 4 + 4].copy_from_slice(&self.ink);
            }
        }
    }

    pub(crate) fn vertical(&mut self, x: i64, y0: i64, y1: i64, width: i64) {
        for px in centered_span(x, width) {
            if !(self.first_x <= px && px < self.last_x) {
                continue;
            }
            for y in y0.max(self.first_y)..(y1 + 1).min(self.last_y) {
                let start = self.offset(px, y);
                self.buffer[start..start + 4].copy_from_slice(&self.ink);
            }
        }
    }

    pub(crate) fn pixels(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    fn offset(&self, x: i64, y: i64) -> usize {
        (((y - self.first_y) * self.span + (x - self.first_x)) * 4) as usize
    }
}

fn python_round(value: f64) -> f64 {
    let floor = value.floor();
    let diff = value - floor;
    if diff < 0.5 {
        floor
    } else if diff > 0.5 {
        floor + 1.0
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    }
}

pub(crate) fn body_row(
    width: i64,
    border: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
    first_x: i64,
    last_x: i64,
) -> Vec<u8> {
    let span = last_x - first_x;
    let edge_px = [edge.0, edge.1, edge.2, edge.3];
    let fill_px = [fill.0, fill.1, fill.2, fill.3];
    if width <= 2 * border {
        return edge_px.repeat(span.max(0) as usize);
    }
    let mut row = Vec::new();
    let left_edge = (border - first_x).max(0);
    row.extend(edge_px.repeat(left_edge as usize));
    let right_edge = (border - (width - last_x)).max(0);
    let fill_count = (span - left_edge - right_edge).max(0);
    row.extend(fill_px.repeat(fill_count as usize));
    row.extend(edge_px.repeat(right_edge as usize));
    row
}

pub(crate) fn square_pixels(
    width: i64,
    height: i64,
    border: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
    first_x: i64,
    last_x: i64,
    first_y: i64,
    last_y: i64,
) -> Vec<u8> {
    let edge_px = [edge.0, edge.1, edge.2, edge.3];
    let edge_row = edge_px.repeat((last_x - first_x).max(0) as usize);
    let body = body_row(width, border, edge, fill, first_x, last_x);
    let mut pixels = Vec::new();
    if height <= 2 * border {
        pixels.extend(edge_row.repeat((last_y - first_y).max(0) as usize));
    } else {
        let top_edge = (border - first_y).max(0);
        pixels.extend(edge_row.repeat(top_edge as usize));
        let bottom_edge = (border - (height - last_y)).max(0);
        let body_count = ((last_y - first_y) - top_edge - bottom_edge).max(0);
        pixels.extend(body.repeat(body_count as usize));
        pixels.extend(edge_row.repeat(bottom_edge as usize));
    }
    pixels
}

pub(crate) struct RoundedBox {
    width: i64,
    height: i64,
    border: i64,
    radius: i64,
    outer: i64,
    edge: (u8, u8, u8, u8),
    fill: (u8, u8, u8, u8),
}

impl RoundedBox {
    pub(crate) fn new(
        width: i64,
        height: i64,
        radius: i64,
        border: i64,
        edge: (u8, u8, u8, u8),
        fill: (u8, u8, u8, u8),
    ) -> Self {
        let outer = (radius + border).min(width / 2).min(height / 2);
        RoundedBox { width, height, border, radius, outer, edge, fill }
    }

    pub(crate) fn pixels(&self, first_x: i64, last_x: i64, first_y: i64, last_y: i64) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut straight_row: Option<Vec<u8>> = None;
        for y in first_y..last_y {
            if self.outer <= y && y < self.height - self.outer {
                if straight_row.is_none() {
                    straight_row = Some(body_row(
                        self.width, self.border, self.edge, self.fill, first_x, last_x,
                    ));
                }
                buffer.extend(straight_row.as_ref().unwrap());
            } else {
                buffer.extend(self.corner_row(y, first_x, last_x));
            }
        }
        buffer
    }

    fn corner_row(&self, y: i64, first_x: i64, last_x: i64) -> Vec<u8> {
        let mut row = Vec::new();
        for x in first_x..self.outer.min(last_x) {
            row.extend(self.pixel(x, y));
        }
        let middle = (self.width - self.outer).min(last_x) - self.outer.max(first_x);
        if middle > 0 {
            let straight = if y < self.border || y >= self.height - self.border {
                self.edge
            } else {
                self.fill
            };
            let straight_px = [straight.0, straight.1, straight.2, straight.3];
            row.extend(straight_px.repeat(middle as usize));
        }
        for x in (self.width - self.outer).max(first_x)..last_x {
            row.extend(self.pixel(x, y));
        }
        row
    }

    fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let outer_coverage =
            Self::coverage(px, py, self.width as f64, self.height as f64, self.outer as f64);
        let inner_coverage = Self::coverage(
            px - self.border as f64,
            py - self.border as f64,
            (self.width - 2 * self.border) as f64,
            (self.height - 2 * self.border) as f64,
            self.radius as f64,
        );
        let edge_coverage = outer_coverage - inner_coverage;
        let alpha = edge_coverage * self.edge.3 as f64 + inner_coverage * self.fill.3 as f64;
        if alpha == 0.0 {
            return [0, 0, 0, 0];
        }
        let edge_channels = [self.edge.0, self.edge.1, self.edge.2];
        let fill_channels = [self.fill.0, self.fill.1, self.fill.2];
        let mut channels = [0u8; 4];
        for c in 0..3 {
            let value = (edge_channels[c] as f64 * edge_coverage * self.edge.3 as f64
                + fill_channels[c] as f64 * inner_coverage * self.fill.3 as f64)
                / alpha;
            channels[c] = python_round(value) as u8;
        }
        channels[3] = python_round(alpha) as u8;
        channels
    }

    fn coverage(px: f64, py: f64, width: f64, height: f64, radius: f64) -> f64 {
        let half_x = width / 2.0;
        let half_y = height / 2.0;
        let qx = (px - half_x).abs() - (half_x - radius);
        let qy = (py - half_y).abs() - (half_y - radius);
        let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
        (0.5 - distance).max(0.0).min(1.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SpriteKey {
    Box {
        width: i64,
        height: i64,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
        colour: i64,
        fill: i64,
        rounded: bool,
    },
    Arrow {
        width: i64,
        height: i64,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
        stops: Vec<i64>,
        shaft: i64,
    },
}

pub(crate) fn sprite_key(
    placement: &crate::layout::Placement,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
) -> SpriteKey {
    use crate::layout::PlacementNode;
    match &placement.node {
        PlacementNode::Node(node) => SpriteKey::Box {
            width: placement.width,
            height: placement.height,
            left: left - placement.x,
            top: top - placement.y,
            right: right - placement.x,
            bottom: bottom - placement.y,
            colour: node.colour,
            fill: node.fill,
            rounded: node.rounded,
        },
        PlacementNode::Arrow(arrow) => SpriteKey::Arrow {
            width: placement.width,
            height: placement.height,
            left: left - placement.x,
            top: top - placement.y,
            right: right - placement.x,
            bottom: bottom - placement.y,
            stops: arrow.stops.clone(),
            shaft: arrow.shaft,
        },
        _ => unreachable!("sprite_key is only called for Box and Arrow placements"),
    }
}

pub(crate) const BLANK_CELL: (char, i64, i64) = (BLANK, crate::state::PLAIN, crate::state::PLAIN);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sprite {
    pub(crate) pixels: Vec<u8>,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) col: i64,
    pub(crate) row: i64,
}

pub(crate) struct TerminalRenderer {
    pub(crate) graphics: crate::KittyGraphics,
    pub(crate) cell_width: i64,
    pub(crate) cell_height: i64,
    pub(crate) cache: std::collections::HashMap<SpriteKey, Sprite>,
}

impl TerminalRenderer {
    pub(crate) fn new(graphics: crate::KittyGraphics, cell_width: i64, cell_height: i64) -> Self {
        TerminalRenderer {
            graphics,
            cell_width,
            cell_height,
            cache: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn render(
        &mut self,
        placements: &[crate::layout::Placement],
        cols: i64,
        rows: i64,
    ) -> Vec<String> {
        let mut lines = self.grid(placements, cols, rows);
        let sprites = self.sprites(placements, cols, rows);
        let payload = self.graphics.draw(sprites);
        let last = lines.len() - 1;
        lines[last] = format!("{}{}", lines[last], payload);
        lines
    }

    fn grid(&self, placements: &[crate::layout::Placement], cols: i64, rows: i64) -> Vec<String> {
        use crate::layout::PlacementNode;

        let mut grid = vec![vec![BLANK_CELL; cols as usize]; rows as usize];
        for placement in placements {
            match &placement.node {
                PlacementNode::Node(_) => self.draw_box(&mut grid, placement),
                PlacementNode::Cursor(_) => self.draw_cursor(&mut grid, placement),
                PlacementNode::Label(label) => self.draw_label(&mut grid, placement, label),
                PlacementNode::Arrow(_) => {}
            }
        }
        grid.into_iter()
            .map(|row| row.into_iter().map(|(c, colour, fill)| cell(c, colour, fill)).collect())
            .collect()
    }

    fn draw_box(&self, grid: &mut [Vec<(char, i64, i64)>], placement: &crate::layout::Placement) {
        for y in placement.y..placement.y + placement.height {
            for x in placement.x..placement.x + placement.width {
                self.put(grid, x, y, BLANK_CELL);
            }
        }
    }

    fn draw_cursor(&self, grid: &mut [Vec<(char, i64, i64)>], placement: &crate::layout::Placement) {
        self.stamp(grid, placement.x, placement.y, CURSOR);
    }

    fn draw_label(
        &self,
        grid: &mut [Vec<(char, i64, i64)>],
        placement: &crate::layout::Placement,
        label: &crate::layout::Label,
    ) {
        for (offset, character) in label.text.chars().enumerate() {
            self.stamp(grid, placement.x + offset as i64, placement.y, character);
        }
    }

    fn put(&self, grid: &mut [Vec<(char, i64, i64)>], x: i64, y: i64, cell: (char, i64, i64)) {
        if 0 <= y && (y as usize) < grid.len() {
            let row = &mut grid[y as usize];
            if 0 <= x && (x as usize) < row.len() {
                row[x as usize] = cell;
            }
        }
    }

    fn stamp(&self, grid: &mut [Vec<(char, i64, i64)>], x: i64, y: i64, character: char) {
        if 0 <= y && (y as usize) < grid.len() {
            let row = &mut grid[y as usize];
            if 0 <= x && (x as usize) < row.len() {
                let (_, colour, fill) = row[x as usize];
                row[x as usize] = (character, colour, fill);
            }
        }
    }

    fn sprites(&mut self, placements: &[crate::layout::Placement], cols: i64, rows: i64) -> Vec<Sprite> {
        use crate::layout::PlacementNode;

        let mut sprites = Vec::new();
        for placement in placements {
            if !matches!(placement.node, PlacementNode::Node(_) | PlacementNode::Arrow(_)) {
                continue;
            }
            let left = placement.x.max(0);
            let top = placement.y.max(0);
            let right = (placement.x + placement.width).min(cols);
            let bottom = (placement.y + placement.height).min(rows);
            if left >= right || top >= bottom {
                continue;
            }
            sprites.push(self.sprite(placement, left, top, right, bottom));
        }
        sprites
    }

    fn sprite(
        &mut self,
        placement: &crate::layout::Placement,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
    ) -> Sprite {
        use crate::layout::PlacementNode;

        let key = sprite_key(placement, left, top, right, bottom);
        if !self.cache.contains_key(&key) {
            let drawn = match &placement.node {
                PlacementNode::Node(_) => self.outline_box(placement, left, top, right, bottom),
                _ => self.outline_arrow(placement, left, top, right, bottom),
            };
            if self.cache.len() >= CACHE_LIMIT {
                self.cache.clear();
            }
            self.cache.insert(key.clone(), drawn);
        }
        let drawn = &self.cache[&key];
        Sprite {
            pixels: drawn.pixels.clone(),
            width: drawn.width,
            height: drawn.height,
            col: left,
            row: top,
        }
    }

    fn outline_box(
        &self,
        placement: &crate::layout::Placement,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
    ) -> Sprite {
        use crate::layout::PlacementNode;

        let node = match &placement.node {
            PlacementNode::Node(node) => node,
            _ => unreachable!("outline_box is only called for Box placements"),
        };
        let width = placement.width * self.cell_width;
        let height = placement.height * self.cell_height;
        let border = BORDER;
        let (r, g, b) = colour(node.colour);
        let edge = (r, g, b, OPAQUE);
        let fill = fill_colour(node.fill);
        let first_x = (left - placement.x) * self.cell_width;
        let last_x = (right - placement.x) * self.cell_width;
        let first_y = (top - placement.y) * self.cell_height;
        let last_y = (bottom - placement.y) * self.cell_height;
        let span = last_x - first_x;
        let radius = if node.rounded { ROUNDED_RADIUS } else { 0 };
        let pixels = if radius != 0 {
            RoundedBox::new(width, height, radius, border, edge, fill)
                .pixels(first_x, last_x, first_y, last_y)
        } else {
            square_pixels(width, height, border, edge, fill, first_x, last_x, first_y, last_y)
        };
        Sprite { pixels, width: span, height: last_y - first_y, col: left, row: top }
    }

    fn outline_arrow(
        &self,
        placement: &crate::layout::Placement,
        left: i64,
        top: i64,
        right: i64,
        bottom: i64,
    ) -> Sprite {
        use crate::layout::PlacementNode;

        let arrow = match &placement.node {
            PlacementNode::Arrow(arrow) => arrow,
            _ => unreachable!("outline_arrow is only called for Arrow placements"),
        };
        let width = placement.width * self.cell_width;
        let stop_rows: Vec<i64> = arrow
            .stops
            .iter()
            .map(|stop| stop * self.cell_height + self.cell_height / 2)
            .collect();
        let shaft_row = arrow.shaft * self.cell_height + self.cell_height / 2;
        let trunk_top = *stop_rows.iter().min().expect("an arrow always has at least one stop");
        let trunk_bottom = *stop_rows.iter().max().expect("an arrow always has at least one stop");
        let midpoint = width / 2;
        let (r, g, b) = colour(crate::state::PLAIN);
        let ink = [r, g, b, OPAQUE];
        let first_x = (left - placement.x) * self.cell_width;
        let last_x = (right - placement.x) * self.cell_width;
        let first_y = (top - placement.y) * self.cell_height;
        let last_y = (bottom - placement.y) * self.cell_height;
        let mut canvas = Canvas::new(first_x, last_x, first_y, last_y, ink);
        canvas.horizontal(shaft_row, 0, midpoint, ARROW_STROKE);
        canvas.vertical(midpoint, trunk_top, trunk_bottom, ARROW_STROKE);
        for &stop_row in &stop_rows {
            canvas.horizontal(stop_row, midpoint, width - 1, ARROW_STROKE);
            self.arrowhead(&mut canvas, stop_row, midpoint, width - 1);
        }
        Sprite {
            pixels: canvas.pixels(),
            width: last_x - first_x,
            height: last_y - first_y,
            col: left,
            row: top,
        }
    }

    fn arrowhead(&self, canvas: &mut Canvas, stop_row: i64, midpoint: i64, right_edge: i64) {
        let depth = arrowhead_depth();
        let slope = arrowhead_slope();
        for distance in 0..=(depth as i64) {
            if distance as f64 >= depth {
                break;
            }
            let x = right_edge - distance;
            if x < midpoint {
                break;
            }
            let spread = python_round(distance as f64 * slope) as i64;
            canvas.point(x, stop_row - spread, ARROW_STROKE);
            canvas.point(x, stop_row + spread, ARROW_STROKE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink() -> [u8; 4] {
        let (r, g, b) = colour(1);
        [r, g, b, OPAQUE]
    }

    fn canvas(first_x: i64, last_x: i64, first_y: i64, last_y: i64) -> Canvas {
        Canvas::new(first_x, last_x, first_y, last_y, ink())
    }

    fn pixel(canvas: &Canvas, x: i64, y: i64) -> (u8, u8, u8, u8) {
        let offset = (((y - canvas.first_y) * canvas.span + (x - canvas.first_x)) * 4) as usize;
        (
            canvas.buffer[offset],
            canvas.buffer[offset + 1],
            canvas.buffer[offset + 2],
            canvas.buffer[offset + 3],
        )
    }

    fn ink_pixel() -> (u8, u8, u8, u8) {
        let [r, g, b, a] = ink();
        (r, g, b, a)
    }

    fn blank() -> (u8, u8, u8, u8) {
        TRANSPARENT
    }

    #[test]
    fn point_with_width_one_stamps_a_single_pixel() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(10, 10, 1);
        assert_eq!(pixel(&canvas, 10, 10), ink_pixel());
        for (x, y) in [(9, 10), (11, 10), (10, 9), (10, 11)] {
            assert_eq!(pixel(&canvas, x, y), blank());
        }
    }

    #[test]
    fn point_with_width_four_stamps_a_four_by_four_block() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(10, 10, 4);
        for x in [9, 10, 11, 12] {
            for y in [9, 10, 11, 12] {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for (x, y) in [(8, 10), (13, 10), (10, 8), (10, 13)] {
            assert_eq!(pixel(&canvas, x, y), blank());
        }
    }

    #[test]
    fn horizontal_with_width_four_paints_four_rows() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.horizontal(10, 2, 6, 4);
        for y in [9, 10, 11, 12] {
            for x in 2..7 {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for y in [8, 13] {
            for x in 2..7 {
                assert_eq!(pixel(&canvas, x, y), blank());
            }
        }
        assert_eq!(pixel(&canvas, 1, 10), blank());
        assert_eq!(pixel(&canvas, 7, 10), blank());
    }

    #[test]
    fn vertical_with_width_four_paints_four_columns() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.vertical(10, 2, 6, 4);
        for x in [9, 10, 11, 12] {
            for y in 2..7 {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
        for x in [8, 13] {
            for y in 2..7 {
                assert_eq!(pixel(&canvas, x, y), blank());
            }
        }
        assert_eq!(pixel(&canvas, 10, 1), blank());
        assert_eq!(pixel(&canvas, 10, 7), blank());
    }

    #[test]
    fn a_thick_point_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.point(0, 0, 4);
        for x in [0, 1] {
            for y in [0, 1] {
                assert_eq!(pixel(&canvas, x, y), ink_pixel());
            }
        }
    }

    #[test]
    fn a_thick_horizontal_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.horizontal(0, 2, 6, 4);
        for x in 2..7 {
            assert_eq!(pixel(&canvas, x, 0), ink_pixel());
            assert_eq!(pixel(&canvas, x, 1), ink_pixel());
        }
    }

    #[test]
    fn a_thick_vertical_near_the_edge_is_clipped() {
        let mut canvas = canvas(0, 20, 0, 20);
        canvas.vertical(0, 2, 6, 4);
        for y in 2..7 {
            assert_eq!(pixel(&canvas, 0, y), ink_pixel());
            assert_eq!(pixel(&canvas, 1, y), ink_pixel());
        }
    }

    #[test]
    fn colour_of_plain_is_the_plain_grey() {
        assert_eq!(colour(crate::state::PLAIN), PLAIN_COLOUR);
    }

    #[test]
    fn colour_of_a_palette_index_is_the_palette_entry() {
        assert_eq!(colour(2), PALETTE[2]);
    }

    #[test]
    fn fill_colour_of_plain_is_transparent() {
        assert_eq!(fill_colour(crate::state::PLAIN), TRANSPARENT);
    }

    #[test]
    fn fill_colour_of_a_palette_index_is_alpha_composited_and_opaque() {
        let (r, g, b) = PALETTE[2];
        let round = |channel: u8| (channel as f64 * FILL_ALPHA as f64 / OPAQUE as f64).round() as u8;
        let expected = (round(r), round(g), round(b), OPAQUE);
        assert_eq!(fill_colour(2), expected);
    }

    #[test]
    fn cell_with_plain_colour_and_fill_is_the_bare_character() {
        assert_eq!(cell('x', crate::state::PLAIN, crate::state::PLAIN), "x");
    }

    #[test]
    fn cell_with_a_colour_only_emits_a_foreground_code() {
        assert_eq!(cell('x', 2, crate::state::PLAIN), "\x1b[32mx\x1b[0m");
    }

    #[test]
    fn cell_with_a_fill_only_emits_a_background_code() {
        assert_eq!(cell('x', crate::state::PLAIN, 3), "\x1b[43mx\x1b[0m");
    }

    #[test]
    fn cell_with_colour_and_fill_emits_both_codes() {
        assert_eq!(cell('x', 1, 4), "\x1b[31;44mx\x1b[0m");
    }

    fn edge_rgba(index: i64) -> (u8, u8, u8, u8) {
        let (r, g, b) = colour(index);
        (r, g, b, OPAQUE)
    }

    fn pixel_at(pixels: &[u8], width: i64, x: i64, y: i64) -> (u8, u8, u8, u8) {
        let offset = ((y * width + x) * 4) as usize;
        (pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3])
    }

    #[test]
    fn plain_fill_renders_transparent_interior() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(crate::state::PLAIN);
        let fill = fill_colour(crate::state::PLAIN);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_and_made_opaque() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(crate::state::PLAIN);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), fill_colour(2));
    }

    #[test]
    fn border_pixels_are_unaffected_by_fill() {
        let size = 2 * BORDER + 3;
        let edge = edge_rgba(3);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        assert_eq!(pixel_at(&pixels, size, 0, 0), edge_rgba(3));
        assert_eq!(pixel_at(&pixels, size, BORDER + 1, BORDER + 1), fill_colour(2));
    }

    #[test]
    fn a_border_is_bold_at_every_edge() {
        let size = 3 * 4;
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = square_pixels(size, size, BORDER, edge, fill, 0, size, 0, size);
        for offset in 0..BORDER {
            assert_eq!(pixel_at(&pixels, size, 5, offset), edge);
            assert_eq!(pixel_at(&pixels, size, 5, size - 1 - offset), edge);
        }
        assert_eq!(pixel_at(&pixels, size, 5, BORDER), fill);
        assert_eq!(pixel_at(&pixels, size, 5, size - 1 - BORDER), fill);
        for offset in 0..BORDER {
            assert_eq!(pixel_at(&pixels, size, offset, 5), edge);
            assert_eq!(pixel_at(&pixels, size, size - 1 - offset, 5), edge);
        }
        assert_eq!(pixel_at(&pixels, size, BORDER, 5), fill);
        assert_eq!(pixel_at(&pixels, size, size - 1 - BORDER, 5), fill);
    }

    const CORNER_SIZE: i64 = 80;

    #[test]
    fn a_square_box_is_built_from_flat_edge_and_body_rows() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels =
            square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);

        let edge_px = [edge.0, edge.1, edge.2, edge.3];
        let fill_px = [fill.0, fill.1, fill.2, fill.3];
        let edge_row = edge_px.repeat(CORNER_SIZE as usize);
        let mut expected_body_row = Vec::new();
        expected_body_row.extend(edge_px.repeat(BORDER as usize));
        expected_body_row.extend(fill_px.repeat((CORNER_SIZE - 2 * BORDER) as usize));
        expected_body_row.extend(edge_px.repeat(BORDER as usize));

        let mut expected = Vec::new();
        expected.extend(edge_row.repeat(BORDER as usize));
        expected.extend(expected_body_row.repeat((CORNER_SIZE - 2 * BORDER) as usize));
        expected.extend(edge_row.repeat(BORDER as usize));
        assert_eq!(pixels, expected);
    }

    #[test]
    fn a_rounded_box_cuts_away_its_extreme_corners() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let rounded = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let pixels = rounded.pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (last_x, last_y) = (CORNER_SIZE - 1, CORNER_SIZE - 1);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn straight_edges_stay_as_crisp_as_a_square_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let middle_y = CORNER_SIZE / 2;
        let middle_x = CORNER_SIZE / 2;
        for x in 0..CORNER_SIZE {
            assert_eq!(
                pixel_at(&rounded, CORNER_SIZE, x, middle_y),
                pixel_at(&square, CORNER_SIZE, x, middle_y)
            );
        }
        for y in 0..CORNER_SIZE {
            assert_eq!(
                pixel_at(&rounded, CORNER_SIZE, middle_x, y),
                pixel_at(&square, CORNER_SIZE, middle_x, y)
            );
        }
    }

    #[test]
    fn the_arc_is_anti_aliased() {
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        assert!(!square.iter().skip(3).step_by(4).any(|&alpha| alpha > 0 && alpha < OPAQUE));
        assert!(rounded.iter().skip(3).step_by(4).any(|&alpha| alpha > 0 && alpha < OPAQUE));
    }

    #[test]
    fn arc_coverage_is_continuous_at_the_pixel_centre() {
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (r, g, b) = colour(1);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 14, 2), (r, g, b, 254));
    }

    #[test]
    fn border_coverage_is_composed_over_the_opaque_fill() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        assert_eq!(pixel_at(&pixels, CORNER_SIZE, 20, 4), (131, 27, 25, OPAQUE));
    }

    #[test]
    fn a_rounded_box_cuts_away_more_than_a_square_one() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let square = square_pixels(CORNER_SIZE, CORNER_SIZE, BORDER, edge, fill, 0, CORNER_SIZE, 0, CORNER_SIZE);
        let rounded =
            RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
                .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let alpha_total = |pixels: &[u8]| pixels.iter().skip(3).step_by(4).map(|&a| a as u64).sum::<u64>();
        assert!(alpha_total(&rounded) < alpha_total(&square));
    }

    #[test]
    fn the_fringe_keeps_the_edge_colour_instead_of_fading_to_black() {
        let edge = edge_rgba(1);
        let fill = TRANSPARENT;
        let pixels = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let (r, g, b) = colour(1);
        let mut found_partial = false;
        for y in 0..CORNER_SIZE {
            for x in 0..CORNER_SIZE {
                let (pr, pg, pb, pa) = pixel_at(&pixels, CORNER_SIZE, x, y);
                if pa > 0 && pa < OPAQUE {
                    found_partial = true;
                    assert_eq!((pr, pg, pb), (r, g, b));
                }
            }
        }
        assert!(found_partial);
    }

    #[test]
    fn clipping_a_rounded_box_is_a_pure_crop_of_the_whole_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let cell_width = 8;
        let hidden_cols = 2;
        let offset = hidden_cols * cell_width;
        let rounded_box = RoundedBox::new(CORNER_SIZE, CORNER_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let whole = rounded_box.pixels(0, CORNER_SIZE, 0, CORNER_SIZE);
        let clipped = rounded_box.pixels(offset, CORNER_SIZE, 0, CORNER_SIZE);
        let clipped_width = CORNER_SIZE - offset;
        for y in 0..CORNER_SIZE {
            for x in 0..clipped_width {
                assert_eq!(
                    pixel_at(&clipped, clipped_width, x, y),
                    pixel_at(&whole, CORNER_SIZE, x + offset, y)
                );
            }
        }
    }

    const SMALL_SIZE: i64 = 20;

    #[test]
    fn the_sprite_holds_exactly_one_pixel_per_cell_of_its_area() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let pixels = RoundedBox::new(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, BORDER, edge, fill)
            .pixels(0, SMALL_SIZE, 0, SMALL_SIZE);
        assert_eq!(pixels.len() as i64, SMALL_SIZE * SMALL_SIZE * 4);
    }

    #[test]
    fn clipping_a_small_box_is_a_pure_crop_of_the_whole_box() {
        let edge = edge_rgba(1);
        let fill = fill_colour(2);
        let cell_width = 10;
        let hidden_cols = 1;
        let offset = hidden_cols * cell_width;
        let small_box = RoundedBox::new(SMALL_SIZE, SMALL_SIZE, ROUNDED_RADIUS, BORDER, edge, fill);
        let whole = small_box.pixels(0, SMALL_SIZE, 0, SMALL_SIZE);
        let clipped = small_box.pixels(offset, SMALL_SIZE, 0, SMALL_SIZE);
        let clipped_width = SMALL_SIZE - offset;
        assert_eq!(clipped.len() as i64, clipped_width * SMALL_SIZE * 4);
        for y in 0..SMALL_SIZE {
            for x in 0..clipped_width {
                assert_eq!(
                    pixel_at(&clipped, clipped_width, x, y),
                    pixel_at(&whole, SMALL_SIZE, x + offset, y)
                );
            }
        }
    }

    fn box_node(colour: i64, fill: i64, rounded: bool) -> crate::state::Node {
        crate::state::Node { label: String::new(), colour, fill, rounded, children: vec![] }
    }

    fn box_placement(node: &crate::state::Node, x: i64, y: i64, width: i64, height: i64) -> crate::layout::Placement<'_> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Node(node),
            x,
            y,
            width,
            height,
        }
    }

    fn arrow_placement(
        stops: Vec<i64>,
        shaft: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> crate::layout::Placement<'static> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Arrow(crate::layout::Arrow { stops, shaft }),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn sprite_key_of_two_identically_shaped_boxes_is_equal() {
        let node_a = box_node(1, 2, true);
        let node_b = box_node(1, 2, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_eq!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_colour() {
        let node_a = box_node(1, 2, true);
        let node_b = box_node(2, 2, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_fill() {
        let node_a = box_node(1, 2, true);
        let node_b = box_node(1, 3, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_rounded() {
        let node_a = box_node(1, 2, true);
        let node_b = box_node(1, 2, false);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        let b = box_placement(&node_b, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&b, 0, 0, 10, 10));
    }

    #[test]
    fn sprite_key_differs_by_crop() {
        let node_a = box_node(1, 2, true);
        let a = box_placement(&node_a, 0, 0, 10, 10);
        assert_ne!(sprite_key(&a, 0, 0, 10, 10), sprite_key(&a, 1, 0, 10, 10));
    }

    #[test]
    fn sprite_key_of_arrows_with_different_stops_differs() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 3], 1, 0, 0, 4, 3);
        assert_ne!(sprite_key(&a, 0, 0, 4, 3), sprite_key(&b, 0, 0, 4, 3));
    }

    #[test]
    fn sprite_key_of_identical_arrows_is_equal() {
        let a = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        let b = arrow_placement(vec![0, 2], 1, 0, 0, 4, 3);
        assert_eq!(sprite_key(&a, 0, 0, 4, 3), sprite_key(&b, 0, 0, 4, 3));
    }

    fn renderer(cell_width: i64, cell_height: i64) -> TerminalRenderer {
        TerminalRenderer::new(crate::KittyGraphics::new(), cell_width, cell_height)
    }

    fn label_placement(text: &str, x: i64, y: i64, width: i64, height: i64) -> crate::layout::Placement<'_> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Label(crate::layout::Label {
                text,
                path: crate::state::Path { head: 0, tail: vec![] },
            }),
            x,
            y,
            width,
            height,
        }
    }

    fn cursor_placement(x: i64, y: i64, width: i64, height: i64) -> crate::layout::Placement<'static> {
        crate::layout::Placement {
            node: crate::layout::PlacementNode::Cursor(crate::layout::Cursor),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn line_count_is_unchanged() {
        let mut r = renderer(2, 4);
        assert_eq!(r.render(&[], 3, 3).len(), 3);
    }

    #[test]
    fn the_graphics_payload_is_appended_to_the_last_line_only() {
        let mut r = renderer(2, 4);
        let lines = r.render(&[], 3, 2);
        assert_eq!(lines[0], BLANK.to_string().repeat(3));
        assert_eq!(lines[1], format!("{}{}", BLANK.to_string().repeat(3), crate::DELETE_ALL));
    }

    #[test]
    fn empty_canvas_fills_terminal() {
        let grid = renderer(1, 1).grid(&[], 11, 5);
        assert_eq!(grid, vec![BLANK.to_string().repeat(11); 5]);
    }

    #[test]
    fn grid_matches_the_requested_size() {
        let (cols, rows) = (20, 7);
        let grid = renderer(1, 1).grid(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 4, 4, 3, 3)], cols, rows);
        assert_eq!(grid.len() as i64, rows);
        for line in &grid {
            assert_eq!(line.chars().count() as i64, cols);
        }
    }

    #[test]
    fn a_box_claims_its_cells_without_border_characters() {
        let grid = renderer(1, 1).grid(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 4, 4, 3, 3)], 11, 11);
        assert_eq!(&grid[4][4..7], "   ");
    }

    #[test]
    fn a_box_reaching_past_the_edge_is_clipped() {
        let grid = renderer(1, 1).grid(&[box_placement(&box_node(crate::state::PLAIN, 3, false), 3, 1, 3, 3)], 4, 2);
        assert_eq!(grid, vec!["    ".to_string(), "    ".to_string()]);
    }

    #[test]
    fn label_is_drawn_inside_the_box() {
        let node = box_node(crate::state::PLAIN, crate::state::PLAIN, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
        ];
        let grid = renderer(1, 1).grid(&placements, 5, 3);
        assert_eq!(grid[1], " hi  ");
    }

    #[test]
    fn cursor_is_drawn_after_the_label() {
        let node = box_node(crate::state::PLAIN, crate::state::PLAIN, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            cursor_placement(3, 1, 1, 1),
        ];
        let grid = renderer(1, 1).grid(&placements, 5, 3);
        assert_eq!(grid[1], format!(" hi{} ", CURSOR));
    }

    #[test]
    fn label_and_cursor_past_the_edge_are_clipped() {
        let node = box_node(crate::state::PLAIN, crate::state::PLAIN, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            cursor_placement(3, 1, 1, 1),
        ];
        let grid = renderer(1, 1).grid(&placements, 3, 3);
        assert_eq!(grid[1], " hi");
    }

    #[test]
    fn a_box_does_not_draw_a_cursor() {
        let grid = renderer(1, 1).grid(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 5, 3)], 5, 3);
        assert!(!grid.join("").contains(CURSOR));
    }

    #[test]
    fn cursor_placement_is_drawn_at_its_own_position() {
        let grid = renderer(1, 1).grid(&[cursor_placement(2, 1, 1, 1)], 4, 3);
        assert_eq!(
            grid,
            vec!["    ".to_string(), format!("  {} ", CURSOR), "    ".to_string()]
        );
    }

    #[test]
    fn a_cursor_outside_the_grid_is_clipped() {
        let grid = renderer(1, 1).grid(&[cursor_placement(9, 9, 1, 1)], 4, 3);
        assert_eq!(grid, vec!["    ".to_string(); 3]);
    }

    #[test]
    fn an_arrow_leaves_the_gap_blank() {
        let grid = renderer(1, 1).grid(&[arrow_placement(vec![0], 0, 2, 1, 1, 2)], 4, 4);
        assert_eq!(grid, vec!["    ".to_string(); 4]);
    }

    #[test]
    fn a_plain_box_emits_no_escapes() {
        let node = box_node(crate::state::PLAIN, crate::state::PLAIN, false);
        let placements = vec![
            box_placement(&node, 0, 0, 5, 3),
            label_placement("hi", 1, 1, 2, 1),
            cursor_placement(3, 1, 1, 1),
        ];
        let grid = renderer(1, 1).grid(&placements, 5, 3);
        assert!(!grid.join("").contains('\x1b'));
    }

    #[test]
    fn a_coloured_box_puts_no_colour_in_the_grid() {
        let grid = renderer(1, 1).grid(&[box_placement(&box_node(2, crate::state::PLAIN, false), 0, 0, 5, 3)], 5, 3);
        assert_eq!(grid, vec!["     ".to_string(); 3]);
    }

    #[test]
    fn a_cursor_has_no_sprite() {
        let mut r = renderer(2, 4);
        let sprites = r.sprites(&[cursor_placement(1, 1, 1, 1)], 40, 20);
        assert!(sprites.is_empty());
    }

    #[test]
    fn a_box_off_screen_has_no_sprite() {
        let mut r = renderer(4, 4);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 10, 0, 4, 3)], 5, 20);
        assert!(sprites.is_empty());
    }

    #[test]
    fn a_box_overhanging_the_left_is_cropped() {
        let mut r = renderer(4, 4);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), -2, 1, 5, 3)], 40, 20);
        assert_eq!(sprites.len(), 1);
        assert_eq!(sprites[0].col, 0);
        assert_eq!(sprites[0].width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_top_is_cropped() {
        let mut r = renderer(4, 4);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 1, -2, 4, 5)], 40, 20);
        assert_eq!(sprites[0].row, 0);
        assert_eq!(sprites[0].height, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_right_is_cropped() {
        let mut r = renderer(4, 4);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 1, 0, 6, 3)], 4, 20);
        assert_eq!(sprites[0].col, 1);
        assert_eq!(sprites[0].width, 3 * 4);
    }

    #[test]
    fn a_box_overhanging_the_bottom_is_cropped() {
        let mut r = renderer(4, 4);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 1, 3, 6)], 40, 4);
        assert_eq!(sprites[0].row, 1);
        assert_eq!(sprites[0].height, 3 * 4);
    }

    #[test]
    fn a_box_sprite_sits_at_the_placement_cell() {
        let mut r = renderer(6, 12);
        let sprites = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 1, 2, 4, 3)], 40, 20);
        assert_eq!((sprites[0].col, sprites[0].row), (1, 2));
        assert_eq!((sprites[0].width, sprites[0].height), (24, 36));
    }

    #[test]
    fn a_border_takes_the_colour_of_its_palette_index() {
        for index in 0..PALETTE.len() as i64 {
            let mut r = renderer(2, 4);
            let sprites = r.sprites(&[box_placement(&box_node(index, crate::state::PLAIN, false), 0, 0, 2, 2)], 40, 20);
            let (px, py, pz) = colour(index);
            assert_eq!(&sprites[0].pixels[0..4], &[px, py, pz, OPAQUE]);
        }
    }

    #[test]
    fn an_unchanged_box_is_not_redrawn() {
        let mut r = renderer(2, 4);
        let node = box_node(1, 2, false);
        let placement = box_placement(&node, 0, 0, 4, 3);
        let first = r.sprites(&[placement.clone()], 40, 20);
        assert_eq!(r.cache.len(), 1);
        let second = r.sprites(&[placement], 40, 20);
        assert_eq!(first[0].pixels, second[0].pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_recoloured_box_is_redrawn() {
        let mut r = renderer(2, 4);
        let plain = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        let blue = r.sprites(&[box_placement(&box_node(4, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        assert_ne!(plain[0].pixels, blue[0].pixels);
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn rounded_and_square_are_cached_distinctly() {
        let mut r = renderer(2, 4);
        for rounded in [false, true] {
            r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, rounded), 0, 0, 4, 3)], 40, 20);
        }
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn a_relabelled_box_of_the_same_size_reuses_its_pixels() {
        let mut r = renderer(2, 4);
        let first = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        let second = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        assert_eq!(first[0].pixels, second[0].pixels);
        assert_eq!(r.cache.len(), 1);
    }

    #[test]
    fn a_cached_sprite_moves_to_its_own_position() {
        let mut r = renderer(2, 4);
        r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        let moved = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 5, 2, 4, 3)], 40, 20);
        assert_eq!((moved[0].col, moved[0].row), (5, 2));
    }

    #[test]
    fn a_differently_cropped_box_is_redrawn() {
        let mut r = renderer(2, 4);
        let whole = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 40, 20);
        let cropped = r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, 4, 3)], 2, 20);
        assert_ne!(whole[0].width, cropped[0].width);
        assert_eq!(r.cache.len(), 2);
    }

    #[test]
    fn arrows_with_different_stops_are_redrawn() {
        let mut r = renderer(2, 4);
        let one = r.sprites(&[arrow_placement(vec![0], 0, 0, 0, 4, 6)], 40, 20);
        let two = r.sprites(&[arrow_placement(vec![0, 2], 0, 0, 0, 4, 6)], 40, 20);
        assert_ne!(one[0].pixels, two[0].pixels);
    }

    #[test]
    fn the_cache_is_bounded() {
        let mut r = renderer(2, 4);
        for width in 0..(CACHE_LIMIT as i64 + 2) {
            r.sprites(&[box_placement(&box_node(crate::state::PLAIN, crate::state::PLAIN, false), 0, 0, width + 1, 3)], 4000, 20);
        }
        assert!(r.cache.len() <= CACHE_LIMIT);
    }

    fn box_outline(r: &TerminalRenderer, node: &crate::state::Node, width: i64, height: i64) -> Sprite {
        let placement = box_placement(node, 0, 0, width, height);
        r.outline_box(&placement, 0, 0, width, height)
    }

    fn pixel_of(sprite: &Sprite, x: i64, y: i64) -> (u8, u8, u8, u8) {
        pixel_at(&sprite.pixels, sprite.width, x, y)
    }

    #[test]
    fn plain_fill_renders_transparent_interior_via_outline_box() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let sprite = box_outline(&r, &box_node(crate::state::PLAIN, crate::state::PLAIN, false), size, size);
        assert_eq!(pixel_of(&sprite, BORDER + 1, BORDER + 1), TRANSPARENT);
    }

    #[test]
    fn a_fill_colour_is_composited_over_black_via_outline_box() {
        let r = renderer(1, 1);
        let size = 2 * BORDER + 3;
        let sprite = box_outline(&r, &box_node(crate::state::PLAIN, 2, false), size, size);
        assert_eq!(pixel_of(&sprite, BORDER + 1, BORDER + 1), fill_colour(2));
    }

    #[test]
    fn a_rounded_box_cuts_away_its_extreme_corners_via_outline_box() {
        let r = renderer(8, 8);
        let sprite = box_outline(&r, &box_node(1, 2, true), 10, 10);
        let (last_x, last_y) = (sprite.width - 1, sprite.height - 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, 0), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 0, last_y), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, last_x, last_y), TRANSPARENT);
    }

    #[test]
    fn the_radius_leaves_the_sprite_size_and_position_alone() {
        let r = renderer(8, 8);
        let square = box_outline(&r, &box_node(1, 2, false), 10, 10);
        let rounded = box_outline(&r, &box_node(1, 2, true), 10, 10);
        assert_eq!((square.width, square.height), (rounded.width, rounded.height));
        assert_eq!((square.col, square.row), (rounded.col, rounded.row));
    }

    #[test]
    fn clipping_via_outline_box_is_a_pure_crop_of_the_whole_box() {
        let r = renderer(8, 8);
        let node = box_node(1, 2, true);
        let whole = box_outline(&r, &node, 10, 10);
        let hidden_cols = 2;
        let placement = crate::layout::Placement {
            node: crate::layout::PlacementNode::Node(&node),
            x: -hidden_cols,
            y: 0,
            width: 10,
            height: 10,
        };
        let clipped = r.outline_box(&placement, 0, 0, 10 - hidden_cols, 10);
        let offset = hidden_cols * r.cell_width;
        for y in 0..clipped.height {
            for x in 0..clipped.width {
                assert_eq!(pixel_of(&clipped, x, y), pixel_of(&whole, x + offset, y));
            }
        }
    }

    fn arrow_outline(r: &TerminalRenderer, stops: Vec<i64>, shaft: i64, width: i64, height: i64) -> Sprite {
        let placement = crate::layout::Placement {
            node: crate::layout::PlacementNode::Arrow(crate::layout::Arrow { stops, shaft }),
            x: 0,
            y: 0,
            width,
            height,
        };
        r.outline_arrow(&placement, 0, 0, width, height)
    }

    #[test]
    fn the_shaft_is_arrow_stroke_pixels_thick() {
        let r = renderer(10, 10);
        let ink = colour(crate::state::PLAIN);
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let shaft_row = 1 * 10 + 5;
        let rows: Vec<i64> = centered_span(shaft_row, ARROW_STROKE).collect();
        for &y in &rows {
            assert_eq!(pixel_of(&sprite, 5, y), ink);
        }
        assert_eq!(pixel_of(&sprite, 5, rows[0] - 1), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, 5, rows[rows.len() - 1] + 1), TRANSPARENT);
    }

    #[test]
    fn the_trunk_is_arrow_stroke_pixels_thick() {
        let r = renderer(10, 10);
        let ink = colour(crate::state::PLAIN);
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let midpoint = (4 * 10) / 2;
        let columns: Vec<i64> = centered_span(midpoint, ARROW_STROKE).collect();
        for &x in &columns {
            assert_eq!(pixel_of(&sprite, x, 10), ink);
        }
        assert_eq!(pixel_of(&sprite, columns[0] - 1, 10), TRANSPARENT);
        assert_eq!(pixel_of(&sprite, columns[columns.len() - 1] + 1, 10), TRANSPARENT);
    }

    #[test]
    fn the_arrowhead_tip_sits_at_the_stop_row() {
        let r = renderer(10, 10);
        let ink = colour(crate::state::PLAIN);
        let ink = (ink.0, ink.1, ink.2, OPAQUE);
        let sprite = arrow_outline(&r, vec![0, 2], 1, 4, 3);
        let right_edge = 4 * 10 - 1;
        for stop_row in [5, 25] {
            assert_eq!(pixel_of(&sprite, right_edge, stop_row), ink);
            assert_eq!(pixel_of(&sprite, right_edge - 1, stop_row), ink);
        }
    }

    #[test]
    fn a_single_stop_arrow_is_a_straight_line_across_every_column() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let shaft_row = sprite.height / 2;
        for x in 0..sprite.width {
            assert_eq!(pixel_of(&sprite, x, shaft_row).3, OPAQUE);
        }
    }

    #[test]
    fn arrow_off_shape_pixels_are_transparent() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        assert_eq!(pixel_of(&sprite, 0, 0), TRANSPARENT);
    }

    #[test]
    fn an_arrow_is_plain_grey() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0], 0, 2, 1);
        let (pr, pg, pb) = PLAIN_COLOUR;
        let shaft_row = sprite.height / 2;
        assert_eq!(pixel_of(&sprite, 0, shaft_row), (pr, pg, pb, OPAQUE));
    }

    #[test]
    fn a_branching_arrow_has_a_stub_at_every_stop() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0, 3], 0, 2, 4);
        let midpoint = sprite.width / 2;
        for stop in [0, 3] {
            let row = stop * 5 + 5 / 2;
            for x in midpoint..sprite.width {
                assert_eq!(pixel_of(&sprite, x, row).3, OPAQUE);
            }
        }
    }

    #[test]
    fn a_branching_arrow_rows_between_stops_are_blank_past_the_trunk() {
        let r = renderer(4, 5);
        let sprite = arrow_outline(&r, vec![0, 3], 0, 2, 4);
        let midpoint = sprite.width / 2;
        let trunk_columns: std::collections::HashSet<i64> = centered_span(midpoint, ARROW_STROKE).collect();
        let row_between_stops = 1 * 5 + 5 / 2;
        for x in 0..sprite.width {
            let expected = if trunk_columns.contains(&x) { OPAQUE } else { 0 };
            assert_eq!(pixel_of(&sprite, x, row_between_stops).3, expected);
        }
    }
}
