/// NOTE: This is a crude implementation of a software renderer which does not draw correctly
///
use ct_lib::draw::*;
use ct_lib::grid::*;
use ct_lib::math::*;

use std::collections::HashMap;

type Depthbuffer = Grid<f32>;
struct Framebuffer {
    color: Bitmap,
    depth: Depthbuffer,
}

impl Framebuffer {
    fn new(width: u32, height: u32) -> Framebuffer {
        Framebuffer {
            color: Bitmap::new(width, height),
            depth: Depthbuffer::new(width, height),
        }
    }
}

pub struct Renderer {
    textures: HashMap<TextureInfo, Bitmap>,
    framebuffers: HashMap<FramebufferTarget, Framebuffer>,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {
            textures: HashMap::new(),
            framebuffers: HashMap::new(),
        }
    }

    pub fn get_framebuffer_bitmap(&mut self) -> &mut Bitmap {
        &mut self
            .framebuffers
            .get_mut(&FramebufferTarget::Screen)
            .expect("Screen framebuffer does not exist")
            .color
    }

    pub fn process_drawcommands(
        &mut self,
        screen_width: u32,
        screen_height: u32,
        drawcommands: &[Drawcommand],
    ) {
        // Update our screen framebuffer
        {
            let screen_framebuffer = self
                .framebuffers
                .entry(FramebufferTarget::Screen)
                .or_insert(Framebuffer::new(screen_width, screen_height));
            if screen_width != screen_framebuffer.color.width as u32
                || screen_height != screen_framebuffer.color.height as u32
            {
                screen_framebuffer.color = Bitmap::new(screen_width, screen_height);
                screen_framebuffer.depth = Depthbuffer::new(screen_width, screen_height);
            }
        }

        for drawcommand in drawcommands {
            match drawcommand {
                Drawcommand::Draw {
                    framebuffer_target,
                    texture_info,
                    shader,
                    vertexbuffer,
                } => {
                    // TODO: Instead of just borrowing our value we need to take it out
                    //       of the map here to please the borrowchecker. Lets find a better way
                    let mut framebuffer =
                        self.framebuffers
                            .remove(framebuffer_target)
                            .expect(&format!(
                                "No framebuffer found for '{:?}'",
                                framebuffer_target
                            ));
                    let texture = self
                        .textures
                        .remove(texture_info)
                        .expect(&format!("No texture found for '{:?}'", texture_info));

                    match shader {
                        ShaderType::Simple(shader_params) => {
                            assert!(vertexbuffer.indices.len() % 3 == 0);
                            for triangle_indices in vertexbuffer.indices.chunks_exact(3) {
                                let triangle = [
                                    vertexbuffer.vertices[triangle_indices[0] as usize],
                                    vertexbuffer.vertices[triangle_indices[1] as usize],
                                    vertexbuffer.vertices[triangle_indices[2] as usize],
                                ];
                                draw_triangle(
                                    &triangle,
                                    shader_params.texture_color_modulate,
                                    &texture,
                                    &mut framebuffer,
                                );
                            }
                        }
                    }

                    // TODO: Re-insert our 'borrowed' values (see the note above)
                    self.textures.insert(texture_info.clone(), texture);
                    self.framebuffers
                        .insert(framebuffer_target.clone(), framebuffer);
                }
                Drawcommand::TextureCreate(texture_info) => {
                    assert!(
                        !self.textures.contains_key(&texture_info),
                        "A texture already exists for: '{:?}'",
                        texture_info
                    );

                    let bitmap_texture = Bitmap::new(texture_info.width, texture_info.height);
                    self.textures.insert(texture_info.clone(), bitmap_texture);
                }
                Drawcommand::TextureUpdate {
                    texture_info,
                    offset_x,
                    offset_y,
                    bitmap,
                } => {
                    assert!(
                        self.textures.contains_key(&texture_info),
                        "No texture found for '{:?}'",
                        texture_info
                    );

                    let source_bitmap = bitmap;
                    let source_rect =
                        Recti::from_width_height(bitmap.width as i32, bitmap.height as i32);
                    let dest_bitmap = self.textures.get_mut(&texture_info).unwrap();
                    let dest_rect = source_rect.translated_by(Vec2i::new(*offset_x, *offset_y));
                    Bitmap::copy_region(source_bitmap, source_rect, dest_bitmap, dest_rect);
                }
                Drawcommand::TextureFree(texture_info) => {
                    assert!(
                        self.textures.contains_key(&texture_info),
                        "No texture found for '{:?}'",
                        texture_info
                    );
                    self.textures.remove(texture_info);
                }
                Drawcommand::FramebufferCreate(framebuffer_info) => {
                    assert!(
                        !self
                            .framebuffers
                            .contains_key(&FramebufferTarget::Offscreen(framebuffer_info.clone())),
                        "A framebuffer already exists for: '{:?}'",
                        framebuffer_info,
                    );
                    self.framebuffers.insert(
                        FramebufferTarget::Offscreen(framebuffer_info.clone()),
                        Framebuffer::new(framebuffer_info.width, framebuffer_info.height),
                    );
                }
                Drawcommand::FramebufferFree(framebuffer_info) => {
                    assert!(
                        self.framebuffers
                            .contains_key(&FramebufferTarget::Offscreen(framebuffer_info.clone())),
                        "No framebuffer found for '{:?}'",
                        framebuffer_info
                    );
                    self.framebuffers
                        .remove(&FramebufferTarget::Offscreen(framebuffer_info.clone()));
                }
                Drawcommand::FramebufferClear {
                    framebuffer_target,
                    new_color,
                    new_depth,
                } => {
                    let framebuffer =
                        self.framebuffers
                            .get_mut(&framebuffer_target)
                            .expect(&format!(
                                "No framebuffer found for '{:?}'",
                                framebuffer_target
                            ));
                    if let Some(color) = new_color {
                        framebuffer
                            .color
                            .clear(PixelRGBA::from_color(color.clone()));
                    }
                    if let Some(depth) = new_depth {
                        framebuffer.depth.clear(*depth);
                    }
                }
                Drawcommand::FramebufferBlit {
                    source_framebuffer_info,
                    source_rect,
                    dest_framebuffer_target,
                    dest_rect,
                } => {
                    assert!(
                        *dest_framebuffer_target
                            != FramebufferTarget::Offscreen(source_framebuffer_info.clone()),
                        "Cannot blit from and to the same framebuffer '{:?}'",
                        source_framebuffer_info,
                    );

                    // TODO: Instead of just borrowing our value we need to take it out
                    //       of the map here to please the borrowchecker. Lets find a better way
                    let source_framebuffer = self
                        .framebuffers
                        .remove(&FramebufferTarget::Offscreen(
                            source_framebuffer_info.clone(),
                        ))
                        .expect(&format!(
                            "No framebuffer found for '{:?}'",
                            source_framebuffer_info
                        ));
                    let mut dest_framebuffer = self
                        .framebuffers
                        .remove(dest_framebuffer_target)
                        .expect(&format!(
                            "No framebuffer found for '{:?}'",
                            source_framebuffer_info
                        ));
                    let source_rect = Recti::from_xy_width_height(
                        source_rect.offset_x,
                        source_rect.offset_y,
                        source_rect.width,
                        source_rect.height,
                    );
                    let dest_rect = Recti::from_xy_width_height(
                        dest_rect.offset_x,
                        dest_rect.offset_y,
                        dest_rect.width,
                        dest_rect.height,
                    );

                    Bitmap::copy_region_sample_nearest_neighbor(
                        &source_framebuffer.color,
                        source_rect,
                        &mut dest_framebuffer.color,
                        dest_rect,
                    );
                    Depthbuffer::copy_region_sample_nearest_neighbor(
                        &source_framebuffer.depth,
                        source_rect,
                        &mut dest_framebuffer.depth,
                        dest_rect,
                    );

                    // TODO: Re-insert our 'borrowed' values (see the note above)
                    self.framebuffers.insert(
                        FramebufferTarget::Offscreen(source_framebuffer_info.clone()),
                        source_framebuffer,
                    );
                    self.framebuffers
                        .insert(dest_framebuffer_target.clone(), dest_framebuffer);
                }
            }
        }
    }
}

#[inline]
fn draw_triangle(
    triangle: &[Vertex; 3],
    texture_color_modulate: Color,
    source_texture: &Bitmap,
    framebuffer: &mut Framebuffer,
) {
    let triangle_bounds = triangle_get_bounds(
        triangle[0].pos.dropped_z(),
        triangle[1].pos.dropped_z(),
        triangle[2].pos.dropped_z(),
    )
    .clipped_by(Rect::from(framebuffer.color.rect()));

    if let Some(bounds) = triangle_bounds {
        for y in bounds.top as i32..bounds.bottom as i32 {
            for x in bounds.left as i32..bounds.right as i32 {
                let (u, v, w) = triangle_barycentric_2d(
                    Vec2::new(x as f32, y as f32),
                    triangle[0].pos.dropped_z(),
                    triangle[1].pos.dropped_z(),
                    triangle[2].pos.dropped_z(),
                );
                if u < -EPSILON || v < -EPSILON || w < -EPSILON {
                    continue;
                }
                assert!(v <= 1.0 && u <= 1.0 && w <= 1.0);

                let vertex_depth =
                    u * triangle[0].pos.z + v * triangle[1].pos.z + w * triangle[2].pos.z;

                if vertex_depth < framebuffer.depth.get(x, y) {
                    continue;
                }

                let vertex_color =
                    u * triangle[0].color + v * triangle[1].color + w * triangle[2].color;
                let vertex_additivity = u * triangle[0].additivity
                    + v * triangle[1].additivity
                    + w * triangle[2].additivity;

                let texel_uv = u * triangle[0].uv + v * triangle[1].uv + w * triangle[2].uv;
                let texel_x = roundi(texel_uv.x * source_texture.width as f32);
                let texel_y = roundi(texel_uv.y * source_texture.height as f32);
                let mut tex_color = Color::from_pixelrgba(source_texture.get(texel_x, texel_y));
                tex_color = tex_color * texture_color_modulate;

                // Based on
                // http://amindforeverprogramming.blogspot.com/2013/07/why-alpha-premultiplied-colour-blending.html
                //
                let color = Color::new(
                    tex_color.r * vertex_color.r,
                    tex_color.g * vertex_color.g,
                    tex_color.b * vertex_color.b,
                    tex_color.a * vertex_color.a * (1.0 - vertex_additivity),
                );

                if Color::dot(color, color) == 0.0 {
                    // NOTE: We assume pre-multiplied colors, therefore a fully transparent pixel requires that
                    //       all channels being zero
                    continue;
                }

                let dest_color = Color::from_pixelrgba(framebuffer.color.get(x, y));
                let source_color = color;

                let final_color = source_color + (dest_color * (1.0 - source_color.a));
                framebuffer
                    .color
                    .set(x, y, PixelRGBA::from_color(final_color));
                framebuffer.depth.set(x, y, vertex_depth);
            }
        }
    }
}
