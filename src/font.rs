use image::{DynamicImage, Rgba};
use rusttype::{point, Font, Scale};

use crate::{model::Material, texture::Texture};

pub struct FontRenderer {
    font: Font<'static>,
}

impl FontRenderer {
    pub fn load() -> Self {
        let font_data = include_bytes!("../res/GillSans.ttc");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

        Self { font }
    }

    pub fn render_material(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        font_size: f32,
        padding: (f32, f32),
        color: (u8, u8, u8),
    ) -> Material {
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);

        // Layout in a line with 20 pixels padding
        let glyphs = self
            .font
            .layout(text, scale, point(padding.0, padding.1 + v_metrics.ascent))
            .collect::<Vec<_>>();

        // Layout size
        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let glyphs_width = {
            let min_x = glyphs
                .first()
                .map(|g| g.pixel_bounding_box().unwrap().min.x)
                .unwrap();

            let max_x = glyphs
                .last()
                .map(|g| g.pixel_bounding_box().unwrap().max.x)
                .unwrap();

            (max_x - min_x) as u32
        };

        let mut image = DynamicImage::new_rgba8(
            glyphs_width + (padding.0 * 2.) as u32,
            glyphs_height + (padding.1 * 2.) as u32,
        )
        .to_rgba8();

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    image.put_pixel(
                        x + bounding_box.min.x as u32,
                        y + bounding_box.min.y as u32,
                        Rgba([color.0, color.1, color.2, (v * 255.0) as u8]),
                    )
                });
            }
        }

        let diffuse_texture = Texture::from_image(
            device,
            queue,
            &DynamicImage::ImageRgba8(image),
            Some("Font texture"),
            false,
        )
        .unwrap();
        let text_material = Material::from_texture(device, queue, "", diffuse_texture).unwrap();

        text_material
    }
}
