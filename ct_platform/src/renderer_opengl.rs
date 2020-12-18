use ct_lib::draw::*;
use ct_lib::draw_common::*;
use ct_lib::math::*;

use ct_lib::log;

use glow::*;

use std::collections::HashMap;

type GLProgramId = <glow::Context as glow::HasContext>::Program;
type GLTextureId = <glow::Context as glow::HasContext>::Texture;
type GLFramebufferId = <glow::Context as glow::HasContext>::Framebuffer;
type GLRenderbufferId = <glow::Context as glow::HasContext>::Renderbuffer;
type GLUniformLocation = <glow::Context as glow::HasContext>::UniformLocation;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Error checking

fn gl_state_ok(gl: &glow::Context) -> bool {
    let mut is_ok = true;

    loop {
        let error = unsafe { gl.get_error() };
        if error == glow::NO_ERROR {
            break;
        } else {
            log::warn!("OpenGL error: {}", gl_error_string(error));
            is_ok = false;
        }
    }

    return is_ok;
}

fn gl_error_string(error: u32) -> String {
    if error == glow::NO_ERROR {
        return "NO_ERROR".to_owned();
    } else if error == glow::INVALID_ENUM {
        return "INVALID_ENUM".to_owned();
    } else if error == glow::INVALID_VALUE {
        return "INVALID_VALUE".to_owned();
    } else if error == glow::INVALID_OPERATION {
        return "INVALID_OPERATION".to_owned();
    } else if error == glow::STACK_OVERFLOW {
        return "STACK_OVERFLOW".to_owned();
    } else if error == glow::STACK_UNDERFLOW {
        return "STACK_UNDERFLOW".to_owned();
    } else if error == glow::OUT_OF_MEMORY {
        return "OUT_OF_MEMORY".to_owned();
    } else if error == glow::INVALID_FRAMEBUFFER_OPERATION {
        return "INVALID_FRAMEBUFFER_OPERATION".to_owned();
    }

    panic!("Got unknown GL error {:X}", error);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Creating shaders from sourcecode

struct GLProgram {
    pub id: GLProgramId,
}

fn gl_program_create(
    gl: &glow::Context,
    vertex_shader_source: &str,
    fragment_shader_source: &str,
) -> GLProgram {
    let program = unsafe {
        // Vertex shader
        let vertex_shader = gl
            .create_shader(glow::VERTEX_SHADER)
            .expect("Cannot create vertex shader");
        gl.shader_source(vertex_shader, vertex_shader_source);
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!(
                "Failed to compile vertex shader:\n{}",
                gl.get_shader_info_log(vertex_shader)
            );
        }

        // Fragment shader
        let fragment_shader = gl
            .create_shader(glow::FRAGMENT_SHADER)
            .expect("Cannot create fragment shader");
        gl.shader_source(fragment_shader, fragment_shader_source);
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!(
                "Failed to compile fragment shader:\n{}",
                gl.get_shader_info_log(fragment_shader)
            );
        }

        // Program
        let program = gl.create_program().expect("Cannot create shader program");
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            panic!(
                "Failed to link shader program:\n{}",
                gl.get_program_info_log(program)
            );
        }

        // Program is successfully compiled and linked - we don't need the shaders anymore
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        program
    };

    assert!(gl_state_ok(gl), "Could not compile shader program");
    log::info!("Shaderprogram {} successfully compiled", program);

    GLProgram { id: program }
}

fn gl_program_enable(gl: &glow::Context, program: &GLProgram) {
    unsafe {
        gl.use_program(Some(program.id));
    }
}

fn gl_program_delete(gl: &glow::Context, program: GLProgram) {
    unsafe {
        gl.use_program(None);
        gl.delete_program(program.id);
    }
}

fn gl_program_get_attribute_location(
    gl: &glow::Context,
    program: &GLProgram,
    attribute_name: &str,
) -> u32 {
    unsafe {
        gl.get_attrib_location(program.id, attribute_name)
            .expect(&format!(
                "Program {:?} has no attribute '{}'",
                program.id, attribute_name
            ))
    }
}

fn gl_program_get_uniform_location(
    gl: &glow::Context,
    program: &GLProgram,
    uniform_name: &str,
) -> GLUniformLocation {
    unsafe {
        gl.get_uniform_location(program.id, uniform_name)
            .expect(&format!(
                "Program {:?} has no uniform '{}'",
                program.id, uniform_name
            ))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Creating textures from pixelbuffers

struct GLTexture {
    pub id: GLTextureId,
    pub width: u32,
    pub height: u32,
}

fn gl_texture_create(gl: &glow::Context, width: u32, height: u32) -> GLTexture {
    let texture = unsafe {
        let texture = gl.create_texture().expect("Cannot create texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            None,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        texture
    };

    GLTexture {
        id: texture,
        width,
        height,
    }
}

fn gl_texture_update(
    gl: &glow::Context,
    texture: &GLTexture,
    offset_x: u32,
    offset_y: u32,
    region_width: u32,
    region_height: u32,
    pixels: &[PixelRGBA],
) {
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            offset_x as i32,
            offset_y as i32,
            region_width as i32,
            region_height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(std::mem::transmute::<&[PixelRGBA], &[u8]>(pixels)),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

fn gl_texture_delete(gl: &glow::Context, texture: GLTexture) {
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.delete_texture(texture.id);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Creating depthbuffer

struct GLDepthbuffer {
    pub id: GLRenderbufferId,
    pub width: u32,
    pub height: u32,
}

fn gl_depthbuffer_create(gl: &glow::Context, width: u32, height: u32) -> GLDepthbuffer {
    let depth = unsafe {
        let depth = gl
            .create_renderbuffer()
            .expect("Cannot create renderbuffer");
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
        gl.renderbuffer_storage(
            glow::RENDERBUFFER,
            glow::DEPTH_COMPONENT16,
            width as i32,
            height as i32,
        );
        gl.bind_renderbuffer(glow::RENDERBUFFER, None);

        depth
    };

    GLDepthbuffer {
        id: depth,
        width,
        height,
    }
}

fn gl_depthbuffer_delete(gl: &glow::Context, depthbuffer: GLDepthbuffer) {
    unsafe {
        gl.bind_renderbuffer(glow::RENDERBUFFER, None);
        gl.delete_renderbuffer(depthbuffer.id);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// General purpose offscreen-framebuffers

struct GLFramebuffer {
    framebuffer_object: Option<GLFramebufferId>,
    color: Option<GLTexture>,
    depth: Option<GLDepthbuffer>,
    width: u32,
    height: u32,
}

fn gl_framebuffer_screen(width: u32, height: u32) -> GLFramebuffer {
    GLFramebuffer {
        framebuffer_object: None,
        color: None,
        depth: None,
        width,
        height,
    }
}

fn gl_framebuffer_create(gl: &glow::Context, width: u32, height: u32) -> GLFramebuffer {
    unsafe {
        // The color texture
        let color = gl_texture_create(gl, width, height);
        let depth = gl_depthbuffer_create(gl, width, height);

        // Create offscreen framebuffer
        let framebuffer = gl.create_framebuffer().expect("Cannot create framebuffer");
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        // Attach color and depth buffers
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(color.id),
            0,
        );
        gl.framebuffer_renderbuffer(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::RENDERBUFFER,
            Some(depth.id),
        );

        assert!(gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        GLFramebuffer {
            framebuffer_object: Some(framebuffer),
            color: Some(color),
            depth: Some(depth),
            width,
            height,
        }
    }
}

fn gl_framebuffer_delete(gl: &glow::Context, framebuffer: GLFramebuffer) {
    if let Some(framebuffer_object) = framebuffer.framebuffer_object {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer_object);
        }
    }
    if let Some(color) = framebuffer.color {
        gl_texture_delete(gl, color);
    }
    if let Some(depth) = framebuffer.depth {
        gl_depthbuffer_delete(gl, depth);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawobjects

struct GLDrawobject {
    vertex_array: u32,
    vertex_buffer: u32,
    index_buffer: u32,
}

fn gl_drawobject_create(
    gl: &glow::Context,
    vertex_type: &VertexAttributeConfiguration,
) -> GLDrawobject {
    let (vertex_array, vertex_buffer, index_buffer) = unsafe {
        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));

        let vertex_buffer = gl.create_buffer().expect("Cannot create vertex buffer");
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

        let index_buffer = gl.create_buffer().expect("Cannot create index buffer");
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));

        // Assing attributes
        let stride = vertex_type.size_in_bytes;
        for attribute in &vertex_type.attributes {
            let index = attribute.location;
            let size = attribute.float_component_count;
            let offset = attribute.byte_offset_in_vertex;
            let normalized = false;

            gl.enable_vertex_attrib_array(index);
            gl.vertex_attrib_pointer_f32(index, size, glow::FLOAT, normalized, stride, offset);
        }

        // NOTE: The buffers must not be unbound before the vertex array, so the order here is important
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);

        (vertex_array, vertex_buffer, index_buffer)
    };

    GLDrawobject {
        vertex_array,
        vertex_buffer,
        index_buffer,
    }
}

fn gl_drawobject_draw<VertexType: Default + Clone + Copy>(
    gl: &glow::Context,
    object: &GLDrawobject,
    vertexbuffer: &Vertexbuffer<VertexType>,
) {
    unsafe {
        // NOTE: We use buffer object streaming as described in
        //       https://www.khronos.org/opengl/wiki/Buffer_Object_Streaming#Buffer_re-specification
        //       Because of that we call glBufferData with a NULL first

        // Vertices
        let vertices_raw = ct_lib::transmute_to_byte_slice(&vertexbuffer.vertices);
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(object.vertex_buffer));
        gl.buffer_data_size(
            glow::ARRAY_BUFFER,
            vertices_raw.len() as i32,
            glow::STREAM_DRAW,
        );
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices_raw, glow::STREAM_DRAW);

        // Indices
        let indices_raw = ct_lib::transmute_to_byte_slice(&vertexbuffer.indices);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(object.index_buffer));
        gl.buffer_data_size(
            glow::ELEMENT_ARRAY_BUFFER,
            indices_raw.len() as i32,
            glow::STREAM_DRAW,
        );
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_raw, glow::STREAM_DRAW);

        // Draw
        gl.bind_vertex_array(Some(object.vertex_array));
        gl.draw_elements(
            glow::TRIANGLES,
            vertexbuffer.indices.len() as i32,
            glow::UNSIGNED_INT,
            0,
        );
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
    }
}

fn gl_drawobject_free(gl: &glow::Context, object: GLDrawobject) {
    unsafe {
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);

        gl.delete_vertex_array(object.vertex_array);
        gl.delete_buffer(object.vertex_buffer);
        gl.delete_buffer(object.index_buffer);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Vertex Attributes

struct VertexAttribute {
    pub name: String,
    pub location: u32,
    pub float_component_count: i32,
    pub byte_offset_in_vertex: i32,
}

struct VertexAttributeConfiguration {
    pub size_in_bytes: i32,
    pub attributes: Vec<VertexAttribute>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shader Simple

const VERTEX_SHADER_SOURCE_SIMPLE: &str = r#"
attribute vec3 a_pos;
attribute vec2 a_uv;
attribute vec4 a_color;
attribute float a_additivity;

uniform mat4 u_transform;

varying vec4 v_color;
varying vec2 v_uv;
varying float v_additivity;

void main()
{
    gl_Position = u_transform * vec4(a_pos, 1.0);
    v_color = a_color;
    v_uv = a_uv;
    v_additivity = a_additivity;
}
"#;

const FRAGMENT_SHADER_SOURCE_SIMPLE: &str = r#"
precision mediump float;

varying vec4 v_color;
varying vec2 v_uv;
varying float v_additivity;

uniform vec4 u_texture_color_modulate;

uniform sampler2D u_texture;

void main()
{
    vec4 tex_color = texture2D(u_texture, vec2(v_uv.x, v_uv.y));
    tex_color = tex_color * u_texture_color_modulate;

    // Premultiplied-Alpha color-blending based on
    // http://amindforeverprogramming.blogspot.com/2013/07/why-alpha-premultiplied-colour-blending.html
    //
    vec4 color = vec4(tex_color.r * v_color.r,
                      tex_color.g * v_color.g,
                      tex_color.b * v_color.b,
                      tex_color.a * v_color.a * (1.0 - v_additivity));

    if (dot(color, color) == 0.0) {
        // NOTE: We assume pre-multiplied colors, therefore a fully transparent pixel requires that
        //       all channels are zero
        discard;
    }

    gl_FragColor = color;
}
"#;

struct ShaderSimple {
    program: GLProgram,
    vertex_config: VertexAttributeConfiguration,

    u_texture: GLUniformLocation,
    u_transform: GLUniformLocation,
    u_texture_color_modulate: GLUniformLocation,
}

impl ShaderSimple {
    fn new(gl: &glow::Context) -> ShaderSimple {
        let program = gl_program_create(
            gl,
            VERTEX_SHADER_SOURCE_SIMPLE,
            FRAGMENT_SHADER_SOURCE_SIMPLE,
        );

        // Attributes
        let a_pos = VertexAttribute {
            name: String::from("a_pos"),
            location: gl_program_get_attribute_location(gl, &program, "a_pos"),
            float_component_count: 3,
            byte_offset_in_vertex: 0,
        };
        let a_uv = VertexAttribute {
            name: String::from("a_uv"),
            location: gl_program_get_attribute_location(gl, &program, "a_uv"),
            float_component_count: 2,
            byte_offset_in_vertex: a_pos.byte_offset_in_vertex
                + a_pos.float_component_count * std::mem::size_of::<f32>() as i32,
        };
        let a_color = VertexAttribute {
            name: String::from("a_color"),
            location: gl_program_get_attribute_location(gl, &program, "a_color"),
            float_component_count: 4,
            byte_offset_in_vertex: a_uv.byte_offset_in_vertex
                + a_uv.float_component_count * std::mem::size_of::<f32>() as i32,
        };
        let a_additivity = VertexAttribute {
            name: String::from("a_additivity"),
            location: gl_program_get_attribute_location(gl, &program, "a_additivity"),
            float_component_count: 1,
            byte_offset_in_vertex: a_color.byte_offset_in_vertex
                + a_color.float_component_count * std::mem::size_of::<f32>() as i32,
        };

        // Uniforms
        let u_texture = gl_program_get_uniform_location(gl, &program, "u_texture");
        let u_transform = gl_program_get_uniform_location(gl, &program, "u_transform");
        let u_texture_color_modulate =
            gl_program_get_uniform_location(gl, &program, "u_texture_color_modulate");

        ShaderSimple {
            program,
            vertex_config: VertexAttributeConfiguration {
                size_in_bytes: std::mem::size_of::<Vertex>() as i32,
                attributes: vec![a_pos, a_uv, a_color, a_additivity],
            },
            u_texture,
            u_transform,
            u_texture_color_modulate,
        }
    }

    fn activate(&self, gl: &glow::Context, params: &ShaderParamsSimple) {
        gl_program_enable(gl, &self.program);
        unsafe {
            gl.uniform_1_i32(Some(&self.u_texture), 0);
            gl.uniform_matrix_4_f32_slice(
                Some(&self.u_transform),
                false,
                &params.transform.into_column_array(),
            );
            gl.uniform_4_f32(
                Some(&self.u_texture_color_modulate),
                params.texture_color_modulate.r,
                params.texture_color_modulate.g,
                params.texture_color_modulate.b,
                params.texture_color_modulate.a,
            );
        }
    }

    fn delete(self, gl: &glow::Context) {
        gl_program_delete(gl, self.program);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shader for blitting

const VERTEX_SHADER_SOURCE_BLIT: &str = r#"
attribute vec2 a_pos;
attribute vec2 a_uv;

uniform mat4 u_transform;

varying vec2 v_uv;

void main()
{
    gl_Position = u_transform * vec4(a_pos, 0.0, 1.0);
    v_uv = a_uv;
}
"#;

const FRAGMENT_SHADER_SOURCE_BLIT: &str = r#"
precision mediump float;

varying vec2 v_uv;

uniform sampler2D u_texture;

void main()
{
    gl_FragColor = texture2D(u_texture, v_uv);
}
"#;

struct ShaderBlit {
    program: GLProgram,
    vertex_config: VertexAttributeConfiguration,

    u_texture: GLUniformLocation,
    u_transform: GLUniformLocation,
}

impl ShaderBlit {
    fn new(gl: &glow::Context) -> ShaderBlit {
        let program = gl_program_create(gl, VERTEX_SHADER_SOURCE_BLIT, FRAGMENT_SHADER_SOURCE_BLIT);

        // Attributes
        let a_pos = VertexAttribute {
            name: String::from("a_pos"),
            location: gl_program_get_attribute_location(gl, &program, "a_pos"),
            float_component_count: 2,
            byte_offset_in_vertex: 0,
        };
        let a_uv = VertexAttribute {
            name: String::from("a_uv"),
            location: gl_program_get_attribute_location(gl, &program, "a_uv"),
            float_component_count: 2,
            byte_offset_in_vertex: a_pos.byte_offset_in_vertex
                + a_pos.float_component_count * std::mem::size_of::<f32>() as i32,
        };

        // Uniforms
        let u_texture = gl_program_get_uniform_location(gl, &program, "u_texture");
        let u_transform = gl_program_get_uniform_location(gl, &program, "u_transform");

        ShaderBlit {
            program,
            vertex_config: VertexAttributeConfiguration {
                size_in_bytes: std::mem::size_of::<VertexBlit>() as i32,
                attributes: vec![a_pos, a_uv],
            },
            u_texture,
            u_transform,
        }
    }

    fn activate(&self, gl: &glow::Context, params: &ShaderParamsBlit) {
        gl_program_enable(gl, &self.program);
        unsafe {
            gl.uniform_1_i32(Some(&self.u_texture), 0);
            gl.uniform_matrix_4_f32_slice(
                Some(&self.u_transform),
                false,
                &params.transform.into_column_array(),
            );
        }
    }

    fn delete(self, gl: &glow::Context) {
        gl_program_delete(gl, self.program);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Renderstate

pub struct Renderer {
    gl: glow::Context,

    shader_simple: Option<ShaderSimple>,
    shader_blit: Option<ShaderBlit>,

    drawobject_simple: Option<GLDrawobject>,
    drawobject_blit: Option<GLDrawobject>,

    framebuffers: HashMap<FramebufferTarget, GLFramebuffer>,
    textures: HashMap<TextureInfo, GLTexture>,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.reset();

        self.shader_simple.take().unwrap().delete(&self.gl);
        self.shader_blit.take().unwrap().delete(&self.gl);

        gl_drawobject_free(&self.gl, self.drawobject_simple.take().unwrap());
        gl_drawobject_free(&self.gl, self.drawobject_blit.take().unwrap());
    }
}

impl Renderer {
    pub fn new(gl: glow::Context) -> Renderer {
        unsafe {
            assert!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                "Mainscreen framebuffer invalid!"
            );

            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.blend_equation(glow::FUNC_ADD);

            // Enable wireframe mode
            // gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::GEQUAL);
        }

        let shader_simple = ShaderSimple::new(&gl);
        let shader_blit = ShaderBlit::new(&gl);

        let drawobject_simple = gl_drawobject_create(&gl, &shader_simple.vertex_config);
        let drawobject_blit = gl_drawobject_create(&gl, &shader_blit.vertex_config);

        assert!(gl_state_ok(&gl));

        Renderer {
            gl,

            shader_simple: Some(shader_simple),
            shader_blit: Some(shader_blit),

            drawobject_simple: Some(drawobject_simple),
            drawobject_blit: Some(drawobject_blit),

            framebuffers: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    pub fn clear(&self) {
        unsafe {
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear_depth_f32(0.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }

    pub fn reset(&mut self) {
        for (framebuffer_target, framebuffer) in self.framebuffers.drain() {
            if let FramebufferTarget::Offscreen(_framebuffer_info) = framebuffer_target {
                gl_framebuffer_delete(&self.gl, framebuffer);
            }
        }
        for (_texture_info, texture) in self.textures.drain() {
            gl_texture_delete(&self.gl, texture);
        }
    }

    pub fn process_drawcommands(
        &mut self,
        screen_width: u32,
        screen_height: u32,
        drawcommands: &[Drawcommand],
    ) {
        // Update our screen framebuffer
        self.framebuffers.insert(
            FramebufferTarget::Screen,
            gl_framebuffer_screen(screen_width, screen_height),
        );

        for drawcommand in drawcommands {
            match drawcommand {
                Drawcommand::Draw {
                    framebuffer_target,
                    texture_info,
                    shader_params,
                    vertexbuffer,
                } => {
                    let framebuffer = self.framebuffers.get(framebuffer_target).expect(&format!(
                        "No framebuffer found for '{:?}'",
                        framebuffer_target
                    ));
                    let texture = self
                        .textures
                        .get(texture_info)
                        .expect(&format!("No texture found for '{:?}'", texture_info));

                    unsafe {
                        self.gl
                            .bind_framebuffer(glow::FRAMEBUFFER, framebuffer.framebuffer_object);
                        self.gl
                            .viewport(0, 0, framebuffer.width as i32, framebuffer.height as i32);
                    }

                    match shader_params {
                        ShaderParams::Simple(shader_params_simple) => {
                            assert!(vertexbuffer.indices.len() % 3 == 0);

                            self.shader_simple
                                .as_ref()
                                .unwrap()
                                .activate(&self.gl, shader_params_simple);

                            // NOTE: We need to bind the texture here as the activation of the
                            //       shader might have invalidated our texture unit
                            unsafe {
                                self.gl.active_texture(glow::TEXTURE0);
                                self.gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
                            }

                            gl_drawobject_draw(
                                &self.gl,
                                &self.drawobject_simple.as_ref().unwrap(),
                                &vertexbuffer,
                            );
                        }
                        ShaderParams::Blit(_shader_params_blit) => {
                            panic!("The blit shader does not support drawing operations")
                        }
                    }
                }
                Drawcommand::TextureCreate(texture_info) => {
                    assert!(
                        !self.textures.contains_key(&texture_info),
                        "A texture already exists for: '{:?}'",
                        texture_info
                    );
                    let texture =
                        gl_texture_create(&self.gl, texture_info.width, texture_info.height);
                    self.textures.insert(texture_info.clone(), texture);
                }
                Drawcommand::TextureUpdate {
                    texture_info,
                    offset_x,
                    offset_y,
                    bitmap,
                } => {
                    let texture = self
                        .textures
                        .get(&texture_info)
                        .expect(&format!("No texture found for '{:?}'", texture_info));
                    gl_texture_update(
                        &self.gl,
                        texture,
                        *offset_x,
                        *offset_y,
                        bitmap.width as u32,
                        bitmap.height as u32,
                        &bitmap.data,
                    );
                }
                Drawcommand::TextureFree(texture_info) => {
                    let texture = self
                        .textures
                        .remove(texture_info)
                        .expect(&format!("No texture found for '{:?}'", texture_info));
                    gl_texture_delete(&self.gl, texture);
                }
                Drawcommand::FramebufferCreate(framebuffer_info) => {
                    assert!(
                        !self
                            .framebuffers
                            .contains_key(&FramebufferTarget::Offscreen(framebuffer_info.clone())),
                        "A framebuffer already exists for: '{:?}'",
                        framebuffer_info,
                    );
                    let framebuffer = gl_framebuffer_create(
                        &self.gl,
                        framebuffer_info.width,
                        framebuffer_info.height,
                    );
                    self.framebuffers.insert(
                        FramebufferTarget::Offscreen(framebuffer_info.clone()),
                        framebuffer,
                    );
                }
                Drawcommand::FramebufferFree(framebuffer_info) => {
                    let framebuffer = self
                        .framebuffers
                        .remove(&FramebufferTarget::Offscreen(framebuffer_info.clone()))
                        .expect(&format!(
                            "No framebuffer found for '{:?}'",
                            framebuffer_info
                        ));
                    gl_framebuffer_delete(&self.gl, framebuffer);
                }
                Drawcommand::FramebufferClear {
                    framebuffer_target,
                    new_color,
                    new_depth,
                } => {
                    let framebuffer = self.framebuffers.get(&framebuffer_target).expect(&format!(
                        "No framebuffer found for '{:?}'",
                        framebuffer_target
                    ));

                    unsafe {
                        self.gl
                            .bind_framebuffer(glow::FRAMEBUFFER, framebuffer.framebuffer_object);
                        self.gl
                            .viewport(0, 0, framebuffer.width as i32, framebuffer.height as i32);

                        let mut clear_mask = 0;
                        if let Some(color) = new_color {
                            clear_mask |= glow::COLOR_BUFFER_BIT;
                            self.gl.clear_color(color.r, color.g, color.b, color.a);
                        }
                        if let Some(depth) = new_depth {
                            clear_mask |= glow::DEPTH_BUFFER_BIT;
                            self.gl.clear_depth_f32(*depth);
                        }
                        self.gl.clear(clear_mask);
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

                    // NOTE: Instead of just borrowing our value we need to take it out
                    //       of the map here to please the borrowchecker. Lets find a better way
                    let source_framebuffer = self
                        .framebuffers
                        .get(&FramebufferTarget::Offscreen(
                            source_framebuffer_info.clone(),
                        ))
                        .expect(&format!(
                            "No framebuffer found for '{:?}'",
                            source_framebuffer_info
                        ));
                    let dest_framebuffer =
                        self.framebuffers
                            .get(dest_framebuffer_target)
                            .expect(&format!(
                                "No framebuffer found for '{:?}'",
                                source_framebuffer_info
                            ));

                    self.framebuffer_blit(
                        dest_framebuffer,
                        source_framebuffer,
                        *dest_rect,
                        *source_rect,
                    );
                }
            }
        }

        assert!(gl_state_ok(&self.gl));
    }

    fn framebuffer_blit(
        &self,
        framebuffer_target: &GLFramebuffer,
        framebuffer_source: &GLFramebuffer,
        rect_target: BlitRect,
        rect_source: BlitRect,
    ) {
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, framebuffer_target.framebuffer_object);
            self.gl.viewport(
                0,
                0,
                framebuffer_target.width as i32,
                framebuffer_target.height as i32,
            );

            self.gl.disable(glow::BLEND);
            self.gl.disable(glow::DEPTH_TEST);

            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(
                glow::TEXTURE_2D,
                if let Some(color) = &framebuffer_source.color {
                    Some(color.id)
                } else {
                    None
                },
            );
        }

        let transform = Mat4::ortho_origin_left_bottom(
            framebuffer_target.width as f32,
            framebuffer_target.height as f32,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
        );
        self.shader_blit
            .as_ref()
            .unwrap()
            .activate(&self.gl, &ShaderParamsBlit { transform });

        let mut vertexbuffer_blit = VertexbufferBlit::new(0);
        vertexbuffer_blit.push_blit_quad(
            rect_target,
            rect_source,
            framebuffer_source.width,
            framebuffer_source.height,
        );

        gl_drawobject_draw(
            &self.gl,
            &self.drawobject_blit.as_ref().unwrap(),
            &vertexbuffer_blit,
        );

        unsafe {
            self.gl.enable(glow::BLEND);
            self.gl.enable(glow::DEPTH_TEST);
        }
    }
}
