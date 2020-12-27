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

pub trait Vertex: Sized {
    const FLOAT_COMPONENT_COUNT: usize = std::mem::size_of::<Self>() / std::mem::size_of::<f32>();
    fn as_floats(&self) -> &[f32] {
        unsafe { super::core::transmute_to_slice(self) }
    }
}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexSimple {
    pub pos: Vec3,
    pub uv: Vec2,
    pub color: Color,
    pub additivity: Additivity,
}
impl Vertex for VertexSimple {}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBlit {
    pub pos: Vec2,
    pub uv: Vec2,
}
impl Vertex for VertexBlit {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertexbuffers

// NOTE: We don't want to make the vertexbuffer dependent on a specific vertex type via <..>
//       generics because then it is harder to code share between drawstate and the renderer
#[derive(Debug, Default)]
pub struct Vertexbuffer {
    pub vertices: Vec<f32>,
    pub indices: Vec<VertexIndex>,
    vertices_count: usize,
    const_vertices_float_component_count: usize,
}

impl Vertexbuffer {
    pub fn new<VertexType: Vertex>() -> Vertexbuffer {
        Vertexbuffer {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertices_count: 0,
            const_vertices_float_component_count: VertexType::FLOAT_COMPONENT_COUNT,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.vertices_count = 0;
    }

    pub fn current_offset(&self) -> VertexIndex {
        self.indices.len() as VertexIndex
    }

    /// Returns (start_index_offset, index_count) of pushed object
    pub fn push_blit_quad(
        &mut self,
        rect_target: BlitRect,
        rect_source: BlitRect,
        framebuffer_source_width: u32,
        framebuffer_source_height: u32,
    ) -> (VertexIndex, usize) {
        debug_assert!(
            self.const_vertices_float_component_count == VertexBlit::FLOAT_COMPONENT_COUNT
        );

        let start_index = self.vertices_count as VertexIndex;
        let index_count = 6;

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
        self.vertices.extend_from_slice(
            VertexBlit {
                pos: Vec2::new(dim.right(), dim.top()),
                uv: Vec2::new(uvs.right(), uvs.top()),
            }
            .as_floats(),
        );
        // right bottom
        self.vertices.extend_from_slice(
            VertexBlit {
                pos: Vec2::new(dim.right(), dim.bottom()),
                uv: Vec2::new(uvs.right(), uvs.bottom()),
            }
            .as_floats(),
        );
        // left bottom
        self.vertices.extend_from_slice(
            VertexBlit {
                pos: Vec2::new(dim.left(), dim.bottom()),
                uv: Vec2::new(uvs.left(), uvs.bottom()),
            }
            .as_floats(),
        );
        // left top
        self.vertices.extend_from_slice(
            VertexBlit {
                pos: Vec2::new(dim.left(), dim.top()),
                uv: Vec2::new(uvs.left(), uvs.top()),
            }
            .as_floats(),
        );
        self.vertices_count += 4;

        (start_index, index_count)
    }

    /// Returns (start_index, index_count) of pushed object
    pub fn push_drawable(&mut self, drawable: Drawable) -> (VertexIndex, usize) {
        debug_assert!(
            self.const_vertices_float_component_count == VertexSimple::FLOAT_COMPONENT_COUNT
        );
        let depth = drawable.depth;
        let color = drawable.color_modulate;
        let additivity = drawable.additivity;
        let indices_start_offset = self.vertices_count as VertexIndex;

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
                self.vertices.extend_from_slice(
                    VertexSimple {
                        pos: Vec3::from_vec2(quad.vert_right_top, depth),
                        uv: Vec2::new(uvs.right, uvs.top),
                        color,
                        additivity,
                    }
                    .as_floats(),
                );
                // right bottom
                self.vertices.extend_from_slice(
                    VertexSimple {
                        pos: Vec3::from_vec2(quad.vert_right_bottom, depth),
                        uv: Vec2::new(uvs.right, uvs.bottom),
                        color,
                        additivity,
                    }
                    .as_floats(),
                );
                // left bottom
                self.vertices.extend_from_slice(
                    VertexSimple {
                        pos: Vec3::from_vec2(quad.vert_left_bottom, depth),
                        uv: Vec2::new(uvs.left, uvs.bottom),
                        color,
                        additivity,
                    }
                    .as_floats(),
                );
                // left top
                self.vertices.extend_from_slice(
                    VertexSimple {
                        pos: Vec3::from_vec2(quad.vert_left_top, depth),
                        uv: Vec2::new(uvs.left, uvs.top),
                        color,
                        additivity,
                    }
                    .as_floats(),
                );
                self.vertices_count += 4;

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
                    self.vertices.extend_from_slice(
                        VertexSimple {
                            pos: Vec3::from_vec2(*pos, depth),
                            uv: *uv,
                            color,
                            additivity,
                        }
                        .as_floats(),
                    );
                    self.vertices_count += 1;
                }

                index_count
            }
            Geometry::LineMesh { vertices, indices } => {
                let index_count = indices.len();

                for index in indices {
                    self.indices.push(indices_start_offset + index);
                }
                for vertex in vertices {
                    self.vertices.extend_from_slice(vertex.as_floats());
                    self.vertices_count += 1;
                }
                index_count
            }
        };
        (indices_start_offset, index_count)
    }
}

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
        vertices: Vec<VertexSimple>,
        indices: Vec<VertexIndex>,
    },
}

#[derive(Debug, Clone)]
pub struct Drawable {
    pub drawspace: DrawSpace,
    pub texture_index: TextureIndex,
    pub uv_region_contains_translucency: bool,
    pub depth: Depth,
    pub color_modulate: Color,
    pub additivity: Additivity,

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
// Drawcommands

pub trait UniformBlock: Sized {
    fn as_slice(&self) -> &[f32] {
        unsafe { transmute_to_slice(self) }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ShaderParamsSimple {
    pub transform: Mat4,
    pub texture_color_modulate: Color,
}
impl UniformBlock for ShaderParamsSimple {}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ShaderParamsBlit {
    pub transform: Mat4,
}
impl UniformBlock for ShaderParamsBlit {}

#[derive(Debug, Clone)]
pub enum ShaderParams {
    Simple { uniform_block: Vec<f32> },
    Blit { uniform_block: Vec<f32> },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FramebufferInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
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
    AssignDrawBuffers {
        shader: String,
        vertexbuffer: Rc<RefCell<Vertexbuffer>>,
    },
    Draw {
        shader: String,
        uniform_block: Vec<f32>,
        framebuffer_target: FramebufferTarget,
        texture_info: TextureInfo,
        indices_start_offset: VertexIndex,
        indices_count: usize,
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
            Drawcommand::AssignDrawBuffers {
                shader,
                vertexbuffer,
            } => write!(
                f,
                "AssignDrawBuffers: shader: {}, vertexbuffer floatcount: {}, indices count: {}", 
                shader,
                vertexbuffer.borrow().vertices_count,
                vertexbuffer.borrow().indices.len()
            ),
            Drawcommand::Draw {
                shader,
                uniform_block,
                framebuffer_target,
                texture_info,
                indices_count,
                indices_start_offset,
            } => write!(
                f,
                "Draw: shader: {}, uniform_block_length: {}, {:?}, {:?}, indices_count: {}, indices_start_offset: {}",
                shader,
                uniform_block.len(),
                framebuffer_target,
                texture_info,
                indices_count,
                indices_start_offset,
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

#[derive(Clone)]
pub struct Drawstate {
    textures: Vec<Bitmap>,
    textures_size: u32,
    textures_dirty: Vec<bool>,

    untextured_uv_center_coord: AAQuad,
    untextured_uv_center_atlas_page: TextureIndex,

    current_letterbox_color: Color,
    current_clear_color: Color,
    current_clear_depth: Depth,

    simple_shaderparams_world: ShaderParamsSimple,
    simple_shaderparams_canvas: ShaderParamsSimple,
    simple_shaderparams_screen: ShaderParamsSimple,
    simple_batches_world: Vec<DrawBatch>,
    simple_batches_canvas: Vec<DrawBatch>,
    simple_batches_screen: Vec<DrawBatch>,
    simple_drawables: Vec<Drawable>,
    simple_vertexbuffer: Rc<RefCell<Vertexbuffer>>,
    simple_vertexbuffer_dirty: bool,

    canvas_framebuffer: Option<FramebufferInfo>,
    canvas_blit_offset: Vec2,

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
    pub fn new(
        textures: Vec<Bitmap>,
        untextured_sprite: Sprite,
        debug_log_font: SpriteFont,
    ) -> Drawstate {
        let textures_size = textures
            .first()
            .expect("Drawstate: No Textures given")
            .width as u32;

        let textures_dirty = vec![true; textures.len()];

        // Reserves a white pixel for special usage on the first page
        let untextured_uv_center_coord = untextured_sprite.trimmed_uvs;
        let untextured_uv_center_atlas_page = untextured_sprite.atlas_texture_index;

        Drawstate {
            textures,
            textures_size,
            textures_dirty,

            untextured_uv_center_coord,
            untextured_uv_center_atlas_page,

            current_letterbox_color: Color::black(),
            current_clear_color: Color::black(),
            current_clear_depth: DEPTH_CLEAR,

            simple_shaderparams_world: ShaderParamsSimple::default(),
            simple_shaderparams_canvas: ShaderParamsSimple::default(),
            simple_shaderparams_screen: ShaderParamsSimple::default(),
            simple_batches_world: Vec::new(),
            simple_batches_canvas: Vec::new(),
            simple_batches_screen: Vec::new(),
            simple_drawables: Vec::new(),
            simple_vertexbuffer: Rc::new(RefCell::new(Vertexbuffer::new::<VertexSimple>())),
            simple_vertexbuffer_dirty: true,

            canvas_framebuffer: None,
            canvas_blit_offset: Vec2::zero(),

            debug_use_flat_color_mode: false,
            debug_log_font,
            debug_log_font_scale: 2.0,
            debug_log_origin: Vec2::new(5.0, 5.0),
            debug_log_offset: Vec2::zero(),
            debug_log_depth: DEPTH_MAX,
        }
    }

    pub fn texturename_for_atlaspage(
        textures_size: u32,
        textures_count: usize,
        page_index: TextureIndex,
    ) -> String {
        assert!((page_index as usize) < textures_count);
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
        canvas_blit_offset: Vec2,
    ) {
        self.simple_shaderparams_world.texture_color_modulate = color_modulate;
        self.simple_shaderparams_world.transform = transform_world;

        self.simple_shaderparams_canvas.texture_color_modulate = color_modulate;
        self.simple_shaderparams_canvas.transform = transform_canvas;

        self.simple_shaderparams_screen.texture_color_modulate = color_modulate;
        self.simple_shaderparams_screen.transform = transform_screen;

        self.canvas_blit_offset = canvas_blit_offset;
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
            self.debug_log_font = font;
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

        self.simple_drawables.sort_by(Drawable::compare);
        if self.simple_drawables.is_empty() {
            return;
        }

        // Collect draw batches
        self.simple_vertexbuffer_dirty = true;
        let mut vertexbuffer = self.simple_vertexbuffer.borrow_mut();
        let mut current_batch = DrawBatch {
            drawspace: self.simple_drawables[0].drawspace,
            texture_index: self.simple_drawables[0].texture_index,
            indices_start_offset: 0,
            indices_count: 0,
        };

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
                    indices_start_offset: vertexbuffer.current_offset(),
                    indices_count: 0,
                };
            }

            let (_indices_start_offset, indices_count) = vertexbuffer.push_drawable(drawable);
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
                    self.textures.len(),
                    atlas_page as TextureIndex,
                );
                let atlas_page_bitmap = &self.textures[atlas_page];
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

        let canvas_framebuffer_name = if let Some(canvas_framebuffer) = &self.canvas_framebuffer {
            renderer.framebuffer_create_or_update(
                &canvas_framebuffer.name,
                canvas_framebuffer.width,
                canvas_framebuffer.height,
            );
            &canvas_framebuffer.name
        } else {
            "screen"
        };

        // Clear canvas
        renderer.framebuffer_clear(
            &canvas_framebuffer_name,
            Some(self.current_clear_color.to_slice()),
            Some(self.current_clear_depth),
        );

        // Upload vertexbuffers
        if self.simple_vertexbuffer_dirty {
            let simple_vertexbuffer = self.simple_vertexbuffer.borrow();
            unsafe {
                renderer.assign_buffers(
                    "simple",
                    &transmute_slice_to_byte_slice(&simple_vertexbuffer.vertices),
                    &transmute_slice_to_byte_slice(&simple_vertexbuffer.indices),
                );
            }
        }

        // Draw world- and canvas-space batches
        for world_batch in &self.simple_batches_world {
            renderer.draw(
                "simple",
                &self.simple_shaderparams_world.as_slice(),
                &canvas_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    self.textures.len(),
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
                &canvas_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    self.textures.len(),
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
            let mut rect_screen = BlitRect::new_for_fixed_canvas_size(
                screen_width,
                screen_height,
                canvas_framebuffer.width,
                canvas_framebuffer.height,
            );

            rect_screen.offset_x -= (self.canvas_blit_offset.x
                * (screen_width as f32 / canvas_framebuffer.width as f32))
                as i32;
            rect_screen.offset_y += (self.canvas_blit_offset.y
                * (screen_width as f32 / canvas_framebuffer.width as f32))
                as i32;

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
                    self.textures.len(),
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
