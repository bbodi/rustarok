use nalgebra::{Matrix4, Matrix3, Vector3, Rotation3, Point3};
use std::ffi::{CString, CStr};
use sdl2::surface::Surface;
use std::path::Path;
use sdl2::pixels::{PixelFormatEnum, Color};
use std::fmt::Display;
use std::sync::Arc;
use sdl2::{Sdl, EventPump};
use sdl2::video::{Window, GLContext};
use imgui::ImGui;
use imgui_sdl2::ImguiSdl2;
use imgui_opengl_renderer::Renderer;
use sdl2::event::{EventPollIterator, Event};
use crate::systems::SystemVariables;

pub struct Video {
    pub sdl_context: Sdl,
    pub window: Window,
    pub imgui: ImGui,
    pub imgui_sdl2: ImguiSdl2,
    pub renderer: Renderer,
    pub event_pump: EventPump,
    // these two variables must be in scope, so don't remove their variables
    _gl_context: GLContext,
//    _gl: *const (),
}

pub const VIDEO_WIDTH: u32 = 900;
pub const VIDEO_HEIGHT: u32 = 700;

impl Video {
    pub fn init() -> Video {
        let sdl_context = sdl2::init().unwrap();
        sdl_context.mouse().show_cursor(false);
        let video = sdl_context.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 5);
        let mut window = video
            .window("Rustarok", VIDEO_WIDTH, VIDEO_HEIGHT)
            .opengl()
            .allow_highdpi()
//            .resizable()
            .input_grabbed()
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
        }
        let mut imgui = imgui::ImGui::init();
        imgui.set_ini_filename(None);
        let mut imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui);
        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);
        let event_pump = sdl_context.event_pump().unwrap();
        sdl_context.mouse().show_cursor(false);
        Video {
            sdl_context,
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

pub fn draw_lines_inefficiently2(trimesh_shader: &ShaderProgram,
                                 projection: &Matrix4<f32>,
                                 view: &Matrix4<f32>,
                                 points: &[Point3<f32>],
                                 color: &[f32; 4]) {
    let points: Vec<Vector3<f32>> = points.iter().map(|&p| p.coords).collect();
    draw_lines_inefficiently(trimesh_shader, projection, view,
                             points.as_slice(),
                             color);
}

pub fn draw_lines_inefficiently(trimesh_shader: &ShaderProgram,
                                projection: &Matrix4<f32>,
                                view: &Matrix4<f32>,
                                points: &[Vector3<f32>],
                                color: &[f32; 4]) {
    trimesh_shader.gl_use();
    trimesh_shader.set_mat4("projection", &projection);
    trimesh_shader.set_mat4("view", view);
    trimesh_shader.set_vec4("color", color);
    trimesh_shader.set_mat4("model", &Matrix4::identity());
    VertexArray::new(
        gl::LINE_LOOP,
        points, points.len(), None, vec![
            VertexAttribDefinition {
                number_of_components: 3,
                offset_of_first_element: 0,
            }
        ]).bind().draw();
}

pub fn draw_circle_inefficiently(trimesh_shader: &ShaderProgram,
                                 projection: &Matrix4<f32>,
                                 view: &Matrix4<f32>,
                                 center: &Vector3<f32>,
                                 r: f32,
                                 color: &[f32; 4]) {
    trimesh_shader.gl_use();
    trimesh_shader.set_mat4("projection", &projection);
    trimesh_shader.set_mat4("view", view);
    trimesh_shader.set_vec4("color", color);
    let mut matrix = Matrix4::identity();
    matrix.prepend_translation_mut(center);
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::x()), std::f32::consts::FRAC_PI_2).to_homogeneous();
    matrix = matrix * rotation;
    trimesh_shader.set_mat4("model", &matrix);
    let mut capsule_mesh = ncollide2d::procedural::circle(
        &(r * 2.0),
        32,
    );

    let coords = capsule_mesh.coords();
    let capsule_vertex_array = VertexArray::new(
        gl::LINE_LOOP,
        coords,
        coords.len(),
        None,
        vec![
            VertexAttribDefinition {
                number_of_components: 2,
                offset_of_first_element: 0,
            }
        ]).bind().draw();
}

#[derive(Hash, Eq, PartialEq)]
struct GlTextureContext(gl::types::GLuint);

impl Drop for GlTextureContext {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.0 as *const gl::types::GLuint)
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct GlTexture {
    context: Arc<GlTextureContext>,
    pub width: i32,
    pub height: i32,
}

impl GlTexture {
    pub fn id(&self) -> gl::types::GLuint {
        self.context.0
    }

    pub fn bind(&self, texture_index: gl::types::GLuint) {
        unsafe {
            gl::ActiveTexture(texture_index);
            gl::BindTexture(gl::TEXTURE_2D, self.context.0);
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> GlTexture
        where P: Display
    {
        use sdl2::image::LoadSurface;
        let mut surface = sdl2::surface::Surface::from_file(&path).unwrap();
        let mut optimized_surf = sdl2::surface::Surface::new(
            surface.width(),
            surface.height(),
            PixelFormatEnum::RGBA32).unwrap();
        surface.set_color_key(true, Color::RGB(255, 0, 255)).unwrap();
        surface.blit(None, &mut optimized_surf, None).unwrap();
        trace!("Texture from file --> {}", &path);
        GlTexture::from_surface(optimized_surf)
    }

    pub fn from_surface(surface: Surface) -> GlTexture {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            let mode = if surface.pixel_format_enum().byte_size_per_pixel() == 4 {
                if surface.pixel_format_enum().into_masks().unwrap().rmask == 0x000000ff {
                    gl::RGBA
                } else {
                    gl::BGRA
                }
            } else {
                if surface.pixel_format_enum().into_masks().unwrap().rmask == 0x000000ff {
                    gl::RGB
                } else {
                    gl::BGR
                }
            };
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0, // Pyramid level (for mip-mapping) - 0 is the top level
                gl::RGBA as i32, // Internal colour format to convert to
                surface.width() as i32,
                surface.height() as i32,
                0, // border
                mode as u32, // Input image format (i.e. GL_RGB, GL_RGBA, GL_BGR etc.)
                gl::UNSIGNED_BYTE,
                surface.without_lock().unwrap().as_ptr() as *const gl::types::GLvoid,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        GlTexture {
            context: Arc::new(GlTextureContext(texture_id)),
            width: surface.width() as i32,
            height: surface.height() as i32,
        }
    }

    pub fn from_data(data: &Vec<u8>, width: i32, height: i32) -> GlTexture {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            debug!("Texture from_data {}", texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            let mode = gl::RGBA;
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0, // Pyramid level (for mip-mapping) - 0 is the top level
                mode as i32, // Internal colour format to convert to
                width,
                height,
                0, // border
                mode, // Input image format (i.e. GL_RGB, GL_RGBA, GL_BGR etc.)
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const gl::types::GLvoid,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        GlTexture {
            context: Arc::new(GlTextureContext(texture_id)),
            width,
            height,
        }
    }
}

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
            if let Some(index_vbo) = self.vertex_array.index_vbo {
                gl::DrawElements(
                    self.vertex_array.draw_mode,      // mode
                    self.vertex_array.vertex_count as i32,    // count
                    gl::UNSIGNED_INT,   // type
                    std::ptr::null(),           // element array buffer offset
                );
            } else {
                gl::DrawArrays(
                    self.vertex_array.draw_mode, // mode
                    0, // starting index in the enabled arrays
                    self.vertex_array.vertex_count as i32, // number of indices to be rendered
                );
            }
        }
    }
}

impl<'a> Drop for VertexArrayBind<'a> {
    fn drop(&mut self) {
        unsafe {
            for (i, _def) in self.vertex_array.vertex_attrib_pointer_defs.iter().enumerate() {
                gl::DisableVertexAttribArray(i as u32);
            }
            gl::BindVertexArray(0);
        }
    }
}

pub struct VertexArray {
    buffer_id: gl::types::GLuint,
    vertex_array_id: gl::types::GLuint,
    index_vbo: Option<gl::types::GLuint>,
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
            gl::BindVertexArray(self.vertex_array_id);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.buffer_id);

            if let Some(index_vbo) = self.index_vbo {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_vbo);
            }

            for (i, def) in self.vertex_attrib_pointer_defs.iter().enumerate() {
                gl::EnableVertexAttribArray(i as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT, // data type
                    gl::FALSE, // normalized (int-to-float conversion)
                    self.stride, // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const gl::types::GLvoid,
                );
            }

            VertexArrayBind {
                vertex_array: &self,
            }
        }
    }

    pub fn new<T>(
        draw_mode: u32,
        vertices: &[T],
        vertex_count: usize,
        indices: Option<&[u32]>,
        definitions: Vec<VertexAttribDefinition>,
    ) -> VertexArray {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER, // target
                (vertices.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr, // size of data in bytes
                vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::STATIC_DRAW, // usage
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
                    stride, // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const gl::types::GLvoid,
                );
            }
        }
        let index_vbo = indices.map(|indices| {
            let mut index_vbo: gl::types::GLuint = 0;
            unsafe {
                gl::GenBuffers(1, &mut index_vbo);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_vbo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER, // target
                    (indices.len() * std::mem::size_of::<u32>()) as gl::types::GLsizeiptr, // size of data in bytes
                    indices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                    gl::STATIC_DRAW, // usage
                );
            }
            index_vbo
        });

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        VertexArray {
            draw_mode,
            buffer_id: vbo,
            vertex_array_id: vao,
            index_vbo,
            vertex_count,
            stride,
            vertex_attrib_pointer_defs: definitions,
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer_id);
            if let Some(index_vbo) = self.index_vbo {
                gl::DeleteBuffers(1, &index_vbo);
            }
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
    pub fn from_source(
        source: &str,
        kind: gl::types::GLenum,
    ) -> Result<Shader, String> {
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
                gl::GetShaderInfoLog(id, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
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

impl ShaderProgram {
    pub fn gl_use(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_mat4(&self, name: &str, matrix: &Matrix4<f32>) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::UniformMatrix4fv(
                location,
                1, // count
                gl::FALSE, // transpose
                matrix.as_slice().as_ptr() as *const f32,
            );
        }
    }


    pub fn set_mat3(&self, name: &str, matrix: &Matrix3<f32>) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::UniformMatrix3fv(
                location,
                1, // count
                gl::FALSE, // transpose
                matrix.as_slice().as_ptr() as *const f32,
            );
        }
    }

    pub fn set_vec3(&self, name: &str, vector: &[f32; 3]) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
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
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
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
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform4fv(
                location,
                1, // count
                vector.as_ptr() as *const f32,
            );
        }
    }

    pub fn set_int(&self, name: &str, value: i32) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform1i(location, value);
        }
    }

    pub fn set_f32(&self, name: &str, value: f32) {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let location = gl::GetUniformLocation(self.id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            gl::Uniform1f(location, value);
        }
    }

    pub fn from_shaders(shaders: &[Shader]) -> Result<ShaderProgram, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe { gl::AttachShader(program_id, shader.id()); }
        }

        unsafe { gl::LinkProgram(program_id); }

        let mut success: gl::types::GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            return ShaderProgram::get_program_err(program_id);
        }

        for shader in shaders {
            unsafe { gl::DetachShader(program_id, shader.id()); }
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