pub use super::color::{Color, PixelRGBA};
pub use super::grid::GluePosition;
pub use super::math;
pub use super::system;

pub type Bitmap = super::grid::Grid<PixelRGBA>;

impl Bitmap {
    pub fn from_premultiplied(&self) -> Bitmap {
        let mut result = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = self.get(x, y);
                if color.a > 0 {
                    let alpha = color.a as f32 / 255.0;
                    color.r = i32::min(math::roundi(color.r as f32 / alpha), 255) as u8;
                    color.g = i32::min(math::roundi(color.g as f32 / alpha), 255) as u8;
                    color.b = i32::min(math::roundi(color.b as f32 / alpha), 255) as u8;
                }
                result.set(x, y, color);
            }
        }
        result
    }

    pub fn to_premultiplied(&self) -> Bitmap {
        let mut result = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = self.get(x, y);
                let alpha = color.a as f32 / 255.0;
                color.r = math::roundi(color.r as f32 * alpha) as u8;
                color.g = math::roundi(color.g as f32 * alpha) as u8;
                color.b = math::roundi(color.b as f32 * alpha) as u8;
                result.set(x, y, color);
            }
        }
        result
    }

    pub fn create_from_png_file(png_filepath: &str) -> Bitmap {
        let image = lodepng::decode32_file(png_filepath)
            .expect(&format!("Could not decode png file '{}'", png_filepath));

        let buffer: Vec<PixelRGBA> = image
            .buffer
            .into_iter()
            // NOTE: We use our own color type because rbg::RRBA8 does not properly compile with serde
            .map(|color| unsafe { std::mem::transmute::<lodepng::RGBA, PixelRGBA>(color) })
            .collect();

        Bitmap::new_from_buffer(image.width as u32, image.height as u32, buffer)
    }

    pub fn write_to_png_file(bitmap: &Bitmap, png_filepath: &str) {
        std::fs::create_dir_all(system::path_without_filename(png_filepath)).expect(&format!(
            "Could not create necessary directories to write to '{}'",
            png_filepath
        ));
        lodepng::encode32_file(
            png_filepath,
            &bitmap.data,
            bitmap.width as usize,
            bitmap.height as usize,
        )
        .expect(&format!("Could not write png file to '{}'", png_filepath));
    }
}
