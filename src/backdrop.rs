use crate::gamestate::geometry::Rect;
use crate::model::Material;
use crate::texture::{Texture, TextureRenderer};

const BACKDROP_COLOR_UNIFORM: [f32; 4] = [0.0, 0.01, 0.02, 1.0];

pub struct Backdrop {
    vertex_buffer: wgpu::Buffer,
    texture_renderer: TextureRenderer,
    material: Material,
}

impl Backdrop {
    pub fn init(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture_renderer = TextureRenderer::init(&device);
        let vertex_buffer = TextureRenderer::init_vertex_buffer(&device);
        TextureRenderer::update_vertex_buffer(
            &vertex_buffer,
            &Rect::IDENTITY,
            BACKDROP_COLOR_UNIFORM,
            queue,
        );

        let diffuse_texture = Texture::create_transparent_texture(device, queue).unwrap();
        let material =
            Material::from_texture(device, queue, "Transparent", diffuse_texture).unwrap();

        Self {
            vertex_buffer,
            texture_renderer,
            material,
        }
    }

    pub fn render<'a, 'b>(&'b self, render_pass: &mut wgpu::RenderPass<'a>)
    where
        'b: 'a,
    {
        self.texture_renderer
            .draw(&self.vertex_buffer, &self.material, render_pass);
    }
}
