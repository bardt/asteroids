use cgmath::InnerSpace;
use cgmath::Zero;
use core::fmt::Debug;
use std::time::Duration;

use crate::{components, world::WorldPosition};

#[derive(Clone, Copy)]
pub struct Entity {
    pub name: &'static str,
    pub position: WorldPosition,
    pub rotation: cgmath::Quaternion<f32>,
    pub physics: Option<components::Physics>,
    pub collision: Option<components::Collision>,
    pub control: Option<components::Control>,
    pub health: Option<components::Health>,
    pub lifetime: Option<components::Lifetime>,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            name: "",
            position: WorldPosition::default(),
            rotation: cgmath::Quaternion::zero(),
            physics: None,
            collision: None,
            control: None,
            health: None,
            lifetime: None,
        }
    }
}

impl Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("[{} at {}]", self.name, self.position))
    }
}

impl Entity {
    pub fn _direction(&self, _dtime: &Duration) {}

    pub fn update_physics(&mut self, dtime: &Duration) {
        match &mut self.physics {
            Some(physics) => {
                // Limit maximum speed
                if physics.linear_speed.magnitude2() > 0. {
                    let new_magnitude = physics
                        .max_linear_speed
                        .min(physics.linear_speed.magnitude());
                    physics.linear_speed = physics.linear_speed.normalize_to(new_magnitude);
                }

                // Move
                self.position = self
                    .position
                    .translate(physics.linear_speed * (dtime.as_millis() as f32) / 1000.0);

                // Rotate
                self.rotation = cgmath::Quaternion::nlerp(
                    self.rotation,
                    self.rotation * physics.angular_speed,
                    (dtime.as_millis() as f32) / 1000.0,
                );
            }
            None => (),
        }
    }
}
