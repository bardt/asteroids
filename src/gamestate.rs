mod collision;
pub mod components;
mod entity;
pub mod geometry;
pub mod world;

use crate::debug;
use crate::instance::InstanceRaw;

use crate::resource::Resources;
use crate::shaders::ShaderName;
use crate::{input::Input, instance::Instance};
use cgmath::prelude::*;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use shared::LightUniform;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use rand::Rng;

use self::components::{Health, Renderable};
use self::entity::{Entity, EntityFactory};
use self::world::World;

pub struct GameState {
    entities: Vec<Option<Entity>>,
    pub world: World,
    last_update: Instant,
    score: usize,
    pub entity_factory: EntityFactory,
    pub cutscene_mode: bool,
}

#[allow(dead_code)]
type EntityIndex = usize;

type GroupedForRender<T> = BTreeMap<ShaderName, BTreeMap<usize, BTreeMap<usize, Vec<T>>>>;

impl GameState {
    pub fn new_game(aspect: f32, resources: Rc<Resources>, cutscene_mode: bool) -> Self {
        let mut game = Self {
            entities: vec![],
            world: World::init(aspect),
            last_update: Instant::now(),
            score: 0,
            entity_factory: EntityFactory { resources },
            cutscene_mode,
        };

        let mut spaceship = game
            .entity_factory
            .make_spaceship(game.world.new_position((0.0, 0.0).into()), 0.);

        if cutscene_mode {
            spaceship.health = Some(Health {
                level: 3,
                invincible: true,
            });
            spaceship.renderable = None;
        }

        game.push(spaceship);

        game.spawn_asteroid();
        game.spawn_asteroid();
        game.spawn_asteroid();

        game
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
    }

    pub fn kill(&mut self, index: EntityIndex) {
        self.entities[index] = None;

        debug(&format!("Killing {}", index));
        debug(&format!("Entites: {:?}", self.entities));
    }

    pub fn score(&self) -> usize {
        self.score
    }

    pub fn spaceship_health(&self) -> Option<Health> {
        self.entities.iter().flatten().find_map(|entity| {
            if let Some(health) = entity.health {
                if entity.name == "Spaceship" {
                    Some(health)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn is_over(&self) -> bool {
        let health_level = self
            .spaceship_health()
            .map(|health| health.level)
            .unwrap_or(0);

        health_level == 0
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

        let mut asteroid = self
            .entity_factory
            .make_asteroid_l(self.world.new_position(position.into()));
        let direction_towards_world_center = asteroid.position().to_vector2() * -1.;
        if let Some(physics) = &mut asteroid.physics {
            physics.linear_speed =
                direction_towards_world_center.normalize_to(physics.linear_speed.magnitude());
        }
        self.push(asteroid);
    }

    pub fn get_entity(&self, id: EntityIndex) -> Option<&Entity> {
        self.entities.get(id).unwrap().as_ref()
    }

    pub fn get_entity_mut(&mut self, id: EntityIndex) -> Option<&mut Entity> {
        self.entities.get_mut(id).unwrap().as_mut()
    }

    pub fn entities_grouped_by_name(&self) -> Vec<(&str, Vec<&Entity>)> {
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

    pub fn entities_grouped(&self) -> GroupedForRender<&Entity> {
        let mut shaders_map: GroupedForRender<&Entity> = BTreeMap::new();

        self.entities.iter().flatten().for_each(|entity| {
            if let Some(Renderable {
                shader,
                mesh,
                material,
            }) = entity.renderable
            {
                match shaders_map.get_mut(&shader) {
                    Some(mesh_map) => match mesh_map.get_mut(&mesh) {
                        Some(material_map) => match material_map.get_mut(&material) {
                            Some(entities) => {
                                entities.push(entity);
                            }
                            None => {
                                material_map.insert(material, vec![entity]);
                            }
                        },
                        None => {
                            let mut material_map = BTreeMap::new();
                            material_map.insert(material, vec![entity]);
                            mesh_map.insert(mesh, material_map);
                        }
                    },

                    None => {
                        let mut material_map = BTreeMap::new();
                        let mut mesh_map = BTreeMap::new();

                        material_map.insert(material, vec![entity]);
                        mesh_map.insert(mesh, material_map);
                        shaders_map.insert(shader, mesh_map);
                    }
                }
            }
        });

        shaders_map
    }

    pub fn instances_grouped(&self) -> GroupedForRender<Instance> {
        let world = &self.world;
        map_btreemap(&self.entities_grouped(), |mesh_map| {
            map_btreemap(mesh_map, |mat_map| {
                map_btreemap(mat_map, |entities| {
                    entities
                        .par_iter()
                        .map(|entity| world.add_ghost_instances(entity))
                        .flatten()
                        .collect::<Vec<Instance>>()
                })
            })
        })
    }

    pub fn instances_raw(&self) -> Vec<InstanceRaw> {
        self.instances_grouped()
            .iter()
            .map(|(_shader_name, mesh_map)| mesh_map)
            .flatten()
            .map(|(_mesh_id, mat_map)| mat_map)
            .flatten()
            .map(|(_mat_id, instances)| instances)
            .flatten()
            .map(|instance| Instance::to_raw(instance))
            .collect::<Vec<_>>()
    }

    pub fn light_uniforms(&self) -> Vec<LightUniform> {
        self.entities
            .iter()
            .flatten()
            .flat_map(|entity| {
                entity.light.map(|light| {
                    let mut rect = self.world.rect();
                    // Expending world rect so to fit lights which radius touches the visible space from the outside
                    rect.expand(light.radius);

                    let instances = self.world.add_ghost_instances(entity);
                    instances
                        .par_iter()
                        .filter(|instance| rect.contains_point(instance.position.truncate().into()))
                        .map(|instance| light.uniform(instance.position.truncate()))
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    fn delta_time(&self) -> Duration {
        self.last_update.elapsed()
    }

    pub fn control_system(&mut self, input: &Input) -> &mut Self {
        let mut to_spawn = vec![];

        let delta_time = self.delta_time();
        for option_entity in &mut self.entities {
            if let Some(entity) = option_entity {
                let position = entity.position();
                match (&mut entity.control, &mut entity.physics) {
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
                                        to_spawn.push(self.entity_factory.make_laser(
                                            position,
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
                }
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
                    .shape
                    .as_ref()
                    .map(|shape| shape.translate(entity.position().to_vector2())),
                None => None,
            })
            .collect::<Vec<_>>();

        for collision_group in collision::find_collisions(shapes) {
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
            if let Some(Entity {
                lifetime: Some(ref mut lifetime),
                ..
            }) = option_entity
            {
                if lifetime.dies_after >= dtime {
                    lifetime.dies_after -= dtime;
                } else {
                    to_kill.push(id);
                }
            }
        }

        for id in to_kill {
            self.kill(id);
        }

        self
    }

    pub fn asteroids_count(&self) -> usize {
        self.entities
            .par_iter()
            .filter_map(|entity_option| {
                entity_option
                    .as_ref()
                    .filter(|entity| entity.name.starts_with("Asteroid"))
            })
            .count()
    }

    pub fn asteroids_spawn_system(&mut self) -> &mut Self {
        if self.asteroids_count() < 3 {
            self.spawn_asteroid();
        }

        self
    }

    pub fn submit(&mut self) {
        self.last_update = Instant::now();
    }
}

#[test]
fn test_gamestate_asteroids_count() {
    let world = World::init(1.0);
    let default_position = world.new_position((0.0, 0.0).into());
    let a1 = Entity::new("Asteroid_1", default_position.clone());
    let a2 = Entity::new("Asteroid_2", default_position.clone());
    let s = Entity::new("Spaceship", default_position.clone());

    let entities = vec![
        Some(s.clone()),
        Some(a1.clone()),
        None,
        Some(a1.clone()),
        Some(a2.clone()),
    ];

    let gamestate = GameState {
        entities,
        world,
        last_update: Instant::now(),
        score: 0,
        entity_factory: EntityFactory::empty(),
        cutscene_mode: false,
    };

    assert_eq!(gamestate.asteroids_count(), 3);
}

#[test]
fn test_gamestate_entities_grouped_by_name() {
    let world = World::init(1.0);
    let default_position = world.new_position((0.0, 0.0).into());
    let a = Entity::new("A", default_position.clone());
    let b = Entity::new("B", default_position.clone());

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
        world,
        last_update: Instant::now(),
        score: 0,
        entity_factory: EntityFactory::empty(),
        cutscene_mode: false,
    };

    let expected = vec![
        ("A", vec![a.clone(), a.clone()]),
        ("B", vec![a.clone(), a.clone()]),
        ("A", vec![a.clone(), a.clone()]),
    ];

    assert_eq!(gamestate.entities_grouped_by_name().len(), expected.len());
    assert_eq!(gamestate.entities_grouped_by_name()[0].0, expected[0].0);
    assert_eq!(
        gamestate.entities_grouped_by_name()[0].1.len(),
        expected[0].1.len()
    );
    assert_eq!(gamestate.entities_grouped_by_name()[1].0, expected[1].0);
    assert_eq!(
        gamestate.entities_grouped_by_name()[1].1.len(),
        expected[1].1.len()
    );
    assert_eq!(gamestate.entities_grouped_by_name()[2].0, expected[2].0);
    assert_eq!(
        gamestate.entities_grouped_by_name()[2].1.len(),
        expected[2].1.len()
    );
}

fn map_btreemap<K, V, W, F>(btreemap: &BTreeMap<K, V>, f: F) -> BTreeMap<K, W>
where
    F: Fn(&V) -> W,
    K: Copy,
    K: std::cmp::Eq,
    K: std::cmp::Ord,
    K: std::hash::Hash,
{
    BTreeMap::from_iter(btreemap.iter().map(|(k, v)| (*k, f(v))))
}
