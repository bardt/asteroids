
use crate::camera::Camera;
use crate::instance::Instance;
use cgmath::{Rotation3, Deg};

#[derive(Clone)]
pub struct Asteroid {
    pub instance: Instance

}


pub struct World {
    pub asteroids: Vec<Asteroid>,
    pub camera : Camera

}

impl World {
    pub fn init(config: &wgpu::SurfaceConfiguration) -> Self {
        let asteroids : Vec<Asteroid> = [
            Asteroid {
                instance : Instance {
                    position: (0.0, 0.0, 0.0).into(),
                    rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),

                }
            }
        ].to_vec();

        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 5.0, 10.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
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


        Self {
            asteroids,
            camera,

        }
    }
}



