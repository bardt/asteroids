use crate::font::FontRenderer;
use crate::gamestate::geometry::Rect;
use crate::gamestate::GameState;
use crate::model::Material;
use crate::shaders::Shaders;
use crate::texture::TextureRenderer;

pub struct UI {
    font_renderer: FontRenderer,
    textures: Vec<(wgpu::Buffer, Option<Material>)>,
    texture_renderer: TextureRenderer,
}

impl UI {
    pub fn new(device: &wgpu::Device) -> Self {
        let font_renderer = FontRenderer::load();
        let texture_renderer = TextureRenderer::init(&device);

        Self {
            font_renderer,
            textures: vec![],
            texture_renderer,
        }
    }

    pub fn update(
        &mut self,
        gamestate: &GameState,
        fps: u128,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (world_width, world_height) = gamestate.world.size;
        let world_aspect = world_width / world_height;

        let line_height = 0.2;

        // Best quality, but very slow
        // let font_size = self.size.height as f32 * line_height / 2.0;
        // Poor quality, but fast
        let font_size = 20.;

        let padding = (font_size * 0.4, font_size * 0.4);

        let render_text = |text: String| {
            self.font_renderer.render_material(
                device,
                queue,
                text.as_str(),
                font_size,
                padding,
                (180, 100, 40),
            )
        };

        let mut left_column = if let crate::Mode::Debug = crate::MODE {
            gamestate
                .entities_grouped()
                .iter()
                .map(|(name, entities)| render_text(format!("{:?}: {:?}", name, entities.len())))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        left_column.push(render_text(format!("Score: {:?}", gamestate.score())));

        left_column.push(render_text(format!(
            "Asteroids: {:?}",
            gamestate.asteroids_count()
        )));

        let right_column = vec![render_text(format!("{:?} FPS", fps))];

        self.textures
            .resize_with(left_column.len() + right_column.len(), || {
                let vertex_buffer = TextureRenderer::init_vertex_buffer(device);
                (vertex_buffer, None)
            });

        let left_column_len = left_column.len();
        for (index, text_material) in left_column.into_iter().enumerate().collect::<Vec<_>>() {
            let count_rect = Rect {
                left_top: (-1., 1. - (index as f32) * line_height),
                right_bottom: (
                    -1. + text_material.diffuse_texture.size.width as f32
                        / text_material.diffuse_texture.size.height as f32
                        / world_aspect
                        * line_height,
                    1. - (index + 1) as f32 * line_height,
                ),
            };
            TextureRenderer::update_vertex_buffer(&self.textures[index].0, &count_rect, queue);
            self.textures[index].1 = Some(text_material);
        }

        for (index, text_material) in right_column.into_iter().enumerate().collect::<Vec<_>>() {
            let count_rect = Rect {
                left_top: (
                    1. - text_material.diffuse_texture.size.width as f32
                        / text_material.diffuse_texture.size.height as f32
                        / world_aspect
                        * line_height,
                    1. - (index as f32) * line_height,
                ),
                right_bottom: (1., 1. - (index + 1) as f32 * line_height),
            };
            TextureRenderer::update_vertex_buffer(
                &self.textures[index + left_column_len].0,
                &count_rect,
                queue,
            );
            self.textures[index + left_column_len].1 = Some(text_material);
        }
    }

    pub fn render<'a, 'b>(&'b self, shaders: &'a Shaders, render_pass: &mut wgpu::RenderPass<'a>)
    where
        'b: 'a,
    {
        render_pass.set_pipeline(&shaders.texture.pipeline);
        for (vertex_buffer, material) in &self.textures {
            if let Some(material) = material {
                self.texture_renderer
                    .draw(vertex_buffer, material, render_pass);
            }
        }
    }
}
