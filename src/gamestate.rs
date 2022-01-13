use crate::collision;
use crate::{input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;
use cgmath::Vector3;
use rand::Rng;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use std::time::Duration;

#[derive(Default)]
pub struct GameState {
    entities: Vec<Option<Entity>>,
}

#[allow(dead_code)]
type EntityIndex = usize;

impl GameState {
    pub fn push(&mut self, entity: Entity) {
        self.entities.push(Some(entity))
    }

    pub fn _kill(&mut self, index: EntityIndex) {
        self.entities[index] = None
    }

    pub fn instances(&self) -> Vec<(&str, &Instance)> {
        self.entities
            .iter()
            .filter_map(|option_entity| {
                option_entity
                    .as_ref()
                    .map(|entity| (entity.name.as_str(), &entity.instance))
            })
            .collect::<Vec<_>>()
    }

    pub fn control_system(&mut self, input: &Input, delta_time: &Duration) -> &mut Self {
        self.entities
            .par_iter_mut()
            .for_each(|option_entity| match option_entity {
                Some(entity) => entity.update_control(input, delta_time),
                None => (),
            });

        self
    }

    pub fn physics_system(&mut self, world_size: (f32, f32), delta_time: &Duration) -> &mut Self {
        self.entities
            .par_iter_mut()
            .for_each(|option_entity| match option_entity {
                Some(entity) => entity.update_physics(world_size, delta_time),
                None => (),
            });

        self
    }

    pub fn collision_system(&mut self, world_size: (f32, f32)) -> &mut Self {
        let shapes = self
            .entities
            .par_iter_mut()
            .map(|option_entity| match option_entity {
                Some(entity) => entity
                    .collision
                    .as_ref()
                    .map(|shape| shape.translate(entity.instance.position)),
                None => None,
            })
            .collect::<Vec<_>>();

        let collisions = collision::find_collisions(world_size, shapes);
        println!("Collisions: {:?}", collisions);

        self
    }
}

#[derive(Default)]
pub struct Entity {
    pub name: String,
    pub instance: Instance,
    pub physics: Option<Physics>,
    pub collision: Option<Shape>,
    pub control: Option<Control>,
}

impl Entity {
    pub fn make_asteroid(position: (f32, f32, f32)) -> Entity {
        Self {
            name: "Asteroid".to_string(),
            instance: Instance {
                position: position.into(),
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            },
            physics: Some(Physics::random(1., 100.)),
            collision: Some(Shape::Sphere {
                origin: (0.0, 0.0, 0.0).into(),
                radius: 1.0,
            }),
            ..Default::default()
        }
    }

    pub fn make_spaceship(position: (f32, f32, f32), rotation_angle: f32) -> Entity {
        Self {
            name: "Spaceship".to_string(),
            instance: Instance {
                position: position.into(),
                rotation: cgmath::Quaternion::from_angle_z(Deg(rotation_angle)),
            },
            physics: Some(Physics::default()),
            collision: Some(Shape::Sphere {
                origin: (0.0, 0.0, 0.0).into(),
                radius: 5.0,
            }),
            control: Some(Control::enabled()),
            ..Default::default()
        }
    }

    pub fn update_physics(&mut self, world_size: (f32, f32), dtime: &Duration) {
        match &self.physics {
            Some(physics) => {
                /*
                @TODO: limit maximum linear speed
                */
                self.instance.position = world_normalize(
                    self.instance.position
                        + physics.linear_speed * (dtime.as_millis() as f32) / 1000.0,
                    world_size,
                );

                self.instance.rotation = cgmath::Quaternion::nlerp(
                    self.instance.rotation,
                    self.instance.rotation * physics.angular_speed,
                    (dtime.as_millis() as f32) / 1000.0,
                );
            }
            None => (),
        }
    }

    pub fn update_control(&mut self, input: &Input, dtime: &Duration) {
        match (&self.control, &mut self.physics) {
            (Some(Control { enabled: true, .. }), Some(physics)) => {
                let instance = &mut self.instance;

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
                    instance.rotation = instance.rotation
                        * cgmath::Quaternion::from_angle_z(cgmath::Deg(-delta_angle))
                }

                if input.is_left_pressed {
                    instance.rotation = instance.rotation
                        * cgmath::Quaternion::from_angle_z(cgmath::Deg(delta_angle))
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Shape {
    Sphere { origin: Vector3<f32>, radius: f32 },
}

impl Shape {
    pub(crate) fn overlaps(&self, another_shape: &Shape, world_size: (f32, f32)) -> bool {
        match (self, another_shape) {
            (
                Shape::Sphere { origin, radius },
                Shape::Sphere {
                    origin: other_origin,
                    radius: other_radius,
                },
            ) => world_distance(*origin, *other_origin, world_size) < (radius + other_radius),
        }
    }

    pub(crate) fn translate(&self, position: Vector3<f32>) -> Shape {
        match *self {
            Shape::Sphere { origin, radius } => Shape::Sphere {
                origin: origin + position,
                radius,
            },
        }
    }
}

pub struct Control {
    enabled: bool,
}

impl Control {
    fn enabled() -> Self {
        Self { enabled: true }
    }
}

pub struct Physics {
    linear_speed: cgmath::Vector3<f32>,
    angular_speed: cgmath::Quaternion<f32>,
}

impl Physics {
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

impl Default for Physics {
    fn default() -> Self {
        Self {
            linear_speed: (0.0, 0.0, 0.0).into(),
            angular_speed: cgmath::Quaternion::zero(),
        }
    }
}

fn world_distance(a: Vector3<f32>, b: Vector3<f32>, world_size: (f32, f32)) -> f32 {
    let world = Vector3 {
        x: world_size.0,
        y: world_size.1,
        z: 0.0,
    };

    Vector3::distance(
        world_normalize(a, world_size),
        world_normalize(b, world_size),
    )
    .min(Vector3::distance(
        world_normalize(a + world, world_size),
        world_normalize(b + world, world_size),
    ))
}

fn world_normalize(position: Vector3<f32>, world_size: (f32, f32)) -> Vector3<f32> {
    Vector3 {
        x: position.x % world_size.0,
        y: position.y % world_size.1,
        z: position.z,
    }
}
