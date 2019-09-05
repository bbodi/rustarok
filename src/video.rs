use crate::asset::database::AssetDatabase;
use crate::asset::AssetLoader;
use byteorder::{LittleEndian, WriteBytesExt};
use imgui::ImGui;
use imgui_opengl_renderer::Renderer;
use imgui_sdl2::ImguiSdl2;
use nalgebra::{Matrix3, Matrix4};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::BlendMode;
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{GLContext, Window};
use sdl2::EventPump;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::ops::{Index, IndexMut};
use std::path::Path;
use std::sync::Arc;

pub struct Video {
    pub window: Window,
    pub imgui: ImGui,
    pub imgui_sdl2: ImguiSdl2,
    pub renderer: Renderer,
    pub event_pump: EventPump,
    // this variable must be in scope, so don't remove it
    _gl_context: GLContext,
}

pub const VIDEO_WIDTH: u32 = 1024;
pub const VIDEO_HEIGHT: u32 = 768;

impl Video {
    pub fn init(sdl_context: &sdl2::Sdl) -> Video {
        sdl_context.mouse().show_cursor(false);
        let video = sdl_context.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 5);
        let window = video
            .window("Rustarok", VIDEO_WIDTH, VIDEO_HEIGHT)
            .opengl()
            .allow_highdpi()
            //            .resizable()
            .build()
            .unwrap();
        // these two variables must be in scope, so don't remove their variables
        let _gl_context = window.gl_create_context().unwrap();
        let _gl = gl::load_with(|s| video.gl_get_proc_address(s) as *const std::os::raw::c_void);
        unsafe {
            gl::Viewport(0, 0, VIDEO_WIDTH as i32, VIDEO_HEIGHT as i32); // set viewport
            gl::ClearColor(0.3, 0.3, 0.5, 1.0);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::LineWidth(2.0);
        }
        let mut imgui = imgui::ImGui::init();
        imgui.set_ini_filename(None);
        let imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui);
        let renderer =
            imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);
        let event_pump = sdl_context.event_pump().unwrap();
        Video {
            window,
            imgui,
            imgui_sdl2,
            renderer,
            event_pump,
            _gl_context,
        }
    }

    pub fn gl_swap_window(&self) {
        self.window.gl_swap_window();
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title).unwrap();
    }

    //    ttf_context,let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    //    fonts: HashMap::new(),
    //    ttf_context: Sdl2TtfContext,
    //    fonts: HashMap<FontId, sdl2::ttf::Font<'a,'b>>
    pub fn load_font<'a, 'b>(
        ttf_context: &'a Sdl2TtfContext,
        font_path: &str,
        size: u16,
    ) -> Result<sdl2::ttf::Font<'a, 'b>, String> {
        ttf_context.load_font(font_path, size)
    }

    pub fn create_text_texture<'a, 'b>(
        font: &sdl2::ttf::Font<'a, 'b>,
        text: &str,
        asset_database: &mut AssetDatabase,
    ) -> GlTexture {
        let surface = font
            .render(text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();
        return AssetLoader::create_texture_from_surface(
            &format!("text_{}", text),
            surface,
            gl::LINEAR,
            asset_database,
        );
    }

    pub fn create_text_texture_inner<'a, 'b>(
        font: &sdl2::ttf::Font<'a, 'b>,
        text: &str,
    ) -> GlTexture {
        let surface = font
            .render(text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();
        return AssetLoader::create_texture_from_surface_inner(surface, gl::LINEAR);
    }

    pub fn create_outline_text_texture<'a, 'b>(
        font: &sdl2::ttf::Font<'a, 'b>,
        outline_font: &sdl2::ttf::Font<'a, 'b>,
        text: &str,
        asset_database: &mut AssetDatabase,
    ) -> GlTexture {
        let mut bg_surface = outline_font
            .render(text)
            .blended(Color::RGBA(0, 0, 0, 255))
            .unwrap();
        let mut fg_surface = font
            .render(text)
            .blended(Color::RGBA(255, 255, 255, 255))
            .unwrap();
        fg_surface.set_blend_mode(BlendMode::Blend).unwrap();
        fg_surface
            .blit(
                None,
                &mut bg_surface,
                sdl2::rect::Rect::new(
                    outline_font.get_outline_width() as i32,
                    outline_font.get_outline_width() as i32,
                    fg_surface.width(),
                    fg_surface.height(),
                ),
            )
            .unwrap();
        return AssetLoader::create_texture_from_surface(
            &format!("outlinetext_{}", text),
            bg_surface,
            gl::NEAREST,
            asset_database,
        );
    }
}

pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, znear: f32, zfar: f32) -> Matrix4<f32> {
    let two = 2.0;
    let mut mat = Matrix4::<f32>::identity();

    mat[(0, 0)] = two / (right - left);
    mat[(0, 3)] = -(right + left) / (right - left);
    mat[(1, 1)] = two / (top - bottom);
    mat[(1, 3)] = -(top + bottom) / (top - bottom);
    mat[(2, 2)] = -two / (zfar - znear);
    mat[(2, 3)] = -(zfar + znear) / (zfar - znear);

    mat
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct GlTextureContext(GlNativeTextureId);

impl Drop for GlTextureContext {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &(self.0).0 as *const gl::types::GLuint) }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct GlTexture {
    context: Arc<GlTextureContext>,
    pub width: i32,
    pub height: i32,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Serialize)]
pub struct GlNativeTextureId(pub gl::types::GLuint);

pub const TEXTURE_0: GlNativeTextureId = GlNativeTextureId(gl::TEXTURE0);
pub const TEXTURE_1: GlNativeTextureId = GlNativeTextureId(gl::TEXTURE1);
pub const TEXTURE_2: GlNativeTextureId = GlNativeTextureId(gl::TEXTURE2);

impl GlTexture {
    pub fn new(texture_id: GlNativeTextureId, width: i32, height: i32) -> GlTexture {
        GlTexture {
            context: Arc::new(GlTextureContext(texture_id)),
            width,
            height,
        }
    }

    pub fn id(&self) -> GlNativeTextureId {
        (self.context.0).clone()
    }

    pub fn bind(&self, texture_index: GlNativeTextureId) {
        unsafe {
            gl::ActiveTexture(texture_index.0);
            gl::BindTexture(gl::TEXTURE_2D, (self.context.0).0);
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P, asset_database: &mut AssetDatabase) -> GlTexture
    where
        P: Display,
    {
        use sdl2::image::LoadSurface;
        let mut surface = sdl2::surface::Surface::from_file(&path).unwrap();
        let mut optimized_surf =
            sdl2::surface::Surface::new(surface.width(), surface.height(), PixelFormatEnum::RGBA32)
                .unwrap();
        surface
            .set_color_key(true, Color::RGB(255, 0, 255))
            .unwrap();
        surface.blit(None, &mut optimized_surf, None).unwrap();
        log::trace!("Texture from file --> {}", &path);
        return AssetLoader::create_texture_from_surface(
            &path.to_string(),
            optimized_surf,
            gl::NEAREST,
            asset_database,
        );
    }
}

#[derive(Clone, Debug)]
pub struct VertexAttribDefinition {
    pub number_of_components: usize,
    pub offset_of_first_element: usize,
}

pub struct VertexArrayBind<'a> {
    vertex_array: &'a VertexArray,
}

impl<'a> VertexArrayBind<'a> {
    pub fn draw(&self) {
        unsafe {
            gl::DrawArrays(
                self.vertex_array.draw_mode,           // mode
                0,                                     // starting index in the enabled arrays
                self.vertex_array.vertex_count as i32, // number of indices to be rendered
            );
        }
    }
}

impl<'a> Drop for VertexArrayBind<'a> {
    fn drop(&mut self) {
        unsafe {
            for (i, _def) in self
                .vertex_array
                .vertex_attrib_pointer_defs
                .iter()
                .enumerate()
            {
                gl::DisableVertexAttribArray(i as u32);
            }
            gl::BindVertexArray(0);
        }
    }
}

#[derive(Debug, Clone)]
struct VertexArrayResource {
    buffer_id: gl::types::GLuint,
    vertex_array_id: gl::types::GLuint,
}

impl Drop for VertexArrayResource {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer_id);
            gl::DeleteVertexArrays(1, &self.vertex_array_id);
        }
    }
}

#[derive(Clone, Debug)]
pub struct VertexArray {
    pub raw: Vec<u8>,
    buffers: Arc<VertexArrayResource>,
    vertex_count: usize,
    stride: gl::types::GLint,
    vertex_attrib_pointer_defs: Vec<VertexAttribDefinition>,
    draw_mode: u32,
}

impl VertexArray {
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn bind(&self) -> VertexArrayBind {
        unsafe {
            gl::BindVertexArray(self.buffers.vertex_array_id);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.buffers.buffer_id);

            for (i, def) in self.vertex_attrib_pointer_defs.iter().enumerate() {
                gl::EnableVertexAttribArray(i as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT,   // data type
                    gl::FALSE,   // normalized (int-to-float conversion)
                    self.stride, // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element)
                        as *const gl::types::GLvoid,
                );
            }

            VertexArrayBind {
                vertex_array: &self,
            }
        }
    }

    pub fn write_into(&self, dst_buf: &mut Vec<u8>) {
        dst_buf
            .write_u32::<LittleEndian>(self.vertex_count as u32)
            .unwrap();
        dst_buf
            .write_u32::<LittleEndian>(self.raw.len() as u32)
            .unwrap();
        dst_buf.extend_from_slice(self.raw.as_slice());
    }

    pub fn new<T>(
        draw_mode: u32,
        mut vertices: Vec<T>,
        definitions: Vec<VertexAttribDefinition>,
    ) -> VertexArray {
        let vertex_count = vertices.len();
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,                                                     // target
                (vertices.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr, // size of data in bytes
                vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::STATIC_DRAW,                               // usage
            );
        }
        let mut vao: gl::types::GLuint = 0;
        let stride = (std::mem::size_of::<T>()) as gl::types::GLint;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            for (i, def) in definitions.iter().enumerate() {
                gl::EnableVertexAttribArray(i as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT, // data type
                    gl::FALSE, // normalized (int-to-float conversion)
                    stride,    // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element)
                        as *const gl::types::GLvoid,
                );
            }
        }

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        let p = vertices.as_mut_ptr();
        let len = vertices.len() * std::mem::size_of::<T>();
        let cap = vertices.capacity() * std::mem::size_of::<T>();
        std::mem::forget(vertices);
        let raw: Vec<u8> = unsafe { Vec::from_raw_parts(p as *mut u8, len, cap) };
        VertexArray {
            raw,
            draw_mode,
            buffers: Arc::new(VertexArrayResource {
                buffer_id: vbo,
                vertex_array_id: vao,
            }),
            vertex_count,
            stride,
            vertex_attrib_pointer_defs: definitions,
        }
    }
}

pub struct DynamicVertexArrayBind<'a> {
    vertex_array: &'a DynamicVertexArray,
}

impl<'a> DynamicVertexArrayBind<'a> {
    pub fn draw(&self) {
        unsafe {
            gl::DrawArrays(
                self.vertex_array.draw_mode,           // mode
                0,                                     // starting index in the enabled arrays
                self.vertex_array.vertex_count as i32, // number of indices to be rendered
            );
        }
    }
}

impl<'a> Drop for DynamicVertexArrayBind<'a> {
    fn drop(&mut self) {
        unsafe {
            for (i, _def) in self
                .vertex_array
                .vertex_attrib_pointer_defs
                .iter()
                .enumerate()
            {
                gl::DisableVertexAttribArray(i as u32);
            }
            gl::BindVertexArray(0);
        }
    }
}

pub struct DynamicVertexArray {
    buffer_id: gl::types::GLuint,
    vertex_array_id: gl::types::GLuint,
    vertex_count: usize,
    stride: gl::types::GLint,
    vertex_attrib_pointer_defs: Vec<VertexAttribDefinition>,
    draw_mode: u32,
    buffer: Vec<f32>,
}

impl Index<usize> for DynamicVertexArray {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        self.buffer.index(index)
    }
}

impl IndexMut<usize> for DynamicVertexArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.buffer.index_mut(index)
    }
}

impl DynamicVertexArray {
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn bind(&self) -> DynamicVertexArrayBind {
        unsafe {
            gl::BindVertexArray(self.vertex_array_id);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer_id);

            for (attrib_location, def) in self.vertex_attrib_pointer_defs.iter().enumerate() {
                gl::EnableVertexAttribArray(attrib_location as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    attrib_location as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT,   // data type
                    gl::FALSE,   // normalized (int-to-float conversion)
                    self.stride, // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element)
                        as *const gl::types::GLvoid,
                );
            }

            gl::BufferData(
                gl::ARRAY_BUFFER,                                                          // target
                (self.buffer.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, // size of data in bytes
                self.buffer.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::DYNAMIC_DRAW,                                 // usage
            );

            DynamicVertexArrayBind {
                vertex_array: &self,
            }
        }
    }

    pub fn new(
        draw_mode: u32,
        vertices: Vec<f32>,
        vertex_count: usize,
        definitions: Vec<VertexAttribDefinition>,
    ) -> DynamicVertexArray {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,                                                       // target
                (vertices.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr, // size of data in bytes
                vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::DYNAMIC_DRAW,                              // usage
            );
        }
        let mut vao: gl::types::GLuint = 0;
        let component_count_for_one_vertex: usize =
            definitions.iter().map(|def| def.number_of_components).sum();
        let stride =
            (component_count_for_one_vertex * std::mem::size_of::<f32>()) as gl::types::GLint;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            for (attrib_location, def) in definitions.iter().enumerate() {
                gl::EnableVertexAttribArray(attrib_location as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    attrib_location as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT, // data type
                    gl::FALSE, // normalized (int-to-float conversion)
                    stride,    // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element)
                        as *const gl::types::GLvoid,
                );
            }
        }

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        DynamicVertexArray {
            draw_mode,
            buffer_id: vbo,
            vertex_array_id: vao,
            vertex_count,
            stride,
            vertex_attrib_pointer_defs: definitions,
            buffer: vertices,
        }
    }
}

impl Drop for DynamicVertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer_id);
            gl::DeleteVertexArrays(1, &self.vertex_array_id);
        }
    }
}

pub struct Shader {
    id: gl::types::GLuint,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

impl Shader {
    pub fn from_source(source: &str, kind: gl::types::GLenum) -> Result<Shader, String> {
        let c_str: &CStr = &CString::new(source).unwrap();
        let id = unsafe { gl::CreateShader(kind) };
        unsafe {
            gl::ShaderSource(id, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(id);
        }
        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }
        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe {
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
            }
            let mut buffer = Vec::<u8>::with_capacity(len as usize);
            unsafe {
                gl::GetShaderInfoLog(
                    id,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut i8,
                );
                buffer.set_len(len as usize);
                Err(String::from_utf8_unchecked(buffer))
            }
        } else {
            Ok(Shader { id })
        }
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

pub struct ShaderProgram {
    id: gl::types::GLuint,
}

pub struct ActiveShaderProgram {
    id: gl::types::GLuint,
}

impl ActiveShaderProgram {
    pub fn set_mat4(&self, name: &str, matrix: &Matrix4<f32>) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::UniformMatrix4fv(
                location,
                1,         // count
                gl::FALSE, // transpose
                matrix.as_slice().as_ptr() as *const f32,
            );
        }
    }

    pub fn set_mat3(&self, name: &str, matrix: &Matrix3<f32>) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::UniformMatrix3fv(
                location,
                1,         // count
                gl::FALSE, // transpose
                matrix.as_slice().as_ptr() as *const f32,
            );
        }
    }

    pub fn set_vec3(&self, name: &str, vector: &[f32; 3]) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform3fv(
                location,
                1, // count
                vector.as_ptr() as *const f32,
            );
        }
    }

    pub fn set_vec2(&self, name: &str, vector: &[f32; 2]) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform2fv(
                location,
                1, // count
                vector.as_ptr() as *const f32,
            );
        }
    }

    pub fn set_vec4(&self, name: &str, vector: &[f32; 4]) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform4fv(
                location,
                1, // count
                vector.as_ptr() as *const f32,
            );
        }
    }

    pub fn set_vec4u8(&self, name: &str, vector: &[u8; 4]) {
        // TODO: save uniformlocations and reuse them
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        let f32_vec = vec![
            vector[0] as f32 / 255.0,
            vector[1] as f32 / 255.0,
            vector[2] as f32 / 255.0,
            vector[3] as f32 / 255.0,
        ];
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform4fv(
                location,
                1, // count
                f32_vec.as_ptr() as *const f32,
            );
        }
    }

    pub fn set_int(&self, name: &str, value: i32) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform1i(location, value);
        }
    }

    pub fn set_f32(&self, name: &str, value: f32) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location =
                gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform1f(location, value);
        }
    }
}

impl ShaderProgram {
    pub fn gl_use(&self) -> ActiveShaderProgram {
        unsafe {
            gl::UseProgram(self.id);
        }
        ActiveShaderProgram { id: self.id }
    }

    pub fn from_shaders(shaders: &[Shader]) -> Result<ShaderProgram, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(program_id, shader.id());
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            return ShaderProgram::get_program_err(program_id);
        }

        for shader in shaders {
            unsafe {
                gl::DetachShader(program_id, shader.id());
            }
        }

        Ok(ShaderProgram { id: program_id })
    }

    fn get_program_err(program_id: gl::types::GLuint) -> Result<ShaderProgram, String> {
        let mut len: gl::types::GLint = 0;
        unsafe {
            gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
        }
        let error = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl::GetProgramInfoLog(
                program_id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }
        return Err(error.to_string_lossy().into_owned());
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}
