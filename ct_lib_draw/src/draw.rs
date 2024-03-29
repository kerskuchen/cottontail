use ct_lib_window::renderer_opengl::Renderer;

use super::image::bitmap::*;
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
        super::core::transmute_to_slice(self)
    }
}

#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
struct VertexDefault {
    pub pos: Vec3,
    pub uv: Vec2,
    pub color: Color,
    pub additivity: Additivity,
}
impl Vertex for VertexDefault {}

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
pub enum Drawspace {
    World,
    Canvas,
    Screen,
}

impl Default for Drawspace {
    fn default() -> Self {
        Drawspace::World
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
        vertices: Vec<VertexDefault>,
        indices: Vec<VertexIndex>,
    },
}

#[derive(Clone)]
struct Drawable {
    pub texture_index: TextureIndex,
    pub uv_region_contains_translucency: bool,
    pub drawparams: Drawparams,
    pub geometry: Geometry,
}

impl Drawable {
    pub fn is_translucent(&self) -> bool {
        self.uv_region_contains_translucency
            || (self.drawparams.color_modulate.a < 1.0)
            || (self.drawparams.additivity != ADDITIVITY_NONE)
    }

    #[inline]
    pub fn compare(a: &Drawable, b: &Drawable) -> Ordering {
        let a_has_translucency = a.is_translucent();
        let b_has_translucency = b.is_translucent();

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
        if a.drawparams.depth < b.drawparams.depth {
            return Ordering::Less;
        } else if a.drawparams.depth > b.drawparams.depth {
            return Ordering::Greater;
        }

        Ordering::Equal
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertexbuffers

type VertexbufferDefault = Vertexbuffer<VertexDefault>;

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

impl VertexbufferDefault {
    /// Returns index count of pushed object
    pub fn push_drawable(&mut self, drawable: Drawable) -> usize {
        let depth = drawable.drawparams.depth;
        let color = drawable.drawparams.color_modulate;
        let additivity = drawable.drawparams.additivity;
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
                self.vertices.push(VertexDefault {
                    pos: Vec3::from_vec2(quad.vert_right_top, depth),
                    uv: Vec2::new(uvs.right, uvs.top),
                    color,
                    additivity,
                });
                // right bottom
                self.vertices.push(VertexDefault {
                    pos: Vec3::from_vec2(quad.vert_right_bottom, depth),
                    uv: Vec2::new(uvs.right, uvs.bottom),
                    color,
                    additivity,
                });
                // left bottom
                self.vertices.push(VertexDefault {
                    pos: Vec3::from_vec2(quad.vert_left_bottom, depth),
                    uv: Vec2::new(uvs.left, uvs.bottom),
                    color,
                    additivity,
                });
                // left top
                self.vertices.push(VertexDefault {
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
                    self.vertices.push(VertexDefault {
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
        transmute_to_slice(self)
    }
}
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct ShaderParamsDefault {
    pub transform: Mat4,
    pub texture_color_modulate: Color,
}
impl UniformBlock for ShaderParamsDefault {}

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
pub struct Drawparams {
    pub depth: Depth,
    pub color_modulate: Color,
    pub additivity: Additivity,
    pub drawspace: Drawspace,
}

impl Default for Drawparams {
    #[inline]
    fn default() -> Self {
        Drawparams {
            depth: 0.0,
            color_modulate: Color::white(),
            additivity: ADDITIVITY_NONE,
            drawspace: Drawspace::World,
        }
    }
}

impl Drawparams {
    #[inline]
    pub fn new(
        depth: Depth,
        color_modulate: Color,
        additivity: Additivity,
        drawspace: Drawspace,
    ) -> Drawparams {
        Drawparams {
            depth,
            color_modulate,
            additivity,
            drawspace,
        }
    }

    #[inline]
    pub fn with_depth(depth: Depth) -> Drawparams {
        Drawparams {
            depth,
            ..Drawparams::default()
        }
    }
    #[inline]
    pub fn with_debug_depth(
        color_modulate: Color,
        additivity: Additivity,
        drawspace: Drawspace,
    ) -> Drawparams {
        Drawparams {
            depth: DEPTH_MAX,
            color_modulate,
            additivity,
            drawspace,
        }
    }

    #[inline]
    pub fn with_depth_color(depth: Depth, color_modulate: Color) -> Drawparams {
        Drawparams {
            depth,
            color_modulate,
            ..Drawparams::default()
        }
    }

    #[inline]
    pub fn with_depth_drawspace(depth: Depth, drawspace: Drawspace) -> Drawparams {
        Drawparams {
            depth,
            drawspace,
            ..Drawparams::default()
        }
    }

    #[inline]
    pub fn without_additivity(
        depth: Depth,
        color_modulate: Color,
        drawspace: Drawspace,
    ) -> Drawparams {
        Drawparams {
            depth,
            color_modulate,
            drawspace,
            additivity: ADDITIVITY_NONE,
        }
    }
}

#[derive(Clone)]
struct DrawBatch {
    pub drawspace: Drawspace,
    pub texture_index: TextureIndex,
    pub is_translucent: bool,
    pub indices_start_offset: VertexIndex,
    pub indices_count: usize,
}

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

    default_drawables: Vec<Drawable>,
    default_drawables_translucent: Vec<Drawable>,
    default_shaderparams_world: ShaderParamsDefault,
    default_shaderparams_canvas: ShaderParamsDefault,
    default_shaderparams_screen: ShaderParamsDefault,
    default_batches_world: Vec<DrawBatch>,
    default_batches_canvas: Vec<DrawBatch>,
    default_batches_screen: Vec<DrawBatch>,
    default_vertexbuffer: VertexbufferDefault,
    default_vertexbuffer_dirty: bool,

    canvas_framebuffer: Option<FramebufferInfo>,

    debug_use_flat_color_mode: bool,
    debug_draw_depth: bool,
}

//--------------------------------------------------------------------------------------------------
// Creation and configuration

impl Drawstate {
    pub fn new() -> Drawstate {
        // Create a dummy texture and reserve a white pixel for special usage
        let textures = vec![Rc::new(RefCell::new(Bitmap::new_filled(
            1,
            1,
            PixelRGBA::white(),
        )))];
        let textures_dirty = vec![true; textures.len()];
        let untextured_uv_center_coord = AAQuad {
            left: 1.0,
            top: 1.0,
            right: 1.0,
            bottom: 1.0,
        };
        let untextured_uv_center_atlas_page = 0;

        Drawstate {
            textures,
            textures_size: 1,
            textures_dirty,

            untextured_uv_center_coord,
            untextured_uv_center_atlas_page,

            current_letterbox_color: Color::black(),
            current_clear_color: Color::black(),
            current_clear_depth: DEPTH_CLEAR,

            default_drawables: Vec::new(),
            default_drawables_translucent: Vec::new(),
            default_shaderparams_world: ShaderParamsDefault::default(),
            default_shaderparams_canvas: ShaderParamsDefault::default(),
            default_shaderparams_screen: ShaderParamsDefault::default(),
            default_batches_world: Vec::new(),
            default_batches_canvas: Vec::new(),
            default_batches_screen: Vec::new(),
            default_vertexbuffer: VertexbufferDefault::new(),
            default_vertexbuffer_dirty: true,

            canvas_framebuffer: None,

            debug_use_flat_color_mode: false,
            debug_draw_depth: false,
        }
    }

    pub fn assign_textures(&mut self, textures: Vec<Rc<RefCell<Bitmap>>>) {
        let textures_size = textures
            .first()
            .expect("Drawstate: No Textures given")
            .borrow()
            .width as u32;

        // NOTE: We assume that every texture has a white pixel in its bottom-right corner
        for texture in &textures {
            let texture = texture.borrow();
            assert!(
                texture.get(texture.width - 1, texture.height - 1) == PixelRGBA::white(),
                "Last pixel in first texture must be 0xFFFFFFFF"
            );
        }

        let textures_dirty = vec![true; textures.len()];

        self.textures = textures;
        self.textures_size = textures_size;
        self.textures_dirty = textures_dirty;
    }

    fn texturename_for_atlaspage(textures_size: u32, page_index: TextureIndex) -> String {
        format!(
            "atlas_page_{}__{}x{}",
            page_index, textures_size, textures_size
        )
    }

    pub fn set_shaderparams_default(
        &mut self,
        color_modulate: Color,
        transform_world: Mat4,
        transform_canvas: Mat4,
        transform_screen: Mat4,
    ) {
        self.default_shaderparams_world.texture_color_modulate = color_modulate;
        self.default_shaderparams_world.transform = transform_world;

        self.default_shaderparams_canvas.texture_color_modulate = color_modulate;
        self.default_shaderparams_canvas.transform = transform_canvas;

        self.default_shaderparams_screen.texture_color_modulate = color_modulate;
        self.default_shaderparams_screen.transform = transform_screen;
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

    pub fn set_canvas_dimensions(&mut self, width: u32, height: u32) {
        assert!(width > 0);
        assert!(height > 0);
        self.canvas_framebuffer = Some(FramebufferInfo {
            name: FRAMEBUFFER_NAME_CANVAS.to_owned(),
            width,
            height,
        });
    }

    pub fn debug_enable_flat_color_mode(&mut self, enable: bool) {
        self.debug_use_flat_color_mode = enable;
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Beginning and ending frames

    pub fn begin_frame(&mut self) {
        self.default_drawables.clear();
        self.default_drawables_translucent.clear();
    }

    pub fn finish_frame(&mut self) {
        self.default_batches_world.clear();
        self.default_batches_canvas.clear();
        self.default_batches_screen.clear();

        if self.default_drawables.is_empty() && self.default_drawables_translucent.is_empty() {
            return;
        }

        self.default_vertexbuffer.clear();
        self.default_vertexbuffer_dirty = true;

        if !self.default_drawables.is_empty() {
            Drawstate::collect_drawbatches_default(
                &mut self.default_drawables,
                &mut self.default_batches_world,
                &mut self.default_batches_canvas,
                &mut self.default_batches_screen,
                &mut self.default_vertexbuffer,
                false,
            );
        }

        if !self.default_drawables_translucent.is_empty() {
            self.default_drawables_translucent
                .sort_by(Drawable::compare);

            Drawstate::collect_drawbatches_default(
                &mut self.default_drawables_translucent,
                &mut self.default_batches_world,
                &mut self.default_batches_canvas,
                &mut self.default_batches_screen,
                &mut self.default_vertexbuffer,
                true,
            );
        }
    }

    fn collect_drawbatches_default(
        drawables: &mut Vec<Drawable>,
        batches_world: &mut Vec<DrawBatch>,
        batches_canvas: &mut Vec<DrawBatch>,
        batches_screen: &mut Vec<DrawBatch>,
        vertexbuffer: &mut VertexbufferDefault,
        is_translucent: bool,
    ) {
        assert!(!drawables.is_empty());

        let mut current_batch = DrawBatch {
            drawspace: drawables[0].drawparams.drawspace,
            texture_index: drawables[0].texture_index,
            indices_start_offset: vertexbuffer.current_offset(),
            indices_count: 0,
            is_translucent,
        };

        for drawable in drawables.drain(..) {
            if drawable.texture_index != current_batch.texture_index
                || drawable.drawparams.drawspace != current_batch.drawspace
            {
                match current_batch.drawspace {
                    Drawspace::World => batches_world.push(current_batch),
                    Drawspace::Canvas => batches_canvas.push(current_batch),
                    Drawspace::Screen => batches_screen.push(current_batch),
                }
                current_batch = DrawBatch {
                    drawspace: drawable.drawparams.drawspace,
                    texture_index: drawable.texture_index,
                    indices_start_offset: vertexbuffer.current_offset(),
                    indices_count: 0,
                    is_translucent,
                };
            }

            let indices_count = vertexbuffer.push_drawable(drawable);
            current_batch.indices_count += indices_count;
        }

        match current_batch.drawspace {
            Drawspace::World => batches_world.push(current_batch),
            Drawspace::Canvas => batches_canvas.push(current_batch),
            Drawspace::Screen => batches_screen.push(current_batch),
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
            "main",
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
            "main"
        };

        // Upload vertexbuffers
        if self.default_vertexbuffer_dirty {
            renderer.assign_buffers(
                "default",
                &transmute_slice_to_byte_slice(&self.default_vertexbuffer.vertices),
                &transmute_slice_to_byte_slice(&self.default_vertexbuffer.indices),
            );
            self.default_vertexbuffer_dirty = false;
        }

        // Draw world- and canvas-space batches
        for world_batch in &self.default_batches_world {
            renderer.draw(
                "default",
                &self.default_shaderparams_world.as_slice(),
                &draw_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    world_batch.texture_index,
                ),
                world_batch.indices_start_offset,
                world_batch.indices_count,
                !world_batch.is_translucent,
            );
        }
        for canvas_batch in &self.default_batches_canvas {
            renderer.draw(
                "default",
                &self.default_shaderparams_canvas.as_slice(),
                &draw_framebuffer_name,
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    canvas_batch.texture_index,
                ),
                canvas_batch.indices_start_offset,
                canvas_batch.indices_count,
                !canvas_batch.is_translucent,
            );
        }

        // If we drew to an offscreen-canvas we must blit it back to the screen
        if let Some(canvas_framebuffer) = &self.canvas_framebuffer {
            if self.debug_draw_depth {
                renderer.debug_draw_depthbuffer(&canvas_framebuffer.name);
            }

            let (screen_width, screen_height) = renderer.get_main_framebuffer_dimensions();

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
                "main",
                rect_canvas.to_recti(),
                rect_screen.to_recti(),
            );
        }

        // Draw screenspace batches last so they won't get overdrawn by framebuffer blits
        for screen_batch in &self.default_batches_screen {
            renderer.draw(
                "default",
                &self.default_shaderparams_screen.as_slice(),
                "main",
                &Drawstate::texturename_for_atlaspage(
                    self.textures_size,
                    screen_batch.texture_index,
                ),
                screen_batch.indices_start_offset,
                screen_batch.indices_count,
                !screen_batch.is_translucent,
            );
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Drawing

    fn push_drawable(&mut self, drawable: Drawable) {
        if drawable.is_translucent() {
            self.default_drawables_translucent.push(drawable);
        } else {
            self.default_drawables.push(drawable);
        }
    }

    #[inline]
    pub fn draw_quad(
        &mut self,
        quad: &Quad,
        uvs: AAQuad,
        uv_region_contains_translucency: bool,
        texture_index: TextureIndex,
        drawparams: Drawparams,
    ) {
        if !self.debug_use_flat_color_mode {
            self.push_drawable(Drawable {
                texture_index,
                uv_region_contains_translucency,
                drawparams,
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

            self.push_drawable(Drawable {
                texture_index,
                uv_region_contains_translucency,
                drawparams,
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
        drawparams: Drawparams,
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
            drawparams,
        );
    }

    #[inline]
    pub fn draw_sprite_clipped(
        &mut self,
        sprite: &Sprite,
        pos: Vec2,
        scale: Vec2,
        clipping_rect: Rect,
        drawparams: Drawparams,
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
                self.draw_quad(&quad, uvs, has_translucency, atlas_page, drawparams);
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
                    drawparams,
                );
            }
        }
    }

    #[inline]
    pub fn draw_sprite_3d(&mut self, sprite: &Sprite3D, xform: Transform, drawparams: Drawparams) {
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
                Drawparams {
                    depth: drawparams.depth - (index as f32) * 0.5 * depth_increment,
                    ..drawparams
                },
            );
        }
    }

    //----------------------------------------------------------------------------------------------
    // Primitive drawing

    /// This fills the following pixels:
    /// [left, right[ x [top, bottom[
    #[inline]
    pub fn draw_rect(&mut self, rect: Rect, filled: bool, drawparams: Drawparams) {
        let rect = rect.pixel_snapped();
        if filled {
            let quad = Quad::from_rect(rect);
            self.draw_quad(
                &quad,
                self.untextured_uv_center_coord,
                false,
                self.untextured_uv_center_atlas_page,
                drawparams,
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
            self.draw_linestrip_bresenham(&linestrip, true, drawparams);
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
        drawparams: Drawparams,
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
                drawparams,
            );
        } else {
            let linestrip = quad.to_linestrip();
            self.draw_linestrip_bresenham(&linestrip, true, drawparams);
        }
    }

    /// Expects vertices in the form [v_a0, v_a1, v_a2, v_b0, v_b1, v_b2, ...]
    #[inline]
    pub fn draw_polygon(
        &mut self,
        vertices: &[Vec2],
        pivot: Vec2,
        xform: Transform,
        drawparams: Drawparams,
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

        self.push_drawable(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: false,
            drawparams,
            geometry: Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            },
        });
    }

    #[inline]
    pub fn draw_circle_filled(&mut self, center: Vec2, radius: f32, drawparams: Drawparams) {
        if radius < 0.5 {
            self.draw_pixel(center, drawparams);
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

        self.push_drawable(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: false,
            drawparams,
            geometry: Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            },
        });
    }

    #[inline]
    pub fn draw_circle_bresenham(&mut self, center: Vec2, radius: f32, drawparams: Drawparams) {
        // Based on the Paper "A Fast Bresenham Type Algorithm For Drawing Circles" by John Kennedy
        // https://web.engr.oregonstate.edu/~sllu/bcircle.pdf

        let center = center.pixel_snapped();
        let radius = roundi(radius);

        if radius == 0 {
            self.draw_pixel(center, drawparams);
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

            self.draw_pixel(center + octant1, drawparams);
            self.draw_pixel(center + octant4, drawparams);
            self.draw_pixel(center + octant5, drawparams);
            self.draw_pixel(center + octant8, drawparams);

            // NOTE: For x == y the below points have been already drawn in octants 1,4,5,8
            if x != y {
                let octant2 = Vec2::new(y as f32, x as f32);
                let octant3 = Vec2::new(-y as f32, x as f32);
                let octant6 = Vec2::new(-y as f32, -x as f32);
                let octant7 = Vec2::new(y as f32, -x as f32);

                self.draw_pixel(center + octant2, drawparams);
                self.draw_pixel(center + octant3, drawparams);
                self.draw_pixel(center + octant6, drawparams);
                self.draw_pixel(center + octant7, drawparams);
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
        drawparams: Drawparams,
    ) {
        // Based on the Paper "A Fast Bresenham Type AlgorithmFor Drawing Ellipses"
        // https://dai.fmph.uniba.sk/upload/0/01/Ellipse.pdf
        let center = center.pixel_snapped();
        let radius_x = roundi(radius_x);
        let radius_y = roundi(radius_y);

        if radius_x == 0 || radius_y == 0 {
            self.draw_pixel(center, drawparams);
            return;
        }

        todo!()
    }

    #[inline]
    pub fn draw_ring(&mut self, center: Vec2, radius: f32, thickness: f32, drawparams: Drawparams) {
        if radius < 0.5 {
            self.draw_pixel(center, drawparams);
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
                drawparams,
            );

            angle_current += angle_increment;
            last_unit_circle_point = unit_circle_point;
        }
    }

    /// WARNING: This can be slow if used often
    #[inline]
    pub fn draw_pixel(&mut self, pos: Vec2, drawparams: Drawparams) {
        self.draw_rect(Rect::from_pos_dim(pos, Vec2::ones()), true, drawparams);
    }

    /// WARNING: This can be slow if used often
    /// NOTE: Skipping the last pixel is useful i.e. for drawing translucent line loops which start
    ///       and end on the same pixel and pixels must not overlap
    #[inline]
    pub fn draw_linestrip_bresenham(
        &mut self,
        points: &[Vec2],
        skip_last_pixel: bool,
        drawparams: Drawparams,
    ) {
        for pair in points.windows(2) {
            self.draw_line_bresenham(pair[0], pair[1], true, drawparams);
        }
        if !skip_last_pixel && !points.is_empty() {
            self.draw_pixel(*points.last().unwrap(), drawparams);
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
        drawparams: Drawparams,
    ) {
        let start = start.pixel_snapped().to_i32();
        let end = end.pixel_snapped().to_i32();
        iterate_line_bresenham(start, end, skip_last_pixel, &mut |x, y| {
            self.draw_pixel(Vec2::new(x as f32, y as f32), drawparams)
        });
    }

    #[inline]
    pub fn draw_line_with_thickness(
        &mut self,
        start: Vec2,
        end: Vec2,
        thickness: f32,
        smooth_edges: bool,
        drawparams: Drawparams,
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

        let color = drawparams.color_modulate;
        let depth = drawparams.depth;
        let additivity = drawparams.additivity;
        let color_edges = if smooth_edges {
            Color::transparent()
        } else {
            drawparams.color_modulate
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
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_right.vert_right_top, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // right bottom
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_right.vert_right_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left bottom
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_right.vert_left_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left top
        vertices.push(VertexDefault {
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
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_left.vert_right_top, depth),
            uv,
            color,
            additivity,
        });
        // right bottom
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_left.vert_right_bottom, depth),
            uv,
            color,
            additivity,
        });
        // left bottom
        vertices.push(VertexDefault {
            pos: Vec3::from_vec2(quad_left.vert_left_bottom, depth),
            uv,
            color: color_edges,
            additivity,
        });
        // left top
        vertices.push(VertexDefault {
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

        self.push_drawable(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: true,
            drawparams: Drawparams {
                // NOTE: We already set the vertex colors above, we don't need to modulate them
                //       anymore, so we set it to white
                color_modulate: Color::white(),
                ..drawparams
            },
            geometry: Geometry::LineMesh { vertices, indices },
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
        drawparams: Drawparams,
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
                        Drawparams {
                            color_modulate: color_background,
                            ..drawparams
                        },
                    );

                    // Draw glyph
                    self.draw_sprite(
                        &glyph.sprite,
                        Transform::from_pos_scale_uniform(draw_pos.into(), font_scale),
                        false,
                        false,
                        drawparams,
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
                        drawparams,
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
        drawparams: Drawparams,
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
                    drawparams,
                );
            },
        )
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Debug Drawing

    #[inline]
    pub fn debug_draw_checkerboard(
        &mut self,
        origin: Vec2,
        cells_per_side: usize,
        cell_size: f32,
        drawparams: Drawparams,
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
                        Drawparams {
                            color_modulate: if x % 2 == 0 {
                                drawparams.color_modulate
                            } else {
                                drawparams.color_modulate * 0.5
                            },
                            ..drawparams
                        },
                    );
                } else {
                    self.draw_rect(
                        cell_rect,
                        true,
                        Drawparams {
                            color_modulate: if x % 2 == 0 {
                                drawparams.color_modulate * 0.5
                            } else {
                                drawparams.color_modulate
                            },
                            ..drawparams
                        },
                    );
                }
            }
        }
    }

    #[inline]
    pub fn debug_draw_arrow(&mut self, start: Vec2, dir: Vec2, drawparams: Drawparams) {
        let end = start + dir;
        self.draw_line_bresenham(start, end, false, drawparams);

        let size = f32::clamp(dir.magnitude() / 10.0, 1.0, 5.0);
        let perp_left = size * (end - start).perpendicular().normalized();
        let perp_right = -perp_left;

        let point_tip = end;
        let point_stump = end - size * dir.normalized();
        let point_left = point_stump + perp_left;
        let point_right = point_stump + perp_right;
        self.debug_draw_triangle(point_tip, point_left, point_right, drawparams);
    }

    #[inline]
    pub fn debug_draw_arrow_line(&mut self, start: Vec2, end: Vec2, drawparams: Drawparams) {
        let dir = end - start;
        self.debug_draw_arrow(start, dir, drawparams);
    }

    #[inline]
    pub fn debug_draw_triangle(
        &mut self,
        point_a: Vec2,
        point_b: Vec2,
        point_c: Vec2,
        drawparams: Drawparams,
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

        self.push_drawable(Drawable {
            texture_index: self.untextured_uv_center_atlas_page,
            uv_region_contains_translucency: false,
            drawparams,
            geometry: Geometry::PolygonMesh {
                vertices,
                uvs,
                indices,
            },
        });
    }
}
