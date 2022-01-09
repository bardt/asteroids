use std::time::Duration;

use crate::{camera::Camera, input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;
use cgmath::Vector3;
use rand::Rng;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

pub struct Entity {
    pub name: String,
    pub instance: Instance,
    pub components: Vec<Component>,
}

impl Entity {
    pub fn make_asteroid(position: (f32, f32, f32)) -> Entity {
        Self {
            name: "Asteroid".to_string(),
            instance: Instance {
                position: position.into(),
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            },
            components: vec![Component::Physics(Physics::random(1., 100.))],
        }
    }

    pub fn make_spaceship(position: (f32, f32, f32), rotation_angle: f32) -> Entity {
        Self {
            name: "Spaceship".to_string(),
            instance: Instance {
                position: position.into(),
                rotation: cgmath::Quaternion::from_angle_z(Deg(rotation_angle)),
            },
            components: vec![Component::Control, Component::Physics(Physics::init())],
        }
    }

    pub fn update_physics(&mut self, world_size: (f32, f32), dtime: &Duration) {
        self.components
            .par_iter_mut()
            .find_map_first(|component| match component {
                Component::Physics(physics) => Some(&mut *physics),
                _ => None,
            })
            .map(|physics| {
                Entity::update_physics_internal(&mut self.instance, physics, world_size, dtime)
            });
    }

    pub fn update_control(&mut self, input: &Input, dtime: &Duration) {
        let control = self
            .components
            .par_iter_mut()
            .find_map_first(|component| match component {
                Component::Control => Some(()),
                _ => None,
            });

        self.components
            .par_iter_mut()
            .find_map_first(|component| match (&control, component) {
                (Some(()), Component::Physics(physics)) => Some(&mut *physics),
                _ => None,
            })
            .map(|physics| {
                Entity::update_control_internal(&mut self.instance, physics, input, dtime)
            });
    }

    fn update_control_internal(
        instance: &mut Instance,
        physics: &mut Physics,
        input: &Input,
        dtime: &Duration,
    ) {
        let rotation_speed = 180.;
        let linear_acceleration = 50.;

        let delta_time = (dtime.as_millis() as f32) / 1000.0;
        let delta_angle = delta_time * rotation_speed;
        let delta_linear_speed = delta_time * linear_acceleration;

        let direction = instance.rotation.rotate_vector(cgmath::Vector3::unit_y()); //cgmath::Vector3 { x, y, z };

        if input.is_forward_pressed {
            physics.linear_speed += direction * delta_linear_speed;
        }

        if input.is_right_pressed {
            instance.rotation =
                instance.rotation * cgmath::Quaternion::from_angle_z(cgmath::Deg(-delta_angle))
        }

        if input.is_left_pressed {
            instance.rotation =
                instance.rotation * cgmath::Quaternion::from_angle_z(cgmath::Deg(delta_angle))
        }
    }

    fn update_physics_internal(
        instance: &mut Instance,
        physics: &Physics,
        world_size: (f32, f32),
        dtime: &Duration,
    ) {
        /*
        @TODO: limit maximum linear speed
        */
        instance.position =
            instance.position + physics.linear_speed * (dtime.as_millis() as f32) / 1000.0;
        instance.position = Vector3 {
            x: instance.position.x % world_size.0,
            y: instance.position.y % world_size.1,
            z: instance.position.z,
        };
        instance.rotation = cgmath::Quaternion::nlerp(
            instance.rotation,
            instance.rotation * physics.angular_speed,
            (dtime.as_millis() as f32) / 1000.0,
        );
    }
}

pub enum Component {
    Control,
    Physics(Physics),
}

pub struct Physics {
    linear_speed: cgmath::Vector3<f32>,
    angular_speed: cgmath::Quaternion<f32>,
}

impl Physics {
    fn init() -> Self {
        Self {
            linear_speed: (0.0, 0.0, 0.0).into(),
            angular_speed: cgmath::Quaternion::zero(),
        }
    }

    fn random(max_linear_speed: f32, max_angular_speed: f32) -> Self {
        let mut rng = rand::thread_rng();

        let linear_speed = cgmath::Vector3 {
            x: rng.gen_range(-max_linear_speed..max_linear_speed),
            y: rng.gen_range(-max_linear_speed..max_linear_speed),
            z: 0.0,
        };

        let axis = cgmath::Vector3 {
            x: rng.gen_range(0.0..1.0),
            y: rng.gen_range(0.0..1.0),
            z: rng.gen_range(0.0..1.0),
        };
        let angle = Deg(rng.gen_range(0.0..max_angular_speed));
        let angular_speed = cgmath::Quaternion::from_axis_angle(axis, angle);

        Self {
            linear_speed,
            angular_speed,
        }
    }
}

const WORLD_SIZE_MIN: f32 = 100.;

pub struct World {
    pub size: (f32, f32),
    pub entities: Vec<Entity>,
    pub camera: Camera,
}

impl World {
    pub fn init(config: &wgpu::SurfaceConfiguration) -> Self {
        let entities: Vec<Entity> = vec![
            Entity::make_spaceship((0.0, 0.0, 0.0), 90.),
            Entity::make_asteroid((5.0, 5.0, 0.0)),
            Entity::make_asteroid((-5.0, 5.0, 0.0)),
            Entity::make_asteroid((5.0, -5.0, 0.0)),
        ];

        let (size, camera) =
            Self::world_size_and_camera(config.width as f32 / config.height as f32);

        Self {
            size,
            entities,
            camera,
        }
    }

    pub fn resize(&mut self, config: &wgpu::SurfaceConfiguration) {
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
            eye: (0.0, 0.0, WORLD_SIZE_MIN * 2.).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),

            left: -world_width / 2.,
            right: world_width / 2.,
            top: world_height / 2.,
            bottom: -world_height / 2.,
            near: WORLD_SIZE_MIN * 2. - 10.,
            far: WORLD_SIZE_MIN * 2. + 10.,
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
