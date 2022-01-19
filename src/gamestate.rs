use crate::collision;
use crate::world::World;
use crate::world::WorldPosition;
use crate::{input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;
use rand::Rng;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

use std::time::Duration;

pub struct GameState {
    entities: Vec<Option<Entity>>,
    pub world: World,
}

#[allow(dead_code)]
type EntityIndex = usize;

impl GameState {
    pub fn new_game(aspect: f32) -> Self {
        let mut game = Self {
            entities: vec![],
            world: World::init(aspect),
        };

        game.push(game.make_spaceship((0.0, 0.0, 0.0), 0.));
        game.push(game.make_asteroid((5.0, 5.0, 0.0)));
        game.push(game.make_asteroid((-5.0, 5.0, 0.0)));
        game.push(game.make_asteroid((5.0, -5.0, 0.0)));

        game
    }

    pub fn make_asteroid(&self, position: (f32, f32, f32)) -> Entity {
        Entity {
            name: "Asteroid",
            position: self.world.new_position(position.into()),
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            physics: Some(Physics::random(1., 100.)),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: self.world.new_position((0.0, 0.0, 0.0).into()),
                    radius: 1.0,
                },
                on_collision: |gamestate, this_id, _other_ids| gamestate.kill(this_id),
            }),
            ..Default::default()
        }
    }

    pub fn make_spaceship(&self, position: (f32, f32, f32), rotation_angle: f32) -> Entity {
        Entity {
            name: "Spaceship",

            position: self.world.new_position(position.into()),
            rotation: cgmath::Quaternion::from_angle_z(Deg(rotation_angle)),

            physics: Some(Physics::default()),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: self.world.new_position((0.0, 0.0, 0.0).into()),
                    radius: 5.0,
                },
                on_collision: |gamestate, this_id, other_ids| {
                    let asteroids_number = other_ids
                        .iter()
                        .flat_map(|id| gamestate.get_entity(*id))
                        .filter(|entity| entity.name == "Asteroid")
                        .count();

                    let this = gamestate.get_entity_mut(this_id).unwrap();

                    match &mut this.health {
                        Some(health) => {
                            health.deal_damage(asteroids_number);
                            println!("Spaceship health: {:?}", health.level);
                            if health.level == 0 {
                                gamestate.kill(this_id);
                            }
                        }
                        None => (),
                    }
                },
            }),
            control: Some(Control::enabled()),
            health: Some(Health { level: 3 }),
            ..Default::default()
        }
    }

    pub fn push(&mut self, entity: Entity) {
        self.entities.push(Some(entity))
    }

    pub fn kill(&mut self, index: EntityIndex) {
        self.entities[index] = None
    }

    pub fn get_entity(&self, id: EntityIndex) -> Option<&Entity> {
        self.entities.get(id).unwrap().as_ref()
    }

    pub fn get_entity_mut(&mut self, id: EntityIndex) -> Option<&mut Entity> {
        self.entities.get_mut(id).unwrap().as_mut()
    }

    pub fn instances(&self) -> Vec<(&str, Instance)> {
        self.entities
            .iter()
            .filter_map(|option_entity| {
                option_entity.as_ref().map(|entity| {
                    (
                        entity.name,
                        Instance {
                            position: entity.position.to_vector3(),
                            rotation: entity.rotation,
                        },
                    )
                })
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

    pub fn physics_system(&mut self, delta_time: &Duration) -> &mut Self {
        self.entities
            .par_iter_mut()
            .for_each(|option_entity| match option_entity {
                Some(entity) => entity.update_physics(delta_time),
                None => (),
            });

        self
    }

    pub fn collision_system(&mut self) -> &mut Self {
        let shapes = self
            .entities
            .par_iter()
            .map(|option_entity| match option_entity {
                Some(entity) => entity
                    .collision
                    .as_ref()
                    .map(|collision| collision.shape.translate(entity.position)),
                None => None,
            })
            .collect::<Vec<_>>();

        let collisions = collision::find_collisions(shapes);

        for collision_group in collisions {
            for this_id in &collision_group {
                let other_ids = &collision_group
                    .iter()
                    .filter_map(|id| if id == this_id { None } else { Some(*id) })
                    .collect::<Vec<_>>();

                let this = self.get_entity(*this_id).unwrap();

                match this.collision {
                    Some(collision) => {
                        (collision.on_collision)(self, *this_id, other_ids.as_slice());
                    }
                    None => (),
                }
            }
        }

        self
    }
}

#[derive(Clone, Copy)]
pub struct Entity {
    pub name: &'static str,
    pub position: WorldPosition,
    pub rotation: cgmath::Quaternion<f32>,
    pub physics: Option<Physics>,
    pub collision: Option<Collision>,
    pub control: Option<Control>,
    pub health: Option<Health>,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            name: "",
            position: WorldPosition::default(),
            rotation: cgmath::Quaternion::zero(),
            physics: Option::default(),
            collision: Option::default(),
            control: Option::default(),
            health: Option::default(),
        }
    }
}

impl Entity {
    pub fn update_physics(&mut self, dtime: &Duration) {
        match &mut self.physics {
            Some(physics) => {
                // Limit maximum speed
                let max_linear_speed = 60_f32;
                if physics.linear_speed.magnitude2() > 0. {
                    let new_magnitude = max_linear_speed.min(physics.linear_speed.magnitude());
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

    pub fn update_control(&mut self, input: &Input, dtime: &Duration) {
        match (&self.control, &mut self.physics) {
            (Some(Control { enabled: true, .. }), Some(physics)) => {
                let rotation_speed = 180.;
                let linear_acceleration = 50.;

                let delta_time = (dtime.as_millis() as f32) / 1000.0;
                let delta_angle = delta_time * rotation_speed;
                let delta_linear_speed = delta_time * linear_acceleration;

                let direction = self.rotation.rotate_vector(cgmath::Vector3::unit_y()); //cgmath::Vector3 { x, y, z };

                if input.is_forward_pressed {
                    physics.linear_speed += direction * delta_linear_speed;
                }

                if input.is_right_pressed {
                    self.rotation =
                        self.rotation * cgmath::Quaternion::from_angle_z(cgmath::Deg(-delta_angle))
                }

                if input.is_left_pressed {
                    self.rotation =
                        self.rotation * cgmath::Quaternion::from_angle_z(cgmath::Deg(delta_angle))
                }
            }
            _ => (),
        }
    }
}

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
                origin: origin.translate(position.to_vector3()),
                radius,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Control {
    enabled: bool,
}

impl Control {
    fn enabled() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub struct Health {
    level: usize,
}

impl Health {
    fn deal_damage(&mut self, damage: usize) {
        self.level = (self.level - damage).max(0);
    }
}
