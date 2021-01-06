use ct_lib_core::{log, transmute_slice_to_byte_slice, transmute_slice_to_byte_slice_mut};
use ct_lib_math::Recti;
use ct_lib_math::{clampf, Mat4};

use glow::HasContext;

use std::{collections::HashMap, rc::Rc};

type GlowProgramId = <glow::Context as glow::HasContext>::Program;
type GlowTextureId = <glow::Context as glow::HasContext>::Texture;
type GlowFramebufferId = <glow::Context as glow::HasContext>::Framebuffer;
type GlowRenderbufferId = <glow::Context as glow::HasContext>::Renderbuffer;
type GlowUniformLocation = <glow::Context as glow::HasContext>::UniformLocation;
type GlowVertexArray = <glow::Context as glow::HasContext>::VertexArray;
type GlowBuffer = <glow::Context as glow::HasContext>::Buffer;

// NOTE: This translates to the depth range [0, 100] from farthest to nearest (like a paperstack)
//       For more information see: https://stackoverflow.com/a/36046924
const todo: &str =
    "this currently is used for blitting and duplicates drawstate - what do we do with it?";
pub const DEFAULT_WORLD_ZNEAR: f32 = 0.0;
pub const DEFAULT_WORLD_ZFAR: f32 = -100.0;

const ENABLE_LOGS: bool = false;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Error checking

// WARNING: This function is really expensive
fn gl_check_state_ok(gl: &glow::Context) -> Result<(), String> {
    let error = unsafe { gl.get_error() };
    if error == glow::NO_ERROR {
        Ok(())
    } else {
        Err(format!("OpenGL error: {}", gl_errorcode_to_string(error)))
    }
}

fn gl_errorcode_to_string(error: u32) -> String {
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
        name: String,
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

        gl_check_state_ok(&gl).map_err(|error| {
            unsafe { gl.delete_program(program_id) };
            format!(
                "Something went wrong while compiling shader '{}': {}",
                name, error
            )
        })?;

        Ok(Shader {
            name,
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
            "Given uniform block contains more data than described in shader '{}'",
            self.name
        );
        debug_assert!(
            gl_check_state_ok(&gl).is_ok(),
            "Something went wrong while activating shader '{}'",
            self.name
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
            let primitive_type = ShaderPrimitiveType::from_string(type_name).map_err(|error| {
                format!("Error parsing shader primitive '{}': {}", type_name, error)
            })?;
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
            let primitive_type = ShaderPrimitiveType::from_string(type_name).map_err(|error| {
                format!("Error parsing shader primitive '{}': {}", type_name, error)
            })?;
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
    fn new(gl: Rc<glow::Context>, name: String, width: u32, height: u32) -> Texture {
        let texture_id = unsafe {
            let texture = gl
                .create_texture()
                .unwrap_or_else(|error| panic!("Cannot create texture '{}': {}", name, error));
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

        gl_check_state_ok(&gl).unwrap_or_else(|error| {
            panic!(
                "Something went wrong while creating texture '{}': {}",
                name, error
            )
        });
        Texture {
            name: name,
            width,
            height,
            gl,
            texture_id,
        }
    }

    pub fn activate(&self, texture_unit: usize) {
        let texture_unit = match texture_unit {
            0 => glow::TEXTURE0,
            1 => glow::TEXTURE1,
            2 => glow::TEXTURE2,
            3 => glow::TEXTURE3,
            _ => panic!(
                "Activating texture '{}' on texture unit {} not supported",
                self.name, texture_unit
            ),
        };
        let gl = &self.gl;
        unsafe {
            gl.active_texture(texture_unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_id));
        }
    }

    pub fn update_pixels(
        &self,
        offset_x: u32,
        offset_y: u32,
        region_width: u32,
        region_height: u32,
        pixels: &[u8],
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
                glow::PixelUnpackData::Slice(pixels),
            );

            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        gl_check_state_ok(&gl).unwrap_or_else(|error| {
            panic!(
                "Something went wrong while updating texture '{}': {}",
                self.name, error
            )
        });
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
    fn new(gl: Rc<glow::Context>, name: String, width: u32, height: u32) -> Depthbuffer {
        let depth_id = unsafe {
            let depth = gl
                .create_renderbuffer()
                .unwrap_or_else(|error| panic!("Cannot create depthbuffer '{}': {}", name, error));
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

        gl_check_state_ok(&gl).unwrap_or_else(|error| {
            panic!(
                "Something went wrong while creating depthbuffer '{}': {}",
                name, error
            )
        });

        Depthbuffer {
            name,
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

    pub fn new(gl: Rc<glow::Context>, name: String, width: u32, height: u32) -> Framebuffer {
        unsafe {
            // The color texture
            let color = Texture::new(
                gl.clone(),
                format!("{} framebuffer color texture", &name),
                width,
                height,
            );
            let depth = Depthbuffer::new(
                gl.clone(),
                format!("{} framebuffer depth texture", &name),
                width,
                height,
            );

            // Create offscreen framebuffer
            let framebuffer = gl
                .create_framebuffer()
                .unwrap_or_else(|error| panic!("Cannot create framebuffer '{}': {}", name, error));
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

            assert!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                "Framebuffer status was not ok for framebuffer '{}'",
                name,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            gl_check_state_ok(&gl).unwrap_or_else(|error| {
                panic!(
                    "Something went wrong while creating framebuffer '{}': {}",
                    name, error
                )
            });

            Framebuffer {
                name,
                width,
                height,
                gl,
                framebuffer_id: Some(framebuffer),
                color: Some(color),
                _depth: Some(depth),
            }
        }
    }

    pub fn activate(&self) {
        let gl = &self.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer_id);
            gl.viewport(0, 0, self.width as i32, self.height as i32);
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
        let name = shader.name.clone();
        let (vertex_array, vertex_buffer, index_buffer) = unsafe {
            let vertex_array = gl.create_vertex_array().unwrap_or_else(|error| {
                panic!(
                    "Cannot create vertex array object for drawobject '{}': {}",
                    name, error
                )
            });
            gl.bind_vertex_array(Some(vertex_array));

            let vertex_buffer = gl.create_buffer().unwrap_or_else(|error| {
                panic!(
                    "Cannot create vertex buffer for drawobject '{}': {}",
                    name, error
                )
            });
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

            let index_buffer = gl.create_buffer().unwrap_or_else(|error| {
                panic!(
                    "Cannot create index buffer for drawobject '{}': {}",
                    name, error
                )
            });
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));

            // Assign attributes
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

            gl_check_state_ok(&gl).unwrap_or_else(|error| {
                panic!(
                    "Something went wrong while creating drawobject '{}': {}",
                    name, error
                )
            });

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

    fn assign_buffers(&self, vertices: &[u8], indices: &[u8]) {
        let gl = &self.gl;
        unsafe {
            // Vertices
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer_id));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices, glow::STREAM_DRAW);

            // Indices
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer_id));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices, glow::STREAM_DRAW);

            debug_assert!(
                gl_check_state_ok(&gl).is_ok(),
                "Something went wrong while binding buffers for drawobject '{}'",
                self.name
            );
        }
    }

    fn draw(&self, indices_start_offset: u32, indices_count: usize) {
        let gl = &self.gl;
        let indices_offset_in_bytes = std::mem::size_of::<u32>() * indices_start_offset as usize;
        unsafe {
            // Draw
            gl.bind_vertex_array(Some(self.vertex_array_id));
            gl.draw_elements(
                glow::TRIANGLES,
                indices_count as i32,
                glow::UNSIGNED_INT,
                indices_offset_in_bytes as i32,
            );
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }

        debug_assert!(
            gl_check_state_ok(&gl).is_ok(),
            "Something went wrong while drawing buffers from drawobject '{}' with indexcount {} and index offset {}",
            self.name,
            indices_count,
            indices_start_offset
        );
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

    shaders: HashMap<String, Shader>,
    drawobjects: HashMap<String, DrawObject>,

    framebuffers: HashMap<String, Framebuffer>,
    textures: HashMap<String, Texture>,
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
            "simple".to_owned(),
            VERTEX_SHADER_SOURCE_SIMPLE,
            FRAGMENT_SHADER_SOURCE_SIMPLE,
        )
        .expect("Could not compile simple shader");
        let shader_blit = Shader::new(
            gl.clone(),
            "blit".to_owned(),
            VERTEX_SHADER_SOURCE_BLIT,
            FRAGMENT_SHADER_SOURCE_BLIT,
        )
        .expect("Could not compile blit shader");

        let drawobject_simple = DrawObject::new_from_shader(gl.clone(), &shader_simple);
        let drawobject_blit = DrawObject::new_from_shader(gl.clone(), &shader_blit);

        let mut drawobjects = HashMap::new();
        drawobjects.insert("simple".to_owned(), drawobject_simple);
        drawobjects.insert("blit".to_owned(), drawobject_blit);

        let mut shaders = HashMap::new();
        shaders.insert("simple".to_owned(), shader_simple);
        shaders.insert("blit".to_owned(), shader_blit);

        gl_check_state_ok(&gl).expect("Something went wrong while creating renderer");

        Renderer {
            gl,
            shaders,
            drawobjects,
            framebuffers: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.framebuffers.clear();
        self.textures.clear();
    }

    #[inline]
    pub fn update_screen_dimensions(&mut self, screen_width: u32, screen_height: u32) {
        let gl = &self.gl;
        if !self.framebuffers.contains_key("screen") {
            self.framebuffers.insert(
                "screen".to_owned(),
                Framebuffer::new_screen(gl.clone(), screen_width, screen_height),
            );
        }

        let framebuffer = self.framebuffers.get_mut("screen").unwrap();
        framebuffer.width = screen_width;
        framebuffer.height = screen_height;
    }

    #[inline]
    pub fn get_screen_dimensions(&self) -> (u32, u32) {
        let screen = self
            .framebuffers
            .get("screen")
            .unwrap_or_else(|| panic!("Screen framebuffer not created"));
        (screen.width, screen.height)
    }

    #[inline]
    pub fn assign_buffers(&mut self, shader: &str, vertices: &[u8], indices: &[u8]) {
        self.drawobjects
            .get(shader)
            .unwrap_or_else(|| panic!("Drawobject not found for shader '{}'", shader))
            .assign_buffers(&vertices, &indices);

        if ENABLE_LOGS {
            log::trace!(
                "Assigning buffers:
        shader: '{}'
        vertices_bytes: '{}'
        vertices_floatcount: '{}'
        indices_bytes: {}
        indices_count: {}",
                shader,
                vertices.len(),
                vertices.len() / std::mem::size_of::<f32>(),
                indices.len(),
                indices.len() / std::mem::size_of::<u32>(),
            );
        }
    }

    #[inline]
    pub fn draw(
        &mut self,
        shader: &str,
        uniform_block: &[f32],
        framebuffer: &str,
        texture: &str,
        indices_start_offset: u32,
        indices_count: usize,
    ) {
        assert!(
            shader != "blit",
            "The blit shader does not support drawing operations"
        );

        self.framebuffers
            .get(framebuffer)
            .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer))
            .activate();

        self.shaders
            .get(shader)
            .unwrap_or_else(|| panic!("Shader '{}' not found", shader))
            .activate(uniform_block);

        // NOTE: We need to bind the texture after shader activation as it
        //       might have invalidated our texture unit
        self.textures
            .get(texture)
            .unwrap_or_else(|| panic!("Texture '{}' not found", texture))
            .activate(0);

        self.drawobjects
            .get(shader)
            .unwrap_or_else(|| panic!("Drawobject '{}' not found", shader))
            .draw(indices_start_offset, indices_count);

        if ENABLE_LOGS {
            log::trace!(
                "Drawing buffers:
        shader: '{}'
        framebuffer: '{}'
        texture: '{}'
        indices_start_offset: {}
        indices_count: {}",
                shader,
                framebuffer,
                texture,
                indices_start_offset,
                indices_count,
            );
        }
    }

    #[inline]
    pub fn texture_exists(&self, name: &str) -> bool {
        self.textures.contains_key(name)
    }

    #[inline]
    pub fn texture_create_or_update_whole(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) {
        if self.texture_exists(name) {
            self.texture_update_pixels(name, 0, 0, width, height, pixels);
        } else {
            self.texture_create(name.to_owned(), width, height, pixels);
        }
    }

    #[inline]
    pub fn texture_create(&mut self, name: String, width: u32, height: u32, pixels: &[u8]) {
        assert!(
            !self.textures.contains_key(&name),
            "Texture '{}' already exists",
            &name
        );
        if ENABLE_LOGS {
            log::debug!("Creating texture '{}' ({}x{})", &name, width, height);
        }
        self.textures.insert(
            name.clone(),
            Texture::new(self.gl.clone(), name.clone(), width, height),
        );
        self.texture_update_pixels(&name, 0, 0, width, height, pixels);
    }

    #[inline]
    pub fn texture_update_pixels(
        &mut self,
        texture: &str,
        region_offset_x: u32,
        region_offset_y: u32,
        region_width: u32,
        region_height: u32,
        pixels: &[u8],
    ) {
        if ENABLE_LOGS {
            log::debug!(
                "Updating texture '{}' (offset: {}x{}, dim: {}x{})",
                texture,
                region_offset_x,
                region_offset_y,
                region_width,
                region_height
            );
        }
        self.textures
            .get(texture)
            .unwrap_or_else(|| panic!("Texture '{}' not found", texture))
            .update_pixels(
                region_offset_x,
                region_offset_y,
                region_width,
                region_height,
                pixels,
            );
    }

    #[inline]
    pub fn texture_delete(&mut self, texture: &str) {
        if ENABLE_LOGS {
            log::debug!("Deleting texture '{}'", &texture);
        }
        self.textures
            .remove(texture)
            .unwrap_or_else(|| panic!("Texture '{}' not found", texture));
    }

    #[inline]
    pub fn framebuffer_exists(&self, name: &str) -> bool {
        self.framebuffers.contains_key(name)
    }

    #[inline]
    pub fn framebuffer_create_or_update(&mut self, name: &str, width: u32, height: u32) {
        if self.framebuffer_exists(name) {
            self.framebuffer_update(name, width, height);
        } else {
            self.framebuffer_create(name.to_owned(), width, height);
        }
    }

    #[inline]
    pub fn framebuffer_create(&mut self, name: String, width: u32, height: u32) {
        assert!(
            name != "screen",
            "Not allowed to create framebuffer with name 'screen'"
        );
        assert!(
            !self.framebuffers.contains_key(&name),
            "Framebuffer '{}' already exists",
            &name
        );

        if ENABLE_LOGS {
            log::debug!("Creating framebuffer '{}' ({}x{})", &name, width, height);
        }
        self.framebuffers.insert(
            name.clone(),
            Framebuffer::new(self.gl.clone(), name, width, height),
        );
    }

    #[inline]
    pub fn framebuffer_update(&mut self, framebuffer: &str, width: u32, height: u32) {
        assert!(
            framebuffer != "screen",
            "Not allowed to update framebuffer with name 'screen'"
        );
        {
            // If our framebuffer already has the given dimensions we do nothing
            let framebuffer = self
                .framebuffers
                .get(framebuffer)
                .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer));
            if framebuffer.width == width && framebuffer.height == height {
                // Nothing to do
                return;
            } else {
                if ENABLE_LOGS {
                    log::debug!(
                        "Updating framebuffer '{}' ({}x{}) -> ({}x{})",
                        &framebuffer.name,
                        framebuffer.width,
                        framebuffer.height,
                        width,
                        height
                    );
                }
            }
        }
        self.framebuffer_delete(framebuffer);
        self.framebuffer_create(framebuffer.to_owned(), width, height);
    }

    #[inline]
    pub fn framebuffer_delete(&mut self, framebuffer: &str) {
        assert!(
            framebuffer != "screen",
            "Not allowed to delete framebuffer with name 'screen'"
        );
        if ENABLE_LOGS {
            log::debug!("Deleting framebuffer '{}'", framebuffer);
        }
        self.framebuffers
            .remove(framebuffer)
            .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer));
    }

    #[inline]
    pub fn framebuffer_clear(
        &mut self,
        framebuffer: &str,
        new_color: Option<[f32; 4]>,
        new_depth: Option<f32>,
    ) {
        let framebuffer = self
            .framebuffers
            .get(framebuffer)
            .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer));
        framebuffer.activate();

        assert!(
            new_color.is_some() || new_depth.is_some(),
            "Clear command was empty for framebuffer '{}'",
            framebuffer.name
        );

        unsafe {
            let gl = &self.gl;
            let mut clear_mask = 0;
            if let Some(color) = new_color {
                clear_mask |= glow::COLOR_BUFFER_BIT;
                gl.clear_color(color[0], color[1], color[2], color[3]);
            }
            if let Some(depth) = new_depth {
                clear_mask |= glow::DEPTH_BUFFER_BIT;
                gl.clear_depth_f32(depth);
            }
            gl.clear(clear_mask);
        }
    }

    #[inline]
    pub fn framebuffer_blit(
        &mut self,
        framebuffer_source: &str,
        framebuffer_target: &str,
        rect_source: Recti,
        rect_target: Recti,
    ) {
        assert!(
            framebuffer_source != framebuffer_target,
            "Cannot blit from and to the same framebuffer '{:?}'",
            framebuffer_source,
        );

        let framebuffer_source = self
            .framebuffers
            .get(framebuffer_source)
            .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer_source));
        let framebuffer_target = self
            .framebuffers
            .get(framebuffer_target)
            .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer_target));

        if ENABLE_LOGS {
            log::trace!(
            "Blitting framebuffers '{}' (offset: {}x{}, dim: {}x{}) -> '{}' (offset: {}x{}, dim: {}x{})",
            &framebuffer_source.name,
            rect_source.pos.x,
            rect_source.pos.y,
            rect_source.dim.x,
            rect_source.dim.y,
            &framebuffer_target.name,
            rect_target.pos.x,
            rect_target.pos.y,
            rect_target.dim.x,
            rect_target.dim.y,
        );
        }
        unsafe {
            let gl = &self.gl;
            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);

            framebuffer_target.activate();

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
        self.shaders
            .get("blit")
            .expect("Blit shader not found '{}' not found")
            .activate(&transform.into_column_array());

        let (vertices, indices) = {
            let vert_left = rect_target.pos.x as f32;
            let vert_top = rect_target.pos.y as f32;
            let vert_right = (rect_target.pos.x + rect_target.width()) as f32;
            let vert_bottom = (rect_target.pos.y + rect_target.height()) as f32;

            let uvs_left = rect_source.pos.x as f32 / framebuffer_source.width as f32;
            let uvs_top = rect_source.pos.y as f32 / framebuffer_source.height as f32;
            let uvs_right =
                (rect_source.pos.x + rect_source.width()) as f32 / framebuffer_source.width as f32;
            let uvs_bottom = (rect_source.pos.y + rect_source.height()) as f32
                / framebuffer_source.height as f32;

            let vertices = [
                // right top
                vert_right,
                vert_top,
                uvs_right,
                uvs_top, //
                // right bottom
                vert_right,
                vert_bottom,
                uvs_right,
                uvs_bottom, //
                // left bottom
                vert_left,
                vert_bottom,
                uvs_left,
                uvs_bottom, //
                // left top
                vert_left,
                vert_top,
                uvs_left,
                uvs_top, //
            ];
            let indices = [
                // first triangle
                3, // left top
                0, // right top
                1, // right bottom
                // second triangle
                2, // left bottom
                1, // right bottom
                3, // left top
            ];
            (vertices, indices)
        };

        unsafe {
            let drawobject_blit = self
                .drawobjects
                .get("blit")
                .expect("Blit drawobject not found for shader");
            drawobject_blit.assign_buffers(
                transmute_slice_to_byte_slice(&vertices),
                transmute_slice_to_byte_slice(&indices),
            );
            drawobject_blit.draw(0, 6);
        }

        unsafe {
            let gl = &self.gl;
            gl.enable(glow::BLEND);
            gl.enable(glow::DEPTH_TEST);
        }
    }

    /// Draws the depthbuffer content of the given framebuffer onto itself
    #[inline]
    #[cfg(target_arch = "wasm32")]
    pub fn debug_draw_depthbuffer(&mut self, _framebuffer: &str) {
        // Not implemented yet
    }

    /// Draws the depthbuffer content of the given framebuffer onto itself
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn debug_draw_depthbuffer(&mut self, framebuffer: &str) {
        unsafe {
            let gl = &self.gl;
            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
        }

        let (framebuffer_width, framebuffer_height) = {
            let framebuffer = self
                .framebuffers
                .get(framebuffer)
                .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer));

            (framebuffer.width, framebuffer.height)
        };

        // Create pixels from normalized depthbuffer values
        let depthbuffer_pixels = {
            let depthbuffer_values = self.debug_read_depthbuffer(framebuffer);
            let (val_min, val_max) = {
                let val_min = depthbuffer_values
                    .iter()
                    .fold(std::f32::MAX, |acc, val| f32::min(acc, *val));
                let val_max = depthbuffer_values
                    .iter()
                    .fold(std::f32::MIN, |acc, val| f32::max(acc, *val));

                if val_min == val_max {
                    (0.0, 1.0)
                } else {
                    (val_min, val_max)
                }
            };

            let mut depthbuffer_pixels =
                vec![0u32; framebuffer_width as usize * framebuffer_height as usize];

            for (pixel, value) in depthbuffer_pixels.iter_mut().zip(depthbuffer_values.iter()) {
                let depth = (*value - val_min) / (val_max - val_min);

                let r = (255 as f32 * depth) as u32;
                let g = (255 as f32 * depth) as u32;
                let b = (255 as f32 * depth) as u32;
                let a = 255;

                *pixel = (a << 24) | (b << 16) | (g << 8) | (r << 0);
            }

            depthbuffer_pixels
        };

        // Upload depth pixel to texture
        unsafe {
            let depthbuffer_pixels_raw = transmute_slice_to_byte_slice(&depthbuffer_pixels[..]);
            self.texture_create_or_update_whole(
                "debug_depth",
                framebuffer_width,
                framebuffer_height,
                &depthbuffer_pixels_raw,
            );
        }

        // Draw texture back to framebuffer
        let transform = Mat4::ortho_origin_left_bottom(
            framebuffer_width as f32,
            framebuffer_height as f32,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
        );
        self.shaders
            .get("blit")
            .expect("Blit shader not found '{}' not found")
            .activate(&transform.into_column_array());
        self.textures
            .get("debug_depth")
            .expect("No depthbuffer texture")
            .activate(0);

        let (vertices, indices) = {
            let vert_left = 0.0;
            let vert_top = 0.0;
            let vert_right = framebuffer_width as f32;
            let vert_bottom = framebuffer_height as f32;

            let uv_left = 0.0;
            let uv_top = 0.0;
            let uv_right = 1.0;
            let uv_bottom = 1.0;

            let vertices = [
                // right top
                vert_right,
                vert_top,
                uv_right,
                uv_top, //
                // right bottom
                vert_right,
                vert_bottom,
                uv_right,
                uv_bottom, //
                // left bottom
                vert_left,
                vert_bottom,
                uv_left,
                uv_bottom, //
                // left top
                vert_left,
                vert_top,
                uv_left,
                uv_top, //
            ];
            let indices = [
                // first triangle
                3, // left top
                0, // right top
                1, // right bottom
                // second triangle
                2, // left bottom
                1, // right bottom
                3, // left top
            ];
            (vertices, indices)
        };

        unsafe {
            let drawobject_blit = self
                .drawobjects
                .get("blit")
                .expect("Blit drawobject not found for shader");
            drawobject_blit.assign_buffers(
                transmute_slice_to_byte_slice(&vertices),
                transmute_slice_to_byte_slice(&indices),
            );
            drawobject_blit.draw(0, 6);
        }
        unsafe {
            let gl = &self.gl;
            gl.enable(glow::BLEND);
            gl.enable(glow::DEPTH_TEST);
        }
    }

    fn debug_read_depthbuffer(&mut self, framebuffer: &str) -> Vec<f32> {
        let (framebuffer_width, framebuffer_height) = {
            let framebuffer = self
                .framebuffers
                .get(framebuffer)
                .unwrap_or_else(|| panic!("Framebuffer '{}' not found", framebuffer));

            (framebuffer.width, framebuffer.height)
        };

        let mut depthbuffer_values =
            vec![0.0f32; framebuffer_width as usize * framebuffer_height as usize];
        unsafe {
            let gl = &self.gl;
            let mut depthbuffer_values_raw =
                transmute_slice_to_byte_slice_mut(&mut depthbuffer_values[..]);
            gl.read_pixels(
                0,
                0,
                framebuffer_width as i32,
                framebuffer_height as i32,
                glow::DEPTH_COMPONENT,
                glow::FLOAT,
                glow::PixelPackData::Slice(&mut depthbuffer_values_raw),
            );
        }
        depthbuffer_values
    }
}
