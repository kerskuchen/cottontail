pub use super::bitmap::*;
use super::bitmap_font;
pub use super::color::*;
use super::math::*;

pub use hsl;
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Coordinates

/// A point in world-coordinate-space. One 1x1 unit-square in world-space equals to a pixel on the
/// canvas on a default zoom level
pub type Worldpoint = Point;

/// Same as Worldpoint only as vector
pub type Worldvec = Vec2;

/// A point in canvas-coordinate-space. Given in the range
/// [0, CANVAS_WIDTH - 1]x[0, CANVAS_HEIGHT - 1]
/// where (0,0) is the top-left corner
pub type Canvaspoint = Point;

/// Same as Canvaspoint only as vector
pub type Canvasvec = Vec2;

/// For a given Worldpoint returns the nearest Worldpoint that is aligned to the
/// canvas's pixel grid when drawn.
///
/// For example pixel-snapping the cameras position before drawing prevents pixel-jittering
/// artifacts on visible objects if the camera is moving at sub-pixel distances.
pub fn worldpoint_pixel_snapped(point: Worldpoint) -> Worldpoint {
    Worldpoint {
        x: f32::floor(point.x),
        y: f32::floor(point.y),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sprites and SpriteAtlas

// NOTE: We used u32 here instead of usize for safer serialization / deserialization between
//       32Bit and 64Bit platforms
pub type SpriteIndex = u32;
pub type TextureIndex = u32;
pub type FramebufferIndex = u32;

pub const SPRITE_ATTACHMENT_POINTS_MAX_COUNT: usize = 4;

/// This is similar to a Rect but allows mirroring horizontally/vertically
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sprite {
    pub name: String,
    pub index: SpriteIndex,
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
        pos: Vec2,
        scale: Vec2,
        rotation_dir: Vec2,
    ) -> Vec2 {
        // NOTE: The `sprite.pivot_offset` is relative to the left top corner of the untrimmed sprite.
        //       But we need the offset relative to the trimmed sprite which may have its own offset.
        let sprite_pivot = self.pivot_offset - self.trimmed_rect.pos;
        let attachment_point = self.attachment_points[attachment_index] - self.trimmed_rect.pos;
        attachment_point.transformed(sprite_pivot, pos, scale, rotation_dir)
    }

    #[inline]
    pub fn get_quad_transformed(&self, pos: Vec2, scale: Vec2, rotation_dir: Vec2) -> Quad {
        let sprite_dim = self.trimmed_rect.dim;
        // NOTE: The `sprite.pivot_offset` is relative to the left top corner of the untrimmed sprite.
        //       But we need the offset relative to the trimmed sprite which may have its own offset.
        let sprite_pivot = self.pivot_offset - self.trimmed_rect.pos;
        Quad::from_rect_transformed(
            sprite_dim,
            sprite_pivot,
            worldpoint_pixel_snapped(pos),
            scale,
            rotation_dir,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteAtlas {
    pub textures: Vec<Bitmap>,
    pub textures_size: u32,

    pub sprites: Vec<Sprite>,
    pub sprites_by_name: HashMap<String, Sprite>,
    pub sprites_indices: HashMap<String, SpriteIndex>,

    pub fonts: HashMap<String, SpriteFont>,
}

impl SpriteAtlas {
    /// NOTE: This expects all bitmaps to be powers-of-two sized rectangles with the same size
    pub fn new(
        textures: Vec<Bitmap>,
        sprites: Vec<Sprite>,
        fonts: HashMap<String, SpriteFont>,
    ) -> SpriteAtlas {
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

        // Create indexing hashmaps
        let mut sprites_by_name = HashMap::<String, Sprite>::new();
        let mut sprites_indices = HashMap::<String, SpriteIndex>::new();
        for (index, sprite) in sprites.iter().enumerate() {
            sprites_by_name.insert(sprite.name.clone(), sprite.clone());
            sprites_indices.insert(sprite.name.clone(), index as SpriteIndex);
        }

        SpriteAtlas {
            textures_size,
            textures,
            sprites,
            sprites_by_name,
            sprites_indices,
            fonts,
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
    ) -> SpriteIndex {
        debug_assert!(!self.sprites_by_name.contains_key(&sprite_name));

        let sprite_rect = Rect::from(sprite_rect);
        let draw_offset = Vec2::from(draw_offset);
        let uv_scale = 1.0 / self.textures_size as f32;
        let index = self.sprites.len() as SpriteIndex;
        let sprite = Sprite {
            index: 0,
            name: sprite_name.clone(),

            atlas_texture_index: atlas_texture_index,
            has_translucency,
            pivot_offset: Vec2::zero(),
            attachment_points: [Vec2::zero(); SPRITE_ATTACHMENT_POINTS_MAX_COUNT],
            untrimmed_dimensions: sprite_rect.dim,
            trimmed_rect: sprite_rect.translated_by(draw_offset),
            trimmed_uvs: AAQuad::from_rect(sprite_rect.scaled_from_origin(Vec2::filled(uv_scale))),
        };

        self.sprites.push(sprite.clone());
        self.sprites_by_name.insert(sprite_name.clone(), sprite);
        self.sprites_indices.insert(sprite_name, index);

        index
    }

    pub fn debug_get_bitmap_for_sprite(&self, sprite_index: SpriteIndex) -> Bitmap {
        let sprite = &self.sprites[sprite_index as usize];
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

    pub fn textureinfo_for_page(&self, page_index: TextureIndex) -> TextureInfo {
        assert!((page_index as usize) < self.textures.len());
        TextureInfo {
            name: format!("atlas_page_{}", page_index),
            index: page_index,
            width: self.textures_size,
            height: self.textures_size,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Font

pub type Codepoint = i32;

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SpriteGlyph {
    pub horizontal_advance: f32,
    pub sprite_index: SpriteIndex,
}

pub const FONT_MAX_NUM_FASTPATH_CODEPOINTS: usize = 256;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteFont {
    pub name: String,

    pub baseline: f32,
    pub vertical_advance: f32,

    /// Fastpath glyphs for quick access (mainly latin glyphs)
    pub ascii_glyphs: Vec<SpriteGlyph>,
    /// Non-fastpath unicode glyphs for codepoints > FONT_MAX_NUM_FASTPATH_CODEPOINTS
    pub unicode_glyphs: HashMap<Codepoint, SpriteGlyph>,
}

impl SpriteFont {
    #[inline]
    pub fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            self.ascii_glyphs[codepoint as usize]
        } else {
            *self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs['?' as usize])
        }
    }

    /// Returns width and height of a given utf8 text for a given font and font scale.
    pub fn get_text_dimensions(&self, font_scale: f32, text: &str) -> Vec2 {
        if text.len() == 0 {
            return Vec2::zero();
        }

        let mut dimensions = Vec2::new(0.0, font_scale * self.vertical_advance);
        let mut pos = Vec2::new(0.0, font_scale * self.baseline);

        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = self.get_glyph_for_codepoint(codepoint as i32);
                pos.x += font_scale * glyph.horizontal_advance;
            } else {
                dimensions.x = f32::max(dimensions.x, pos.x);
                dimensions.y += font_scale * self.vertical_advance;

                pos.x = 0.0;
                pos.y += font_scale * self.vertical_advance;
            }
        }

        // In case we did not find a newline character
        dimensions.x = f32::max(dimensions.x, pos.x);

        dimensions
    }

    pub fn get_text_width(&self, font_scale: f32, text: &str) -> f32 {
        let mut text_width = 0.0;
        for codepoint in text.chars() {
            let glyph = self.get_glyph_for_codepoint(codepoint as i32);
            text_width += font_scale * glyph.horizontal_advance;
        }
        text_width
    }

    pub fn get_text_height(font: &SpriteFont, font_scale: f32, linecount: usize) -> f32 {
        assert!(linecount > 0);
        (font_scale * font.baseline) + (linecount - 1) as f32 * font_scale * font.vertical_advance
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Canvas and screen blitting and transformations

/// Returns the `blit_rectangle` of for given canvas and screen dimensions.
/// The `blit-rectange` is the area of the screen where the content of the canvas is drawn onto.
/// It is as big as the canvas proportionally stretched and centered to fill the whole
/// screen.
///
/// It may or may not be smaller than the full screen size depending on the aspect
/// ratio of both the screen and the canvas. The `blit_rectange` is guaranteed to either have
/// the same width a as the screen (with letterboxing if needed) or the same height as the
/// screen (with columnboxing if needed) or completely fill the screen.
///
/// # Examples
/// ```
/// // +------+  +--------------+  +---------------+
/// // |canvas|  |   screen     |  |               | <- screen
/// // | 8x4  |  |    16x12     |  +---------------+
/// // +------+  |              |  |   blit-rect   |
/// //           |              |  |     16x10     |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  +---------------+
/// //           |              |  |               |
/// //           +--------------+  +---------------+
/// //
/// // +------+  +----------------+  +-+-------------+-+
/// // |canvas|  |     screen     |  | |             | |
/// // | 8x4  |  |      18x8      |  | |             | |
/// // +------+  |                |  | |  blit-rect  | |
/// //           |                |  | |    16x8     | |
/// //           |                |  | |             | |
/// //           |                |  | |             | |
/// //           +----------------+  +-+-------------+-+
/// //                                                ^---- screen
/// //
/// // +------+  +----------------+  +-----------------+
/// // |canvas|  |     screen     |  |                 |
/// // | 8x4  |  |      16x8      |  |                 |
/// // +------+  |                |  |    blit-rect    |
/// //           |                |  |      16x8       |
/// //           |                |  |                 |
/// //           |                |  |                 |
/// //           +----------------+  +-----------------+
/// //                                                ^---- blit-rect == screen
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct BlitRect {
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: i32,
    pub height: i32,
}

impl BlitRect {
    #[inline]
    pub fn new_from_dimensions(width: u32, height: u32) -> BlitRect {
        BlitRect {
            offset_x: 0,
            offset_y: 0,
            width: width as i32,
            height: height as i32,
        }
    }

    /// Creates a canvas of fixed size that is stretched to the screen with aspect ratio correction
    #[inline]
    pub fn new_for_fixed_canvas_size(
        screen_width: u32,
        screen_height: u32,
        canvas_width: u32,
        canvas_height: u32,
    ) -> BlitRect {
        let aspect_ratio = canvas_height as f32 / canvas_width as f32;
        let mut blit_width = screen_width as f32;
        let mut blit_height = blit_width * aspect_ratio;

        if blit_height > screen_height as f32 {
            blit_height = screen_height as f32;
            blit_width = blit_height / aspect_ratio;
        }

        BlitRect {
            offset_x: f32::round((screen_width as f32 / 2.0) - (blit_width / 2.0)) as i32,
            offset_y: f32::round((screen_height as f32 / 2.0) - (blit_height / 2.0)) as i32,
            width: f32::round(blit_width) as i32,
            height: f32::round(blit_height) as i32,
        }
    }
}

/// Converts a screen point to coordinates respecting the canvas
/// dimensions and its offsets
///
/// screen_pos_x in [0, screen_width - 1] (left to right)
/// screen_pos_y in [0, screen_height - 1] (top to bottom)
/// result in [0, canvas_width - 1]x[0, canvas_height - 1] (relative to clamped canvas area,
///                                                         top-left to bottom-right)
///
/// WARNING: This does not work optimally if the pixel-scale-factor
/// (which is screen_width / canvas_width) is not an integer value
///
#[inline]
pub fn screen_point_to_canvas_point(
    screen_width: u32,
    screen_height: u32,
    canvas_width: u32,
    canvas_height: u32,
    screen_pos_x: i32,
    screen_pos_y: i32,
) -> Pointi {
    let blit_rect = BlitRect::new_for_fixed_canvas_size(
        screen_width,
        screen_height,
        canvas_width,
        canvas_height,
    );

    let pos_blitrect_x = clampi(screen_pos_x - blit_rect.offset_x, 0, blit_rect.width - 1);
    let pos_blitrect_y = clampi(screen_pos_y - blit_rect.offset_y, 0, blit_rect.height - 1);

    let pos_canvas_x = canvas_width as f32 * (pos_blitrect_x as f32 / blit_rect.width as f32);
    let pos_canvas_y = canvas_height as f32 * (pos_blitrect_y as f32 / blit_rect.height as f32);

    Pointi::new(floori(pos_canvas_x), floori(pos_canvas_y))
}

pub fn letterbox_rects_create(
    center_width: i32,
    center_height: i32,
    canvas_width: i32,
    canvas_height: i32,
) -> (Recti, [Recti; 4]) {
    let pos_x = floori(block_centered_in_point(
        center_width as f32,
        canvas_width as f32 / 2.0,
    ));
    let pos_y = floori(block_centered_in_point(
        center_height as f32,
        canvas_height as f32 / 2.0,
    ));
    let center_rect = Recti::from_xy_width_height(pos_x, pos_y, center_width, center_height);

    let letterbox_rects = [
        // Top
        Recti::from_bounds_left_top_right_bottom(0, 0, canvas_width, center_rect.top()),
        // Left
        Recti::from_bounds_left_top_right_bottom(
            0,
            center_rect.top(),
            center_rect.left(),
            center_rect.bottom(),
        ),
        // Right
        Recti::from_bounds_left_top_right_bottom(
            center_rect.right(),
            center_rect.top(),
            canvas_width,
            center_rect.bottom(),
        ),
        // Bottom
        Recti::from_bounds_left_top_right_bottom(
            0,
            center_rect.bottom(),
            canvas_width,
            canvas_height,
        ),
    ];
    (center_rect, letterbox_rects)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertex format

pub type VertexIndex = u32;
pub type Depth = f32;
pub type Additivity = f32;

const FRAMEBUFFER_INDEX_CANVAS: u32 = 0;
const FRAMEBUFFER_NAME_CANVAS: &str = "canvas";

pub const DEPTH_CLEAR: Depth = 0.0;
pub const DEPTH_MAX: Depth = 100.0;

// NOTE: This translates to the depth range [0, 100] from farthest to nearest (like a paperstack)
//       For more information see: https://stackoverflow.com/a/36046924
pub const DEFAULT_WORLD_ZNEAR: Depth = 0.0;
pub const DEFAULT_WORLD_ZFAR: Depth = -100.0;

pub const ADDITIVITY_NONE: Additivity = 0.0;
pub const ADDITIVITY_MAX: Additivity = 1.0;

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: Vec2,
    pub color: Color,
    pub additivity: Additivity,
}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBlit {
    pub pos: Vec2,
    pub uv: Vec2,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertexbuffers

pub type VertexbufferSimple = Vertexbuffer<Vertex>;
pub type VertexbufferBlit = Vertexbuffer<VertexBlit>;

#[derive(Debug, Default, Clone)]
pub struct Vertexbuffer<VertexType: Copy + Clone + Default> {
    pub texture_index: TextureIndex,
    pub vertices: Vec<VertexType>,
    pub indices: Vec<VertexIndex>,
}

impl<VertexType: Copy + Clone + Default> Vertexbuffer<VertexType> {
    pub fn new(texture_index: TextureIndex) -> Vertexbuffer<VertexType> {
        Vertexbuffer {
            texture_index,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl VertexbufferBlit {
    pub fn push_blit_quad(
        &mut self,
        rect_target: BlitRect,
        rect_source: BlitRect,
        framebuffer_source_width: u32,
        framebuffer_source_height: u32,
    ) {
        let start_index = self.vertices.len() as VertexIndex;

        // first triangle
        self.indices.push(start_index + 3); // left top
        self.indices.push(start_index + 0); // right top
        self.indices.push(start_index + 1); // right bottom

        // second triangle
        self.indices.push(start_index + 2); // left bottom
        self.indices.push(start_index + 1); // right bottom
        self.indices.push(start_index + 3); // left top

        let dim = Rect::from_xy_width_height(
            rect_target.offset_x as f32,
            rect_target.offset_y as f32,
            rect_target.width as f32,
            rect_target.height as f32,
        );

        let uvs = Rect::from_xy_width_height(
            rect_source.offset_x as f32,
            rect_source.offset_y as f32,
            rect_source.width as f32,
            rect_source.height as f32,
        )
        .scaled_from_origin(Vec2::new(
            1.0 / framebuffer_source_width as f32,
            1.0 / framebuffer_source_height as f32,
        ));

        // right top
        self.vertices.push(VertexBlit {
            pos: Vec2::new(dim.right(), dim.top()),
            uv: Vec2::new(uvs.right(), uvs.top()),
        });
        // right bottom
        self.vertices.push(VertexBlit {
            pos: Vec2::new(dim.right(), dim.bottom()),
            uv: Vec2::new(uvs.right(), uvs.bottom()),
        });
        // left bottom
        self.vertices.push(VertexBlit {
            pos: Vec2::new(dim.left(), dim.bottom()),
            uv: Vec2::new(uvs.left(), uvs.bottom()),
        });
        // left top
        self.vertices.push(VertexBlit {
            pos: Vec2::new(dim.left(), dim.top()),
            uv: Vec2::new(uvs.left(), uvs.top()),
        });
    }
}

impl VertexbufferSimple {
    pub fn push_drawable(&mut self, drawable: Drawable) {
        let depth = drawable.depth;
        let color = drawable.color_modulate;
        let additivity = drawable.additivity;

        match drawable.geometry {
            Geometry::QuadMesh { uvs, quad } => {
                let start_index = self.vertices.len() as VertexIndex;

                // first triangle
                self.indices.push(start_index + 3); // left top
                self.indices.push(start_index + 0); // right top
                self.indices.push(start_index + 1); // right bottom

                // second triangle
                self.indices.push(start_index + 2); // left bottom
                self.indices.push(start_index + 1); // right bottom
                self.indices.push(start_index + 3); // left top

                // right top
                self.vertices.push(Vertex {
                    pos: Vec3::from_vec2(quad.vert_right_top, depth),
                    uv: Vec2::new(uvs.right, uvs.top),
                    color,
                    additivity,
                });
                // right bottom
                self.vertices.push(Vertex {
                    pos: Vec3::from_vec2(quad.vert_right_bottom, depth),
                    uv: Vec2::new(uvs.right, uvs.bottom),
                    color,
                    additivity,
                });
                // left bottom
                self.vertices.push(Vertex {
                    pos: Vec3::from_vec2(quad.vert_left_bottom, depth),
                    uv: Vec2::new(uvs.left, uvs.bottom),
                    color,
                    additivity,
                });
                // left top
                self.vertices.push(Vertex {
                    pos: Vec3::from_vec2(quad.vert_left_top, depth),
                    uv: Vec2::new(uvs.left, uvs.top),
                    color,
                    additivity,
                });
            }
            Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            } => {
                let start_index = self.vertices.len() as VertexIndex;
                for index in indices {
                    self.indices.push(start_index + index);
                }
                for (vertex, uv) in vertices.iter().zip(uvs.iter()) {
                    self.vertices.push(Vertex {
                        pos: Vec3::from_vec2(*vertex, depth),
                        uv: *uv,
                        color,
                        additivity,
                    });
                }
            }
            Geometry::LineMesh { vertices, indices } => {
                let start_index = self.vertices.len() as VertexIndex;
                for index in indices {
                    self.indices.push(start_index + index);
                }
                self.vertices.extend(vertices.iter());
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawawbles

#[derive(Debug, Clone)]
pub enum Geometry {
    QuadMesh {
        uvs: AAQuad,
        quad: Quad,
    },
    PolygonMesh {
        vertices: Vec<Vec2>,
        uvs: Vec<Vec2>,
        indices: Vec<VertexIndex>,
    },
    LineMesh {
        vertices: Vec<Vertex>,
        indices: Vec<VertexIndex>,
    },
}

#[derive(Debug, Clone)]
pub struct Drawable {
    pub texture_index: TextureIndex,
    pub uv_region_contains_translucency: bool,
    pub depth: Depth,
    pub color_modulate: Color,
    pub additivity: Additivity,

    pub geometry: Geometry,
}

use std::cmp::Ordering;

impl Drawable {
    #[inline]
    pub fn compare(a: &Drawable, b: &Drawable) -> Ordering {
        let a_has_translucency = a.uv_region_contains_translucency
            || (a.color_modulate.a < 1.0)
            || (a.additivity != ADDITIVITY_NONE);
        let b_has_translucency = b.uv_region_contains_translucency
            || (b.color_modulate.a < 1.0)
            || (b.additivity != ADDITIVITY_NONE);

        // NOTE: We want all translucent objectes to be rendered last
        if a_has_translucency != b_has_translucency {
            if b_has_translucency {
                return Ordering::Less;
            } else {
                return Ordering::Greater;
            }
        }

        if a.texture_index != b.texture_index {
            if a.texture_index < b.texture_index {
                return Ordering::Less;
            } else {
                return Ordering::Greater;
            }
        }

        // NOTE: We want to draw the items with smaller z-level first
        //       so a.depth < b.depth => a is first
        if a.depth < b.depth {
            return Ordering::Less;
        } else if a.depth > b.depth {
            return Ordering::Greater;
        }

        Ordering::Equal
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawcommands

#[derive(Debug, Default, Clone, Copy)]
pub struct ShaderParamsSimple {
    pub transform: Mat4,
    pub texture_color_modulate: Color,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ShaderParamsBlit {
    pub transform: Mat4,
}

#[derive(Debug, Clone, Copy)]
pub enum ShaderParams {
    Simple(ShaderParamsSimple),
    Blit(ShaderParamsBlit),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FramebufferInfo {
    pub index: FramebufferIndex,
    pub width: u32,
    pub height: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FramebufferTarget {
    Screen,
    Offscreen(FramebufferInfo),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextureInfo {
    pub name: String,
    pub index: TextureIndex,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub enum Drawcommand {
    Draw {
        framebuffer_target: FramebufferTarget,
        texture_info: TextureInfo,
        shader_params: ShaderParams,
        vertexbuffer: VertexbufferSimple,
    },
    TextureCreate(TextureInfo),
    TextureUpdate {
        texture_info: TextureInfo,
        offset_x: u32,
        offset_y: u32,
        bitmap: Bitmap,
    },
    TextureFree(TextureInfo),
    FramebufferCreate(FramebufferInfo),
    FramebufferFree(FramebufferInfo),
    FramebufferClear {
        framebuffer_target: FramebufferTarget,
        new_color: Option<Color>,
        new_depth: Option<Depth>,
    },
    FramebufferBlit {
        source_framebuffer_info: FramebufferInfo,
        source_rect: BlitRect,
        dest_framebuffer_target: FramebufferTarget,
        dest_rect: BlitRect,
    },
}

impl std::fmt::Debug for Drawcommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Drawcommand::Draw {
                framebuffer_target,
                texture_info,
                shader_params,
                vertexbuffer,
            } => write!(
                f,
                "Draw: {:?} {:?} {:?} vertexcount: {}",
                framebuffer_target,
                texture_info,
                shader_params,
                vertexbuffer.vertices.len()
            ),
            Drawcommand::TextureCreate(texture_info) => {
                write!(f, "TextureCreate: {:?} ", texture_info)
            }
            Drawcommand::TextureUpdate {
                texture_info,
                offset_x,
                offset_y,
                bitmap,
            } => write!(
                f,
                "TextureUpdate: {:?} rect: x:{},y:{},w:{},h:{}",
                texture_info, offset_x, offset_y, bitmap.width, bitmap.height
            ),
            Drawcommand::TextureFree(texture_info) => write!(f, "TextureFree: {:?}", texture_info,),
            Drawcommand::FramebufferCreate(framebuffer_info) => {
                write!(f, "FramebufferCreate: {:?}", framebuffer_info,)
            }
            Drawcommand::FramebufferFree(framebuffer_info) => {
                write!(f, "FramebufferFree: {:?}", framebuffer_info,)
            }
            Drawcommand::FramebufferClear {
                framebuffer_target,
                new_color,
                new_depth,
            } => write!(
                f,
                "FramebufferClear: {:?} {:?} {:?}",
                framebuffer_target, new_color, new_depth
            ),
            Drawcommand::FramebufferBlit {
                source_framebuffer_info,
                source_rect,
                dest_framebuffer_target,
                dest_rect,
            } => write!(
                f,
                "FramebufferBlit: source: {:?} source_rect: {:?}, source: {:?} source_rect: {:?}",
                source_framebuffer_info, source_rect, dest_framebuffer_target, dest_rect
            ),
        }
    }
}
////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawstate

#[derive(Clone)]
pub struct Drawstate {
    pub canvas_framebuffer_target: FramebufferTarget,

    is_first_run: bool,

    atlas: SpriteAtlas,
    textures_dirty: Vec<bool>,

    untextured_uv_center_coord: AAQuad,
    untextured_uv_center_atlas_page: TextureIndex,

    current_letterbox_color: Color,
    current_clear_color: Color,
    current_clear_depth: Depth,

    simple_shaderparams: ShaderParamsSimple,
    simple_drawables: Vec<Drawable>,
    simple_vertexbuffer: VertexbufferSimple,

    pub drawcommands: Vec<Drawcommand>,

    debug_use_flat_color_mode: bool,
    debug_log_font: SpriteFont,
    debug_log_font_scale: f32,
    debug_log_origin: Vec2,
    debug_log_offset: Vec2,
    debug_log_depth: Depth,
}

//--------------------------------------------------------------------------------------------------
// Creation and configuration

impl Drawstate {
    pub fn new(mut atlas: SpriteAtlas) -> Drawstate {
        // Make sprites out of the atlas pages themselves for debug purposes
        for page_index in 0..atlas.textures.len() {
            let sprite_name = format!("debug_sprite_whole_page_{}", page_index);
            atlas.add_sprite_for_region(
                sprite_name,
                page_index as TextureIndex,
                Recti::from_width_height(atlas.textures_size as i32, atlas.textures_size as i32),
                Vec2i::zero(),
                true,
            );
        }

        // Allocate textures for atlas pages
        let mut drawcommands = Vec::<Drawcommand>::new();
        for page_index in 0..atlas.textures.len() {
            let texture_info = atlas.textureinfo_for_page(page_index as TextureIndex);
            drawcommands.push(Drawcommand::TextureCreate(texture_info));
        }

        let textures_dirty = vec![true; atlas.textures.len()];

        // Reserves a white pixel for special usage on the first page
        let untextured_sprite = atlas
            .sprites_by_name
            .get("untextured")
            .expect("'untextured' sprite missing in atlas");
        let untextured_uv_center_coord = untextured_sprite.trimmed_uvs;
        let untextured_uv_center_atlas_page = untextured_sprite.atlas_texture_index;

        let debug_log_font_name = bitmap_font::FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
        let debug_log_font = atlas
            .fonts
            .get(&debug_log_font_name)
            .expect(&format!(
                "Cannot find default debug log font '{}'",
                &debug_log_font_name,
            ))
            .clone();

        Drawstate {
            canvas_framebuffer_target: FramebufferTarget::Screen,

            is_first_run: true,

            atlas,
            textures_dirty,

            untextured_uv_center_coord,
            untextured_uv_center_atlas_page,

            current_letterbox_color: Color::black(),
            current_clear_color: Color::black(),
            current_clear_depth: DEPTH_CLEAR,

            simple_shaderparams: ShaderParamsSimple::default(),
            simple_drawables: Vec::new(),
            simple_vertexbuffer: VertexbufferSimple::default(),

            drawcommands,

            debug_use_flat_color_mode: false,
            debug_log_font,
            debug_log_font_scale: 1.0,
            debug_log_origin: Vec2::new(5.0, 5.0),
            debug_log_offset: Vec2::zero(),
            debug_log_depth: DEPTH_MAX,
        }
    }

    pub fn get_font(&self, font_name: &str) -> SpriteFont {
        self.atlas
            .fonts
            .get(font_name)
            .expect(&format!("Could not find font '{}'", font_name))
            .clone()
    }

    pub fn set_shaderparams_simple(&mut self, color_modulate: Color, transform: Mat4) {
        self.simple_shaderparams.texture_color_modulate = color_modulate;
        self.simple_shaderparams.transform = transform;
    }

    pub fn set_letterbox_color(&mut self, color: Color) {
        self.current_letterbox_color = color;
    }

    pub fn set_clear_color_and_depth(&mut self, color: Color, depth: Depth) {
        self.current_clear_color = color;
        self.current_clear_depth = depth;
    }

    pub fn get_canvas_dimensions(&self) -> Option<(u32, u32)> {
        if let FramebufferTarget::Offscreen(canvas_frambuffer_info) =
            &self.canvas_framebuffer_target
        {
            Some((canvas_frambuffer_info.width, canvas_frambuffer_info.height))
        } else {
            None
        }
    }

    pub fn change_canvas_dimensions(&mut self, width: u32, height: u32) {
        assert!(width > 0);
        assert!(height > 0);

        if let FramebufferTarget::Offscreen(canvas_framebuffer_info) =
            &self.canvas_framebuffer_target
        {
            if canvas_framebuffer_info.width == width && canvas_framebuffer_info.height == height {
                // Nothing changed
                return;
            } else {
                // We already have a canvas set, so we delete it first
                self.drawcommands.push(Drawcommand::FramebufferFree(
                    canvas_framebuffer_info.clone(),
                ));
            }
        }

        let new_canvas_framebuffer_info = FramebufferInfo {
            index: FRAMEBUFFER_INDEX_CANVAS,
            width,
            height,
            name: FRAMEBUFFER_NAME_CANVAS.to_owned(),
        };

        self.drawcommands.push(Drawcommand::FramebufferCreate(
            new_canvas_framebuffer_info.clone(),
        ));
        self.canvas_framebuffer_target = FramebufferTarget::Offscreen(new_canvas_framebuffer_info);
    }

    pub fn debug_init_logging(&mut self, font: Option<SpriteFont>, origin: Vec2, depth: Depth) {
        if let Some(font) = font {
            self.debug_log_font = font;
        }
        self.debug_log_origin = origin;
        self.debug_log_depth = depth;
    }

    pub fn debug_enable_flat_color_mode(&mut self, enable: bool) {
        self.debug_use_flat_color_mode = enable;
    }

    pub fn debug_get_sprite_as_bitmap(&self, sprite: SpriteBy) -> Bitmap {
        let sprite_index = match sprite {
            SpriteBy::Ref(sprite_reference) => sprite_reference.index,
            SpriteBy::Index(sprite_index) => sprite_index,
            SpriteBy::Name(sprite_name) => self.get_sprite_by_name(sprite_name).index,
        };
        self.atlas.debug_get_bitmap_for_sprite(sprite_index)
    }
}

//--------------------------------------------------------------------------------------------------
// Beginning and ending frames

impl Drawstate {
    pub fn begin_frame(&mut self) {
        if !self.is_first_run {
            self.drawcommands.clear();
            self.simple_drawables.clear();
            self.simple_vertexbuffer.indices.clear();
            self.simple_vertexbuffer.vertices.clear();
            self.debug_log_offset = Vec2::zero();
        } else {
            self.is_first_run = false;
        }
    }

    pub fn finish_frame(&mut self, screen_framebuffer_width: u32, screen_framebuffer_height: u32) {
        // Re-upload modified atlas pages
        for atlas_page in 0..self.textures_dirty.len() {
            if self.textures_dirty[atlas_page] {
                // An atlas page was modified, re-upload the whole page
                self.textures_dirty[atlas_page] = false;

                let atlas_page_bitmap = self.atlas.textures[atlas_page].clone();
                self.drawcommands.push(Drawcommand::TextureUpdate {
                    texture_info: self.atlas.textureinfo_for_page(atlas_page as TextureIndex),
                    offset_x: 0,
                    offset_y: 0,
                    bitmap: atlas_page_bitmap,
                });
            }
        }

        // NOTE: If we have our own offscreen framebuffer that we want to draw to, we still need to
        //       clear the screen framebuffer
        if let FramebufferTarget::Offscreen(_) = &self.canvas_framebuffer_target {
            self.drawcommands.push(Drawcommand::FramebufferClear {
                framebuffer_target: FramebufferTarget::Screen,
                new_color: Some(self.current_letterbox_color),
                new_depth: Some(DEPTH_CLEAR),
            });
        }

        // Clear canvas
        self.drawcommands.push(Drawcommand::FramebufferClear {
            framebuffer_target: self.canvas_framebuffer_target.clone(),
            new_color: Some(self.current_clear_color),
            new_depth: Some(self.current_clear_depth),
        });

        // Draw quadbatches
        self.simple_drawables.sort_by(Drawable::compare);
        if self.simple_drawables.len() > 0 {
            let mut batches = Vec::new();
            let mut current_batch = VertexbufferSimple::new(self.simple_drawables[0].texture_index);

            for drawable in self.simple_drawables.drain(..) {
                if drawable.texture_index != current_batch.texture_index {
                    batches.push(current_batch);
                    current_batch = VertexbufferSimple::new(drawable.texture_index);
                }

                current_batch.push_drawable(drawable);
            }
            batches.push(current_batch);

            for batch in batches.into_iter() {
                self.drawcommands.push(Drawcommand::Draw {
                    framebuffer_target: self.canvas_framebuffer_target.clone(),
                    texture_info: self.atlas.textureinfo_for_page(batch.texture_index),
                    shader_params: ShaderParams::Simple(self.simple_shaderparams),
                    vertexbuffer: batch,
                });
            }
        }

        // If we drew to an offscreen-canvas we must blit it back to the screen
        if let FramebufferTarget::Offscreen(canvas_framebuffer_info) =
            &self.canvas_framebuffer_target
        {
            let rect_canvas = BlitRect::new_from_dimensions(
                canvas_framebuffer_info.width,
                canvas_framebuffer_info.height,
            );
            let rect_screen = BlitRect::new_for_fixed_canvas_size(
                screen_framebuffer_width,
                screen_framebuffer_height,
                canvas_framebuffer_info.width,
                canvas_framebuffer_info.height,
            );
            self.drawcommands.push(Drawcommand::FramebufferBlit {
                source_framebuffer_info: canvas_framebuffer_info.clone(),
                source_rect: rect_canvas,
                dest_framebuffer_target: FramebufferTarget::Screen,
                dest_rect: rect_screen,
            });
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Drawing

/// NOTE: SpriteBy::Ref should be preferred as it is the fastest to use. SpriteBy::Index /
///       SpriteBy::Name require array / hashmap lookup
pub enum SpriteBy<'a> {
    Ref(&'a Sprite),
    Index(SpriteIndex),
    Name(&'a str),
}

impl Drawstate {
    //----------------------------------------------------------------------------------------------
    // Quad drawing

    #[inline]
    pub fn draw_quad(
        &mut self,
        quad: &Quad,
        uvs: AAQuad,
        uv_region_contains_translucency: bool,
        texture_index: TextureIndex,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) {
        if !self.debug_use_flat_color_mode {
            self.simple_drawables.push(Drawable {
                texture_index,
                uv_region_contains_translucency,
                depth,
                color_modulate,
                additivity,
                geometry: Geometry::QuadMesh { uvs, quad: *quad },
            });
        } else {
            let coords_horizontal = (uvs.left + uvs.right) / 2.0;
            let coords_vertical = (uvs.top + uvs.bottom) / 2.0;
            let uvs = AAQuad {
                left: coords_horizontal,
                top: coords_vertical,
                right: coords_horizontal,
                bottom: coords_vertical,
            };

            self.simple_drawables.push(Drawable {
                texture_index,
                uv_region_contains_translucency,
                depth,
                color_modulate,
                additivity,
                geometry: Geometry::QuadMesh { uvs, quad: *quad },
            });
        };
    }

    //----------------------------------------------------------------------------------------------
    // Sprite drawing

    /// NOTE: Rotation is performed around the sprites pivot point
    pub fn draw_sprite_pixel_snapped(
        &mut self,
        sprite: SpriteBy,
        pos: Vec2,
        scale: Vec2,
        rotation_dir: Vec2,
        flip_horizontally: bool,
        flip_vertically: bool,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) {
        let (sprite_quad, sprite_uvs, texture_index, has_translucency) = {
            let sprite = match sprite {
                SpriteBy::Ref(sprite_reference) => sprite_reference,
                SpriteBy::Index(sprite_index) => self.get_sprite_by_index(sprite_index),
                SpriteBy::Name(sprite_name) => self.get_sprite_by_name(sprite_name),
            };

            let quad =
                sprite.get_quad_transformed(worldpoint_pixel_snapped(pos), scale, rotation_dir);

            let mut sprite_uvs = sprite.trimmed_uvs;
            if flip_horizontally {
                std::mem::swap(&mut sprite_uvs.left, &mut sprite_uvs.right);
            }
            if flip_vertically {
                std::mem::swap(&mut sprite_uvs.top, &mut sprite_uvs.bottom);
            }

            (
                quad,
                sprite_uvs,
                sprite.atlas_texture_index,
                sprite.has_translucency,
            )
        };

        self.draw_quad(
            &sprite_quad,
            sprite_uvs,
            has_translucency,
            texture_index,
            depth,
            color_modulate,
            additivity,
        );
    }

    pub fn draw_sprite_clipped(
        &mut self,
        sprite: SpriteBy,
        pos: Vec2,
        scale: Vec2,
        clipping_rect: Rect,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) {
        let sprite = match sprite {
            SpriteBy::Ref(sprite_reference) => sprite_reference,
            SpriteBy::Index(sprite_index) => self.get_sprite_by_index(sprite_index),
            SpriteBy::Name(sprite_name) => self.get_sprite_by_name(sprite_name),
        };

        let mut sprite_rect = sprite.trimmed_rect.scaled_from_origin(scale);

        sprite_rect = sprite_rect.translated_by(-sprite.pivot_offset);
        // NOTE: This scales the embedded offsets correctly as well
        sprite_rect = sprite_rect.scaled_from_origin(scale);
        sprite_rect = sprite_rect.translated_by(pos);

        let rect = sprite_rect;
        let uvs = sprite.trimmed_uvs;
        let atlas_page = sprite.atlas_texture_index;
        let has_translucency = sprite.has_translucency;

        match Rect::intersect(clipping_rect, rect) {
            RectIntersectionResult::None => {
                // Our sprite has no intersection with our clipping rect -> Nothing visible
                return;
            }
            RectIntersectionResult::AContainsB(_) => {
                // Our clipping rect contains our sprite fully - no clipping needed
                let quad = Quad::from_rect(rect);
                self.draw_quad(
                    &quad,
                    uvs,
                    has_translucency,
                    atlas_page,
                    depth,
                    color_modulate,
                    additivity,
                );
                return;
            }
            RectIntersectionResult::BContainsA(_) => {
                // Our Sprite rect contains the clipping rect - we don't support this case yet
                todo!(
                    "Cannot clip sprite because given clipping rect is contained in sprite\
                     to be clipped:\nclipping_rect:{:?}\ngiven_rect:{:?}",
                    clipping_rect,
                    rect
                )
            }
            RectIntersectionResult::Real(intersection) => {
                // Calculate how the uvs need to look now for the clipped sprite rectangle
                let sprite_width = rect.width();
                let sprite_height = rect.height();

                let relative_offset_left = (intersection.left() - rect.left()) / sprite_width;
                let relative_offset_right = (intersection.right() - rect.right()) / sprite_width;
                let relative_offset_top = (intersection.top() - rect.top()) / sprite_height;
                let relative_offset_bottom =
                    (intersection.bottom() - rect.bottom()) / sprite_height;

                let uvs_width = uvs.right - uvs.left;
                let uvs_height = uvs.bottom - uvs.top;

                let intersection_uvs = AAQuad {
                    left: uvs.left + relative_offset_left * uvs_width,
                    top: uvs.top + relative_offset_top * uvs_height,
                    right: uvs.right + relative_offset_right * uvs_width,
                    bottom: uvs.bottom + relative_offset_bottom * uvs_height,
                };

                let quad = Quad::from_rect(intersection);
                self.draw_quad(
                    &quad,
                    intersection_uvs,
                    has_translucency,
                    atlas_page,
                    depth,
                    color_modulate,
                    additivity,
                );
            }
        }
    }

    //----------------------------------------------------------------------------------------------
    // Sprite 'by index' and 'by name' functions

    pub fn get_sprite_by_index(&self, sprite_index: SpriteIndex) -> &Sprite {
        &self.atlas.sprites[sprite_index as usize]
    }

    pub fn get_sprite_by_name(&self, spritename: &str) -> &Sprite {
        self.atlas
            .sprites_by_name
            .get(spritename)
            .expect(&format!("Sprite with name '{}' does not exist", spritename))
    }

    pub fn get_sprite_index_by_name(&self, spritename: &str) -> SpriteIndex {
        self.atlas
            .sprites_by_name
            .get(spritename)
            .expect(&format!("Sprite with name '{}' does not exist", spritename))
            .index
    }

    //----------------------------------------------------------------------------------------------
    // Primitive drawing

    /// This fills the following pixels:
    /// [left, right[ x [top, bottom[
    pub fn draw_rect(&mut self, rect: Rect, depth: Depth, color: Color, additivity: Additivity) {
        let quad = Quad::from_rect(rect);
        self.draw_quad(
            &quad,
            self.untextured_uv_center_coord,
            false,
            self.untextured_uv_center_atlas_page,
            depth,
            color,
            additivity,
        );
    }

    /// Draws a rotated rectangle where `rotation_dir` = (1,0) corresponds to angle zero.
    /// IMPORTANT: `rotation_dir` is assumed to be normalized
    /// IMPORTANT: The `pivot` is the rotation pivot and position pivot
    /// This fills the following pixels when given `rotation_dir` = (1,0), `rotation_pivot` = (0,0):
    /// [left, right[ x [top, bottom[
    pub fn draw_rect_transformed(
        &mut self,
        rect_dim: Vec2,
        pivot: Vec2,
        pos: Vec2,
        scale: Vec2,
        rotation_dir: Vec2,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let quad = Quad::from_rect_transformed(rect_dim, pivot, pos, scale, rotation_dir);
        self.draw_quad(
            &quad,
            self.untextured_uv_center_coord,
            false,
            self.untextured_uv_center_atlas_page,
            depth,
            color,
            additivity,
        );
    }

    pub fn draw_circle_filled(
        &mut self,
        center: Vec2,
        radius: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let num_vertices = Circle::get_optimal_vertex_count(radius);
        let segment_count = make_even(num_vertices as u32 + 1);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        vertices.push(center);

        let mut angle_current = 0.0;
        let angle_increment = 360.0 / segment_count as f32;
        for _ in 0..segment_count {
            let mut pos = center;
            pos.x += radius * f32::cos(deg_to_rad(angle_current));
            pos.y += radius * f32::sin(deg_to_rad(angle_current));
            vertices.push(pos);

            angle_current += angle_increment;
        }

        let center_index = 0;
        for index in 0..(segment_count - 1) {
            indices.push(center_index);
            indices.push(center_index + (index + 1));
            indices.push(center_index + (index + 2));
        }
        indices.push(center_index);
        indices.push(center_index + segment_count);
        indices.push(center_index + 1);

        let uvs = vec![
            Vec2::new(
                self.untextured_uv_center_coord.left,
                self.untextured_uv_center_coord.top
            );
            vertices.len()
        ];

        self.simple_drawables.push(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: false,
            depth,
            color_modulate: color,
            additivity,
            geometry: Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            },
        });
    }

    pub fn draw_circle_bresenham(
        &mut self,
        center: Vec2,
        radius: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        // Based on the Paper "A Fast Bresenham Type Algorithm For Drawing Circles" by John Kennedy
        // https://web.engr.oregonstate.edu/~sllu/bcircle.pdf

        let center = worldpoint_pixel_snapped(center);
        let radius = roundi(radius);

        if radius == 0 {
            self.draw_pixel(center, depth, color, additivity);
            return;
        }

        // NOTE: We only calculate the first octant, the other ones are symmetrical
        //
        //    .  3 | 2  .
        //      .  |  .
        //   4    .|.   1
        // --------+--------
        //   5   . | .  8
        //     .   |   .
        //   .   6 | 7   .
        //
        // 1: ( x, y)
        // 2: ( y, x)
        // 3: (-y, x)
        // 4: (-x, y)
        // 5: (-x,-y)
        // 6: (-y,-x)
        // 7: ( y,-x)
        // 8: ( x,-y)

        let mut x = radius;
        let mut y = 0;
        let mut x_change = 1 - 2 * radius;
        let mut y_change = 1;
        let mut radius_error = 0;

        while x >= y {
            let octant1 = Vec2::new(x as f32, y as f32);
            let octant4 = Vec2::new(-x as f32, y as f32);
            let octant5 = Vec2::new(-x as f32, -y as f32);
            let octant8 = Vec2::new(x as f32, -y as f32);

            self.draw_pixel(center + octant1, depth, color, additivity);
            self.draw_pixel(center + octant4, depth, color, additivity);
            self.draw_pixel(center + octant5, depth, color, additivity);
            self.draw_pixel(center + octant8, depth, color, additivity);

            // NOTE: For x == y the below points have been already drawn in octants 1,4,5,8
            if x != y {
                let octant2 = Vec2::new(y as f32, x as f32);
                let octant3 = Vec2::new(-y as f32, x as f32);
                let octant6 = Vec2::new(-y as f32, -x as f32);
                let octant7 = Vec2::new(y as f32, -x as f32);

                self.draw_pixel(center + octant2, depth, color, additivity);
                self.draw_pixel(center + octant3, depth, color, additivity);
                self.draw_pixel(center + octant6, depth, color, additivity);
                self.draw_pixel(center + octant7, depth, color, additivity);
            }

            y += 1;
            radius_error += y_change;
            y_change += 2;

            if (2 * radius_error + x_change) > 0 {
                x -= 1;
                radius_error += x_change;
                x_change += 2;
            }
        }
    }

    pub fn ellipse_bresenham(
        &mut self,
        _center: Vec2,
        _radius_x: f32,
        _radius_y: f32,
        _depth: Depth,
        _color: Color,
        _additivity: Additivity,
    ) {
        // Based on the Paper "A Fast Bresenham Type AlgorithmFor Drawing Ellipses"
        // https://dai.fmph.uniba.sk/upload/0/01/Ellipse.pdf
        todo!()
    }

    pub fn draw_ring(
        &mut self,
        center: Vec2,
        radius: f32,
        thickness: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let num_vertices = Circle::get_optimal_vertex_count(radius);
        let segment_count = make_even(num_vertices as u32 + 1);

        let radius_outer = radius + 0.5 * thickness;
        let radius_inner = radius - 0.5 * thickness;
        assert!(0.0 < radius_inner && radius_inner < radius_outer);

        let angle_increment = 360.0 / segment_count as f32;
        let mut angle_current = 0.0;
        let mut last_unit_circle_point = Vec2::new(
            f32::cos(deg_to_rad(angle_current)),
            f32::sin(deg_to_rad(angle_current)),
        );

        angle_current += angle_increment;
        for _ in 0..segment_count {
            let unit_circle_point = Vec2::new(
                f32::cos(deg_to_rad(angle_current)),
                f32::sin(deg_to_rad(angle_current)),
            );

            let last_outer_point = center + radius_outer * last_unit_circle_point;
            let last_inner_point = center + radius_inner * last_unit_circle_point;
            let outer_point = center + radius_outer * unit_circle_point;
            let inner_point = center + radius_inner * unit_circle_point;

            let quad = Quad {
                vert_right_top: outer_point,
                vert_right_bottom: inner_point,
                vert_left_bottom: last_inner_point,
                vert_left_top: last_outer_point,
            };

            self.draw_quad(
                &quad,
                self.untextured_uv_center_coord,
                false,
                self.untextured_uv_center_atlas_page,
                depth,
                color,
                additivity,
            );

            angle_current += angle_increment;
            last_unit_circle_point = unit_circle_point;
        }
    }

    /// WARNING: This can be slow if used often
    pub fn draw_pixel(&mut self, pos: Vec2, depth: Depth, color: Color, additivity: Additivity) {
        self.draw_rect(
            Rect::from_point_dimensions(worldpoint_pixel_snapped(pos), Vec2::ones()),
            depth,
            color,
            additivity,
        );
    }

    /// WARNING: This can be slow if used often
    pub fn draw_linestrip_bresenham(
        &mut self,
        points: &[Vec2],
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        for pair in points.windows(2) {
            self.draw_line_bresenham(pair[0], pair[1], depth, color, additivity);
        }
    }

    /// WARNING: This can be slow if used often
    pub fn draw_line_bresenham(
        &mut self,
        start: Vec2,
        end: Vec2,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let mut start = Vec2i::from_vec2_floored(start);
        let mut end = Vec2i::from_vec2_floored(end);

        let mut transpose = false;
        let mut w = i32::abs(end.x - start.x);
        let mut h = i32::abs(end.y - start.y);

        if h > w {
            transpose = true;
            std::mem::swap(&mut start.x, &mut start.y);
            std::mem::swap(&mut end.x, &mut end.y);
            std::mem::swap(&mut w, &mut h);
        }

        if start.x > end.x {
            std::mem::swap(&mut start.x, &mut end.x);
            std::mem::swap(&mut start.y, &mut end.y);
        }

        let derror = 2 * h;
        let mut error = 0;
        let mut y = start.y;
        for x in (start.x)..=(end.x) {
            if transpose {
                self.draw_pixel(Vec2::new(y as f32, x as f32), depth, color, additivity);
            } else {
                self.draw_pixel(Vec2::new(x as f32, y as f32), depth, color, additivity);
            }

            error += derror;
            if error > w {
                y += if start.y < end.y { 1 } else { -1 };
                error -= 2 * w;
            }
        }
    }

    pub fn draw_line_with_thickness(
        &mut self,
        start: Vec2,
        end: Vec2,
        thickness: f32,
        smooth_edges: bool,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let perp_left = 0.5 * thickness * (end - start).perpendicular().normalized();
        let perp_right = -perp_left;

        let quad_right = Quad {
            vert_right_top: end + perp_right,
            vert_right_bottom: start + perp_right,
            vert_left_bottom: start,
            vert_left_top: end,
        };

        let quad_left = Quad {
            vert_right_top: end,
            vert_right_bottom: start,
            vert_left_bottom: start + perp_left,
            vert_left_top: end + perp_left,
        };

        let color_edges = if smooth_edges {
            Color::transparent()
        } else {
            color
        };
        let uv = Vec2::new(
            self.untextured_uv_center_coord.left,
            self.untextured_uv_center_coord.top,
        );
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        /////////////
        // Right quad

        // right top
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_right.vert_right_top, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // right bottom
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_right.vert_right_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left bottom
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_right.vert_left_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left top
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_right.vert_left_top, depth),
            uv,
            color,
            additivity,
        });

        // first triangle
        indices.push(0 + 3); // left top
        indices.push(0 + 0); // right top
        indices.push(0 + 1); // right bottom

        // second triangle
        indices.push(0 + 2); // left bottom
        indices.push(0 + 1); // right bottom
        indices.push(0 + 3); // left top

        /////////////
        // Left quad

        // right top
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_left.vert_right_top, depth),
            uv,
            color,
            additivity,
        });
        // right bottom
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_left.vert_right_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left bottom
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_left.vert_left_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left top
        vertices.push(Vertex {
            pos: Vec3::from_vec2(quad_left.vert_left_top, depth),
            uv,
            color: color_edges,
            additivity,
        });

        // first triangle
        indices.push(4 + 3); // left top
        indices.push(4 + 0); // right top
        indices.push(4 + 1); // right bottom

        // second triangle
        indices.push(4 + 2); // left bottom
        indices.push(4 + 1); // right bottom
        indices.push(4 + 3); // left top

        self.simple_drawables.push(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: true,
            depth,
            color_modulate: color,
            additivity,
            geometry: Geometry::LineMesh { vertices, indices },
        });
    }

    //--------------------------------------------------------------------------------------------------
    // Text drawing

    /// Returns width and height of a given utf8 text for a given font and font scale.
    /// NOTE: This returns a more accurate dimension than `font_get_text_dimensions` which calculates
    ///       the text-width based on the horizontal advance. This function on the other hand calculates
    ///       the text width based on the actual glyph bitmap widths.
    pub fn get_text_dimensions(&mut self, font: &SpriteFont, font_scale: f32, text: &str) -> Vec2 {
        if text.len() == 0 {
            return Vec2::zero();
        }

        let mut dimensions = Vec2::new(0.0, font_scale * font.vertical_advance);
        let mut pos = Vec2::new(0.0, font_scale * font.baseline);

        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);

                let sprite = self.get_sprite_by_index(glyph.sprite_index);
                let sprite_width = sprite.trimmed_rect.width() * font_scale;
                dimensions.x = f32::max(dimensions.x, pos.x + sprite_width);

                pos.x += font_scale * glyph.horizontal_advance;
            } else {
                pos.x = 0.0;
                pos.y += font_scale * font.vertical_advance;
                dimensions.y += font_scale * font.vertical_advance;
            }
        }

        dimensions
    }

    /// Draws a given utf8 text with a given font
    /// Returns the starting_offset for the next `text` or `text_formatted` call
    pub fn draw_text(
        &mut self,
        text: &str,
        font: &SpriteFont,
        font_scale: f32,
        starting_origin: Vec2,
        starting_offset: Vec2,
        origin_is_baseline: bool,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) -> Vec2 {
        let mut origin = worldpoint_pixel_snapped(starting_origin);
        if origin_is_baseline {
            // NOTE: Ascent will be drawn above the origin and descent below the origin
            origin.y -= font_scale * font.baseline;
        } else {
            // NOTE: Everything is drawn below the origin
        }

        let mut pos = starting_offset;
        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);

                self.draw_sprite_pixel_snapped(
                    SpriteBy::Index(glyph.sprite_index),
                    origin + pos,
                    Vec2::new(font_scale, font_scale),
                    Vec2::unit_x(),
                    false,
                    false,
                    depth,
                    color_modulate,
                    additivity,
                );

                pos.x += font_scale * glyph.horizontal_advance;
            } else {
                pos.x = 0.0;
                pos.y += font_scale * font.vertical_advance;
            }
        }

        pos
    }

    /// Draws a given utf8 text in a given font using a clipping rectangle
    /// NOTE: The given text should be already pre-wrapped for a good result
    pub fn draw_text_clipped(
        &mut self,
        text: &str,
        font: &SpriteFont,
        font_scale: f32,
        starting_origin: Vec2,
        starting_offset: Vec2,
        origin_is_baseline: bool,
        clipping_rect: Rect,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) {
        let mut origin = worldpoint_pixel_snapped(starting_origin);
        if origin_is_baseline {
            // NOTE: Ascent will be drawn above the origin and descent below the origin
            origin.y -= font_scale * font.baseline;
        } else {
            // NOTE: Everything is drawn below the origin
        }

        // Check if we would begin drawing below our clipping rectangle
        let mut current_line_top = origin.y - font_scale * font.baseline;
        let mut current_line_bottom = current_line_top + font.vertical_advance;
        current_line_top += starting_offset.y;
        current_line_bottom += starting_offset.y;
        if current_line_top > clipping_rect.bottom() {
            // NOTE: Our text begins past the lower border of the bounding rect and all following
            //       lines would not be visible anymore
            return;
        }

        let mut pos = starting_offset;
        for line in text.lines() {
            // Skip lines until we are within our bounding rectangle
            //
            if current_line_bottom >= clipping_rect.top() {
                for codepoint in line.chars() {
                    let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);
                    self.draw_sprite_clipped(
                        SpriteBy::Index(glyph.sprite_index),
                        origin + pos,
                        Vec2::new(font_scale, font_scale),
                        clipping_rect,
                        depth,
                        color_modulate,
                        additivity,
                    );

                    pos.x += font_scale * glyph.horizontal_advance;
                }
            }

            // We finished a line and need advance to the next line
            pos.x = 0.0;
            pos.y += font_scale * font.vertical_advance;

            current_line_top += font_scale * font.vertical_advance;
            current_line_bottom += font_scale * font.vertical_advance;
            if clipping_rect.bottom() <= current_line_top {
                // NOTE: We skipped past the lower border of the bounding rect and all following
                //       lines will not be visible anymore
                return;
            }
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Debug Drawing

    pub fn debug_draw_checkerboard(
        &mut self,
        origin: Vec2,
        cells_per_side: i32,
        cell_size: i32,
        color_a: Color,
        color_b: Color,
        depth: Depth,
    ) {
        for y in 0..cells_per_side {
            for x in 0..cells_per_side {
                let pos =
                    worldpoint_pixel_snapped(origin) + Vec2::new(x as f32, y as f32) * cell_size;
                let dim = Vec2::filled(cell_size as f32);
                let cell_rect = Rect::from_point_dimensions(pos, dim);
                if y % 2 == 0 {
                    self.draw_rect(
                        cell_rect,
                        depth,
                        if x % 2 == 0 { color_a } else { color_b },
                        ADDITIVITY_NONE,
                    );
                } else {
                    self.draw_rect(
                        cell_rect,
                        depth,
                        if x % 2 == 0 { color_b } else { color_a },
                        ADDITIVITY_NONE,
                    );
                }
            }
        }
    }

    pub fn debug_draw_rect_outline(
        &mut self,
        rect: Recti,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        let dim = rect.dim;
        let left_top = rect.pos;
        let right_top = left_top + Vec2i::unit_x() * (dim.x - 1);
        let right_bottom = left_top + Vec2i::unit_x() * (dim.x - 1) + Vec2i::unit_y() * (dim.y - 1);
        let left_bottom = left_top + Vec2i::unit_y() * (dim.y - 1);
        self.draw_line_bresenham(
            Vec2::from(left_top + Vec2i::unit_x()),
            Vec2::from(right_top),
            depth,
            color,
            additivity,
        );
        self.draw_line_bresenham(
            Vec2::from(right_top + Vec2i::unit_y()),
            Vec2::from(right_bottom),
            depth,
            color,
            additivity,
        );
        self.draw_line_bresenham(
            Vec2::from(right_bottom - Vec2i::unit_x()),
            Vec2::from(left_bottom),
            depth,
            color,
            additivity,
        );
        self.draw_line_bresenham(
            Vec2::from(left_bottom - Vec2i::unit_y()),
            Vec2::from(left_top),
            depth,
            color,
            additivity,
        );
    }

    pub fn debug_draw_arrow(
        &mut self,
        start: Vec2,
        dir: Vec2,
        color: Color,
        additivity: Additivity,
    ) {
        let end = start + dir;
        self.draw_line_bresenham(start, end, DEPTH_MAX, color, additivity);

        let size = clampf(dir.magnitude() / 10.0, 1.0, 5.0);
        let perp_left = size * (end - start).perpendicular().normalized();
        let perp_right = -perp_left;

        let point_tip = end;
        let point_stump = end - size * dir.normalized();
        let point_left = point_stump + perp_left;
        let point_right = point_stump + perp_right;
        self.debug_draw_triangle(point_tip, point_left, point_right, color, additivity);
    }

    pub fn debug_draw_triangle(
        &mut self,
        point_a: Vec2,
        point_b: Vec2,
        point_c: Vec2,
        color: Color,
        additivity: Additivity,
    ) {
        let vertices = vec![point_a, point_b, point_c];
        let indices = vec![0, 1, 2];
        let uvs = vec![
            Vec2::new(
                self.untextured_uv_center_coord.left,
                self.untextured_uv_center_coord.top
            );
            vertices.len()
        ];

        self.simple_drawables.push(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: false,
            depth: DEPTH_MAX,
            color_modulate: color,
            additivity,
            geometry: Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            },
        });
    }

    pub fn debug_log<S: Into<String>>(&mut self, text: S) {
        self.debug_log_color(Color::white(), text)
    }

    pub fn debug_log_color<S: Into<String>>(&mut self, color: Color, text: S) {
        // NOTE: We needed to re-implement this because the borrow-checker does not let us borrow
        //       `self.debug_log_font` to use in `self.text(...)`
        let origin = worldpoint_pixel_snapped(self.debug_log_origin);
        let mut pos = self.debug_log_offset;
        for codepoint in text.into().chars() {
            if codepoint != '\n' {
                let glyph = self
                    .debug_log_font
                    .get_glyph_for_codepoint(codepoint as Codepoint);

                self.draw_sprite_pixel_snapped(
                    SpriteBy::Index(glyph.sprite_index),
                    origin + pos,
                    Vec2::filled(self.debug_log_font_scale),
                    Vec2::unit_x(),
                    false,
                    false,
                    self.debug_log_depth,
                    color,
                    ADDITIVITY_NONE,
                );

                pos.x += self.debug_log_font_scale * glyph.horizontal_advance;
            } else {
                pos.x = 0.0;
                pos.y += self.debug_log_font_scale * self.debug_log_font.vertical_advance;
            }
        }
        // Add final '\n'
        pos.x = 0.0;
        pos.y += self.debug_log_font_scale * self.debug_log_font.vertical_advance;

        self.debug_log_offset = pos;
    }
}

// NOTE: SOME OLD STUFF THAT WE WANT TO IMPLEMENT BELOW
/*
#if 0

struct Coord
{
    r32 x1, y1;
    r32 x2, y2;
};

struct UV
{
    r32 u1, v1;
    r32 u2, Vec2;
};

typedef struct
{
    union
    {
        struct
        {
            r32 x1, y1;
            r32 x2, y2;
        };
        Coord coord;
    };
    union
    {
        struct
        {
            r32 u1, v1;
            r32 u2, Vec2;
        };
        UV uvs;
    };
} Quad;

typedef struct
{
    u32 hash;
    u32 atlasIndex;
    Quad quad;
} Sprite;

pub fn vertexbuffer_putQuad(verts: r32*, coords: Quad, depth: r32, col: Color)
{
    verts[0 + 0] = coords.x2; // aVertexPos.x
    verts[0 + 1] = coords.y2; // aVertexPos.y
    verts[0 + 2] = depth;     // aVertexPos.z
    verts[0 + 3] = col.r;     // aVertexCol.r
    verts[0 + 4] = col.g;     // aVertexCol.g
    verts[0 + 5] = col.b;     // aVertexCol.b
    verts[0 + 6] = col.a;     // aVertexCol.a
    verts[0 + 7] = coords.u2; // aVertexUV.u
    verts[0 + 8] = coords.Vec2; // aVertexUV.v

    verts[9 + 0] = coords.x2;
    verts[9 + 1] = coords.y1;
    verts[9 + 2] = depth;
    verts[9 + 3] = col.r;
    verts[9 + 4] = col.g;
    verts[9 + 5] = col.b;
    verts[9 + 6] = col.a;
    verts[9 + 7] = coords.u2;
    verts[9 + 8] = coords.v1;

    verts[18 + 0] = coords.x1;
    verts[18 + 1] = coords.y1;
    verts[18 + 2] = depth;
    verts[18 + 3] = col.r;
    verts[18 + 4] = col.g;
    verts[18 + 5] = col.b;
    verts[18 + 6] = col.a;
    verts[18 + 7] = coords.u1;
    verts[18 + 8] = coords.v1;

    verts[27 + 0] = coords.x1;
    verts[27 + 1] = coords.y2;
    verts[27 + 2] = depth;
    verts[27 + 3] = col.r;
    verts[27 + 4] = col.g;
    verts[27 + 5] = col.b;
    verts[27 + 6] = col.a;
    verts[27 + 7] = coords.u1;
    verts[27 + 8] = coords.Vec2;
}

pub fn vertexbuffer_putQuadRotated(verts: f32*, pos: Vec2, dim: Vec2, dir: Vec2, uvs: UV, depth: Depth, col: Color) )
{

    f32 x1 = -0.5f * dim.w;
    f32 y1 = -0.5f * dim.y;
    f32 x2 = 0.5f * dim.w;
    f32 y2 = 0.5f * dim.y;

    // NOTE:
    // This describes a matrix multiplication with the rotation matrix
    // | cos(a) -sin(a) |  = | dir.x  -dir.y | = | dir  Vec2_perpendicular(dir) |
    // | sin(a)  cos(a) |    | dir.y   dir.x |
    // and an additional translation by pos

    f32 x11 = pos.x + x1 * dir.x - y1 * dir.y;
    f32 y11 = pos.y + x1 * dir.y + y1 * dir.x;

    f32 x12 = pos.x + x1 * dir.x - y2 * dir.y;
    f32 y12 = pos.y + x1 * dir.y + y2 * dir.x;

    f32 x21 = pos.x + x2 * dir.x - y1 * dir.y;
    f32 y21 = pos.y + x2 * dir.y + y1 * dir.x;

    f32 x22 = pos.x + x2 * dir.x - y2 * dir.y;
    f32 y22 = pos.y + x2 * dir.y + y2 * dir.x;

    verts[0 + 0] = x22;    // aVertexPos.x
    verts[0 + 1] = y22;    // aVertexPos.y
    verts[0 + 2] = depth;  // aVertexPos.z
    verts[0 + 3] = col.r;  // aVertexCol.r
    verts[0 + 4] = col.g;  // aVertexCol.g
    verts[0 + 5] = col.b;  // aVertexCol.b
    verts[0 + 6] = col.a;  // aVertexCol.a
    verts[0 + 7] = uvs.u2; // aVertexUV.u
    verts[0 + 8] = uvs.Vec2; // aVertexUV.v

    verts[9 + 0] = x21;
    verts[9 + 1] = y21;
    verts[9 + 2] = depth;
    verts[9 + 3] = col.r;
    verts[9 + 4] = col.g;
    verts[9 + 5] = col.b;
    verts[9 + 6] = col.a;
    verts[9 + 7] = uvs.u2;
    verts[9 + 8] = uvs.v1;

    verts[18 + 0] = x11;
    verts[18 + 1] = y11;
    verts[18 + 2] = depth;
    verts[18 + 3] = col.r;
    verts[18 + 4] = col.g;
    verts[18 + 5] = col.b;
    verts[18 + 6] = col.a;
    verts[18 + 7] = uvs.u1;
    verts[18 + 8] = uvs.v1;

    verts[27 + 0] = x12;
    verts[27 + 1] = y12;
    verts[27 + 2] = depth;
    verts[27 + 3] = col.r;
    verts[27 + 4] = col.g;
    verts[27 + 5] = col.b;
    verts[27 + 6] = col.a;
    verts[27 + 7] = uvs.u1;
    verts[27 + 8] = uvs.Vec2;
}



pub fn drawbatch_pushSprite(batch: Drawbatch*, pos: Vec2, zdepth: Depth, col: Color, sprite: Sprite)
{
    assert(batch.drawMode == DRAWMODE_QUADS);

    Quad coords = sprite.quad;
    coords.x1 /= PIXELS_PER_UNIT;
    coords.y1 /= PIXELS_PER_UNIT;
    coords.x2 /= PIXELS_PER_UNIT;
    coords.y2 /= PIXELS_PER_UNIT;

    coords.x1 += pos.x;
    coords.y1 += pos.y;
    coords.x2 += pos.x;
    coords.y2 += pos.y;

    f32* verts = arena_push_array(&batch.drawBuffer, VERTEXBUFFER_NUM_FLOATS_PER_QUAD, f32);
    vertexbuffer_putQuad(verts, coords, zDepth, Color(col));

    batch.numItems += 1;
    batch.vertexBufferSize += VERTEXBUFFER_NUM_BYTES_PER_QUAD;
}

pub fn drawbatch_pushQuadRotated(batch: Drawbatch*, pos: Vec2, dim: Vec2, dir: Vec2, uvs: UV, zdepth: Depth, col: Color) )
{
    assert(batch.drawMode == DRAWMODE_QUADS);

    f32* verts = arena_push_array(&batch.drawBuffer, VERTEXBUFFER_NUM_FLOATS_PER_QUAD, f32);
    vertexbuffer_putQuadRotated(verts, pos, dim, dir, uvs, zDepth, Color(col));

    batch.numItems += 1;
    batch.vertexBufferSize += VERTEXBUFFER_NUM_BYTES_PER_QUAD;
}

pub fn drawbatch_pushQuad(batch: Drawbatch*, coords: Quad, zdepth: Depth, col: Color)
{
    assert(batch.drawMode == DRAWMODE_QUADS);

    f32* verts = arena_push_array(&batch.drawBuffer, VERTEXBUFFER_NUM_FLOATS_PER_QUAD, f32);
    vertexbuffer_putQuad(verts, coords, zDepth, Color(col));

    batch.numItems += 1;
    batch.vertexBufferSize += VERTEXBUFFER_NUM_BYTES_PER_QUAD;
}

pub fn debugQuadRotated(ds: &mut Drawstate, pos: Vec2, dim: Vec2, dir: Vec2, depth: Depth, col: Color)
{
    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_QUADS);
    Quad quad = ds.whiteSprite.quad;
    drawbatch_pushQuadRotated(&batch, pos, dim, dir, quad.uvs, depth, col);
    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugQuad(ds: &mut Drawstate, pos: Vec2, dim: Vec2, depth: Depth, col: Color, centered: bool)
{
    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_QUADS);
    Quad coords = ds.whiteSprite.quad;
    if (centered)
    {
        coords.x1 = pos.x - 0.5f * dim.w;
        coords.y1 = pos.y - 0.5f * dim.h;
        coords.x2 = pos.x + 0.5f * dim.w;
        coords.y2 = pos.y + 0.5f * dim.h;
    }
    else
    {
        coords.x1 = pos.x;
        coords.y1 = pos.y;
        coords.x2 = pos.x + dim.w;
        coords.y2 = pos.y + dim.h;
    }
    drawbatch_pushQuad(&batch, coords, depth, col);
    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugSprite(ds: &mut Drawstate, sprite: Sprite, pos: Vec2, depth: Depth, col: Color)
{
    pos = pixelsnapWorldCoord(pos);
    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_QUADS);
    drawbatch_pushSprite(&batch, pos, depth, col, sprite);
    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugSpriteRotated(ds: &mut Drawstate, sprite: Sprite, pos: Vec2, dir: Vec2, depth: Depth, col: Color)
{
    pos = pixelsnapWorldCoord(pos);
    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_QUADS);
    drawbatch_pushSpriteRotated(&batch, sprite, pos, dir, depth, col);
    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugLine(ds: &mut Drawstate, from: Vec2, to: Vec2, col: Color, depth: Depth, thickness: f32)
{
    glLineWidth(thickness);

    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_LINES);
    drawbatch_pushLine(&batch, ds, from, to, depth, col, thickness);
    drawbatch_submit(&batch, ds, ds.atlas);
}


pub fn debugGrid(ds: &mut Drawstate, cam: Camera, stepSize: f32, intensity: f32, thickness: f32)
{
    glLineWidth(thickness);
    f32 camLeft = floorf((cam.pos.x - 0.5f * cam.dim.w) / stepSize) * stepSize;
    f32 camRight = ceilf((cam.pos.x + 0.5f * cam.dim.w) / stepSize) * stepSize;
    f32 camBottom = floorf((cam.pos.y - 0.5f * cam.dim.h) / stepSize) * stepSize;
    f32 camTop = ceilf((cam.pos.y + 0.5f * cam.dim.h) / stepSize) * stepSize;

    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_LINES);
    for (f32 x = camLeft; x <= camRight; x += stepSize)
    {
        Vec2 from = Vec2_new(x, camBottom);
        Vec2 to = Vec2_new(x, camTop);
        drawbatch_pushLine(&batch, ds, from, to, 0.0f, color_grey(intensity), thickness);
    }
    for (f32 y = camBottom; y <= camTop; y += stepSize)
    {
        Vec2 from = Vec2_new(camLeft, y);
        Vec2 to = Vec2_new(camRight, y);
        drawbatch_pushLine(&batch, ds, from, to, 0.0f, color_grey(intensity), thickness);
    }

    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugCrosshair(ds: &mut Drawstate, cam: Camera, pos: Vec2, col: Color, thickness: f32)
{
    glLineWidth(thickness);
    f32 camLeft = floorf(cam.pos.x - 0.5f * cam.dim.w);
    f32 camRight = ceilf(cam.pos.x + 0.5f * cam.dim.w);
    f32 camBottom = floorf(cam.pos.y - 0.5f * cam.dim.h);
    f32 camTop = ceilf(cam.pos.y + 0.5f * cam.dim.h);

    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_LINES);

    Vec2 fromH = { camLeft, pos.y };
    Vec2 toH = Vec2_new(camRight, pos.y);
    drawbatch_pushLine(&batch, ds, fromH, toH, 0.0f, col, thickness);

    Vec2 fromV = { pos.x, camBottom };
    Vec2 toV = { pos.x, camTop };
    drawbatch_pushLine(&batch, ds, fromV, toV, 0.0f, col, thickness);

    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugCamFrustum(ds: &mut Drawstate, cam: Camera*, col: Color, thickness: f32)
{
    Rect bounds = camera_bounds(cam);
    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_LINES);

    drawbatch_pushLine(&batch,
                       ds,
                       Vec2_new(bounds.left, bounds.top),
                       Vec2_new(bounds.left, bounds.bottom),
                       0.0f,
                       col,
                       thickness);
    drawbatch_pushLine(&batch,
                       ds,
                       Vec2_new(bounds.right, bounds.top),
                       Vec2_new(bounds.right, bounds.bottom),
                       0.0f,
                       col,
                       thickness);
    drawbatch_pushLine(&batch,
                       ds,
                       Vec2_new(bounds.left, bounds.top),
                       Vec2_new(bounds.right, bounds.top),
                       0.0f,
                       col,
                       thickness);
    drawbatch_pushLine(&batch,
                       ds,
                       Vec2_new(bounds.left, bounds.bottom),
                       Vec2_new(bounds.right, bounds.bottom),
                       0.0f,
                       col,
                       thickness);

    drawbatch_submit(&batch, ds, ds.atlas);
}

pub fn debugDepthBuffer(ds: Drawstate*)
{
    int width = (int)ds.canvas.framebuffer.width;
    int height = (int)ds.canvas.framebuffer.height;
    debugi(width);
    f32* depthBuffer = (f32*)malloc((usize)(width * height) * sizeof(f32));
    glReadPixels(0, 0, width, height, GL_DEPTH_COMPONENT, GL_FLOAT, depthBuffer);

    f32 minVal = 1000.0f;
    f32 maxVal = -1000.0f;
    for (int i = 0; i < width * height; i++)
    {
        f32 depth = clampf(depthBuffer[i], 0, 1);
        minVal = minf(minVal, depth);
        maxVal = maxf(maxVal, depth);
    }

    if (minVal == maxVal)
    {
        minVal = 0.0f;
        maxVal = 1.0f;
    }

    u32* depthBufferReal = (u32*)malloc((usize)(width * height) * sizeof(f32));
    for (int i = 0; i < width * height; i++)
    {
        f32 depth = clampf(depthBuffer[i], 0, 1);
        depth = (depth - minVal) / (maxVal - minVal);

        u32 r = (u32)(255 * depth);
        u32 g = (u32)(255 * depth);
        u32 b = (u32)(255 * depth);
        u32 a = 255;

        depthBufferReal[i] = ((a << 24) | (b << 16) | (g << 8) | (r << 0));
    }
    free(depthBuffer);

    Texture depthTex = createTexture(depthBufferReal, width, height);

    Drawbatch batch = drawbatch_begin(ds.debug_drawBuffer, DRAWMODE_QUADS);
    Quad quad = {};
    quad.x1 = 0;
    quad.y1 = 0;
    quad.x2 = width;
    quad.y2 = height;
    quad.u1 = 0;
    quad.v1 = 0;
    quad.u2 = 1;
    quad.Vec2 = 1;
    drawbatch_pushQuad(&batch, quad, 0.0f, color_const(1));
    drawbatch_submit(&batch, ds, depthTex);

    freeTexture(&depthTex);
    free(depthBufferReal);
}

#endif

*/
