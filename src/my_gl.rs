extern crate singleton_gl_dont_use_it as gl;

use singleton_gl_dont_use_it::types::*;
use std::os::raw::c_void;

// OpenGL is usable only through this struct
// So errors caused by uninitialized OpenGL can be avoided
#[derive(Clone, Hash)]
pub struct Gl;

pub const TEXTURE0: GLenum = gl::TEXTURE0;
pub const RGBA: GLenum = gl::RGBA;
pub const UNSIGNED_BYTE: GLenum = gl::UNSIGNED_BYTE;
pub const TEXTURE_2D: GLenum = gl::TEXTURE_2D;
pub const LINEAR: GLenum = gl::LINEAR;
pub const NEAREST: GLenum = gl::NEAREST;
pub const TRIANGLES: GLenum = gl::TRIANGLES;
pub const TRIANGLE_STRIP: GLenum = gl::TRIANGLE_STRIP;
pub const LINE_STRIP: GLenum = gl::LINE_STRIP;
pub const LINE_LOOP: GLenum = gl::LINE_LOOP;
pub const ZERO: GLenum = gl::ZERO;
pub const ONE: GLenum = gl::ONE;
pub const VERTEX_SHADER: GLenum = gl::VERTEX_SHADER;
pub const FRAGMENT_SHADER: GLenum = gl::FRAGMENT_SHADER;

pub const COLOR_BUFFER_BIT: GLenum = gl::COLOR_BUFFER_BIT;
pub const DEPTH_BUFFER_BIT: GLenum = gl::DEPTH_BUFFER_BIT;

pub const TEXTURE_MIN_FILTER: GLenum = gl::TEXTURE_MIN_FILTER;
pub const TEXTURE_MAG_FILTER: GLenum = gl::TEXTURE_MAG_FILTER;
pub const CLAMP_TO_EDGE: GLenum = gl::CLAMP_TO_EDGE;
pub const TEXTURE_WRAP_S: GLenum = gl::TEXTURE_WRAP_S;
pub const TEXTURE_WRAP_T: GLenum = gl::TEXTURE_WRAP_T;

pub const ARRAY_BUFFER: GLenum = gl::ARRAY_BUFFER;
pub const FLOAT: GLenum = gl::FLOAT;
pub const FALSE: GLboolean = gl::FALSE;

pub const STATIC_DRAW: GLenum = gl::STATIC_DRAW;
pub const INFO_LOG_LENGTH: GLenum = gl::INFO_LOG_LENGTH;
pub const COMPILE_STATUS: GLenum = gl::COMPILE_STATUS;
pub const LINK_STATUS: GLenum = gl::LINK_STATUS;

pub const SRC_COLOR: GLenum = gl::SRC_COLOR;
pub const ONE_MINUS_SRC_COLOR: GLenum = gl::ONE_MINUS_SRC_COLOR;
pub const SRC_ALPHA: GLenum = gl::SRC_ALPHA;
pub const ONE_MINUS_SRC_ALPHA: GLenum = gl::ONE_MINUS_SRC_ALPHA;
pub const DST_ALPHA: GLenum = gl::DST_ALPHA;
pub const ONE_MINUS_DST_ALPHA: GLenum = gl::ONE_MINUS_DST_ALPHA;
pub const DST_COLOR: GLenum = gl::DST_COLOR;
pub const ONE_MINUS_DST_COLOR: GLenum = gl::ONE_MINUS_DST_COLOR;
pub const SRC_ALPHA_SATURATE: GLenum = gl::SRC_ALPHA_SATURATE;
pub const CONSTANT_COLOR: GLenum = gl::CONSTANT_COLOR;
pub const ONE_MINUS_CONSTANT_ALPHA: GLenum = gl::ONE_MINUS_CONSTANT_ALPHA;

pub const DEPTH_TEST: GLenum = gl::DEPTH_TEST;

impl Gl {
    pub fn new(
        video: &sdl2::VideoSubsystem,
        window: &sdl2::video::Window,
        width: i32,
        height: i32,
    ) -> (Gl, sdl2::video::GLContext) {
        // these two variables must be in scope, so don't remove their variables
        let gl_context = window.gl_create_context().unwrap();
        let _gl = gl::load_with(|s| video.gl_get_proc_address(s) as *const std::os::raw::c_void);
        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.3, 0.3, 0.5, 1.0);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::LineWidth(2.0);
        }
        return (Gl, gl_context);
    }

    pub unsafe fn GenBuffers(&self, n: GLsizei, buffers: *mut GLuint) {
        gl::GenBuffers(n, buffers);
    }

    pub unsafe fn BindBuffer(&self, target: GLenum, buffer: GLuint) {
        gl::BindBuffer(target, buffer);
    }

    pub unsafe fn BufferData(
        &self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const c_void,
        usage: GLenum,
    ) {
        gl::BufferData(target, size, data, usage);
    }

    pub unsafe fn GenVertexArrays(&self, n: GLsizei, arrays: *mut GLuint) {
        gl::GenVertexArrays(n, arrays);
    }

    pub unsafe fn BindVertexArray(&self, array: GLuint) {
        gl::BindVertexArray(array);
    }

    pub unsafe fn EnableVertexAttribArray(&self, index: GLuint) {
        gl::EnableVertexAttribArray(index);
    }

    pub unsafe fn VertexAttribPointer(
        &self,
        index: GLuint,
        size: GLint,
        type_: GLenum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const c_void,
    ) {
        gl::VertexAttribPointer(index, size, type_, normalized, stride, pointer);
    }

    pub unsafe fn DrawArrays(&self, mode: GLenum, first: GLint, count: GLsizei) {
        gl::DrawArrays(mode, first, count);
    }

    pub unsafe fn BindTexture(&self, target: GLenum, texture: GLuint) {
        gl::BindTexture(target, texture);
    }

    pub unsafe fn ActiveTexture(&self, texture: GLenum) {
        gl::ActiveTexture(texture);
    }

    pub unsafe fn GetTexImage(
        &self,
        target: GLenum,
        level: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *mut c_void,
    ) {
        gl::GetTexImage(target, level, format, type_, pixels);
    }

    pub unsafe fn TexParameteri(&self, target: GLenum, pname: GLenum, param: GLint) {
        gl::TexParameteri(target, pname, param);
    }

    pub unsafe fn GenerateMipmap(&self, target: GLenum) {
        gl::GenerateMipmap(target);
    }

    pub unsafe fn DisableVertexAttribArray(&self, index: GLuint) {
        gl::DisableVertexAttribArray(index);
    }

    pub unsafe fn Clear(&self, mask: GLbitfield) {
        gl::Clear(mask);
    }

    pub unsafe fn GenTextures(&self, n: GLsizei, textures: *mut GLuint) {
        gl::GenTextures(n, textures);
    }

    pub unsafe fn TexImage2D(
        &self,
        target: GLenum,
        level: GLint,
        internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const c_void,
    ) {
        gl::TexImage2D(
            target,
            level,
            internalformat,
            width,
            height,
            border,
            format,
            type_,
            pixels,
        );
    }

    pub unsafe fn DeleteTextures(&self, n: GLsizei, textures: *const GLuint) {
        gl::DeleteTextures(n, textures);
    }

    pub unsafe fn DeleteBuffers(&self, n: GLsizei, buffers: *const GLuint) {
        gl::DeleteBuffers(n, buffers);
    }

    pub unsafe fn DeleteVertexArrays(&self, n: GLsizei, buffers: *const GLuint) {
        gl::DeleteVertexArrays(n, buffers);
    }

    pub unsafe fn DeleteShader(&self, n: GLuint) {
        gl::DeleteShader(n);
    }

    pub unsafe fn ShaderSource(
        &self,
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ) {
        gl::ShaderSource(shader, count, string, length);
    }

    pub unsafe fn CompileShader(&self, n: GLuint) {
        gl::CompileShader(n);
    }

    pub unsafe fn GetProgramiv(&self, program: GLuint, pname: GLenum, params: *mut GLint) {
        gl::GetProgramiv(program, pname, params);
    }

    pub unsafe fn GetShaderiv(&self, program: GLuint, pname: GLenum, params: *mut GLint) {
        gl::GetShaderiv(program, pname, params);
    }

    pub unsafe fn GetUniformLocation(&self, program: GLuint, name: *const GLchar) -> GLint {
        return gl::GetUniformLocation(program, name);
    }

    pub unsafe fn CreateShader(&self, kind: GLenum) -> GLuint {
        return gl::CreateShader(kind);
    }

    pub unsafe fn GetProgramInfoLog(
        &self,
        program: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gl::GetProgramInfoLog(program, bufSize, length, infoLog);
    }

    pub unsafe fn GetShaderInfoLog(
        &self,
        program: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gl::GetShaderInfoLog(program, bufSize, length, infoLog);
    }

    pub unsafe fn UniformMatrix3fv(
        &self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gl::UniformMatrix3fv(location, count, transpose, value);
    }

    pub unsafe fn UniformMatrix4fv(
        &self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gl::UniformMatrix4fv(location, count, transpose, value);
    }

    pub unsafe fn Uniform3fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform3fv(location, count, value);
    }

    pub unsafe fn Uniform4fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform4fv(location, count, value);
    }

    pub unsafe fn Uniform2fv(&self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gl::Uniform2fv(location, count, value);
    }

    pub unsafe fn Uniform2i(&self, location: GLint, a: GLint, b: GLint) {
        gl::Uniform2i(location, a, b);
    }

    pub unsafe fn Uniform1f(&self, location: GLint, value: GLfloat) {
        gl::Uniform1f(location, value);
    }

    pub unsafe fn Uniform1i(&self, location: GLint, value: GLint) {
        gl::Uniform1i(location, value);
    }

    pub unsafe fn CreateProgram(&self) -> GLuint {
        return gl::CreateProgram();
    }

    pub unsafe fn AttachShader(&self, program: GLuint, shader: GLuint) {
        gl::AttachShader(program, shader);
    }

    pub unsafe fn Disable(&self, n: GLenum) {
        gl::Disable(n);
    }

    pub unsafe fn Enable(&self, n: GLenum) {
        gl::Enable(n);
    }

    pub unsafe fn BlendFunc(&self, n: GLenum, b: GLenum) {
        gl::BlendFunc(n, b);
    }

    pub unsafe fn DetachShader(&self, program: GLuint, shader: GLuint) {
        gl::DetachShader(program, shader);
    }

    pub unsafe fn UseProgram(&self, program: GLuint) {
        gl::UseProgram(program);
    }

    pub unsafe fn DeleteProgram(&self, program: GLuint) {
        gl::DeleteProgram(program);
    }

    pub unsafe fn LinkProgram(&self, program: GLuint) {
        gl::LinkProgram(program);
    }
}
