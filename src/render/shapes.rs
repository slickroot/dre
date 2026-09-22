use super::terminal::{centered_span, python_round};
use super::{arrowhead_depth, arrowhead_slope};
use crate::canvas::{Rgba, Shape};

pub(super) struct BoxShape {
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) border: i64,
    pub(super) radius: i64,
    pub(super) edge: Rgba,
    pub(super) fill: Rgba,
}

impl BoxShape {
    fn outer(&self) -> i64 {
        if self.radius == 0 {
            return 0;
        }
        (self.radius + self.border)
            .min(self.width / 2)
            .min(self.height / 2)
    }

    fn coverage(px: f64, py: f64, width: f64, height: f64, radius: f64) -> f64 {
        let half_x = width / 2.0;
        let half_y = height / 2.0;
        let qx = (px - half_x).abs() - (half_x - radius);
        let qy = (py - half_y).abs() - (half_y - radius);
        let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
        (0.5 - distance).clamp(0.0, 1.0)
    }
}

impl Shape for BoxShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let outer_coverage = Self::coverage(
            px,
            py,
            self.width as f64,
            self.height as f64,
            self.outer() as f64,
        );
        let inner_coverage = Self::coverage(
            px - self.border as f64,
            py - self.border as f64,
            (self.width - 2 * self.border) as f64,
            (self.height - 2 * self.border) as f64,
            self.radius as f64,
        );
        let edge_coverage = outer_coverage - inner_coverage;
        let alpha = edge_coverage * self.edge[3] as f64 + inner_coverage * self.fill[3] as f64;
        if alpha == 0.0 {
            return None;
        }
        let mut colour = [0u8; 4];
        for (channel, colour) in colour.iter_mut().enumerate().take(3) {
            let value = (self.edge[channel] as f64 * edge_coverage * self.edge[3] as f64
                + self.fill[channel] as f64 * inner_coverage * self.fill[3] as f64)
                / alpha;
            *colour = python_round(value) as u8;
        }
        colour[3] = python_round(alpha) as u8;
        Some(colour)
    }
}

pub(super) struct ArrowShape {
    pub(super) width: i64,
    pub(super) stop_rows: Vec<i64>,
    pub(super) shaft_row: i64,
    pub(super) trunk: (i64, i64),
    pub(super) stroke: i64,
    pub(super) ink: Rgba,
}

impl ArrowShape {
    fn midpoint(&self) -> i64 {
        self.width / 2
    }

    fn in_shaft(&self, x: i64, y: i64) -> bool {
        (0..=self.midpoint()).contains(&x)
            && centered_span(self.shaft_row, self.stroke).contains(&y)
    }

    fn in_trunk(&self, x: i64, y: i64) -> bool {
        centered_span(self.midpoint(), self.stroke).contains(&x)
            && (self.trunk.0..=self.trunk.1).contains(&y)
    }

    fn in_stop(&self, x: i64, y: i64, stop_row: i64) -> bool {
        (self.midpoint()..=self.width - 1).contains(&x)
            && centered_span(stop_row, self.stroke).contains(&y)
    }

    fn in_arrowhead(&self, x: i64, y: i64, stop_row: i64) -> bool {
        let depth = arrowhead_depth();
        let slope = arrowhead_slope();
        let right_edge = self.width - 1;
        (0..=(depth as i64))
            .take_while(|&distance| (distance as f64) < depth)
            .take_while(|&distance| right_edge - distance >= self.midpoint())
            .any(|distance| {
                let spread = python_round(distance as f64 * slope) as i64;
                centered_span(right_edge - distance, self.stroke).contains(&x)
                    && (centered_span(stop_row - spread, self.stroke).contains(&y)
                        || centered_span(stop_row + spread, self.stroke).contains(&y))
            })
    }
}

impl Shape for ArrowShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let is_ink = self.in_shaft(x, y)
            || self.in_trunk(x, y)
            || self
                .stop_rows
                .iter()
                .any(|&row| self.in_stop(x, y, row) || self.in_arrowhead(x, y, row));
        is_ink.then_some(self.ink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{BORDER, OPAQUE};

    const EDGE: Rgba = [10, 20, 30, OPAQUE];
    const FILL: Rgba = [1, 2, 3, OPAQUE];

    fn box_shape(width: i64, height: i64, radius: i64) -> BoxShape {
        BoxShape {
            width,
            height,
            border: BORDER,
            radius,
            edge: EDGE,
            fill: FILL,
        }
    }

    #[test]
    fn a_box_pixel_with_no_alpha_has_no_colour() {
        let shape = BoxShape {
            fill: [0; 4],
            ..box_shape(40, 40, 10)
        };
        assert_eq!(shape.colour_at(0, 0), None);
    }

    #[test]
    fn a_square_box_is_edge_on_the_border_and_fill_inside() {
        let shape = box_shape(30, 30, 0);
        assert_eq!(shape.colour_at(0, 15), Some(EDGE));
        assert_eq!(shape.colour_at(BORDER, BORDER), Some(FILL));
    }

    #[test]
    fn a_box_narrower_than_two_borders_is_solid_edge() {
        let shape = box_shape(2 * BORDER - 1, 30, 0);
        for x in 0..shape.width {
            assert_eq!(shape.colour_at(x, 15), Some(EDGE));
        }
    }
}
