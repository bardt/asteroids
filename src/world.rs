use cgmath::Vector3;

use crate::{
    camera::Camera,
    gamestate::{Entity, GameState},
    instance::Instance,
};

const WORLD_SIZE_MIN: f32 = 100.;

pub struct World {
    pub size: (f32, f32),
    pub gamestate: GameState,
    pub camera: Camera,
}

impl World {
    pub fn init(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut gamestate = GameState::default();

        gamestate.push(Entity::make_spaceship((0.0, 0.0, 0.0), 0.));

        gamestate.push(Entity::make_asteroid((5.0, 5.0, 0.0)));
        gamestate.push(Entity::make_asteroid((-5.0, 5.0, 0.0)));
        gamestate.push(Entity::make_asteroid((5.0, -5.0, 0.0)));

        let (size, camera) =
            Self::world_size_and_camera(config.width as f32 / config.height as f32);

        Self {
            size,
            gamestate,
            camera,
        }
    }

    pub fn resize(&mut self, config: &wgpu::SurfaceConfiguration) {
        let aspect = config.width as f32 / config.height as f32;
        let (size, camera) = Self::world_size_and_camera(aspect);

        self.size = size;
        self.camera = camera;
    }

    fn world_size_and_camera(aspect: f32) -> ((f32, f32), Camera) {
        let mut world_width = WORLD_SIZE_MIN;
        let mut world_height = WORLD_SIZE_MIN;
        if aspect > 1. {
            world_width = world_height * aspect;
        } else {
            world_height = world_width / aspect;
        }

        let size = (world_width, world_height);

        let camera = Camera {
            eye: (0.0, -1.0, WORLD_SIZE_MIN * 10.).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            left: -world_width / 2.,
            right: world_width / 2.,
            top: world_height / 2.,
            bottom: -world_height / 2.,
            near: WORLD_SIZE_MIN * 10. - 2.,
            far: WORLD_SIZE_MIN * 10. + 2.,
        };

        (size, camera)
    }

    /// Add fake instances to make the world visually looping
    pub(crate) fn add_ghost_instances(&self, instance: &Instance) -> Vec<Instance> {
        let mut instances = Vec::with_capacity(9);

        for row in (-1)..=1 {
            for col in (-1)..=1 {
                let mut ghost_instance = instance.clone();
                ghost_instance.position = Vector3 {
                    x: ghost_instance.position.x + self.size.0 * (col as f32),
                    y: ghost_instance.position.y + self.size.1 * (row as f32),
                    z: ghost_instance.position.z,
                };

                instances.push(ghost_instance)
            }
        }

        instances
    }
}
