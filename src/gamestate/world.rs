use super::entity::Entity;
use crate::{camera::Camera, instance::Instance};
use cgmath::prelude::*;
use cgmath::Vector2;
use cgmath::Vector3;
use std::fmt::Display;

const WORLD_SIZE_MIN: f32 = 100.;

pub struct World {
    pub size: (f32, f32),
    pub camera: Camera,
}

impl World {
    pub fn init(aspect: f32) -> Self {
        let (size, camera) = Self::world_size_and_camera(aspect);

        Self { size, camera }
    }

    pub fn new_position(&self, position: cgmath::Vector2<f32>) -> WorldPosition {
        WorldPosition {
            position,
            world_size: self.size,
        }
    }

    pub fn _resize(&mut self, config: &wgpu::SurfaceConfiguration) {
        let aspect = config.width as f32 / config.height as f32;
        let (size, camera) = Self::world_size_and_camera(aspect);

        self.size = size;
        self.camera = camera;
    }

    pub fn left_top(&self) -> (f32, f32) {
        let (w, h) = self.size;
        (-w / 2., h / 2.)
    }

    pub fn right_bottom(&self) -> (f32, f32) {
        let (w, h) = self.size;
        (w / 2., -h / 2.)
    }

    fn world_size_and_camera(aspect: f32) -> ((f32, f32), Camera) {
        let mut world_width = WORLD_SIZE_MIN;
        let mut world_height = WORLD_SIZE_MIN;
        if aspect > 1. {
            world_width = world_height * aspect;
        } else {
            world_height = world_width / aspect;
        }

        let size = (world_width, world_height);

        let camera = Camera {
            eye: (0.0, -1.0, WORLD_SIZE_MIN).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            left: -world_width / 2.,
            right: world_width / 2.,
            top: world_height / 2.,
            bottom: -world_height / 2.,
            near: WORLD_SIZE_MIN - 25.,
            far: WORLD_SIZE_MIN + 25.,
        };

        (size, camera)
    }

    /// Add fake instances to make the world visually looping
    pub(crate) fn add_ghost_instances(&self, entity: &Entity) -> Vec<Instance> {
        let instance = entity.to_instance();
        if !entity.entered_world() {
            return vec![instance];
        }

        let mut instances = Vec::with_capacity(9);
        for row in (-1)..=1 {
            for col in (-1)..=1 {
                let mut ghost_instance = instance.clone();
                ghost_instance.position = Vector3 {
                    x: ghost_instance.position.x + self.size.0 * (col as f32),
                    y: ghost_instance.position.y + self.size.1 * (row as f32),
                    z: ghost_instance.position.z,
                };

                instances.push(ghost_instance)
            }
        }

        instances
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WorldPosition {
    position: cgmath::Vector2<f32>,
    world_size: (f32, f32),
}

impl Default for WorldPosition {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            world_size: (100., 100.),
        }
    }
}

impl Display for WorldPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("({:.2}, {:.2})", self.position.x, self.position.y))
    }
}

impl WorldPosition {
    pub fn to_vector2(&self) -> cgmath::Vector2<f32> {
        self.position
    }

    pub fn to_vector3(&self) -> cgmath::Vector3<f32> {
        self.position.extend(0.)
    }

    pub fn to_tuple(&self) -> (f32, f32) {
        let Vector2 { x, y } = self.position;
        (x, y)
    }

    pub fn world_size(&self) -> (f32, f32) {
        self.world_size
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let (w, h) = self.world_size;

        let world = cgmath::Vector2 {
            x: w / 2.,
            y: h / 2.,
        };

        cgmath::Vector2::distance(self.position, other.position).min(cgmath::Vector2::distance(
            Self::normalize(&self.position + world, self.world_size),
            Self::normalize(other.position + world, self.world_size),
        ))
    }

    pub fn to_zero(&self) -> Self {
        Self {
            position: (0.0, 0.0).into(),
            world_size: self.world_size,
        }
    }

    /// Translate with normalization. The result position is always inside world bounds.
    pub fn translate(&self, v: cgmath::Vector2<f32>) -> Self {
        Self {
            position: Self::normalize(self.position + v, self.world_size),
            world_size: self.world_size,
        }
    }

    /// Translate without normalization. The result position can be outside world bounds.
    pub fn translate_unsafe(&self, v: cgmath::Vector2<f32>) -> Self {
        Self {
            position: self.position + v,
            world_size: self.world_size,
        }
    }

    fn normalize(position: cgmath::Vector2<f32>, world_size: (f32, f32)) -> cgmath::Vector2<f32> {
        cgmath::vec2(
            Self::normalize_coord(position.x, world_size.0),
            Self::normalize_coord(position.y, world_size.1),
        )
    }

    fn normalize_coord(x: f32, world: f32) -> f32 {
        let x_clamped = x % world;
        let half_world = world / 2.;

        if (-half_world..=half_world).contains(&x_clamped) {
            x_clamped
        } else {
            x_clamped - x_clamped / x_clamped.abs() * world
        }
    }
}

#[test]
fn world_position_distance() {
    let mut a = WorldPosition::default();
    let mut b = WorldPosition::default();
    let mut c = WorldPosition::default();

    a.position = cgmath::vec2(45., 0.);
    b.position = cgmath::vec2(-45., 0.);
    c.position = cgmath::vec2(30., 0.);

    assert_eq!(a.distance(&b), 10.);
    assert_eq!(a.distance(&c), 15.);
    assert_eq!(b.distance(&c), 25.);
}

#[test]
fn test_world_position_normalize() {
    let size = (100., 100.);
    assert_eq!(WorldPosition::normalize(cgmath::vec2(0., 0.), size).x, 0.);
    assert_eq!(
        WorldPosition::normalize(cgmath::vec2(60., 0.), size).x,
        -40.
    );
    assert_eq!(
        WorldPosition::normalize(WorldPosition::normalize(cgmath::vec2(60., 0.), size), size).x,
        -40.
    );
    assert_eq!(
        WorldPosition::normalize(cgmath::vec2(-60., 0.), size).x,
        40.
    );
    assert_eq!(
        WorldPosition::normalize(cgmath::vec2(-160., 0.), size).x,
        40.
    );
}

#[test]
fn test_world_position_translate() {
    let world_postion = WorldPosition::default();
    assert_eq!(
        world_postion.translate(cgmath::vec2(55., 0.)).position.x,
        -45.
    );
}
