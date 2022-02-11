use super::world::WorldPosition;

pub struct Rect {
    pub left_top: (f32, f32),
    pub right_bottom: (f32, f32),
}

impl Rect {
    pub fn expand(&mut self, v: f32) {
        self.left_top.0 -= v;
        self.left_top.1 += v;
        self.right_bottom.0 += v;
        self.right_bottom.1 -= v;
    }

    pub fn contains_point(&self, point: (f32, f32)) -> bool {
        let (left, top) = self.left_top;
        let (right, bottom) = self.right_bottom;
        let (x, y) = point;

        (left..right).contains(&x) && (bottom..top).contains(&y)
    }

    pub fn contains_circle(&self, center: (f32, f32), radius: f32) -> bool {
        let (left, top) = self.left_top;
        let (right, bottom) = self.right_bottom;
        let (x, y) = center;

        let r_sqr = radius.powf(2.);

        if self.contains_point(center) {
            (left - x).powf(2.) > r_sqr
                && (right - x).powf(2.) > r_sqr
                && (top - y).powf(2.) > r_sqr
                && (bottom - y).powf(2.) > r_sqr
        } else {
            false
        }
    }
}

#[test]
fn test_rect_contains_circle() {
    let rect = Rect {
        left_top: (-50., 50.),
        right_bottom: (50., -50.),
    };

    assert_eq!(rect.contains_circle((-40., 0.), 9.), true);
    assert_eq!(rect.contains_circle((-40., 0.), 11.), false);
}

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Circle { origin: WorldPosition, radius: f32 },
}

impl Shape {
    pub(crate) fn overlaps(&self, another_shape: &Shape) -> bool {
        match (self, another_shape) {
            (
                Shape::Circle { origin, radius },
                Shape::Circle {
                    origin: other_origin,
                    radius: other_radius,
                },
            ) => WorldPosition::distance(origin, other_origin) < (radius + other_radius),
        }
    }

    pub(crate) fn translate(&self, position: cgmath::Vector2<f32>) -> Shape {
        match *self {
            Shape::Circle { origin, radius } => Shape::Circle {
                origin: origin.translate(position),
                radius,
            },
        }
    }
}
