use std::fmt::Display;

use crate::{camera::Camera, instance::Instance};
use cgmath::prelude::*;
use cgmath::Vector3;

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

    pub fn new_position(&self, position: cgmath::Vector3<f32>) -> WorldPosition {
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
    pub(crate) fn add_ghost_instances(&self, instance: &Instance) -> Vec<Instance> {
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
    position: cgmath::Vector3<f32>,
    world_size: (f32, f32),
}

impl Default for WorldPosition {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0, 0.0).into(),
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
    pub fn to_vector3(&self) -> cgmath::Vector3<f32> {
        self.position
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) {
        let Vector3 { x, y, z } = self.position;
        (x, y, z)
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let world_size = self.world_size;

        let world = cgmath::Vector3 {
            x: world_size.0,
            y: world_size.1,
            z: 0.0,
        };

        cgmath::Vector3::distance(self.position, other.position).min(cgmath::Vector3::distance(
            Self::normalize(&self.position + world, world_size),
            Self::normalize(other.position + world, world_size),
        ))
    }

    pub fn to_zero(&self) -> Self {
        Self {
            position: (0.0, 0.0, 0.0).into(),
            world_size: self.world_size,
        }
    }

    pub fn translate(&self, v: cgmath::Vector3<f32>) -> Self {
        Self {
            position: Self::normalize(self.position + v, self.world_size),
            world_size: self.world_size,
        }
    }

    fn normalize(position: cgmath::Vector3<f32>, world_size: (f32, f32)) -> cgmath::Vector3<f32> {
        cgmath::Vector3 {
            x: position.x % world_size.0,
            y: position.y % world_size.1,
            z: position.z,
        }
    }
}
