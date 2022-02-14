use image::{DynamicImage, Rgba};
use rusttype::{point, Font, Scale};

pub struct FontRenderer {
    font: Font<'static>,
}

impl FontRenderer {
    pub fn load() -> Self {
        let font_data = include_bytes!("../res/GillSans.ttc");
        let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

        Self { font }
    }

    pub fn render(&self, text: &str, font_size: f32, color: (u8, u8, u8)) -> DynamicImage {
        let scale = Scale::uniform(font_size);
        let v_metrics = self.font.v_metrics(scale);

        // Layout in a line with 20 pixels padding
        let glyphs = self
            .font
            .layout(text, scale, point(20.0, 20.0 + v_metrics.ascent))
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

        let mut image = DynamicImage::new_rgba8(glyphs_width + 40, glyphs_height + 40).to_rgba8();

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

        DynamicImage::ImageRgba8(image)
    }
}
