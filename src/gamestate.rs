use crate::collision;
use crate::components::Collision;
use crate::components::Control;
use crate::components::Health;
use crate::components::Lifetime;
use crate::components::Physics;

use crate::components::Shape;
use crate::debug;
use crate::entity::Entity;

use crate::instance::InstanceRaw;
use crate::world::World;
use crate::world::WorldPosition;

use crate::{input::Input, instance::Instance};
use cgmath::prelude::*;
use cgmath::Deg;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

use std::time::Duration;
use std::time::Instant;

use rand::Rng;

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

        game.push(game.make_spaceship((0.0, 0.0), 0.));
        game.push(game.make_asteroid_s((25.0, 25.0)));
        game.push(game.make_asteroid_m((-25.0, 25.0)));
        game.push(game.make_asteroid_l((25.0, -25.0)));

        game
    }

    pub fn make_asteroid_s(&self, position: (f32, f32)) -> Entity {
        Entity {
            name: "Asteroid_S",
            position: self.world.new_position(position.into()),
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            physics: Some(Physics::random(10., 100.)),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: self.world.new_position((0.0, 0.0).into()),
                    radius: 1.0,
                },
                on_collision: |gamestate, this_id, _other_ids| gamestate.kill(this_id),
            }),
            ..Default::default()
        }
    }

    pub fn make_asteroid_m(&self, position: (f32, f32)) -> Entity {
        Entity {
            name: "Asteroid_M",
            position: self.world.new_position(position.into()),
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            physics: Some(Physics::random(10., 100.)),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: self.world.new_position((0.0, 0.0).into()),
                    radius: 3.0,
                },
                on_collision: |gamestate, this_id, _other_ids| {
                    let this_option = gamestate.get_entity(this_id);
                    let mut to_spawn = Vec::with_capacity(2);
                    match this_option {
                        Some(this) => {
                            to_spawn.push(gamestate.make_asteroid_s(
                                this.position.translate((1.5, 0.0).into()).to_tuple(),
                            ));
                            to_spawn.push(gamestate.make_asteroid_s(
                                this.position.translate((-1.5, 0.0).into()).to_tuple(),
                            ));
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

    pub fn make_asteroid_l(&self, position: (f32, f32)) -> Entity {
        Entity {
            name: "Asteroid_L",
            position: self.world.new_position(position.into()),
            rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), Deg(0.0)),
            physics: Some(Physics::random(5., 100.)),
            collision: Some(Collision {
                shape: Shape::Sphere {
                    origin: self.world.new_position((0.0, 0.0).into()),
                    radius: 5.0,
                },
                on_collision: |gamestate, this_id, _other_ids| {
                    let this_option = gamestate.get_entity(this_id);
                    let mut to_spawn = Vec::with_capacity(2);
                    match this_option {
                        Some(this) => {
                            to_spawn.push(gamestate.make_asteroid_m(
                                this.position.translate((3.5, 0.0).into()).to_tuple(),
                            ));
                            to_spawn.push(gamestate.make_asteroid_m(
                                this.position.translate((-3.5, 0.0).into()).to_tuple(),
                            ));
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

    pub fn make_spaceship(&self, position: (f32, f32), rotation_angle: f32) -> Entity {
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
                    origin: self.world.new_position((0.0, 0.0).into()),
                    radius: 5.0,
                },
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
            health: Some(Health { level: 3 }),
            ..Default::default()
        }
    }

    pub fn make_laser(
        position: WorldPosition,
        rotation: cgmath::Quaternion<f32>,
        relative_speed: cgmath::Vector2<f32>,
    ) -> Entity {
        let init_speed = 80.;

        Entity {
            name: "Laser",
            position,
            rotation,
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
                                if other.name.starts_with("Asteroid") {
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
        let first_vacant_id = self.entities.iter().enumerate().find_map(|(id, entity)| {
            if Option::is_none(entity) {
                Some(id)
            } else {
                None
            }
        });

        match first_vacant_id {
            Some(id) => self.entities[id] = Some(entity),
            None => self.entities.push(Some(entity)),
        }

        debug(&format!("Pushing {:?}", &entity));
        debug(&format!("Entites: {:?}", self.entities));
    }

    pub fn kill(&mut self, index: EntityIndex) {
        self.entities[index] = None;

        debug(&format!("Killing {}", index));
        debug(&format!("Entites: {:?}", self.entities));
    }

    pub fn spawn_asteroid(&mut self) {
        // Spawn outside of the world
        let mut rng = rand::thread_rng();
        let asteroid_radius = 5.;
        let (w, h) = self.world.size;

        let mut position = (
            rng.gen_range(0.0..w) - w / 2.,
            rng.gen_range(0.0..h) - h / 2.,
        );
        if rng.gen_bool(0.5) {
            let left = rng.gen_bool(0.5);
            position.0 = (w / 2. + asteroid_radius) * if left { -1. } else { 1. };
        } else {
            let bottom = rng.gen_bool(0.5);
            position.1 = (h / 2. + asteroid_radius) * if bottom { -1. } else { 1. };
        }

        // @TODO: make it always directing routhly towards the center of the world
        let mut asteroid = self.make_asteroid_l(position);
        asteroid.collision = None; // @TODO: turn collision back on
        self.push(asteroid);
    }

    pub fn get_entity(&self, id: EntityIndex) -> Option<&Entity> {
        self.entities.get(id).unwrap().as_ref()
    }

    pub fn get_entity_mut(&mut self, id: EntityIndex) -> Option<&mut Entity> {
        self.entities.get_mut(id).unwrap().as_mut()
    }

    pub fn entities_grouped(&self) -> Vec<(&str, Vec<&Entity>)> {
        let mut groups = Vec::new();
        let mut group = Vec::new();
        let mut entity_name = "";

        for entity in &self.entities {
            if let Some(entity) = entity {
                if entity_name == "" {
                    entity_name = entity.name;
                }

                if entity_name == entity.name {
                    group.push(entity);
                } else {
                    groups.push((entity_name, group));
                    entity_name = entity.name;
                    group = vec![entity];
                }
            }
        }

        groups.push((entity_name, group));
        groups
    }

    pub fn instances_grouped(&self) -> Vec<(&str, Vec<Instance>)> {
        let world = &self.world;
        self.entities_grouped()
            .par_iter()
            .map(|(name, entities)| {
                (
                    *name,
                    entities
                        .par_iter()
                        .map(|entity| world.add_ghost_instances(&entity.to_instance()))
                        .flatten()
                        .collect::<Vec<Instance>>(),
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn instances_raw(&self) -> Vec<InstanceRaw> {
        self.instances_grouped()
            .par_iter()
            .map(|(_name, instances)| instances)
            .flatten()
            .map(|instance| Instance::to_raw(instance))
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

                                let direction = entity
                                    .rotation
                                    .rotate_vector(cgmath::Vector3::unit_y())
                                    .truncate();

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
                                        control.weapon_cooldown = Duration::from_millis(200);
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
                    .map(|collision| collision.shape.translate(entity.position.to_vector2())),
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

                match self.get_entity(*this_id) {
                    Some(this) => match this.collision {
                        Some(collision) => {
                            (collision.on_collision)(self, *this_id, other_ids.as_slice());
                        }
                        None => (),
                    },
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

    pub fn asteroids_spawn_system(&mut self) -> &mut Self {
        let number_of_asteroids = self
            .entities
            .par_iter()
            .filter_map(|entity_option| {
                entity_option.map(|entity| entity.name.starts_with("Asteroid"))
            })
            .count();
        if number_of_asteroids < 3 {
            self.spawn_asteroid();
        }

        self
    }

    pub fn submit(&mut self) {
        self.last_update = Instant::now();
    }

    pub fn global_input_system(&mut self, input: &Input) -> &mut Self {
        if input.is_spawn_pressed {
            self.spawn_asteroid();
        }
        self
    }
}

#[test]
fn test_gamestate_entities_grouped() {
    let a = Entity {
        name: "A",
        ..Default::default()
    };
    let b = Entity {
        name: "B",
        ..Default::default()
    };

    let entities = vec![
        Some(a.clone()),
        Some(a.clone()),
        Some(b.clone()),
        Some(b.clone()),
        Some(a.clone()),
        None,
        Some(a.clone()),
    ];

    let gamestate = GameState {
        entities,
        world: World::init(1.0),
        last_update: Instant::now(),
    };

    let expected = vec![
        ("A", vec![a.clone(), a.clone()]),
        ("B", vec![a.clone(), a.clone()]),
        ("A", vec![a.clone(), a.clone()]),
    ];

    assert_eq!(gamestate.entities_grouped().len(), expected.len());
    assert_eq!(gamestate.entities_grouped()[0].0, expected[0].0);
    assert_eq!(gamestate.entities_grouped()[0].1.len(), expected[0].1.len());
    assert_eq!(gamestate.entities_grouped()[1].0, expected[1].0);
    assert_eq!(gamestate.entities_grouped()[1].1.len(), expected[1].1.len());
    assert_eq!(gamestate.entities_grouped()[2].0, expected[2].0);
    assert_eq!(gamestate.entities_grouped()[2].1.len(), expected[2].1.len());
}
