use crate::collision;
use crate::components::Collision;
use crate::components::Control;
use crate::components::Health;
use crate::components::Lifetime;
use crate::components::Physics;

use crate::components::Shape;
use crate::entity::Entity;

use crate::world::World;
use crate::world::WorldPosition;
use crate::Mode;
use crate::MODE;
use crate::{input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

use std::time::Duration;
use std::time::Instant;

pub struct GameState {
    entities: Vec<Option<Entity>>,
    pub world: World,
    last_update: Instant,
}

#[allow(dead_code)]
type EntityIndex = usize;

impl GameState {
    pub fn new_game(aspect: f32) -> Self {
        let mut game = Self {
            entities: vec![],
            world: World::init(aspect),
            last_update: Instant::now(),
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

            physics: Some(Physics {
                max_linear_speed: 60.,
                ..Default::default()
            }),
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

    pub fn make_laser(
        position: WorldPosition,
        rotation: cgmath::Quaternion<f32>,
        relative_speed: cgmath::Vector3<f32>,
    ) -> Entity {
        let init_speed = 80.;

        Entity {
            name: "Laser",
            position,
            rotation,
            physics: Some(Physics {
                linear_speed: rotation.rotate_vector(cgmath::Vector3::unit_y()) * init_speed
                    + relative_speed,
                max_linear_speed: 1000.,
                angular_speed: cgmath::Quaternion::zero(),
            }),
            lifetime: Some(Lifetime {
                dies_after: Duration::from_secs(1),
            }),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: position.to_zero(),
                    radius: 1.,
                },
                on_collision: |gamestate, this_id, other_ids| {
                    let mut should_kill_self = false;

                    for id in other_ids {
                        match gamestate.get_entity(*id) {
                            Some(other) => {
                                if other.name == "Asteroid" {
                                    gamestate.kill(*id);
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

    pub fn push(&mut self, entity: Entity) {
        self.entities.push(Some(entity));

        match MODE {
            Mode::Debug => {
                println!("===");
                println!("{:?}", &entity);
                println!("Entites: {:?}", self.entities);
                println!("===");
            }
            _ => (),
        }
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

    fn delta_time(&self) -> Duration {
        self.last_update.elapsed()
    }

    pub fn control_system(&mut self, input: &Input) -> &mut Self {
        let mut to_spawn = vec![];

        let delta_time = self.delta_time();
        for option_entity in &mut self.entities {
            match option_entity {
                Some(entity) => match (&mut entity.control, &mut entity.physics) {
                    (Some(control), Some(physics)) => {
                        if control.enabled {
                            let rotation_speed = 180.;
                            let linear_acceleration = 50.;
                            {
                                let dtime = (delta_time.as_millis() as f32) / 1000.0;
                                let delta_angle = dtime * rotation_speed;
                                let delta_linear_speed = dtime * linear_acceleration;

                                let direction =
                                    entity.rotation.rotate_vector(cgmath::Vector3::unit_y());

                                if input.is_forward_pressed {
                                    physics.linear_speed += direction * delta_linear_speed;
                                }

                                if input.is_right_pressed {
                                    entity.rotation = entity.rotation
                                        * cgmath::Quaternion::from_angle_z(cgmath::Deg(
                                            -delta_angle,
                                        ))
                                }

                                if input.is_left_pressed {
                                    entity.rotation = entity.rotation
                                        * cgmath::Quaternion::from_angle_z(cgmath::Deg(delta_angle))
                                }
                            }

                            {
                                if control.weapon_cooldown < delta_time {
                                    if input.is_backward_pressed {
                                        to_spawn.push(GameState::make_laser(
                                            entity.position,
                                            entity.rotation,
                                            entity.physics.unwrap().linear_speed,
                                        ));
                                        control.weapon_cooldown = Duration::from_millis(300);
                                    } else {
                                        control.weapon_cooldown = Duration::ZERO
                                    }
                                } else {
                                    control.weapon_cooldown -= delta_time;
                                }
                            }
                        }
                    }
                    _ => (),
                },
                None => (),
            }
        }

        to_spawn.into_iter().for_each(|entity| self.push(entity));

        self
    }

    pub fn physics_system(&mut self) -> &mut Self {
        let dtime = self.delta_time();
        self.entities
            .par_iter_mut()
            .for_each(|option_entity| match option_entity {
                Some(entity) => entity.update_physics(&dtime),
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

    pub fn lifetime_system(&mut self) -> &mut Self {
        let mut to_kill = vec![];
        let dtime = self.delta_time();
        for (id, option_entity) in self.entities.iter_mut().enumerate() {
            match option_entity {
                Some(entity) => match &mut entity.lifetime {
                    Some(lifetime) => {
                        if lifetime.dies_after >= dtime {
                            lifetime.dies_after -= dtime;
                        } else {
                            to_kill.push(id);
                        }
                    }
                    None => (),
                },
                None => (),
            }
        }

        for id in to_kill {
            self.kill(id);
        }

        self
    }

    pub fn submit(&mut self) {
        self.last_update = Instant::now();
    }
}
