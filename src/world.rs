use std::time::Duration;

use crate::{camera::Camera, input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;
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
            components: vec![Component::Physics(PhysicsProps::random(1., 100.))],
        }
    }

    pub fn make_spaceship(position: (f32, f32, f32), rotation_angle: f32) -> Entity {
        Self {
            name: "Spaceship".to_string(),
            instance: Instance {
                position: position.into(),
                rotation: cgmath::Quaternion::from_angle_z(Deg(rotation_angle)),
            },
            components: vec![Component::Control, Component::Physics(PhysicsProps::init())],
        }
    }

    pub fn update_physics(&mut self, dtime: &Duration) -> &mut Self {
        self.components
            .par_iter_mut()
            .find_map_first(|component| match component {
                Component::Physics(props) => Some(&mut *props),
                _ => None,
            })
            .map(|props| Entity::update_physics_internal(&mut self.instance, props, dtime));

        self
    }

    pub fn update_control(&mut self, input: &Input, dtime: &Duration) -> &mut Self {
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
                (Some(()), Component::Physics(props)) => Some(&mut *props),
                _ => None,
            })
            .map(|props| Entity::update_control_internal(&mut self.instance, props, input, dtime));

        self
    }

    fn update_control_internal(
        instance: &mut Instance,
        physics: &mut PhysicsProps,
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
        physics_props: &PhysicsProps,
        dtime: &Duration,
    ) {
        /*
        @TODO: limit maximum linear speed
        */
        instance.position =
            instance.position + physics_props.linear_speed * (dtime.as_millis() as f32) / 1000.0;
        instance.rotation = cgmath::Quaternion::nlerp(
            instance.rotation,
            instance.rotation * physics_props.angular_speed,
            (dtime.as_millis() as f32) / 1000.0,
        );
    }
}

pub enum Component {
    Control,
    Physics(PhysicsProps),
}

pub struct PhysicsProps {
    linear_speed: cgmath::Vector3<f32>,
    angular_speed: cgmath::Quaternion<f32>,
}

impl PhysicsProps {
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

pub struct World {
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

        let camera = Camera {
            eye: (0.0, -0.01, 50.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_z(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 200.0,
            speed: 5.0,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        };

        Self { entities, camera }
    }
}
