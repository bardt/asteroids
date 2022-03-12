use crate::gamestate::geometry::Rect;
use crate::model::Material;
use crate::shaders::Shaders;
use crate::texture::{Texture, TextureRenderer};
use image::DynamicImage;

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

        let transparent_image = image::RgbaImage::new(1, 1);
        let transparent_texture = Texture::from_image(
            device,
            queue,
            &DynamicImage::ImageRgba8(transparent_image),
            Some("Transparent 1x1 texture"),
            false,
        )
        .unwrap();

        let material =
            Material::from_texture(device, queue, "Transparent", transparent_texture).unwrap();

        Self {
            vertex_buffer,
            texture_renderer,
            material,
        }
    }

    pub fn render<'a, 'b>(&'b self, shaders: &'a Shaders, render_pass: &mut wgpu::RenderPass<'a>)
    where
        'b: 'a,
    {
        render_pass.set_pipeline(&shaders.texture.pipeline);
        self.texture_renderer
            .draw(&self.vertex_buffer, &self.material, render_pass);
    }
}
