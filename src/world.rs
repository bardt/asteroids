use crate::camera::Camera;
use crate::instance::Instance;
use cgmath::{Deg, Rotation3};

#[derive(Clone)]
pub struct Entity {
    pub name: String,
    pub instance: Instance,
}

pub struct World {
    pub entities: Vec<Entity>,
    pub camera: Camera,
}

impl World {
    pub fn init(config: &wgpu::SurfaceConfiguration) -> Self {
        let entities: Vec<Entity> = [
            Entity {
                name: "Spaceship".to_string(),
                instance: Instance {
                    position: (0.0, 0.0, 0.0).into(),
                    rotation: cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        Deg(0.0),
                    ),
                },
            },
            Entity {
                name: "Asteroid".to_string(),
                instance: Instance {
                    position: (5.0, 5.0, 0.0).into(),
                    rotation: cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        Deg(0.0),
                    ),
                },
            },
            Entity {
                name: "Asteroid".to_string(),
                instance: Instance {
                    position: (-5.0, 5.0, 0.0).into(),
                    rotation: cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        Deg(0.0),
                    ),
                },
            },
            Entity {
                name: "Asteroid".to_string(),
                instance: Instance {
                    position: (5.0, -5.0, 0.0).into(),
                    rotation: cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        Deg(0.0),
                    ),
                },
            },
        ]
        .to_vec();

        let camera = Camera {
            eye: (0.0, -0.01, 20.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_z(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
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
