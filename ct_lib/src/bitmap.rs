pub use super::color::{Color, PixelRGBA};
pub use super::font::{BitmapFont, Font};
pub use super::grid::GluePosition;
pub use super::math;
pub use super::math::{AlignmentHorizontal, AlignmentVertical, Vec2i};
pub use super::system;

use serde_derive::Serialize;

use super::indexmap::IndexMap;

use rect_packer;

pub type Bitmap = super::grid::Grid<PixelRGBA>;

impl Bitmap {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for pixel in &self.data {
            result.push(pixel.r);
            result.push(pixel.g);
            result.push(pixel.b);
            result.push(pixel.a);
        }
        result
    }

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

    pub fn create_from_text(
        font: &BitmapFont,
        text: &str,
        font_scale: i32,
        background_color: PixelRGBA,
    ) -> Bitmap {
        let rect = font.get_text_bounding_rect(text, font_scale);
        let mut result = Bitmap::new_filled(rect.dim.x as u32, rect.dim.y as u32, background_color);

        // NOTE: As it can happen that glyphs have negative vertical offset (i.e. due to being
        //       big/bordered) we must not start drawing at (0,0) in those cases.
        let pos = Vec2i::new(i32::min(0, rect.pos.x).abs(), i32::min(0, rect.pos.y).abs());

        result.draw_text(font, text, font_scale, pos, Vec2i::zero(), false);
        result
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
        origin_is_baseline: bool,
        alignment_x: AlignmentHorizontal,
        alignment_y: AlignmentVertical,
    ) -> Vec2i {
        font.iter_text_glyphs_aligned_in_point(
            text,
            font_scale,
            origin,
            starting_offset,
            origin_is_baseline,
            alignment_x,
            alignment_y,
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

    /// Same as draw_text_aligned_in_point but ignoring whitespace and aligning glyphs as tight
    /// as possible
    pub fn draw_text_aligned_in_point_exact(
        &mut self,
        font: &BitmapFont,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        origin_is_baseline: bool,
        alignment_x: AlignmentHorizontal,
        alignment_y: AlignmentVertical,
    ) -> Vec2i {
        font.iter_text_glyphs_aligned_in_point_exact(
            text,
            font_scale,
            origin,
            starting_offset,
            origin_is_baseline,
            alignment_x,
            alignment_y,
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
    pub rect_packer: rect_packer::DensePacker,
    pub sprite_positions: IndexMap<String, Vec2i>,
}

impl BitmapAtlas {
    pub fn new(atlas_texture_size_initial: i32) -> BitmapAtlas {
        assert!(atlas_texture_size_initial > 0);

        BitmapAtlas {
            atlas_texture: Bitmap::new(
                atlas_texture_size_initial as u32,
                atlas_texture_size_initial as u32,
            ),
            rect_packer: rect_packer::DensePacker::new(
                atlas_texture_size_initial,
                atlas_texture_size_initial,
            ),
            sprite_positions: IndexMap::new(),
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

    /// NOTE: Resizes by squaring the current size
    pub fn pack_bitmap_with_resize(&mut self, name: &str, image: &Bitmap) -> Option<Vec2i> {
        if let Some(pos) = self.pack_bitmap(name, image) {
            return Some(pos);
        }

        // NOTE: At this point our image did not fit in our atlas textures, so we resize it
        let texture_size = self.atlas_texture.width;
        self.atlas_texture
            .extend(0, 0, texture_size, texture_size, PixelRGBA::transparent());
        self.rect_packer.resize(2 * texture_size, 2 * texture_size);

        self.pack_bitmap_with_resize(name, image)
    }
}

/// An atlaspacker that can have multiple fixed size atlas textures
pub struct BitmapMultiAtlas {
    pub atlas_texture_size: i32,
    pub atlas_packers: Vec<BitmapAtlas>,
    pub sprite_positions: IndexMap<String, BitmapAtlasPosition>,
}

impl BitmapMultiAtlas {
    pub fn new(atlas_texture_size: i32) -> BitmapMultiAtlas {
        assert!(atlas_texture_size > 0);

        BitmapMultiAtlas {
            atlas_texture_size,
            atlas_packers: vec![BitmapAtlas::new(atlas_texture_size)],
            sprite_positions: IndexMap::new(),
        }
    }

    pub fn pack_bitmap(&mut self, name: &str, image: &Bitmap) -> Option<BitmapAtlasPosition> {
        for (atlas_index, packer) in self.atlas_packers.iter_mut().enumerate() {
            if let Some(position) = packer.pack_bitmap(name, image) {
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

    pub fn pack_bitmap_allow_growing(
        &mut self,
        sprite_name: &str,
        image: &Bitmap,
    ) -> BitmapAtlasPosition {
        if let Some(atlas_position) = self.pack_bitmap(sprite_name, image) {
            return atlas_position;
        }

        // NOTE: At this point our image did not fit in any of the existing atlas textures, so we
        //       create a new atlas texture and try again
        self.atlas_packers
            .push(BitmapAtlas::new(self.atlas_texture_size));
        if let Some(atlas_position) = self.pack_bitmap(sprite_name, image) {
            atlas_position
        } else {
            panic!(
                "Could not pack image with dimensions {}x{} into atlas with dimensions {}x{}",
                image.width, image.height, self.atlas_texture_size, self.atlas_texture_size
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
}
