use super::terminal::{centered_span, python_round};
use super::{arrowhead_depth, arrowhead_slope, LED_DIM_ALPHA, LED_DOT_RATIO, LED_HALO_ALPHA};
use crate::canvas::{Rgba, Shape};
use crate::view::{Sides, ALL_SIDES};

pub(super) struct BoxShape {
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) border: i64,
    pub(super) radius: i64,
    pub(super) sides: Sides,
    pub(super) edge: Rgba,
    pub(super) fill: Rgba,
}

impl BoxShape {
    fn has_all_sides(&self) -> bool {
        self.sides == ALL_SIDES
    }

    fn inset(&self, side_on: bool) -> i64 {
        if side_on {
            self.border
        } else {
            0
        }
    }

    fn corner_radius(&self) -> i64 {
        if self.has_all_sides() {
            self.radius
        } else {
            0
        }
    }

    fn outer(&self) -> i64 {
        if self.corner_radius() == 0 {
            return 0;
        }
        (self.corner_radius() + self.border)
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
        let (top, right, bottom, left) = self.sides;
        let (inset_top, inset_right) = (self.inset(top), self.inset(right));
        let (inset_bottom, inset_left) = (self.inset(bottom), self.inset(left));
        let inner_coverage = Self::coverage(
            px - inset_left as f64,
            py - inset_top as f64,
            (self.width - inset_left - inset_right) as f64,
            (self.height - inset_top - inset_bottom) as f64,
            self.corner_radius() as f64,
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

fn rounded_rect_distance(px: f64, py: f64, width: f64, height: f64, radius: f64) -> f64 {
    let half_x = width / 2.0;
    let half_y = height / 2.0;
    let qx = (px - half_x).abs() - (half_x - radius);
    let qy = (py - half_y).abs() - (half_y - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

pub(super) struct GlowShape {
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) margin_x: i64,
    pub(super) margin_y: i64,
    pub(super) radius: i64,
    pub(super) colour: Rgba,
}

impl GlowShape {
    fn reach(&self) -> f64 {
        (self.margin_x.min(self.margin_y) - 1).max(1) as f64
    }
}

impl Shape for GlowShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let box_width = self.width - 2 * self.margin_x;
        let box_height = self.height - 2 * self.margin_y;
        let radius = self.radius.min(box_width / 2).min(box_height / 2) as f64;
        let distance = rounded_rect_distance(
            x as f64 + 0.5 - self.margin_x as f64,
            y as f64 + 0.5 - self.margin_y as f64,
            box_width as f64,
            box_height as f64,
            radius,
        );
        if distance <= 0.0 || distance >= self.reach() {
            return None;
        }
        let falloff = (1.0 - distance / self.reach()).powf(1.6);
        let alpha = python_round(self.colour[3] as f64 * falloff) as u8;
        (alpha > 0).then_some([self.colour[0], self.colour[1], self.colour[2], alpha])
    }
}

pub(super) struct ArrowShape {
    pub(super) width: i64,
    pub(super) stop_rows: Vec<i64>,
    pub(super) shaft_row: i64,
    pub(super) trunk: (i64, i64),
    pub(super) stroke: i64,
    pub(super) arrowhead_edge_length: f64,
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
        let depth = arrowhead_depth(self.arrowhead_edge_length);
        let slope = arrowhead_slope(self.arrowhead_edge_length);
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

/// A round status light, centred in its sprite. Lit, it is a solid dot with a
/// soft halo; unlit, it is the same dot, dimmed, with no halo.
pub(super) struct LedShape {
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) colour: (u8, u8, u8),
    pub(super) lit: bool,
}

impl Shape for LedShape {
    fn colour_at(&self, x: i64, y: i64) -> Option<Rgba> {
        let dx = x as f64 + 0.5 - self.width as f64 / 2.0;
        let dy = y as f64 + 0.5 - self.height as f64 / 2.0;
        // Sized to the row height, not the narrower of width/height, so the dot
        // and halo always touch the cell's top and bottom edges.
        let dot_radius = self.height as f64 * LED_DOT_RATIO;
        let reach = self.height as f64 / 2.0 - dot_radius;
        let distance = dx.hypot(dy) - dot_radius;
        let dot = (0.5 - distance).clamp(0.0, 1.0);
        let (dot_alpha, halo) = if self.lit {
            let halo = if distance > 0.0 && reach > 0.0 {
                (1.0 - distance / reach).max(0.0).powi(2) * LED_HALO_ALPHA
            } else {
                0.0
            };
            (1.0, halo)
        } else {
            (LED_DIM_ALPHA, 0.0)
        };
        let alpha = dot * dot_alpha + (1.0 - dot) * halo;
        let alpha = python_round(alpha * 255.0) as u8;
        let (r, g, b) = self.colour;
        (alpha > 0).then_some([r, g, b, alpha])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::OPAQUE;
    use crate::view::BORDER;

    const EDGE: Rgba = [10, 20, 30, OPAQUE];
    const FILL: Rgba = [1, 2, 3, OPAQUE];

    fn box_shape(width: i64, height: i64, radius: i64) -> BoxShape {
        BoxShape {
            width,
            height,
            border: BORDER,
            radius,
            sides: ALL_SIDES,
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

    const TOP_AND_LEFT: Sides = (true, false, false, true);

    fn top_and_left_box() -> BoxShape {
        BoxShape {
            sides: TOP_AND_LEFT,
            ..box_shape(30, 30, 0)
        }
    }

    #[test]
    fn only_the_chosen_sides_get_a_line() {
        let shape = BoxShape {
            fill: [0; 4],
            ..top_and_left_box()
        };
        let last = 30 - 1;
        assert_eq!(shape.colour_at(15, 0), Some(EDGE));
        assert_eq!(shape.colour_at(0, 15), Some(EDGE));
        assert_eq!(shape.colour_at(15, last), None);
        assert_eq!(shape.colour_at(last, 15), None);
    }

    #[test]
    fn the_inside_of_a_partial_box_has_no_line() {
        let shape = top_and_left_box();
        assert_eq!(shape.colour_at(BORDER, BORDER), Some(FILL));
        assert_eq!(shape.colour_at(15, 15), Some(FILL));
    }

    #[test]
    fn a_partial_box_ignores_rounding() {
        let rounded = BoxShape {
            radius: 10,
            ..top_and_left_box()
        };
        let square = top_and_left_box();
        for y in 0..30 {
            for x in 0..30 {
                assert_eq!(rounded.colour_at(x, y), square.colour_at(x, y));
            }
        }
    }

    fn led(lit: bool) -> LedShape {
        LedShape {
            width: 40,
            height: 20,
            colour: (1, 2, 3),
            lit,
        }
    }

    #[test]
    fn a_lit_led_is_a_solid_dot_in_its_colour() {
        assert_eq!(led(true).colour_at(20, 10), Some([1, 2, 3, 255]));
    }

    #[test]
    fn an_unlit_led_is_the_same_dot_dimmed() {
        let alpha = led(false).colour_at(20, 10).unwrap()[3];
        assert_eq!(alpha, python_round(LED_DIM_ALPHA * 255.0) as u8);
    }

    #[test]
    fn only_a_lit_led_has_a_halo() {
        let just_outside = (20, 10 + 7);
        assert!(led(true)
            .colour_at(just_outside.0, just_outside.1)
            .is_some());
        assert_eq!(led(false).colour_at(just_outside.0, just_outside.1), None);
    }

    #[test]
    fn a_lit_led_reaches_the_top_and_bottom_rows_of_its_cell() {
        let shape = led(true);
        assert!(shape.colour_at(shape.width / 2, 0).is_some());
        assert!(shape.colour_at(shape.width / 2, shape.height - 1).is_some());
        assert_eq!(shape.colour_at(shape.width / 2, -1), None);
        assert_eq!(shape.colour_at(shape.width / 2, shape.height), None);
    }

    #[test]
    fn a_lit_led_still_reaches_the_row_edges_when_its_cell_is_narrower_than_it_is_tall() {
        let shape = LedShape {
            width: 10,
            height: 20,
            colour: (1, 2, 3),
            lit: true,
        };
        assert!(shape.colour_at(shape.width / 2, 0).is_some());
        assert!(shape.colour_at(shape.width / 2, shape.height - 1).is_some());
    }

    #[test]
    fn a_box_with_no_sides_has_no_line() {
        let shape = BoxShape {
            sides: (false, false, false, false),
            ..box_shape(30, 30, 0)
        };
        assert_eq!(shape.colour_at(0, 15), Some(FILL));
    }

    const GLOW_COLOUR: [u8; 3] = [40, 50, 60];

    fn glow_shape(width: i64, height: i64, padding_x: i64, padding_y: i64) -> GlowShape {
        GlowShape {
            width: width + 2 * padding_x,
            height: height + 2 * padding_y,
            margin_x: padding_x,
            margin_y: padding_y,
            radius: 0,
            colour: [GLOW_COLOUR[0], GLOW_COLOUR[1], GLOW_COLOUR[2], OPAQUE],
        }
    }

    #[test]
    fn a_glow_fades_inward_from_the_box_edge() {
        let shape = glow_shape(10, 10, 4, 4);
        let edge = shape.colour_at(3, 7).unwrap()[3];
        let farther = shape.colour_at(2, 7).unwrap()[3];
        assert!(edge > farther, "expected {edge} > {farther}");
    }

    #[test]
    fn the_deep_interior_of_a_glow_is_fully_transparent() {
        let shape = glow_shape(10, 10, 4, 4);
        assert_eq!(shape.colour_at(8, 8), None);
    }

    #[test]
    fn just_outside_the_box_edge_the_glow_is_near_peak_opacity() {
        let shape = glow_shape(10, 10, 4, 4);
        let alpha = shape.colour_at(3, 7).unwrap()[3];
        assert!(
            alpha > shape.colour[3] / 2,
            "expected near-peak alpha just outside the edge, got {alpha}"
        );
    }

    #[test]
    fn a_glow_fades_out_with_distance_from_the_box() {
        let shape = glow_shape(10, 10, 4, 4);
        let near = shape.colour_at(2, 7).unwrap()[3];
        let far = shape.colour_at(1, 7).unwrap()[3];
        assert!(near > far, "expected {near} > {far}");
    }

    #[test]
    fn a_glow_has_the_same_pixel_thickness_on_each_axis() {
        let shape = glow_shape(20, 20, 8, 12);
        for distance in 0..shape.margin_x {
            let left = shape.colour_at(shape.margin_x - 1 - distance, shape.margin_y + 10);
            let top = shape.colour_at(shape.margin_x + 10, shape.margin_y - 1 - distance);
            assert_eq!(left, top);
        }
    }

    #[test]
    fn a_glow_is_fully_transparent_past_its_margin() {
        let shape = glow_shape(10, 10, 4, 4);
        assert_eq!(shape.colour_at(0, 0), None);
    }

    #[test]
    fn a_glow_uses_the_box_edge_colour() {
        let shape = glow_shape(10, 10, 4, 4);
        let (r, g, b, _) = {
            let pixel = shape.colour_at(2, 7).unwrap();
            (pixel[0], pixel[1], pixel[2], pixel[3])
        };
        assert_eq!((r, g, b), (GLOW_COLOUR[0], GLOW_COLOUR[1], GLOW_COLOUR[2]));
    }
}
