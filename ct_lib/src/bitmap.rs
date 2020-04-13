pub use super::color::PixelRGBA;
pub use super::grid::GluePosition;

pub type Bitmap = super::grid::Grid<PixelRGBA>;

impl Bitmap {
    pub fn from_premultiplied(&self) -> Bitmap {
        todo!()
    }

    pub fn to_premultiplied(&self) -> Bitmap {
        todo!()
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
        lodepng::encode32_file(
            png_filepath,
            &bitmap.data,
            bitmap.width as usize,
            bitmap.height as usize,
        )
        .expect(&format!("Could not write png file to '{}'", png_filepath));
    }
}
