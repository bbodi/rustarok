extern crate singleton_gl_dont_use_it as gl;

use crate::grf::texture::GlNativeTextureId;
use singleton_gl_dont_use_it::types::*;
use std::os::raw::c_void;

// OpenGL is usable only through this struct
// So errors caused by uninitialized OpenGL can be avoided
#[derive(Clone, Hash)]
pub struct Gl;

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum MyGlEnum {
    TEXTURE0 = gl::TEXTURE0 as isize,
    TEXTURE1 = gl::TEXTURE1 as isize,
    TEXTURE2 = gl::TEXTURE2 as isize,
    RGBA = gl::RGBA as isize,
    UNSIGNED_BYTE = gl::UNSIGNED_BYTE as isize,
    TEXTURE_2D = gl::TEXTURE_2D as isize,
    LINEAR = gl::LINEAR as isize,
    NEAREST = gl::NEAREST as isize,
    TRIANGLES = gl::TRIANGLES as isize,
    TRIANGLE_STRIP = gl::TRIANGLE_STRIP as isize,
    LINE_STRIP = gl::LINE_STRIP as isize,
    LINE_LOOP = gl::LINE_LOOP as isize,
    POINTS = gl::POINTS as isize,
    VERTEX_SHADER = gl::VERTEX_SHADER as isize,
    FRAGMENT_SHADER = gl::FRAGMENT_SHADER as isize,
    COLOR_BUFFER_BIT = gl::COLOR_BUFFER_BIT as isize,
    DEPTH_BUFFER_BIT = gl::DEPTH_BUFFER_BIT as isize,

    TEXTURE_MIN_FILTER = gl::TEXTURE_MIN_FILTER as isize,
    TEXTURE_MAG_FILTER = gl::TEXTURE_MAG_FILTER as isize,
    CLAMP_TO_EDGE = gl::CLAMP_TO_EDGE as isize,
    TEXTURE_WRAP_S = gl::TEXTURE_WRAP_S as isize,
    TEXTURE_WRAP_T = gl::TEXTURE_WRAP_T as isize,

    ARRAY_BUFFER = gl::ARRAY_BUFFER as isize,
    FLOAT = gl::FLOAT as isize,

    STATIC_DRAW = gl::STATIC_DRAW as isize,
    DYNAMIC_DRAW = gl::DYNAMIC_DRAW as isize,
    INFO_LOG_LENGTH = gl::INFO_LOG_LENGTH as isize,
    COMPILE_STATUS = gl::COMPILE_STATUS as isize,
    LINK_STATUS = gl::LINK_STATUS as isize,

    DEPTH_TEST = gl::DEPTH_TEST as isize,
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum MyGlBlendEnum {
    ZERO = gl::ZERO as isize,
    ONE = gl::ONE as isize,
    SRC_COLOR = gl::SRC_COLOR as isize,
    ONE_MINUS_SRC_COLOR = gl::ONE_MINUS_SRC_COLOR as isize,
    SRC_ALPHA = gl::SRC_ALPHA as isize,
    ONE_MINUS_SRC_ALPHA = gl::ONE_MINUS_SRC_ALPHA as isize,
    DST_ALPHA = gl::DST_ALPHA as isize,
    ONE_MINUS_DST_ALPHA = gl::ONE_MINUS_DST_ALPHA as isize,
    DST_COLOR = gl::DST_COLOR as isize,
    ONE_MINUS_DST_COLOR = gl::ONE_MINUS_DST_COLOR as isize,
    SRC_ALPHA_SATURATE = gl::SRC_ALPHA_SATURATE as isize,
    CONSTANT_COLOR = gl::CONSTANT_COLOR as isize,
    ONE_MINUS_CONSTANT_ALPHA = gl::ONE_MINUS_CONSTANT_ALPHA as isize,
}

#[allow(dead_code)]
impl Gl {
    pub fn new(
        video: &sdl2::VideoSubsystem,
        window: &sdl2::video::Window,
        width: i32,
        height: i32,
    ) -> (Gl, sdl2::video::GLContext) {
        // these two variables must be in scope, so don't remove their variables
        let gl_context = window.gl_create_context().unwrap();
        let _gl = gl::load_with(|s| video.gl_get_proc_address(s) as *const c_void);
        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.3, 0.3, 0.5, 1.0);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::LineWidth(2.0);
            gl::PointSize(2.0);
        }
        return (Gl, gl_context);
    }

    pub fn viewport(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            gl::Viewport(x, y, w, h);
        }
    }

    pub unsafe fn gen_buffers(&self, n: GLsizei, buffers: *mut GLuint) {
        gl::GenBuffers(n, buffers);
    }

    pub unsafe fn bind_buffer(&self, target: MyGlEnum, buffer: GLuint) {
        gl::BindBuffer(target as u32, buffer);
    }

    pub unsafe fn buffer_data(
        &self,
        target: MyGlEnum,
        size: GLsizeiptr,
        data: *const c_void,
        usage: MyGlEnum,
    ) {
        gl::BufferData(target as u32, size, data, usage as u32);
    }

    pub unsafe fn gen_vertex_arrays(&self, n: GLsizei, arrays: *mut GLuint) {
        gl::GenVertexArrays(n, arrays);
    }

    pub unsafe fn bind_vertex_array(&self, array: GLuint) {
        gl::BindVertexArray(array);
    }

    pub unsafe fn enable_vertex_attrib_array(&self, index: GLuint) {
        gl::EnableVertexAttribArray(index);
    }

    pub unsafe fn vertex_attrib_pointer(
        &self,
        index: GLuint,
        size: GLint,
        type_: MyGlEnum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const c_void,
    ) {
        gl::VertexAttribPointer(index, size, type_ as u32, normalized, stride, pointer);
    }

    pub unsafe fn draw_arrays(&self, mode: MyGlEnum, first: GLint, count: GLsizei) {
        gl::DrawArrays(mode as u32, first, count);
    }

    pub unsafe fn bind_texture(&self, target: MyGlEnum, texture: GlNativeTextureId) {
        gl::BindTexture(target as u32, texture.0);
    }

    pub unsafe fn active_texture(&self, texture: MyGlEnum) {
        gl::ActiveTexture(texture as u32);
    }

    pub unsafe fn get_tex_image(
        &self,
        target: MyGlEnum,
        level: GLint,
        format: MyGlEnum,
        type_: MyGlEnum,
        pixels: *mut c_void,
    ) {
        gl::GetTexImage(target as u32, level, format as u32, type_ as u32, pixels);
    }

    pub unsafe fn tex_parameteri(&self, target: MyGlEnum, pname: MyGlEnum, param: GLint) {
        gl::TexParameteri(target as u32, pname as u32, param);
    }

    pub unsafe fn generate_mipmap(&self, target: MyGlEnum) {
        gl::GenerateMipmap(target as u32);
    }

    pub unsafe fn disable_vertex_attrib_array(&self, index: GLuint) {
        gl::DisableVertexAttribArray(index);
    }

    pub unsafe fn clear(&self, mask: GLbitfield) {
        gl::Clear(mask);
    }

    pub unsafe fn gen_textures(&self, n: GLsizei, textures: *mut GLuint) {
        gl::GenTextures(n, textures);
    }

    pub unsafe fn tex_image2d(
        &self,
        target: MyGlEnum,
        level: GLint,
        internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: MyGlEnum,
        type_: MyGlEnum,
        pixels: *const c_void,
    ) {
        gl::TexImage2D(
            target as u32,
            level,
            internalformat,
            width,
            height,
            border,
            format as u32,
            type_ as u32,
            pixels,
        );
    }

    pub unsafe fn delete_textures(&self, n: GLsizei, textures: *const GLuint) {
        gl::DeleteTextures(n, textures);
    }

    pub unsafe fn delete_buffers(&self, n: GLsizei, buffers: *const GLuint) {
        gl::DeleteBuffers(n, buffers);
    }

    pub unsafe fn delete_vertex_arrays(&self, n: GLsizei, buffers: *const GLuint) {
        gl::DeleteVertexArrays(n, buffers);
    }

    pub unsafe fn delete_shader(&self, n: GLuint) {
        gl::DeleteShader(n);
    }

    pub unsafe fn shader_source(
        &self,
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ) {
        gl::ShaderSource(shader, count, string, length);
    }

    pub unsafe fn compile_shader(&self, n: GLuint) {
        gl::CompileShader(n);
    }

    pub unsafe fn get_programiv(&self, program: GLuint, pname: MyGlEnum, params: *mut GLint) {
        gl::GetProgramiv(program, pname as u32, params);
    }

    pub unsafe fn get_shaderiv(&self, program: GLuint, pname: MyGlEnum, params: *mut GLint) {
        gl::GetShaderiv(program, pname as u32, params);
    }

    pub unsafe fn get_uniform_location(&self, program: GLuint, name: *const GLchar) -> GLint {
        return gl::GetUniformLocation(program, name);
    }

    pub unsafe fn create_shader(&self, kind: MyGlEnum) -> GLuint {
        return gl::CreateShader(kind as u32);
    }

    pub unsafe fn get_program_info_log(
        &self,
        program: GLuint,
        buf_size: GLsizei,
        length: *mut GLsizei,
        info_log: *mut GLchar,
    ) {
        gl::GetProgramInfoLog(program, buf_size, length, info_log);
    }

    pub unsafe fn get_shader_info_log(
        &self,
        program: GLuint,
        buf_size: GLsizei,
        length: *mut GLsizei,
        info_log: *mut GLchar,
    ) {
        gl::GetShaderInfoLog(program, buf_size, length, info_log);
    }

    pub unsafe fn uniform_matrix3fv(
        &self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gl::UniformMatrix3fv(location, count, transpose, value);
    }

    pub unsafe fn uniform_matrix4fv(
        &self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gl::UniformMatrix4fv(location, count, transpose, value);
    }

    pub unsafe fn uniform3fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform3fv(location, count, value);
    }

    pub unsafe fn uniform4fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform4fv(location, count, value);
    }

    pub unsafe fn uniform2fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform2fv(location, count, value);
    }

    pub unsafe fn uniform2i(&self, location: GLint, a: GLint, b: GLint) {
        gl::Uniform2i(location, a, b);
    }

    pub unsafe fn uniform1f(&self, location: GLint, value: GLfloat) {
        gl::Uniform1f(location, value);
    }

    pub unsafe fn uniform1i(&self, location: GLint, value: GLint) {
        gl::Uniform1i(location, value);
    }

    pub unsafe fn create_program(&self) -> GLuint {
        return gl::CreateProgram();
    }

    pub unsafe fn attach_shader(&self, program: GLuint, shader: GLuint) {
        gl::AttachShader(program, shader);
    }

    pub unsafe fn disable(&self, n: MyGlEnum) {
        gl::Disable(n as u32);
    }

    pub unsafe fn enable(&self, n: MyGlEnum) {
        gl::Enable(n as u32);
    }

    pub unsafe fn blend_func(&self, n: MyGlBlendEnum, b: MyGlBlendEnum) {
        gl::BlendFunc(n as u32, b as u32);
    }

    pub unsafe fn detach_shader(&self, program: GLuint, shader: GLuint) {
        gl::DetachShader(program, shader);
    }

    pub unsafe fn use_program(&self, program: GLuint) {
        gl::UseProgram(program);
    }

    pub unsafe fn delete_program(&self, program: GLuint) {
        gl::DeleteProgram(program);
    }

    pub unsafe fn link_program(&self, program: GLuint) {
        gl::LinkProgram(program);
    }
}
