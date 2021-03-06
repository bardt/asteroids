use super::components::{self, Collision, Control, Health, Lifetime, Light, Physics, Renderable};
use super::geometry::{self, Shape};
use super::world::WorldPosition;

use crate::instance::Instance;
use crate::resource::Resources;
use crate::shaders::ShaderName;
use cgmath::{prelude::*, Deg};
use cgmath::{InnerSpace, Zero};
use core::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone)]
pub struct Entity {
    pub name: &'static str,
    pub rotation: cgmath::Quaternion<f32>,
    position: WorldPosition,
    entered_world: bool, // @TODO: find a way to set it whenever position changes
    pub shape: Option<geometry::Shape>,
    pub renderable: Option<components::Renderable>,
    pub physics: Option<components::Physics>,
    pub collision: Option<components::Collision>,
    pub control: Option<components::Control>,
    pub health: Option<components::Health>,
    pub lifetime: Option<components::Lifetime>,
    pub light: Option<components::Light>,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            name: "",
            position: WorldPosition::default(),
            rotation: cgmath::Quaternion::zero(),
            // @TODO: reconsider if asteroids enter the world by default.
            // Reason: when asteroid breaks into smaller parts at the world's border, those parts can fly away into space before they enter the world and therefore stay unreachable forever.
            entered_world: false,
            shape: None,
            renderable: None,
            physics: None,
            collision: None,
            control: None,
            health: None,
            lifetime: None,
            light: None,
        }
    }
}

impl Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("[{} at {}]", self.name, self.position))
    }
}

impl Entity {
    #[allow(dead_code)]
    pub fn new(name: &'static str, position: WorldPosition) -> Self {
        Self {
            name,
            position,
            ..Default::default()
        }
    }

    pub fn to_instance(&self) -> Instance {
        Instance {
            position: self.position.to_vector3(),
            rotation: self.rotation,
        }
    }

    pub fn position(&self) -> WorldPosition {
        self.position
    }

    pub fn entered_world(&self) -> bool {
        self.entered_world
    }

    pub fn update_physics(&mut self, dtime: &Duration) {
        let speeds = if let Some(ref mut physics) = self.physics {
            // Limit maximum speed
            if physics.linear_speed.magnitude2() > 0. {
                let new_magnitude = physics
                    .max_linear_speed
                    .min(physics.linear_speed.magnitude());
                physics.linear_speed = physics.linear_speed.normalize_to(new_magnitude);
            }

            Some((physics.linear_speed, physics.angular_speed))
        } else {
            None
        };

        if let Some((linear_speed, angular_speed)) = speeds {
            // Move
            self.translate(linear_speed * (dtime.as_millis() as f32) / 1000.0);

            // Rotate
            self.rotation = cgmath::Quaternion::nlerp(
                self.rotation,
                self.rotation * angular_speed,
                (dtime.as_millis() as f32) / 1000.0,
            );
        }
    }

    fn translate(&mut self, v: cgmath::Vector2<f32>) {
        self.position = if self.entered_world {
            self.position.translate(v)
        } else {
            self.position.translate_unsafe(v)
        };

        self.entered_world = self.entered_world
            || if let Some(shape) = self.shape {
                match shape {
                    Shape::Circle { origin, radius } => {
                        let rect = self.position.world_rect();
                        let center = origin
                            .translate_unsafe(self.position.to_vector2())
                            .to_tuple();
                        rect.contains_circle(center, radius)
                    }
                }
            } else {
                // Shapeless entities always fit in the world
                true
            };
    }
}

pub struct EntityFactory {
    pub resources: Rc<Resources>,
}

impl EntityFactory {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        EntityFactory {
            resources: Rc::new(Resources::ZERO),
        }
    }

    pub fn make_asteroid_s(&self, position: WorldPosition) -> Entity {
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Asteroid_S").unwrap();
        Entity {
            name: "Asteroid_S",
            position,
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            renderable: Some(Renderable {
                shader: ShaderName::Model,
                mesh: mesh_id,
                material: mesh.material,
            }),
            physics: Some(Physics::random(10., 100.)),
            shape: Some(Shape::Circle {
                origin: position.to_zero(),
                radius: 1.0,
            }),
            light: Some(Light {
                color: [0., 0.3, 0.7],
                radius: 5.,
                z: 5.,
            }),
            collision: Some(Collision {
                on_collision: |gamestate, this_id, _other_ids| gamestate.kill(this_id),
            }),
            ..Default::default()
        }
    }

    pub fn make_asteroid_m(&self, position: WorldPosition) -> Entity {
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Asteroid_M").unwrap();
        Entity {
            name: "Asteroid_M",
            position,
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            renderable: Some(Renderable {
                shader: ShaderName::Model,
                mesh: mesh_id,
                material: mesh.material,
            }),
            physics: Some(Physics::random(10., 100.)),
            shape: Some(Shape::Circle {
                origin: position.to_zero(),
                radius: 3.0,
            }),
            light: Some(Light {
                color: [0., 0.3, 0.7],
                radius: 10.,
                z: 10.,
            }),
            collision: Some(Collision {
                on_collision: |gamestate, this_id, _other_ids| {
                    let this_option = gamestate.get_entity(this_id);
                    let mut to_spawn = Vec::with_capacity(2);
                    match this_option {
                        Some(this) => {
                            to_spawn.push(
                                gamestate
                                    .entity_factory
                                    .make_asteroid_s(this.position.translate((1.5, 0.0).into())),
                            );
                            to_spawn.push(
                                gamestate
                                    .entity_factory
                                    .make_asteroid_s(this.position.translate((-1.5, 0.0).into())),
                            );
                        }
                        None => (),
                    }

                    for e in to_spawn {
                        gamestate.push(e);
                    }

                    gamestate.kill(this_id)
                },
            }),
            ..Default::default()
        }
    }

    pub fn make_asteroid_l(&self, position: WorldPosition) -> Entity {
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Asteroid_L").unwrap();
        Entity {
            name: "Asteroid_L",
            position,
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            renderable: Some(Renderable {
                shader: ShaderName::Model,
                mesh: mesh_id,
                material: mesh.material,
            }),
            physics: Some(Physics::random(5., 100.)),
            shape: Some(Shape::Circle {
                origin: position.to_zero(),
                radius: 5.0,
            }),
            light: Some(Light {
                color: [0., 0.3, 0.7],
                radius: 15.,
                z: 15.,
            }),
            collision: Some(Collision {
                on_collision: |gamestate, this_id, _other_ids| {
                    let mut to_spawn = Vec::with_capacity(2);
                    if let Some(this) = gamestate.get_entity(this_id) {
                        to_spawn.push(
                            gamestate
                                .entity_factory
                                .make_asteroid_m(this.position.translate((3.5, 0.0).into())),
                        );
                        to_spawn.push(
                            gamestate
                                .entity_factory
                                .make_asteroid_m(this.position.translate((-3.5, 0.0).into())),
                        );
                        to_spawn.push(
                            gamestate
                                .entity_factory
                                .make_cloud(this.position, cgmath::Quaternion::zero()),
                        )
                    }

                    for e in to_spawn {
                        gamestate.push(e);
                    }

                    gamestate.kill(this_id)
                },
            }),
            ..Default::default()
        }
    }

    pub fn make_spaceship(&self, position: WorldPosition, rotation_angle: f32) -> Entity {
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Spaceship").unwrap();
        Entity {
            name: "Spaceship",
            position,
            rotation: cgmath::Quaternion::from_angle_z(Deg(rotation_angle)),
            renderable: Some(Renderable {
                shader: ShaderName::Model,
                mesh: mesh_id,
                material: mesh.material,
            }),
            physics: Some(Physics {
                max_linear_speed: 60.,
                ..Default::default()
            }),
            shape: Some(Shape::Circle {
                origin: position.to_zero(),
                radius: 5.0,
            }),
            light: Some(Light {
                color: [1., 0.7, 0.3],
                radius: 30.,
                z: 15.,
            }),
            collision: Some(Collision {
                on_collision: |gamestate, this_id, other_ids| {
                    let asteroids_number = other_ids
                        .iter()
                        .flat_map(|id| gamestate.get_entity(*id))
                        .filter(|entity| entity.name.starts_with("Asteroid"))
                        .count();

                    let this = gamestate.get_entity_mut(this_id).unwrap();

                    match &mut this.health {
                        Some(health) => {
                            health.deal_damage(asteroids_number);
                            if health.level == 0 {
                                gamestate.kill(this_id);
                            }
                        }
                        None => (),
                    }
                },
            }),
            control: Some(Control::enabled()),
            health: Some(Health {
                level: 3,
                invincible: false,
            }),
            ..Default::default()
        }
    }

    pub fn make_laser(
        &self,
        position: WorldPosition,
        rotation: cgmath::Quaternion<f32>,
        relative_speed: cgmath::Vector2<f32>,
    ) -> Entity {
        let init_speed = 80.;
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Laser").unwrap();
        Entity {
            name: "Laser",
            position,
            rotation,
            renderable: Some(Renderable {
                shader: ShaderName::Model,
                mesh: mesh_id,
                material: mesh.material,
            }),
            physics: Some(Physics {
                linear_speed: (rotation.rotate_vector(cgmath::Vector3::unit_y())).truncate()
                    * init_speed
                    + relative_speed,
                max_linear_speed: 1000.,
                angular_speed: cgmath::Quaternion::zero(),
            }),
            lifetime: Some(Lifetime {
                dies_after: Duration::from_secs(1),
            }),
            shape: Some(Shape::Circle {
                origin: position.to_zero(),
                radius: 1.,
            }),
            light: Some(Light {
                color: [1., 0.7, 0.3],
                radius: 10.,
                z: 0.,
            }),
            collision: Some(Collision {
                on_collision: |gamestate, this_id, other_ids| {
                    let mut should_kill_self = false;

                    for id in other_ids {
                        match gamestate.get_entity(*id) {
                            Some(other) => {
                                if other.name.starts_with("Asteroid") {
                                    gamestate.score += 1;
                                    should_kill_self = true;
                                }
                            }
                            None => (),
                        }
                    }

                    if should_kill_self {
                        gamestate.kill(this_id);
                    }
                },
            }),
            ..Default::default()
        }
    }

    pub fn make_cloud(&self, position: WorldPosition, rotation: cgmath::Quaternion<f32>) -> Entity {
        let (mesh_id, mesh) = self.resources.get_mesh_by_name("Cloud_L").unwrap();
        Entity {
            name: "Cloud_L",
            position,
            rotation,
            entered_world: true,
            renderable: Some(Renderable {
                shader: ShaderName::Texture,
                mesh: mesh_id,
                material: mesh.material,
            }),
            lifetime: Some(Lifetime {
                dies_after: Duration::from_secs(1),
            }),
            ..Default::default()
        }
    }
}
