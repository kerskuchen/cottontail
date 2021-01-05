use ct_lib_window::renderer_opengl::Renderer;

use super::image::bitmap::*;
use super::image::font::*;
use super::sprite::*;
use super::*;

use ct_lib_core::{transmute_slice_to_byte_slice, transmute_to_slice};
use std::{cell::RefCell, cmp::Ordering, rc::Rc};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertex format

pub type VertexIndex = u32;
pub type Depth = f32;
pub type Additivity = f32;

pub const DEPTH_CLEAR: Depth = 0.0;
pub const DEPTH_MAX: Depth = 100.0;

// NOTE: This translates to the depth range [0, 100] from farthest to nearest (like a paperstack)
//       For more information see: https://stackoverflow.com/a/36046924
pub const DEFAULT_WORLD_ZNEAR: Depth = 0.0;
pub const DEFAULT_WORLD_ZFAR: Depth = -100.0;

pub const ADDITIVITY_NONE: Additivity = 0.0;
pub const ADDITIVITY_MAX: Additivity = 1.0;

trait Vertex: Sized + Copy + Clone + Default {
    const FLOAT_COMPONENT_COUNT: usize = std::mem::size_of::<Self>() / std::mem::size_of::<f32>();
    fn as_floats(&self) -> &[f32] {
        unsafe { super::core::transmute_to_slice(self) }
    }
}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
struct VertexSimple {
    pub pos: Vec3,
    pub uv: Vec2,
    pub color: Color,
    pub additivity: Additivity,
}
impl Vertex for VertexSimple {}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
struct VertexBlit {
    pub pos: Vec2,
    pub uv: Vec2,
}
impl Vertex for VertexBlit {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawawbles

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DrawSpace {
    World,
    Canvas,
    Screen,
}

impl Default for DrawSpace {
    fn default() -> Self {
        DrawSpace::World
    }
}

#[derive(Debug, Clone)]
enum Geometry {
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
        vertices: Vec<VertexSimple>,
        indices: Vec<VertexIndex>,
    },
}

#[derive(Debug, Clone)]
struct Drawable {
    pub texture_index: TextureIndex,
    pub uv_region_contains_translucency: bool,
    pub depth: Depth,
    pub color_modulate: Color,
    pub additivity: Additivity,

    pub drawspace: DrawSpace,
    pub geometry: Geometry,
}

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
// Vertexbuffers

type VertexbufferSimple = Vertexbuffer<VertexSimple>;

// NOTE: We don't want to make the vertexbuffer dependent on a specific vertex type via <..>
//       generics because then it is harder to code share between drawstate and the renderer
#[derive(Debug, Default, Clone)]
struct Vertexbuffer<VertexType: Vertex> {
    pub vertices: Vec<VertexType>,
    pub indices: Vec<VertexIndex>,
}

impl<VertexType: Vertex> Vertexbuffer<VertexType> {
    pub fn new() -> Vertexbuffer<VertexType> {
        Vertexbuffer {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn current_offset(&self) -> VertexIndex {
        self.indices.len() as VertexIndex
    }
}

impl VertexbufferSimple {
    /// Returns index count of pushed object
    pub fn push_drawable(&mut self, drawable: Drawable) -> usize {
        let depth = drawable.depth;
        let color = drawable.color_modulate;
        let additivity = drawable.additivity;
        let indices_start_offset = self.vertices.len() as VertexIndex;

        let index_count = match drawable.geometry {
            Geometry::QuadMesh { uvs, quad } => {
                let index_count = 6;

                // first triangle
                self.indices.push(indices_start_offset + 3); // left top
                self.indices.push(indices_start_offset + 0); // right top
                self.indices.push(indices_start_offset + 1); // right bottom

                // second triangle
                self.indices.push(indices_start_offset + 2); // left bottom
                self.indices.push(indices_start_offset + 1); // right bottom
                self.indices.push(indices_start_offset + 3); // left top

                // right top
                self.vertices.push(VertexSimple {
                    pos: Vec3::from_vec2(quad.vert_right_top, depth),
                    uv: Vec2::new(uvs.right, uvs.top),
                    color,
                    additivity,
                });
                // right bottom
                self.vertices.push(VertexSimple {
                    pos: Vec3::from_vec2(quad.vert_right_bottom, depth),
                    uv: Vec2::new(uvs.right, uvs.bottom),
                    color,
                    additivity,
                });
                // left bottom
                self.vertices.push(VertexSimple {
                    pos: Vec3::from_vec2(quad.vert_left_bottom, depth),
                    uv: Vec2::new(uvs.left, uvs.bottom),
                    color,
                    additivity,
                });
                // left top
                self.vertices.push(VertexSimple {
                    pos: Vec3::from_vec2(quad.vert_left_top, depth),
                    uv: Vec2::new(uvs.left, uvs.top),
                    color,
                    additivity,
                });

                index_count
            }
            Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            } => {
                let index_count = indices.len();

                for index in indices {
                    self.indices.push(indices_start_offset + index);
                }
                for (pos, uv) in vertices.iter().zip(uvs.iter()) {
                    self.vertices.push(VertexSimple {
                        pos: Vec3::from_vec2(*pos, depth),
                        uv: *uv,
                        color,
                        additivity,
                    });
                }

                index_count
            }
            Geometry::LineMesh { vertices, indices } => {
                let index_count = indices.len();

                for index in indices {
                    self.indices.push(indices_start_offset + index);
                }
                self.vertices.extend_from_slice(&vertices);
                index_count
            }
        };
        index_count
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawcommands

trait UniformBlock: Sized {
    fn as_slice(&self) -> &[f32] {
        unsafe { transmute_to_slice(self) }
    }
}
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct ShaderParamsSimple {
    pub transform: Mat4,
    pub texture_color_modulate: Color,
}
impl UniformBlock for ShaderParamsSimple {}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct ShaderParamsBlit {
    pub transform: Mat4,
}
impl UniformBlock for ShaderParamsBlit {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct FramebufferInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct TextureInfo {
    pub name: String,
    pub index: TextureIndex,
    pub width: u32,
    pub height: u32,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawstate

const FRAMEBUFFER_NAME_CANVAS: &str = "canvas";

#[derive(Copy, Clone)]
struct Drawparams {
    pub depth: Depth,
    pub color_modulate: Color,
    pub additivity: Additivity,
    pub space: DrawSpace,
}

#[derive(Clone)]
struct DrawBatch {
    pub drawspace: DrawSpace,
    pub texture_index: TextureIndex,
    pub indices_start_offset: VertexIndex,
    pub indices_count: usize,
}

// NOTE: We need this static because if we put this in drawstate, then the borrowchecker won't let
//       us borrow glyphs from it when drawing logs. We can only copy glyphs (which is too expensive
//       on mobile)
// TODO: Maybe we can get rid of this when Rust updates its borrowchecker someday
static mut DRAWSTATE_DEBUG_LOG_FONT: Option<SpriteFont> = None;

#[derive(Clone)]
pub struct Drawstate {
    textures: Vec<Rc<RefCell<Bitmap>>>,
    textures_size: u32,
    textures_dirty: Vec<bool>,

    untextured_uv_center_coord: AAQuad,
    untextured_uv_center_atlas_page: TextureIndex,

    current_letterbox_color: Color,
    current_clear_color: Color,
    current_clear_depth: Depth,

    simple_drawables: Vec<Drawable>,
    simple_shaderparams_world: ShaderParamsSimple,
    simple_shaderparams_canvas: ShaderParamsSimple,
    simple_shaderparams_screen: ShaderParamsSimple,
    simple_batches_world: Vec<DrawBatch>,
    simple_batches_canvas: Vec<DrawBatch>,
    simple_batches_screen: Vec<DrawBatch>,
    simple_vertexbuffer: VertexbufferSimple,
    simple_vertexbuffer_dirty: bool,

    canvas_framebuffer: Option<FramebufferInfo>,

    debug_use_flat_color_mode: bool,
    debug_log_font_scale: f32,
    debug_log_origin: Vec2,
    debug_log_offset: Vec2,
    debug_log_depth: Depth,
}

//--------------------------------------------------------------------------------------------------
// Creation and configuration

impl Drawstate {
    pub fn new(
        textures: Vec<Rc<RefCell<Bitmap>>>,
        untextured_sprite: Sprite,
        debug_log_font: SpriteFont,
    ) -> Drawstate {
        let textures_size = textures
            .first()
            .expect("Drawstate: No Textures given")
            .borrow()
            .width as u32;

        let textures_dirty = vec![true; textures.len()];

        // Reserves a white pixel for special usage on the first page
        let untextured_uv_center_coord = untextured_sprite.trimmed_uvs;
        let untextured_uv_center_atlas_page = untextured_sprite.atlas_texture_index;

        unsafe {
            DRAWSTATE_DEBUG_LOG_FONT = Some(debug_log_font);
        }

        Drawstate {
            textures,
            textures_size,
            textures_dirty,

            untextured_uv_center_coord,
            untextured_uv_center_atlas_page,

            current_letterbox_color: Color::black(),
            current_clear_color: Color::black(),
            current_clear_depth: DEPTH_CLEAR,

            simple_drawables: Vec::new(),
            simple_shaderparams_world: ShaderParamsSimple::default(),
            simple_shaderparams_canvas: ShaderParamsSimple::default(),
            simple_shaderparams_screen: ShaderParamsSimple::default(),
            simple_batches_world: Vec::new(),
            simple_batches_canvas: Vec::new(),
            simple_batches_screen: Vec::new(),
            simple_vertexbuffer: VertexbufferSimple::new(),
            simple_vertexbuffer_dirty: true,

            canvas_framebuffer: None,

            debug_use_flat_color_mode: false,
            debug_log_font_scale: 2.0,
            debug_log_origin: Vec2::new(5.0, 5.0),
            debug_log_offset: Vec2::zero(),
            debug_log_depth: DEPTH_MAX,
        }
    }

    pub fn replace_textures(
        &mut self,
        textures: Vec<Rc<RefCell<Bitmap>>>,
        untextured_sprite: Sprite,
        debug_log_font: SpriteFont,
    ) {
        let textures_size = textures
            .first()
            .expect("Drawstate: No Textures given")
            .borrow()
            .width as u32;

        let textures_dirty = vec![true; textures.len()];

        // Reserves a white pixel for special usage on the first page
        let untextured_uv_center_coord = untextured_sprite.trimmed_uvs;
        let untextured_uv_center_atlas_page = untextured_sprite.atlas_texture_index;

        unsafe {
            DRAWSTATE_DEBUG_LOG_FONT = Some(debug_log_font);
        }

        self.textures = textures;
        self.textures_size = textures_size;
        self.textures_dirty = textures_dirty;
        self.untextured_uv_center_coord = untextured_uv_center_coord;
        self.untextured_uv_center_atlas_page = untextured_uv_center_atlas_page;
    }

    fn texturename_for_atlaspage(textures_size: u32, page_index: TextureIndex) -> String {
        format!(
            "atlas_page_{}__{}x{}",
            page_index, textures_size, textures_size
        )
    }

    pub fn set_shaderparams_simple(
        &mut self,
        color_modulate: Color,
        transform_world: Mat4,
        transform_canvas: Mat4,
        transform_screen: Mat4,
    ) {
        self.simple_shaderparams_world.texture_color_modulate = color_modulate;
        self.simple_shaderparams_world.transform = transform_world;

        self.simple_shaderparams_canvas.texture_color_modulate = color_modulate;
        self.simple_shaderparams_canvas.transform = transform_canvas;

        self.simple_shaderparams_screen.texture_color_modulate = color_modulate;
        self.simple_shaderparams_screen.transform = transform_screen;
    }

    pub fn set_letterbox_color(&mut self, color: Color) {
        self.current_letterbox_color = color;
    }

    pub fn set_clear_color_and_depth(&mut self, color: Color, depth: Depth) {
        self.current_clear_color = color;
        self.current_clear_depth = depth;
    }

    pub fn get_canvas_dimensions(&self) -> Option<(u32, u32)> {
        if let Some(canvas_framebuffer) = &self.canvas_framebuffer {
            Some((canvas_framebuffer.width, canvas_framebuffer.height))
        } else {
            None
        }
    }

    pub fn update_canvas_dimensions(&mut self, width: u32, height: u32) {
        assert!(width > 0);
        assert!(height > 0);
        self.canvas_framebuffer = Some(FramebufferInfo {
            name: FRAMEBUFFER_NAME_CANVAS.to_owned(),
            width,
            height,
        });
    }

    pub fn debug_init_logging(&mut self, font: Option<SpriteFont>, origin: Vec2, depth: Depth) {
        if let Some(font) = font {
            unsafe {
                DRAWSTATE_DEBUG_LOG_FONT = Some(font);
            }
        }
        self.debug_log_origin = origin;
        self.debug_log_depth = depth;
    }

    pub fn debug_enable_flat_color_mode(&mut self, enable: bool) {
        self.debug_use_flat_color_mode = enable;
    }
}

//--------------------------------------------------------------------------------------------------
// Beginning and ending frames

impl Drawstate {
    pub fn begin_frame(&mut self) {
        self.simple_drawables.clear();
        self.debug_log_offset = Vec2::zero();
    }

    pub fn finish_frame(&mut self) {
        self.simple_batches_world.clear();
        self.simple_batches_canvas.clear();
        self.simple_batches_screen.clear();

        if self.simple_drawables.is_empty() {
            return;
        }
        self.simple_drawables.sort_by(Drawable::compare);

        // Collect draw batches
        let mut current_batch = DrawBatch {
            drawspace: self.simple_drawables[0].drawspace,
            texture_index: self.simple_drawables[0].texture_index,
            indices_start_offset: 0,
            indices_count: 0,
        };

        self.simple_vertexbuffer.clear();
        self.simple_vertexbuffer_dirty = true;
        for drawable in self.simple_drawables.drain(..) {
            if drawable.texture_index != current_batch.texture_index
                || drawable.drawspace != current_batch.drawspace
            {
                match current_batch.drawspace {
                    DrawSpace::World => self.simple_batches_world.push(current_batch),
                    DrawSpace::Canvas => self.simple_batches_canvas.push(current_batch),
                    DrawSpace::Screen => self.simple_batches_screen.push(current_batch),
                }
                current_batch = DrawBatch {
                    drawspace: drawable.drawspace,
                    texture_index: drawable.texture_index,
                    indices_start_offset: self.simple_vertexbuffer.current_offset(),
                    indices_count: 0,
                };
            }

            let indices_count = self.simple_vertexbuffer.push_drawable(drawable);
            current_batch.indices_count += indices_count;
        }

        match current_batch.drawspace {
            DrawSpace::World => self.simple_batches_world.push(current_batch),
            DrawSpace::Canvas => self.simple_batches_canvas.push(current_batch),
            DrawSpace::Screen => self.simple_batches_screen.push(current_batch),
        }
    }

    pub fn render_frame(&mut self, renderer: &mut Renderer) {
        // Re-upload modified atlas pages
        for atlas_page in 0..self.textures_dirty.len() {
            if self.textures_dirty[atlas_page] {
                let texture_name = Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    atlas_page as TextureIndex,
                );
                let atlas_page_bitmap = &self.textures[atlas_page].borrow();
                renderer.texture_create_or_update_whole(
                    &texture_name,
                    atlas_page_bitmap.width as u32,
                    atlas_page_bitmap.height as u32,
                    &atlas_page_bitmap.as_bytes(),
                );
                self.textures_dirty[atlas_page] = false;
            }
        }

        // NOTE: Even if we have our own offscreen framebuffer that we want to draw to, we still
        //       need to clear the screen framebuffer
        renderer.framebuffer_clear(
            "screen",
            Some(self.current_letterbox_color.to_slice()),
            Some(DEPTH_CLEAR),
        );

        // Create and Clear canvas
        let draw_framebuffer_name = if let Some(canvas_framebuffer) = &self.canvas_framebuffer {
            renderer.framebuffer_create_or_update(
                &canvas_framebuffer.name,
                canvas_framebuffer.width,
                canvas_framebuffer.height,
            );
            renderer.framebuffer_clear(
                &canvas_framebuffer.name,
                Some(self.current_clear_color.to_slice()),
                Some(self.current_clear_depth),
            );
            &canvas_framebuffer.name
        } else {
            "screen"
        };

        // Upload vertexbuffers
        if self.simple_vertexbuffer_dirty {
            unsafe {
                renderer.assign_buffers(
                    "simple",
                    &transmute_slice_to_byte_slice(&self.simple_vertexbuffer.vertices),
                    &transmute_slice_to_byte_slice(&self.simple_vertexbuffer.indices),
                );
            }
            self.simple_vertexbuffer_dirty = false;
        }

        // Draw world- and canvas-space batches
        for world_batch in &self.simple_batches_world {
            renderer.draw(
                "simple",
                &self.simple_shaderparams_world.as_slice(),
                &draw_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    world_batch.texture_index,
                ),
                world_batch.indices_start_offset,
                world_batch.indices_count,
            );
        }
        for canvas_batch in &self.simple_batches_canvas {
            renderer.draw(
                "simple",
                &self.simple_shaderparams_canvas.as_slice(),
                &draw_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    canvas_batch.texture_index,
                ),
                canvas_batch.indices_start_offset,
                canvas_batch.indices_count,
            );
        }

        // If we drew to an offscreen-canvas we must blit it back to the screen
        if let Some(canvas_framebuffer) = &self.canvas_framebuffer {
            let (screen_width, screen_height) = renderer.get_screen_dimensions();

            let rect_canvas =
                BlitRect::new_from_dimensions(canvas_framebuffer.width, canvas_framebuffer.height);
            let rect_screen = BlitRect::new_for_fixed_canvas_size(
                screen_width,
                screen_height,
                canvas_framebuffer.width,
                canvas_framebuffer.height,
            );

            renderer.framebuffer_blit(
                &canvas_framebuffer.name,
                "screen",
                rect_canvas.to_recti(),
                rect_screen.to_recti(),
            );
        }

        // Draw screenspace batches last so they won't get overdrawn by framebuffer blits
        for screen_batch in &self.simple_batches_screen {
            renderer.draw(
                "simple",
                &self.simple_shaderparams_screen.as_slice(),
                "screen",
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    screen_batch.texture_index,
                ),
                screen_batch.indices_start_offset,
                screen_batch.indices_count,
            );
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Drawing

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
        drawspace: DrawSpace,
    ) {
        if !self.debug_use_flat_color_mode {
            self.simple_drawables.push(Drawable {
                drawspace,
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
                drawspace,
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
    #[inline]
    pub fn draw_sprite(
        &mut self,
        sprite: &Sprite,
        xform: Transform,
        flip_horizontally: bool,
        flip_vertically: bool,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let (sprite_quad, sprite_uvs, texture_index, has_translucency) = {
            let quad = sprite.get_quad_transformed(xform);

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
            drawspace,
        );
    }

    #[inline]
    pub fn draw_sprite_clipped(
        &mut self,
        sprite: &Sprite,
        pos: Vec2,
        scale: Vec2,
        clipping_rect: Rect,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
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
                    drawspace,
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
                    drawspace,
                );
            }
        }
    }

    #[inline]
    pub fn draw_sprite_3d(
        &mut self,
        sprite: &Sprite3D,
        xform: Transform,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let depth_increment = 1.0 / sprite.layers.len() as f32;
        let xform_snapped = xform.pixel_snapped();
        for (index, sprite) in sprite.layers.iter().rev().enumerate() {
            self.draw_sprite(
                sprite,
                Transform {
                    pos: xform_snapped.pos + index as f32 * Vec2::unit_y(),
                    scale: xform_snapped.scale,
                    dir_angle: xform_snapped.dir_angle,
                },
                false,
                false,
                depth - (index as f32) * 0.5 * depth_increment,
                color_modulate,
                additivity,
                drawspace,
            );
        }
    }

    //----------------------------------------------------------------------------------------------
    // Primitive drawing

    /// This fills the following pixels:
    /// [left, right[ x [top, bottom[
    #[inline]
    pub fn draw_rect(
        &mut self,
        rect: Rect,
        filled: bool,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let rect = rect.pixel_snapped();
        if filled {
            let quad = Quad::from_rect(rect);
            self.draw_quad(
                &quad,
                self.untextured_uv_center_coord,
                false,
                self.untextured_uv_center_atlas_page,
                depth,
                color,
                additivity,
                drawspace,
            );
        } else {
            let dim = rect.dim;
            if dim.x == 0.0 || dim.y == 0.0 {
                return;
            }

            // NOTE We want to draw the lines in a way that the pixel at `rect.pos + rect.dim` is
            //      empty. That way we can draw another rect at `rect.pos + rect.dim` without it
            //      overlapping the previous rect or leaving a gap between both. Therefore we use
            //      `dim.x - 1.0` and `dim.y - 1.0`
            let left_top = rect.pos;
            let right_top = left_top + Vec2::filled_x(dim.x - 1.0);
            let right_bottom = left_top + Vec2::new(dim.x - 1.0, dim.y - 1.0);
            let left_bottom = left_top + Vec2::filled_y(dim.y - 1.0);

            let linestrip = [left_top, right_top, right_bottom, left_bottom, left_top];
            self.draw_linestrip_bresenham(&linestrip, true, depth, color, additivity, drawspace);
        }
    }

    /// Draws a rotated rectangle where `rotation_dir` = (1,0) corresponds to angle zero.
    /// IMPORTANT: `rotation_dir` is assumed to be normalized
    /// IMPORTANT: The `pivot` is the rotation pivot and position pivot
    /// This fills the following pixels when given `rotation_dir` = (1,0), `rotation_pivot` = (0,0):
    /// [left, right[ x [top, bottom[
    #[inline]
    pub fn draw_rect_transformed(
        &mut self,
        rect_dim: Vec2,
        filled: bool,
        centered: bool,
        pivot: Vec2,
        xform: Transform,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let pivot = pivot
            + if centered {
                rect_dim / 2.0
            } else {
                Vec2::zero()
            };

        let quad = Quad::from_rect_transformed(rect_dim, pivot, xform);
        if filled {
            self.draw_quad(
                &quad,
                self.untextured_uv_center_coord,
                false,
                self.untextured_uv_center_atlas_page,
                depth,
                color,
                additivity,
                drawspace,
            );
        } else {
            let linestrip = quad.to_linestrip();
            self.draw_linestrip_bresenham(&linestrip, true, depth, color, additivity, drawspace);
        }
    }

    /// Expects vertices in the form [v_a0, v_a1, v_a2, v_b0, v_b1, v_b2, ...]
    #[inline]
    pub fn draw_polygon(
        &mut self,
        vertices: &[Vec2],
        pivot: Vec2,
        xform: Transform,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let vertices = Vec2::multi_transformed(vertices, pivot, xform);
        let indices = (0..vertices.len() as u32).collect();
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
            drawspace,
        });
    }

    #[inline]
    pub fn draw_circle_filled(
        &mut self,
        center: Vec2,
        radius: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        if radius < 0.5 {
            self.draw_pixel(center, depth, color, additivity, drawspace);
            return;
        }

        let num_vertices = Circle::get_optimal_vertex_count_for_drawing(radius);
        let segment_count = (num_vertices + 1) as u32;

        assert!(num_vertices < 32);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        vertices.push(center);

        let mut angle_current = 0.0;
        let angle_increment = DEGREE_TO_RADIANS * (360.0 / segment_count as f32);
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
            drawspace,
        });
    }

    #[inline]
    pub fn draw_circle_bresenham(
        &mut self,
        center: Vec2,
        radius: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        // Based on the Paper "A Fast Bresenham Type Algorithm For Drawing Circles" by John Kennedy
        // https://web.engr.oregonstate.edu/~sllu/bcircle.pdf

        let center = center.pixel_snapped();
        let radius = roundi(radius);

        if radius == 0 {
            self.draw_pixel(center, depth, color, additivity, drawspace);
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

            self.draw_pixel(center + octant1, depth, color, additivity, drawspace);
            self.draw_pixel(center + octant4, depth, color, additivity, drawspace);
            self.draw_pixel(center + octant5, depth, color, additivity, drawspace);
            self.draw_pixel(center + octant8, depth, color, additivity, drawspace);

            // NOTE: For x == y the below points have been already drawn in octants 1,4,5,8
            if x != y {
                let octant2 = Vec2::new(y as f32, x as f32);
                let octant3 = Vec2::new(-y as f32, x as f32);
                let octant6 = Vec2::new(-y as f32, -x as f32);
                let octant7 = Vec2::new(y as f32, -x as f32);

                self.draw_pixel(center + octant2, depth, color, additivity, drawspace);
                self.draw_pixel(center + octant3, depth, color, additivity, drawspace);
                self.draw_pixel(center + octant6, depth, color, additivity, drawspace);
                self.draw_pixel(center + octant7, depth, color, additivity, drawspace);
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

    #[inline]
    pub fn ellipse_bresenham(
        &mut self,
        center: Vec2,
        radius_x: f32,
        radius_y: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        // Based on the Paper "A Fast Bresenham Type AlgorithmFor Drawing Ellipses"
        // https://dai.fmph.uniba.sk/upload/0/01/Ellipse.pdf
        let center = center.pixel_snapped();
        let radius_x = roundi(radius_x);
        let radius_y = roundi(radius_y);

        if radius_x == 0 || radius_y == 0 {
            self.draw_pixel(center, depth, color, additivity, drawspace);
            return;
        }

        todo!()
    }

    #[inline]
    pub fn draw_ring(
        &mut self,
        center: Vec2,
        radius: f32,
        thickness: f32,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        if radius < 0.5 {
            self.draw_pixel(center, depth, color, additivity, drawspace);
            return;
        }

        let num_vertices = Circle::get_optimal_vertex_count_for_drawing(radius);
        let segment_count = num_vertices as u32 + 1;

        let radius_outer = radius + 0.5 * thickness;
        let radius_inner = radius - 0.5 * thickness;
        assert!(0.0 < radius_inner && radius_inner < radius_outer);

        let angle_increment = DEGREE_TO_RADIANS * 360.0 / segment_count as f32;
        let mut angle_current = 0.0;
        let mut last_unit_circle_point =
            Vec2::new(f32::cos(angle_current), f32::sin(angle_current));

        angle_current += angle_increment;
        for _ in 0..segment_count {
            let unit_circle_point = Vec2::new(f32::cos(angle_current), f32::sin(angle_current));

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
                drawspace,
            );

            angle_current += angle_increment;
            last_unit_circle_point = unit_circle_point;
        }
    }

    /// WARNING: This can be slow if used often
    #[inline]
    pub fn draw_pixel(
        &mut self,
        pos: Vec2,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        self.draw_rect(
            Rect::from_pos_dim(pos, Vec2::ones()),
            true,
            depth,
            color,
            additivity,
            drawspace,
        );
    }

    /// WARNING: This can be slow if used often
    /// NOTE: Skipping the last pixel is useful i.e. for drawing translucent line loops which start
    ///       and end on the same pixel and pixels must not overlap
    #[inline]
    pub fn draw_linestrip_bresenham(
        &mut self,
        points: &[Vec2],
        skip_last_pixel: bool,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        for pair in points.windows(2) {
            self.draw_line_bresenham(pair[0], pair[1], true, depth, color, additivity, drawspace);
        }
        if !skip_last_pixel && !points.is_empty() {
            self.draw_pixel(*points.last().unwrap(), depth, color, additivity, drawspace);
        }
    }

    /// WARNING: This can be slow if used often
    /// NOTE: Skipping the last pixel is useful i.e. for drawing translucent linestrips where pixels
    ///       must not overlap
    #[inline]
    pub fn draw_line_bresenham(
        &mut self,
        start: Vec2,
        end: Vec2,
        skip_last_pixel: bool,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let start = start.pixel_snapped().to_i32();
        let end = end.pixel_snapped().to_i32();
        iterate_line_bresenham(start, end, skip_last_pixel, &mut |x, y| {
            self.draw_pixel(
                Vec2::new(x as f32, y as f32),
                depth,
                color,
                additivity,
                drawspace,
            )
        });
    }

    #[inline]
    pub fn draw_line_with_thickness(
        &mut self,
        start: Vec2,
        end: Vec2,
        thickness: f32,
        smooth_edges: bool,
        depth: Depth,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
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
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_right.vert_right_top, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // right bottom
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_right.vert_right_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left bottom
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_right.vert_left_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left top
        vertices.push(VertexSimple {
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
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_left.vert_right_top, depth),
            uv,
            color,
            additivity,
        });
        // right bottom
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_left.vert_right_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left bottom
        vertices.push(VertexSimple {
            pos: Vec3::from_vec2(quad_left.vert_left_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left top
        vertices.push(VertexSimple {
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
            drawspace,
        });
    }

    //--------------------------------------------------------------------------------------------------
    // Text drawing

    /// Draws a given utf8 text with a given font
    /// Returns the starting_offset for the next `draw_text`
    #[inline]
    pub fn draw_text(
        &mut self,
        text: &str,
        font: &SpriteFont,
        font_scale: f32,
        starting_origin: Vec2,
        starting_offset: Vec2,
        alignment: Option<TextAlignment>,
        color_background: Option<Color>,
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) -> Vec2 {
        let origin = starting_origin.pixel_snapped().to_i32();
        let offset = starting_offset.pixel_snapped().to_i32();
        if let Some(color_background) = color_background {
            font.iter_text_glyphs_aligned_in_point(
                text,
                font_scale as i32,
                origin,
                offset,
                alignment,
                &mut |glyph, draw_pos, _codepoint| {
                    // Draw background
                    let quad =
                        glyph
                            .sprite
                            .get_quad_transformed(Transform::from_pos_scale_uniform(
                                draw_pos.into(),
                                font_scale,
                            ));
                    self.draw_quad(
                        &quad,
                        self.untextured_uv_center_coord,
                        false,
                        self.untextured_uv_center_atlas_page,
                        depth,
                        color_background,
                        additivity,
                        drawspace,
                    );

                    // Draw glyph
                    self.draw_sprite(
                        &glyph.sprite,
                        Transform::from_pos_scale_uniform(draw_pos.into(), font_scale),
                        false,
                        false,
                        depth,
                        color_modulate,
                        additivity,
                        drawspace,
                    );
                },
            )
            .into()
        } else {
            font.iter_text_glyphs_aligned_in_point(
                text,
                font_scale as i32,
                origin,
                offset,
                alignment,
                &mut |glyph, draw_pos, _codepoint| {
                    // Draw glyph
                    self.draw_sprite(
                        &glyph.sprite,
                        Transform::from_pos_scale_uniform(draw_pos.into(), font_scale),
                        false,
                        false,
                        depth,
                        color_modulate,
                        additivity,
                        drawspace,
                    );
                },
            )
            .into()
        }
    }

    /// Draws a given utf8 text in a given font using a clipping rectangle
    /// NOTE: This does not do any word wrapping - the given text should be already pre-wrapped
    ///       for a good result
    #[inline]
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
        drawspace: DrawSpace,
    ) {
        let origin = starting_origin.pixel_snapped().to_i32();
        let offset = starting_offset.pixel_snapped().to_i32();
        let clipping_recti = Recti::from_pos_dim(
            clipping_rect.pos.pixel_snapped().to_i32(),
            clipping_rect.dim.roundi(),
        );
        font.iter_text_glyphs_clipped(
            text,
            font_scale as i32,
            origin,
            offset,
            origin_is_baseline,
            clipping_recti,
            &mut |glyph, draw_pos, _codepoint| {
                self.draw_sprite_clipped(
                    &glyph.sprite,
                    draw_pos.into(),
                    Vec2::new(font_scale, font_scale),
                    clipping_rect,
                    depth,
                    color_modulate,
                    additivity,
                    drawspace,
                );
            },
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Debug Drawing

    #[inline]
    pub fn debug_draw_checkerboard(
        &mut self,
        origin: Vec2,
        cells_per_side: i32,
        cell_size: i32,
        color_a: Color,
        color_b: Color,
        depth: Depth,
        drawspace: DrawSpace,
    ) {
        let origin = origin.pixel_snapped();
        for y in 0..cells_per_side {
            for x in 0..cells_per_side {
                let pos = origin + Vec2::new(x as f32, y as f32) * cell_size;
                let dim = Vec2::filled(cell_size as f32);
                let cell_rect = Rect::from_pos_dim(pos, dim);
                if y % 2 == 0 {
                    self.draw_rect(
                        cell_rect,
                        true,
                        depth,
                        if x % 2 == 0 { color_a } else { color_b },
                        ADDITIVITY_NONE,
                        drawspace,
                    );
                } else {
                    self.draw_rect(
                        cell_rect,
                        true,
                        depth,
                        if x % 2 == 0 { color_b } else { color_a },
                        ADDITIVITY_NONE,
                        drawspace,
                    );
                }
            }
        }
    }

    #[inline]
    pub fn debug_draw_arrow(
        &mut self,
        start: Vec2,
        dir: Vec2,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let end = start + dir;
        self.draw_line_bresenham(start, end, false, DEPTH_MAX, color, additivity, drawspace);

        let size = clampf(dir.magnitude() / 10.0, 1.0, 5.0);
        let perp_left = size * (end - start).perpendicular().normalized();
        let perp_right = -perp_left;

        let point_tip = end;
        let point_stump = end - size * dir.normalized();
        let point_left = point_stump + perp_left;
        let point_right = point_stump + perp_right;
        self.debug_draw_triangle(
            point_tip,
            point_left,
            point_right,
            color,
            additivity,
            drawspace,
        );
    }

    #[inline]
    pub fn debug_draw_arrow_line(
        &mut self,
        start: Vec2,
        end: Vec2,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
    ) {
        let dir = end - start;
        self.debug_draw_arrow(start, dir, color, additivity, drawspace);
    }

    #[inline]
    pub fn debug_draw_triangle(
        &mut self,
        point_a: Vec2,
        point_b: Vec2,
        point_c: Vec2,
        color: Color,
        additivity: Additivity,
        drawspace: DrawSpace,
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
            drawspace,
        });
    }

    #[inline]
    pub fn debug_log<StringType: Into<String>>(&mut self, text: StringType) {
        self.debug_log_color(Color::white(), text)
    }

    #[inline]
    pub fn debug_log_color<StringType: Into<String>>(&mut self, color: Color, text: StringType) {
        // NOTE: We needed to re-implement this because the borrow-checker does not let us borrow
        //       `self.debug_log_font` to use in `self.draw_text(...)`
        let origin = self.debug_log_origin.pixel_snapped();
        let mut pos = self.debug_log_offset;
        let debug_font = unsafe {
            // NOTE: See documentation above for explanation why we need this static
            DRAWSTATE_DEBUG_LOG_FONT
                .as_ref()
                .expect("Debug logging font not initialized")
        };
        for codepoint in text.into().chars() {
            if codepoint != '\n' {
                let glyph = debug_font.get_glyph_for_codepoint(codepoint as Codepoint);

                self.draw_sprite(
                    &glyph.sprite,
                    Transform::from_pos_scale_uniform(origin + pos, self.debug_log_font_scale),
                    false,
                    false,
                    self.debug_log_depth,
                    color,
                    ADDITIVITY_NONE,
                    DrawSpace::Screen,
                );

                pos.x += self.debug_log_font_scale * glyph.horizontal_advance as f32;
            } else {
                pos.x = 0.0;
                pos.y += self.debug_log_font_scale * debug_font.vertical_advance as f32;
            }
        }

        // Add final '\n'
        pos.x = 0.0;
        pos.y += self.debug_log_font_scale * debug_font.vertical_advance as f32;

        self.debug_log_offset = pos;
    }
}

// NOTE: SOME OLD STUFF THAT WE WANT TO IMPLEMENT BELOW
/*

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
