use std::time::Duration;

use cgmath::{Deg, Rotation3, Zero};
use rand::Rng;

use crate::{gamestate::GameState, world::WorldPosition};

#[derive(Clone, Copy)]
pub struct Collision {
    pub shape: Shape,
    pub on_collision: fn(&mut GameState, this_id: usize, other_ids: &[usize]),
}

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Sphere { origin: WorldPosition, radius: f32 },
}

impl Shape {
    pub(crate) fn overlaps(&self, another_shape: &Shape) -> bool {
        match (self, another_shape) {
            (
                Shape::Sphere { origin, radius },
                Shape::Sphere {
                    origin: other_origin,
                    radius: other_radius,
                },
            ) => WorldPosition::distance(origin, other_origin) < (radius + other_radius),
        }
    }

    pub(crate) fn translate(&self, position: WorldPosition) -> Shape {
        match *self {
            Shape::Sphere { origin, radius } => Shape::Sphere {
                origin: origin.translate(position.to_vector2()),
                radius,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Control {
    pub enabled: bool,
    pub weapon_cooldown: Duration,
}

impl Control {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            weapon_cooldown: Duration::ZERO,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Physics {
    pub linear_speed: cgmath::Vector2<f32>,
    pub max_linear_speed: f32,
    pub angular_speed: cgmath::Quaternion<f32>,
}

impl Physics {
    pub fn random(max_linear_speed: f32, max_angular_speed: f32) -> Self {
        let mut rng = rand::thread_rng();

        let linear_speed = cgmath::Vector2 {
            x: rng.gen_range(-max_linear_speed..max_linear_speed),
            y: rng.gen_range(-max_linear_speed..max_linear_speed),
        };

        let axis = cgmath::Vector3 {
            x: rng.gen_range(0.0..1.0),
            y: rng.gen_range(0.0..1.0),
            z: rng.gen_range(0.0..1.0),
        };
        let angle = Deg(rng.gen_range(0.0..max_angular_speed));
        let angular_speed = cgmath::Quaternion::from_axis_angle(axis, angle);

        let max_linear_speed = 30.;

        Self {
            linear_speed,
            angular_speed,
            max_linear_speed,
        }
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            linear_speed: (0.0, 0.0).into(),
            max_linear_speed: 30.,
            angular_speed: cgmath::Quaternion::zero(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Health {
    pub level: usize,
}

impl Health {
    pub fn deal_damage(&mut self, damage: usize) {
        self.level = (self.level - damage).max(0);
    }
}

#[derive(Copy, Clone)]
pub struct Lifetime {
    pub dies_after: Duration,
}
