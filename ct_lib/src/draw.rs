pub use super::bitmap::*;
pub use super::color::*;
use super::draw_common::*;
pub use super::font::*;
use super::math::*;
use super::sprite::*;

pub use hsl;

use std::collections::HashMap;

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
    fonts: HashMap<String, SpriteFont>,
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
    pub fn new(mut atlas: SpriteAtlas, fonts: HashMap<String, SpriteFont>) -> Drawstate {
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
            let texture_info = Drawstate::textureinfo_for_page(&atlas, page_index as TextureIndex);
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

        let debug_log_font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
        let debug_log_font = fonts
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
            fonts,
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

    pub fn textureinfo_for_page(atlas: &SpriteAtlas, page_index: TextureIndex) -> TextureInfo {
        assert!((page_index as usize) < atlas.textures.len());
        TextureInfo {
            name: format!("atlas_page_{}", page_index),
            index: page_index,
            width: atlas.textures_size,
            height: atlas.textures_size,
        }
    }

    pub fn get_font(&self, font_name: &str) -> SpriteFont {
        self.fonts
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
                    texture_info: Drawstate::textureinfo_for_page(
                        &self.atlas,
                        atlas_page as TextureIndex,
                    ),
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
                    texture_info: Drawstate::textureinfo_for_page(&self.atlas, batch.texture_index),
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

            let quad = sprite.get_quad_transformed(pos.pixel_snapped(), scale, rotation_dir);

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
        if radius < 0.5 {
            self.draw_pixel(center, depth, color, additivity);
            return;
        }

        let num_vertices = Circle::get_optimal_vertex_count_for_drawing(radius);
        let segment_count = (num_vertices + 1) as u32;

        assert!(num_vertices < 32);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        vertices.push(center);

        let mut angle_current = 0.0;
        let angle_increment = deg_to_rad(360.0 / segment_count as f32);
        for _ in 0..segment_count {
            let pos = center
                + Vec2::new(
                    radius * f32::cos(angle_current),
                    radius * f32::sin(angle_current),
                );
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

        let center = center.pixel_snapped();
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
        center: Vec2,
        radius_x: f32,
        radius_y: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
    ) {
        // Based on the Paper "A Fast Bresenham Type AlgorithmFor Drawing Ellipses"
        // https://dai.fmph.uniba.sk/upload/0/01/Ellipse.pdf
        let center = center.pixel_snapped();
        let radius_x = roundi(radius_x);
        let radius_y = roundi(radius_y);

        if radius_x == 0 || radius_y == 0 {
            self.draw_pixel(center, depth, color, additivity);
            return;
        }

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
        if radius < 0.5 {
            self.draw_pixel(center, depth, color, additivity);
            return;
        }

        let num_vertices = Circle::get_optimal_vertex_count_for_drawing(radius);
        let segment_count = num_vertices as u32 + 1;

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
            Rect::from_point_dimensions(pos.pixel_snapped(), Vec2::ones()),
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

    /// Draws a given utf8 text with a given font
    /// Returns the starting_offset for the next `draw_text`
    pub fn draw_text(
        &mut self,
        text: &str,
        font: &SpriteFont,
        font_scale: f32,
        starting_origin: Vec2,
        starting_offset: Vec2,
        origin_is_baseline: bool,
        color_background: Option<Color>,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
    ) -> Vec2 {
        let origin = starting_origin.pixel_snapped_i32();
        let offset = starting_offset.pixel_snapped_i32();
        font.iter_text_glyphs(
            text,
            font_scale as i32,
            origin,
            offset,
            origin_is_baseline,
            &mut |glyph, draw_pos, _codepoint| {
                // Draw background
                if let Some(color) = color_background {
                    let sprite = self.get_sprite_by_index(glyph.sprite_index);
                    let quad = sprite.get_quad_transformed(
                        draw_pos.into(),
                        Vec2::new(font_scale, font_scale),
                        Vec2::unit_x(),
                    );
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

                // Draw glyph
                self.draw_sprite_pixel_snapped(
                    SpriteBy::Index(glyph.sprite_index),
                    draw_pos.into(),
                    Vec2::new(font_scale, font_scale),
                    Vec2::unit_x(),
                    false,
                    false,
                    depth,
                    color_modulate,
                    additivity,
                );
            },
        )
        .into()
    }

    /// Draws a given utf8 text in a given font using a clipping rectangle
    /// NOTE: This does not do any word wrapping - the given text should be already pre-wrapped
    ///       for a good result
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
        let origin = starting_origin.pixel_snapped_i32();
        let offset = starting_offset.pixel_snapped_i32();
        let clipping_recti = Recti::from_point_dimensions(
            clipping_rect.pos.pixel_snapped_i32(),
            clipping_rect.dim.roundi(),
        );
        clipping_rect.pixel_snapped_i32();
        font.iter_text_glyphs_clipped(
            text,
            font_scale as i32,
            origin,
            offset,
            origin_is_baseline,
            clipping_recti,
            &mut |glyph, draw_pos, _codepoint| {
                self.draw_sprite_clipped(
                    SpriteBy::Index(glyph.sprite_index),
                    draw_pos.into(),
                    Vec2::new(font_scale, font_scale),
                    clipping_rect,
                    depth,
                    color_modulate,
                    additivity,
                );
            },
        )
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
                let pos = origin.pixel_snapped() + Vec2::new(x as f32, y as f32) * cell_size;
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

    pub fn debug_draw_arrow_line(
        &mut self,
        start: Vec2,
        end: Vec2,
        color: Color,
        additivity: Additivity,
    ) {
        let dir = end - start;
        self.debug_draw_arrow(start, dir, color, additivity);
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

    pub fn debug_log<StringType: Into<String>>(&mut self, text: StringType) {
        self.debug_log_color(Color::white(), text)
    }

    pub fn debug_log_color<StringType: Into<String>>(&mut self, color: Color, text: StringType) {
        // NOTE: We needed to re-implement this because the borrow-checker does not let us borrow
        //       `self.debug_log_font` to use in `self.draw_text(...)`
        let origin = self.debug_log_origin.pixel_snapped();
        let mut pos = self.debug_log_offset;
        for codepoint in text.into().chars() {
            if codepoint != '\n' {
                let glyph = self
                    .debug_log_font
                    .get_glyph_for_codepoint_copy(codepoint as Codepoint);

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

                pos.x += self.debug_log_font_scale * glyph.horizontal_advance as f32;
            } else {
                pos.x = 0.0;
                pos.y += self.debug_log_font_scale * self.debug_log_font.vertical_advance as f32;
            }
        }

        // Add final '\n'
        pos.x = 0.0;
        pos.y += self.debug_log_font_scale * self.debug_log_font.vertical_advance as f32;

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
