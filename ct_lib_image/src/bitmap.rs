pub use super::color::{Color, PixelRGBA};
pub use super::font::{BitmapFont, Font, TextAlignment};
pub use super::grid::GluePosition;
pub use super::math::{AlignmentHorizontal, AlignmentVertical, Vec2i};

use super::core::indexmap::IndexMap;
use super::core::serde_derive::Serialize;
use super::core::*;
use super::math;

use rect_packer;

pub type Bitmap = super::grid::Grid<PixelRGBA>;

impl Bitmap {
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { transmute_slice_to_byte_slice(&self.data) }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    pub fn premultiply_alpha(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = self.get(x, y);
                let alpha = color.a as f32 / 255.0;
                color.r = math::roundi(color.r as f32 * alpha) as u8;
                color.g = math::roundi(color.g as f32 * alpha) as u8;
                color.b = math::roundi(color.b as f32 * alpha) as u8;
                self.set(x, y, color);
            }
        }
    }

    #[must_use]
    pub fn to_premultiplied_alpha(&self) -> Bitmap {
        let mut result = self.clone();
        result.premultiply_alpha();
        result
    }

    pub fn unpremultiply_alpha(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = self.get(x, y);
                if color.a > 0 {
                    let alpha = color.a as f32 / 255.0;
                    color.r = i32::min(math::roundi(color.r as f32 / alpha), 255) as u8;
                    color.g = i32::min(math::roundi(color.g as f32 / alpha), 255) as u8;
                    color.b = i32::min(math::roundi(color.b as f32 / alpha), 255) as u8;
                }
                self.set(x, y, color);
            }
        }
    }

    #[must_use]
    pub fn to_unpremultiplied_alpha(&self) -> Bitmap {
        let mut result = self.clone();
        result.unpremultiply_alpha();
        result
    }

    pub fn scale(&mut self, new_width: u32, new_height: u32) {
        *self = self.scaled_to_sample_nearest_neighbor(new_width, new_height);
    }

    #[must_use]
    pub fn scaled_to_sample_nearest_neighbor(&self, new_width: u32, new_height: u32) -> Bitmap {
        assert!(new_width > 0);
        assert!(new_height > 0);

        let mut result = Bitmap::new(new_width, new_height);
        let result_rect = result.rect();
        Bitmap::copy_region_sample_nearest_neighbor(self, self.rect(), &mut result, result_rect);

        result
    }

    pub fn from_png_data(png_data: &[u8]) -> Result<Bitmap, String> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png_data));
        decoder.set_transformations(png::Transformations::EXPAND);
        let (png_info, mut png_reader) = decoder
            .read_info()
            .map_err(|error| format!("Could not read png data info: {}", error))?;

        let size_bytes = png_info.buffer_size();
        let mut buffer =
            vec![PixelRGBA::transparent(); size_bytes / std::mem::size_of::<PixelRGBA>()];
        {
            let buffer_raw = unsafe { super::core::transmute_slice_to_byte_slice_mut(&mut buffer) };
            png_reader
                .next_frame(buffer_raw)
                .map_err(|error| format!("Could not decode png data: {}", error))?;
        }

        Ok(Bitmap::new_from_buffer(
            png_info.width,
            png_info.height,
            buffer,
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_png_file(png_filepath: &str) -> Result<Bitmap, String> {
        let file_content = read_file_whole(png_filepath)
            .map_err(|error| format!("Could not open png file '{}': {}", png_filepath, error))?;
        Bitmap::from_png_data(&file_content)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_png_file_or_panic(png_filepath: &str) -> Bitmap {
        Bitmap::from_png_file(png_filepath).unwrap()
    }

    pub fn create_from_text(
        font: &BitmapFont,
        text: &str,
        font_scale: i32,
        background_color: PixelRGBA,
    ) -> Bitmap {
        let rect = font.get_text_bounding_rect(text, font_scale, false);
        let mut result = Bitmap::new_filled(rect.dim.x as u32, rect.dim.y as u32, background_color);

        // NOTE: As it can happen that glyphs have negative vertical offset (i.e. due to being
        //       big/bordered) we must not start drawing at (0,0) in those cases.
        let pos = Vec2i::new(i32::min(0, rect.pos.x).abs(), i32::min(0, rect.pos.y).abs());

        result.draw_text(font, text, font_scale, pos, Vec2i::zero(), false);
        result
    }

    pub fn encoded_as_png(&self) -> Vec<u8> {
        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(
                std::io::Cursor::new(&mut png_data),
                self.width as u32,
                self.height as u32,
            );
            encoder.set_color(png::ColorType::RGBA);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();

            writer
                .write_image_data(self.as_bytes())
                .expect(&format!("Could not encode png data to"));
        }
        png_data
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_to_png_file(&self, png_filepath: &str) {
        std::fs::create_dir_all(path_without_filename(png_filepath)).expect(&format!(
            "Could not create necessary directories to write to '{}'",
            png_filepath
        ));

        let file = std::fs::File::create(png_filepath)
            .expect(&format!("Could not open png file '{}'", png_filepath));

        let ref mut file_writer = std::io::BufWriter::new(file);
        let mut encoder = png::Encoder::new(file_writer, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        writer
            .write_image_data(self.as_bytes())
            .expect(&format!("Could not write png file to '{}'", png_filepath));
    }

    /// Draws a given utf8 text to a given bitmap
    /// Returns the starting_offset for the next `draw_text` call
    pub fn draw_text(
        &mut self,
        font: &BitmapFont,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        origin_is_baseline: bool,
    ) -> Vec2i {
        font.iter_text_glyphs(
            text,
            font_scale,
            origin,
            starting_offset,
            origin_is_baseline,
            &mut |glyph, draw_pos, _codepoint| {
                if let Some(glyph_bitmap) = &glyph.bitmap {
                    glyph_bitmap.blit_to(
                        self,
                        draw_pos + glyph.offset,
                        Some(PixelRGBA::transparent()),
                    );
                }
            },
        )
    }

    pub fn draw_text_aligned_in_point(
        &mut self,
        font: &BitmapFont,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        alignment: Option<TextAlignment>,
    ) -> Vec2i {
        font.iter_text_glyphs_aligned_in_point(
            text,
            font_scale,
            origin,
            starting_offset,
            alignment,
            &mut |glyph, draw_pos, _codepoint| {
                if let Some(glyph_bitmap) = &glyph.bitmap {
                    glyph_bitmap.blit_to(
                        self,
                        draw_pos + glyph.offset,
                        Some(PixelRGBA::transparent()),
                    );
                }
            },
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// BitmapAtlas

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BitmapAtlasPosition {
    pub atlas_texture_index: u32,
    pub atlas_texture_pixel_offset: Vec2i,
}

/// An atlaspacker that can grow in size
pub struct BitmapAtlas {
    pub atlas_texture: Bitmap,
    pub atlas_texture_size_max: Option<u32>,
    pub rect_packer: rect_packer::DensePacker,
    pub sprite_positions: IndexMap<String, Vec2i>,
}

impl BitmapAtlas {
    pub fn new(
        atlas_texture_size_initial: u32,
        atlas_texture_size_max: Option<u32>,
    ) -> BitmapAtlas {
        BitmapAtlas {
            atlas_texture: Bitmap::new(atlas_texture_size_initial, atlas_texture_size_initial),
            rect_packer: rect_packer::DensePacker::new(
                atlas_texture_size_initial as i32,
                atlas_texture_size_initial as i32,
            ),
            sprite_positions: IndexMap::new(),
            atlas_texture_size_max,
        }
    }

    pub fn finish(self) -> (Bitmap, IndexMap<String, Vec2i>) {
        (self.atlas_texture, self.sprite_positions)
    }

    pub fn pack_bitmap(&mut self, name: &str, image: &Bitmap) -> Option<Vec2i> {
        if let Some(rect) = self.rect_packer.pack(image.width, image.height, false) {
            let position = Vec2i::new(rect.x, rect.y);
            image.blit_to(&mut self.atlas_texture, position, None);
            self.sprite_positions.insert(name.to_owned(), position);
            Some(position)
        } else {
            None
        }
    }

    /// NOTE: Resizing is done by doubling current texture size
    pub fn pack_bitmap_with_resize(&mut self, name: &str, image: &Bitmap) -> Option<Vec2i> {
        if let Some(pos) = self.pack_bitmap(name, image) {
            return Some(pos);
        }

        // NOTE: At this point our image did not fit in our atlas textures, so we resize it
        let texture_size_max = self.atlas_texture_size_max.unwrap_or(std::u32::MAX);
        loop {
            let texture_size = self.atlas_texture.width;
            if texture_size as u32 >= texture_size_max {
                return None;
            }

            self.atlas_texture
                .extend(0, 0, texture_size, texture_size, PixelRGBA::transparent());
            self.rect_packer.resize(2 * texture_size, 2 * texture_size);

            if let Some(pos) = self.pack_bitmap(name, image) {
                return Some(pos);
            }
        }
    }
}

/// An atlaspacker that can have multiple atlas textures
pub struct BitmapMultiAtlas {
    pub atlas_texture_size_initial: u32,
    pub atlas_texture_size_max: Option<u32>,
    pub atlas_packers: Vec<BitmapAtlas>,
    pub sprite_positions: IndexMap<String, BitmapAtlasPosition>,
}

impl BitmapMultiAtlas {
    pub fn new(
        atlas_texture_size_initial: u32,
        atlas_texture_size_max: Option<u32>,
    ) -> BitmapMultiAtlas {
        BitmapMultiAtlas {
            atlas_texture_size_initial,
            atlas_texture_size_max,
            atlas_packers: vec![BitmapAtlas::new(
                atlas_texture_size_initial,
                atlas_texture_size_max,
            )],
            sprite_positions: IndexMap::new(),
        }
    }

    pub fn pack_bitmap(&mut self, sprite_name: &str, image: &Bitmap) -> BitmapAtlasPosition {
        if let Some(atlas_position) = self.pack_bitmap_internal(sprite_name, image) {
            return atlas_position;
        }

        // NOTE: At this point our image did not fit in any of the existing atlas textures, so we
        //       create a new atlas texture and try again
        self.atlas_packers.push(BitmapAtlas::new(
            self.atlas_texture_size_initial,
            self.atlas_texture_size_max,
        ));
        if let Some(atlas_position) = self.pack_bitmap_internal(sprite_name, image) {
            atlas_position
        } else {
            let texture_size_max = self.atlas_texture_size_max.unwrap_or(std::u32::MAX);
            panic!(
                "Could not pack image with dimensions {}x{} into atlas with dimensions with maxium dimensions {}x{}",
                image.width, image.height, texture_size_max, texture_size_max
            )
        }
    }

    pub fn finish(self) -> (Vec<Bitmap>, IndexMap<String, BitmapAtlasPosition>) {
        let atlas_textures = self
            .atlas_packers
            .into_iter()
            .map(|packer| packer.atlas_texture)
            .collect();

        (atlas_textures, self.sprite_positions)
    }

    fn pack_bitmap_internal(&mut self, name: &str, image: &Bitmap) -> Option<BitmapAtlasPosition> {
        for (atlas_index, packer) in self.atlas_packers.iter_mut().enumerate() {
            if let Some(position) = packer.pack_bitmap_with_resize(name, image) {
                let atlas_position = BitmapAtlasPosition {
                    atlas_texture_index: atlas_index as u32,
                    atlas_texture_pixel_offset: position,
                };
                self.sprite_positions
                    .insert(name.to_owned(), atlas_position);
                return Some(atlas_position);
            }
        }
        None
    }
}
