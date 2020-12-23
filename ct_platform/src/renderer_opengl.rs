use ct_lib::draw_common::*;
use ct_lib::math::*;
use ct_lib::{draw::*, transmute_to_slice};

use ct_lib::log;

use glow::*;

use std::{collections::HashMap, rc::Rc};

type GlowProgramId = <glow::Context as glow::HasContext>::Program;
type GlowTextureId = <glow::Context as glow::HasContext>::Texture;
type GlowFramebufferId = <glow::Context as glow::HasContext>::Framebuffer;
type GlowRenderbufferId = <glow::Context as glow::HasContext>::Renderbuffer;
type GlowUniformLocation = <glow::Context as glow::HasContext>::UniformLocation;
type GlowVertexArray = <glow::Context as glow::HasContext>::VertexArray;
type GlowBuffer = <glow::Context as glow::HasContext>::Buffer;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Error checking

fn gl_state_ok(gl: &glow::Context) -> bool {
    let mut is_ok = true;

    loop {
        let error = unsafe { gl.get_error() };
        if error == glow::NO_ERROR {
            break;
        } else {
            log::error!("OpenGL error: {}", gl_error_string(error));
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
// Shader

#[derive(Clone, Copy)]
enum ShaderPrimitiveType {
    Float,
    Vector2,
    Vector3,
    Vector4,
    Matrix4,
    Sampler2D,
}

impl ShaderPrimitiveType {
    pub fn from_string(typestring: &str) -> Result<ShaderPrimitiveType, String> {
        match typestring {
            "float" => Ok(ShaderPrimitiveType::Float),
            "vec2" => Ok(ShaderPrimitiveType::Vector2),
            "vec3" => Ok(ShaderPrimitiveType::Vector3),
            "vec4" => Ok(ShaderPrimitiveType::Vector4),
            "mat4" => Ok(ShaderPrimitiveType::Matrix4),
            "sampler2D" => Ok(ShaderPrimitiveType::Sampler2D),
            _ => Err(format!("Unknown primitive type '{}'", typestring)),
        }
    }

    pub fn float_component_count(&self) -> usize {
        match self {
            ShaderPrimitiveType::Float => 1,
            ShaderPrimitiveType::Vector2 => 2,
            ShaderPrimitiveType::Vector3 => 3,
            ShaderPrimitiveType::Vector4 => 4,
            ShaderPrimitiveType::Matrix4 => 16,
            // NOTE: The sampler will not be part of any uniform blocks
            ShaderPrimitiveType::Sampler2D => 0,
        }
    }

    pub fn size_in_byte(&self) -> usize {
        self.float_component_count() * std::mem::size_of::<f32>()
    }
}

struct ShaderAttribute {
    pub name: String,
    pub location: u32,
    pub primitive_type: ShaderPrimitiveType,
    pub byte_offset_in_vertex: usize,
}

struct ShaderUniform {
    pub name: String,
    pub primitive_type: ShaderPrimitiveType,
    pub location: GlowUniformLocation,
}

struct Shader {
    pub name: String,
    pub attributes: Vec<ShaderAttribute>,
    pub uniforms: Vec<ShaderUniform>,

    gl: Rc<glow::Context>,
    program_id: GlowProgramId,
}

impl Drop for Shader {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.use_program(None);
            gl.delete_program(self.program_id);
        }
    }
}

impl Shader {
    pub fn new(
        gl: Rc<glow::Context>,
        name: &str,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<Shader, String> {
        let program_id =
            Shader::create_program_from_source(&gl, vertex_shader_source, fragment_shader_source)
                .map_err(|error| format!("Could not compile shader '{}': {}", name, error))?;

        let (attributes, uniforms) = {
            Shader::get_attributes_and_uniforms(
                &gl,
                &program_id,
                vertex_shader_source,
                fragment_shader_source,
            )
            .map_err(|error| {
                unsafe {
                    gl.use_program(None);
                    gl.delete_program(program_id);
                }
                format!(
                    "Failed to load attributes and/or uniforms for shader '{}': {}",
                    name, error
                )
            })?
        };

        Ok(Shader {
            name: name.to_owned(),
            attributes,
            uniforms,
            gl,
            program_id,
        })
    }

    fn activate(&self, uniform_block: &[f32]) {
        let gl = &self.gl;
        unsafe {
            gl.use_program(Some(self.program_id));
        }

        let mut uniform_block = uniform_block;
        for uniform in &self.uniforms {
            let float_component_count = uniform.primitive_type.float_component_count();
            let (uniform_data, remainder) = uniform_block.split_at(float_component_count);
            match uniform.primitive_type {
                ShaderPrimitiveType::Float => unsafe {
                    gl.uniform_1_f32_slice(Some(&uniform.location), uniform_data);
                },
                ShaderPrimitiveType::Vector2 => unsafe {
                    gl.uniform_2_f32_slice(Some(&uniform.location), uniform_data);
                },
                ShaderPrimitiveType::Vector3 => unsafe {
                    gl.uniform_3_f32_slice(Some(&uniform.location), uniform_data);
                },
                ShaderPrimitiveType::Vector4 => unsafe {
                    gl.uniform_4_f32_slice(Some(&uniform.location), uniform_data);
                },
                ShaderPrimitiveType::Matrix4 => unsafe {
                    gl.uniform_matrix_4_f32_slice(Some(&uniform.location), false, uniform_data)
                },
                ShaderPrimitiveType::Sampler2D => unsafe {
                    gl.uniform_1_i32(Some(&uniform.location), 0)
                },
            }
            uniform_block = remainder;
        }
        assert!(
            uniform_block.len() == 0,
            "Given uniform block contains more data than described in shader"
        );
    }

    fn create_program_from_source(
        gl: &glow::Context,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<GlowProgramId, String> {
        let program = unsafe {
            // Vertex shader
            let vertex_shader = gl
                .create_shader(glow::VERTEX_SHADER)
                .map_err(|error| format!("Cannot create vertex shader: {}", error))?;
            gl.shader_source(vertex_shader, vertex_shader_source);
            gl.compile_shader(vertex_shader);
            if !gl.get_shader_compile_status(vertex_shader) {
                let result = Err(format!(
                    "Failed to compile vertex shader:\n{}",
                    gl.get_shader_info_log(vertex_shader),
                ));
                gl.delete_shader(vertex_shader);
                return result;
            }

            // Fragment shader
            let fragment_shader = gl
                .create_shader(glow::FRAGMENT_SHADER)
                .map_err(|error| format!("Cannot create fragment shader: {}", error))?;
            gl.shader_source(fragment_shader, fragment_shader_source);
            gl.compile_shader(fragment_shader);
            if !gl.get_shader_compile_status(fragment_shader) {
                let result = Err(format!(
                    "Failed to compile fragment shader:\n{}",
                    gl.get_shader_info_log(fragment_shader)
                ));
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                return result;
            }

            // Program
            let program = gl
                .create_program()
                .map_err(|error| format!("Cannot create shader program: {}", error))?;
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                let result = Err(format!(
                    "Failed to link shader program:\n{}",
                    gl.get_program_info_log(program)
                ));
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                gl.delete_program(program);
                return result;
            }

            // Program is successfully compiled and linked - we don't need the shaders anymore
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            program
        };

        // NOTE: We use assert instead of Err/Result here just to detect programming errors
        assert!(gl_state_ok(gl), "Could not compile shader program");
        log::info!("Shaderprogram {:?} successfully compiled", program);

        Ok(program)
    }

    fn get_attributes_and_uniforms(
        gl: &glow::Context,
        program: &GlowProgramId,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<(Vec<ShaderAttribute>, Vec<ShaderUniform>), String> {
        let attributes = Shader::get_attributes(gl, program, vertex_shader_source)?;
        let uniforms = {
            let mut uniforms_vertexshader =
                Shader::get_uniforms(gl, program, vertex_shader_source)?;
            let mut uniforms_fragmentshader =
                Shader::get_uniforms(gl, program, fragment_shader_source)?;
            uniforms_vertexshader.append(&mut uniforms_fragmentshader);
            uniforms_vertexshader
        };
        Ok((attributes, uniforms))
    }

    fn get_attributes(
        gl: &glow::Context,
        program_id: &GlowProgramId,
        shader_source: &str,
    ) -> Result<Vec<ShaderAttribute>, String> {
        let mut attributes = Vec::new();
        let mut byte_offset_in_vertex = 0;
        let attribute_regex = regex::Regex::new(r"attribute\s(\w+)\s(\w+);").unwrap();
        for capture in attribute_regex.captures_iter(shader_source) {
            let name = &capture[2];
            let type_name = &capture[1];
            let primitive_type = ShaderPrimitiveType::from_string(type_name)
                .map_err(|error| format!("Error parsing shader primitive '{}'", type_name))?;
            let location = unsafe { gl.get_attrib_location(*program_id, name) }
                .ok_or_else(|| format!("Program {:?} has no attribute '{}'", program_id, name))?;

            attributes.push(ShaderAttribute {
                name: name.to_owned(),
                location,
                byte_offset_in_vertex,
                primitive_type,
            });
            byte_offset_in_vertex += primitive_type.size_in_byte();
        }
        Ok(attributes)
    }

    fn get_uniforms(
        gl: &glow::Context,
        program_id: &GlowProgramId,
        shader_source: &str,
    ) -> Result<Vec<ShaderUniform>, String> {
        let mut uniforms = Vec::new();
        let attribute_regex = regex::Regex::new(r"uniform\s(\w+)\s(\w+);").unwrap();
        for capture in attribute_regex.captures_iter(shader_source) {
            let name = &capture[2];
            let type_name = &capture[1];
            let primitive_type = ShaderPrimitiveType::from_string(type_name)
                .map_err(|error| format!("Error parsing shader primitive '{}'", type_name))?;
            let location = unsafe { gl.get_uniform_location(*program_id, name) }
                .ok_or_else(|| format!("Program {:?} has no uniform '{}'", program_id, name))?;

            uniforms.push(ShaderUniform {
                name: name.to_owned(),
                primitive_type,
                location,
            });
        }
        Ok(uniforms)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Creating textures from pixelbuffers

struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,

    gl: Rc<glow::Context>,
    texture_id: GlowTextureId,
}

impl Drop for Texture {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.delete_texture(self.texture_id);
        }
    }
}

impl Texture {
    fn new(gl: Rc<glow::Context>, name: &str, width: u32, height: u32) -> Texture {
        let texture_id = unsafe {
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

        Texture {
            name: name.to_owned(),
            width,
            height,
            gl,
            texture_id,
        }
    }

    fn update_pixels(
        &self,
        offset_x: u32,
        offset_y: u32,
        region_width: u32,
        region_height: u32,
        pixels: &[PixelRGBA],
    ) {
        let gl = &self.gl;
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_id));
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                offset_x as i32,
                offset_y as i32,
                region_width as i32,
                region_height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(ct_lib::transmute_to_byte_slice(pixels)),
            );

            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Creating depthbuffer

struct Depthbuffer {
    pub name: String,
    pub width: u32,
    pub height: u32,

    gl: Rc<glow::Context>,
    depth_id: GlowRenderbufferId,
}

impl Drop for Depthbuffer {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.delete_renderbuffer(self.depth_id);
        }
    }
}

impl Depthbuffer {
    fn new(gl: Rc<glow::Context>, name: &str, width: u32, height: u32) -> Depthbuffer {
        let depth_id = unsafe {
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

        Depthbuffer {
            name: name.to_owned(),
            width,
            height,
            gl,
            depth_id,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// General purpose offscreen-framebuffers

struct Framebuffer {
    pub name: String,
    pub width: u32,
    pub height: u32,

    gl: Rc<glow::Context>,
    // NOTE: These can be `None` if our framebuffer represents the screen framebuffer
    framebuffer_id: Option<GlowFramebufferId>,
    color: Option<Texture>,
    _depth: Option<Depthbuffer>,
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(framebuffer_id) = self.framebuffer_id {
            let gl = &self.gl;
            unsafe {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.delete_framebuffer(framebuffer_id);
            }
        }
    }
}

impl Framebuffer {
    pub fn new_screen(gl: Rc<glow::Context>, width: u32, height: u32) -> Framebuffer {
        Framebuffer {
            name: "screen".to_owned(),
            width,
            height,
            gl,
            framebuffer_id: None,
            color: None,
            _depth: None,
        }
    }

    pub fn new(gl: Rc<glow::Context>, name: &str, width: u32, height: u32) -> Framebuffer {
        unsafe {
            // The color texture
            let color = Texture::new(
                gl.clone(),
                &format!("{} framebuffer color texture", name),
                width,
                height,
            );
            let depth = Depthbuffer::new(
                gl.clone(),
                &format!("{} framebuffer depth texture", name),
                width,
                height,
            );

            // Create offscreen framebuffer
            let framebuffer = gl.create_framebuffer().expect("Cannot create framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

            // Attach color and depth buffers
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(color.texture_id),
                0,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth.depth_id),
            );

            assert!(gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Framebuffer {
                name: name.to_owned(),
                width,
                height,
                gl,
                framebuffer_id: Some(framebuffer),
                color: Some(color),
                _depth: Some(depth),
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drawobjects

struct DrawObject {
    name: String,

    gl: Rc<glow::Context>,
    vertex_array_id: GlowVertexArray,
    vertex_buffer_id: GlowBuffer,
    index_buffer_id: GlowBuffer,
}

impl Drop for DrawObject {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            gl.delete_vertex_array(self.vertex_array_id);
            gl.delete_buffer(self.vertex_buffer_id);
            gl.delete_buffer(self.index_buffer_id);
        }
    }
}

impl DrawObject {
    fn new_from_shader(gl: Rc<glow::Context>, shader: &Shader) -> DrawObject {
        let name = format!("{} drawobject", &shader.name);
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
            let attributes = &shader.attributes;
            let stride = attributes.iter().fold(0, |acc, attribute| {
                acc + attribute.primitive_type.size_in_byte()
            });
            for attribute in attributes {
                let index = attribute.location;
                let size = attribute.primitive_type.float_component_count();
                let offset = attribute.byte_offset_in_vertex;
                let normalized = false;

                gl.enable_vertex_attrib_array(index);
                gl.vertex_attrib_pointer_f32(
                    index,
                    size as i32,
                    glow::FLOAT,
                    normalized,
                    stride as i32,
                    offset as i32,
                );
            }

            // NOTE: The buffers must not be unbound before the vertex array, so the order here is important
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            (vertex_array, vertex_buffer, index_buffer)
        };

        DrawObject {
            name,
            gl,
            vertex_array_id: vertex_array,
            vertex_buffer_id: vertex_buffer,
            index_buffer_id: index_buffer,
        }
    }

    fn assing_buffers(&self, vertices: &[f32], indices: &[u32]) {
        let gl = &self.gl;
        unsafe {
            // Vertices
            let vertices_raw = ct_lib::transmute_to_byte_slice(vertices);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer_id));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices_raw, glow::STREAM_DRAW);

            // Indices
            let indices_raw = ct_lib::transmute_to_byte_slice(indices);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer_id));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_raw, glow::STREAM_DRAW);
        }
    }

    fn draw(&self, indices_count: usize, indices_start_offset: usize) {
        let gl = &self.gl;
        unsafe {
            // Draw
            gl.bind_vertex_array(Some(self.vertex_array_id));
            gl.draw_elements(
                glow::TRIANGLES,
                indices_count as i32,
                glow::UNSIGNED_INT,
                indices_start_offset as i32,
            );
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Renderstate

pub struct Renderer {
    gl: Rc<glow::Context>,

    shader_simple: Shader,
    shader_blit: Shader,

    drawobject_simple: DrawObject,
    drawobject_blit: DrawObject,

    framebuffers: HashMap<FramebufferTarget, Framebuffer>,
    textures: HashMap<TextureInfo, Texture>,
}

impl Renderer {
    pub fn new(gl: glow::Context) -> Renderer {
        let gl = Rc::new(gl);
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

        let shader_simple = Shader::new(
            gl.clone(),
            "simple",
            VERTEX_SHADER_SOURCE_SIMPLE,
            FRAGMENT_SHADER_SOURCE_SIMPLE,
        )
        .expect("Could not compile simple shader");
        let shader_blit = Shader::new(
            gl.clone(),
            "blit",
            VERTEX_SHADER_SOURCE_BLIT,
            FRAGMENT_SHADER_SOURCE_BLIT,
        )
        .expect("Could not compile blit shader");

        let drawobject_simple = DrawObject::new_from_shader(gl.clone(), &shader_simple);
        let drawobject_blit = DrawObject::new_from_shader(gl.clone(), &shader_blit);

        assert!(gl_state_ok(&gl), "Error while creating renderer");

        Renderer {
            gl,
            shader_simple,
            shader_blit,
            drawobject_simple,
            drawobject_blit,
            framebuffers: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    pub fn clear(&self) {
        let gl = &self.gl;
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear_depth_f32(0.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }

    pub fn reset(&mut self) {
        self.framebuffers.clear();
        self.textures.clear();
    }

    pub fn process_drawcommands(
        &mut self,
        screen_width: u32,
        screen_height: u32,
        drawcommands: &[Drawcommand],
    ) {
        let gl = &self.gl;

        // Update our screen framebuffer
        self.framebuffers.insert(
            FramebufferTarget::Screen,
            Framebuffer::new_screen(gl.clone(), screen_width, screen_height),
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
                        gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer.framebuffer_id);
                        gl.viewport(0, 0, framebuffer.width as i32, framebuffer.height as i32);
                    }

                    match shader_params {
                        ShaderParams::Simple { uniform_block } => {
                            assert!(vertexbuffer.indices.len() % 3 == 0);

                            self.shader_simple.activate(uniform_block);

                            // NOTE: We need to bind the texture here as the activation of the
                            //       shader might have invalidated our texture unit
                            unsafe {
                                gl.active_texture(glow::TEXTURE0);
                                gl.bind_texture(glow::TEXTURE_2D, Some(texture.texture_id));
                            }

                            let vertices = unsafe {
                                transmute_to_slice::<Vertex, f32>(&vertexbuffer.vertices)
                            };
                            self.drawobject_simple
                                .assing_buffers(&vertices, &vertexbuffer.indices);
                            self.drawobject_simple.draw(vertexbuffer.indices.len(), 0);
                        }
                        ShaderParams::Blit { .. } => {
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
                    self.textures.insert(
                        texture_info.clone(),
                        Texture::new(
                            gl.clone(),
                            &texture_info.name,
                            texture_info.width,
                            texture_info.height,
                        ),
                    );
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
                    texture.update_pixels(
                        *offset_x,
                        *offset_y,
                        bitmap.width as u32,
                        bitmap.height as u32,
                        &bitmap.data,
                    );
                }
                Drawcommand::TextureFree(texture_info) => {
                    self.textures
                        .remove(texture_info)
                        .expect(&format!("No texture found for '{:?}'", texture_info));
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
                        Framebuffer::new(
                            gl.clone(),
                            &framebuffer_info.name,
                            framebuffer_info.width,
                            framebuffer_info.height,
                        ),
                    );
                }
                Drawcommand::FramebufferFree(framebuffer_info) => {
                    self.framebuffers
                        .remove(&FramebufferTarget::Offscreen(framebuffer_info.clone()))
                        .expect(&format!(
                            "No framebuffer found for '{:?}'",
                            framebuffer_info
                        ));
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
                        gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer.framebuffer_id);
                        gl.viewport(0, 0, framebuffer.width as i32, framebuffer.height as i32);

                        let mut clear_mask = 0;
                        if let Some(color) = new_color {
                            clear_mask |= glow::COLOR_BUFFER_BIT;
                            gl.clear_color(color.r, color.g, color.b, color.a);
                        }
                        if let Some(depth) = new_depth {
                            clear_mask |= glow::DEPTH_BUFFER_BIT;
                            gl.clear_depth_f32(*depth);
                        }
                        gl.clear(clear_mask);
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
            debug_assert!(
                gl_state_ok(&gl),
                "Error after drawcommand {:?}",
                drawcommand
            );
        }

        debug_assert!(gl_state_ok(&gl), "Error after processing drawcommands");
    }

    fn framebuffer_blit(
        &self,
        framebuffer_target: &Framebuffer,
        framebuffer_source: &Framebuffer,
        rect_target: BlitRect,
        rect_source: BlitRect,
    ) {
        let gl = &self.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer_target.framebuffer_id);
            gl.viewport(
                0,
                0,
                framebuffer_target.width as i32,
                framebuffer_target.height as i32,
            );

            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(
                glow::TEXTURE_2D,
                if let Some(color) = &framebuffer_source.color {
                    Some(color.texture_id)
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
        self.shader_blit.activate(&transform.into_column_array());

        let mut vertexbuffer_blit = VertexbufferBlit::new(0);
        vertexbuffer_blit.push_blit_quad(
            rect_target,
            rect_source,
            framebuffer_source.width,
            framebuffer_source.height,
        );

        let vertices =
            unsafe { transmute_to_slice::<VertexBlit, f32>(&vertexbuffer_blit.vertices) };
        self.drawobject_blit
            .assing_buffers(&vertices, &vertexbuffer_blit.indices);
        self.drawobject_blit
            .draw(vertexbuffer_blit.indices.len(), 0);

        unsafe {
            gl.enable(glow::BLEND);
            gl.enable(glow::DEPTH_TEST);
        }
    }
}
