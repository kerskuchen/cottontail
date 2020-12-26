use super::image::bitmap::*;
use super::image::font::*;
use super::*;

use super::core::serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sprites and SpriteAtlas

// NOTE: We used u32 here instead of usize for safer serialization / deserialization between
//       32Bit and 64Bit platforms
pub type TextureIndex = u32;
pub type FramebufferIndex = u32;

pub const SPRITE_ATTACHMENT_POINTS_MAX_COUNT: usize = 4;

/// This is similar to a Rect but allows mirroring horizontally/vertically
#[derive(Debug, Default, Copy, Clone, Deserialize, Serialize)]
pub struct AAQuad {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl AAQuad {
    pub fn to_rect(self) -> Rect {
        Rect::from_bounds_left_top_right_bottom(self.left, self.top, self.right, self.bottom)
    }
    pub fn from_rect(rect: Rect) -> Self {
        AAQuad {
            left: rect.left(),
            top: rect.top(),
            right: rect.right(),
            bottom: rect.bottom(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Sprite {
    pub name: String,
    pub atlas_texture_index: TextureIndex,

    /// Determines if the sprite contains pixels that have alpha that is not 0 and not 1.
    /// This is important for the sorting of sprites before drawing.
    pub has_translucency: bool,

    /// The amount by which the sprite is offsetted when drawn (must be marked in the image
    /// file in special `pivot` layer). This is useful to i.e. have a sprite always drawn centered.
    pub pivot_offset: Vec2,

    /// Optional special points useful for attaching other game objects to a sprite
    /// (must be marked in the image file in special `attachment_0`, `attachment_1` .. layers)
    pub attachment_points: [Vec2; SPRITE_ATTACHMENT_POINTS_MAX_COUNT],

    /// Contains the width and height of the original untrimmed sprite image. Usually only used for
    /// querying the size of the sprite
    pub untrimmed_dimensions: Vec2,

    /// Contains the trimmed dimensions of the sprite as it is stored in the atlas. This thightly
    /// surrounds every non-transparent pixel of the sprite. It also implicitly encodes the draw
    /// offset of the sprite by `trimmed_rect.pos` (not to be confused with `pivot_offset`)
    pub trimmed_rect: Rect,

    /// Texture coordinates of the trimmed sprite
    /// NOTE: We use an AAQuad instead of a Rect to allow us to mirror the texture horizontally
    ///       or vertically
    pub trimmed_uvs: AAQuad,
}

impl Sprite {
    #[inline]
    pub fn get_attachment_point_transformed(
        &self,
        attachment_index: usize,
        xform: Transform,
    ) -> Vec2 {
        // NOTE: The `sprite.pivot_offset` is relative to the left top corner of the untrimmed sprite.
        //       But we need the offset relative to the trimmed sprite which may have its own offset.
        let sprite_pivot = self.pivot_offset - self.trimmed_rect.pos;
        let attachment_point = self.attachment_points[attachment_index] - self.trimmed_rect.pos;
        attachment_point.transformed(sprite_pivot, xform.pixel_snapped())
    }

    #[inline]
    pub fn get_quad_transformed(&self, xform: Transform) -> Quad {
        let sprite_dim = self.trimmed_rect.dim;
        // NOTE: The `sprite.pivot_offset` is relative to the left top corner of the untrimmed sprite.
        //       But we need the offset relative to the trimmed sprite which may have its own offset.
        let sprite_pivot = self.pivot_offset - self.trimmed_rect.pos;
        Quad::from_rect_transformed(sprite_dim, sprite_pivot, xform.pixel_snapped())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sprite3D {
    pub name: String,
    pub layers: Vec<Sprite>,
}
impl Sprite3D {
    pub fn draw_height(&self) -> f32 {
        (self.layers.len() - 1) as f32
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// SpriteAtlas

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpriteAtlas {
    pub textures: Vec<Bitmap>,
    pub textures_size: u32,
    pub sprites: HashMap<String, Sprite>,
}

impl SpriteAtlas {
    /// NOTE: This expects all bitmaps to be powers-of-two sized rectangles with the same size
    pub fn new(textures: Vec<Bitmap>, sprites: HashMap<String, Sprite>) -> SpriteAtlas {
        // Double check bitmap dimensions
        let textures_size = {
            assert!(textures.len() > 0);
            let textures_size = textures[0].width as u32;
            assert!(textures_size > 0);
            assert!(textures_size.is_power_of_two());
            for texture in &textures {
                assert!(texture.width == textures_size as i32);
                assert!(texture.height == textures_size as i32);
            }
            textures_size
        };

        SpriteAtlas {
            textures_size,
            textures,
            sprites,
        }
    }

    /// This does not change the atlas bitmap
    pub fn add_sprite_for_region(
        &mut self,
        sprite_name: String,
        atlas_texture_index: TextureIndex,
        sprite_rect: Recti,
        draw_offset: Vec2i,
        has_translucency: bool,
    ) -> Sprite {
        debug_assert!(!self.sprites.contains_key(&sprite_name));

        let sprite_rect = Rect::from(sprite_rect);
        let draw_offset = Vec2::from(draw_offset);
        let uv_scale = 1.0 / self.textures_size as f32;
        let sprite = Sprite {
            name: sprite_name.clone(),
            atlas_texture_index: atlas_texture_index,
            has_translucency,
            pivot_offset: Vec2::zero(),
            attachment_points: [Vec2::zero(); SPRITE_ATTACHMENT_POINTS_MAX_COUNT],
            untrimmed_dimensions: sprite_rect.dim,
            trimmed_rect: sprite_rect.translated_by(draw_offset),
            trimmed_uvs: AAQuad::from_rect(sprite_rect.scaled_from_origin(Vec2::filled(uv_scale))),
        };

        self.sprites.insert(sprite_name.clone(), sprite.clone());
        sprite
    }

    pub fn debug_get_bitmap_for_sprite(&self, sprite_name: &str) -> Bitmap {
        let sprite = &self
            .sprites
            .get(sprite_name)
            .expect(&format!("Atlas does not contain sprite '{}'", &sprite_name));
        let dim = Vec2i::from_vec2_rounded(sprite.trimmed_rect.dim);
        let texture_coordinates = AAQuad::from_rect(
            sprite
                .trimmed_uvs
                .to_rect()
                .scaled_from_origin(Vec2::filled(self.textures_size as f32)),
        );

        let source_rect = Recti::from_rect_rounded(texture_coordinates.to_rect());
        let source_bitmap = &self.textures[sprite.atlas_texture_index as usize];

        let mut result_bitmap = Bitmap::new(dim.x as u32, dim.y as u32);
        let result_rect = result_bitmap.rect();

        Bitmap::copy_region(
            source_bitmap,
            source_rect,
            &mut result_bitmap,
            result_rect,
            None,
        );

        result_bitmap
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SpriteFont

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteGlyph {
    pub horizontal_advance: i32,
    pub sprite: Sprite,

    /// This is mainly used for text dimension calculations
    pub sprite_dimensions: Vec2i,
    /// This is mainly used for text dimension calculations
    pub sprite_draw_offset: Vec2i,
}

impl Glyph for SpriteGlyph {
    fn get_bitmap_rect(&self) -> Recti {
        Recti::from_pos_dim(self.sprite_draw_offset, self.sprite_dimensions)
    }

    fn horizontal_advance(&self) -> i32 {
        self.horizontal_advance
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteFont {
    pub name: String,

    pub baseline: i32,
    pub vertical_advance: i32,
    pub horizontal_advance_max: i32,
    pub font_height_in_pixels: i32,

    /// Fastpath glyphs for quick access (mainly latin glyphs)
    pub ascii_glyphs: Vec<SpriteGlyph>,
    /// Non-fastpath unicode glyphs for codepoints > FONT_MAX_NUM_FASTPATH_CODEPOINTS
    pub unicode_glyphs: HashMap<Codepoint, SpriteGlyph>,
}

impl Font<SpriteGlyph> for SpriteFont {
    fn baseline(&self) -> i32 {
        self.baseline
    }
    fn vertical_advance(&self) -> i32 {
        self.vertical_advance
    }
    fn horizontal_advance_max(&self) -> i32 {
        self.horizontal_advance_max
    }
    fn font_height_in_pixels(&self) -> i32 {
        self.font_height_in_pixels
    }
    fn get_glyph_for_codepoint_copy(&self, codepoint: Codepoint) -> SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            self.ascii_glyphs[codepoint as usize].clone()
        } else {
            let result = self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs[0usize]);
            if result.sprite.name != "" {
                result.clone()
            } else {
                self.ascii_glyphs['?' as usize].clone()
            }
        }
    }
    fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> &SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            &self.ascii_glyphs[codepoint as usize]
        } else {
            let result = self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs[0usize]);
            if result.sprite.name != "" {
                result
            } else {
                &self.ascii_glyphs['?' as usize]
            }
        }
    }
}