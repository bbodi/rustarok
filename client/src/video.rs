use crate::grf::asset_loader::GrfEntryLoader;
use crate::grf::database::AssetDatabase;
use crate::grf::texture::{GlTexture, TextureId};
use crate::my_gl::{Gl, MyGlEnum};
use rustarok_common::common::{Mat3, Mat4};
use sdl2::render::BlendMode;
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{DisplayMode, FullscreenType, Window};
use sdl2::EventPump;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::Arc;

pub struct Video {
    pub window: Window,
    //    pub imgui: ImGui,
    //    pub imgui_sdl2: ImguiSdl2,
    //    pub renderer: Renderer,
    pub event_pump: EventPump,
    // !!! gl_context: sdl2::video::GLContext THIS MUST BE KEPT IN SCOPE, DON'T REMOVE IT!
    _gl_context: sdl2::video::GLContext,
    original_displaymode: DisplayMode,
}

impl Video {
    pub fn init(
        sdl_context: &sdl2::Sdl,
        resolution_w: u32,
        resolution_h: u32,
    ) -> (Video, Gl, Vec<String>) {
        sdl_context.mouse().show_cursor(true); // false
        let video = sdl_context.video().unwrap();
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(4, 5);
        let window = video
            .window("Rustarok", resolution_w, resolution_h)
            .opengl()
            .allow_highdpi()
            //            .resizable()
            .build()
            .unwrap();
        let (gl, gl_context) = Gl::new(&video, &window, resolution_w as i32, resolution_h as i32);
        //        let mut imgui = imgui::ImGui::init();
        //        imgui.set_ini_filename(None);
        //        let imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui);
        //        let renderer =
        //            imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);

        let display_modes = (0..video.num_display_modes(0).unwrap())
            .map(|i| {
                let display_mode = video.display_mode(0, i).unwrap();
                format!(
                    "{}x{}@{}",
                    display_mode.w, display_mode.h, display_mode.refresh_rate
                )
            })
            .collect();
        let original_displaymode = video.current_display_mode(0).unwrap();
        let event_pump = sdl_context.event_pump().unwrap();
        (
            Video {
                window,
                //                imgui,
                //                imgui_sdl2,
                //                renderer,
                event_pump,
                original_displaymode,
                _gl_context: gl_context,
            },
            gl,
            display_modes,
        )
    }

    pub fn gl_swap_window(&self) {
        self.window.gl_swap_window();
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title).unwrap();
    }

    pub fn load_font<'a, 'b>(
        ttf_context: &'a Sdl2TtfContext,
        font_path: &str,
        size: u16,
    ) -> Result<sdl2::ttf::Font<'a, 'b>, String> {
        ttf_context.load_font(font_path, size)
    }

    pub fn create_text_texture_inner<'a, 'b>(
        gl: &Gl,
        font: &sdl2::ttf::Font<'a, 'b>,
        text: &str,
    ) -> GlTexture {
        let surface = font
            .render(text)
            .blended(sdl2::pixels::Color::RGBA(255, 255, 255, 255))
            .unwrap();
        return GrfEntryLoader::create_texture_from_surface_inner(gl, surface, MyGlEnum::LINEAR);
    }

    pub fn create_outline_text_texture<'a, 'b>(
        gl: &Gl,
        font: &sdl2::ttf::Font<'a, 'b>,
        outline_font: &sdl2::ttf::Font<'a, 'b>,
        text: &str,
        asset_db: &mut AssetDatabase,
    ) -> TextureId {
        let key = format!(
            "outlinetext_{}_{}_{}",
            text,
            font.height(),
            outline_font.get_outline_width()
        );
        return asset_db.get_texture_id(&key).unwrap_or_else(|| {
            let mut bg_surface = outline_font
                .render(text)
                .blended(sdl2::pixels::Color::RGBA(0, 0, 0, 255))
                .unwrap();
            let mut fg_surface = font
                .render(text)
                .blended(sdl2::pixels::Color::RGBA(255, 255, 255, 255))
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
            GrfEntryLoader::create_texture_from_surface(
                gl,
                &key,
                bg_surface,
                MyGlEnum::NEAREST,
                asset_db,
            )
        });
    }
}

impl Drop for Video {
    fn drop(&mut self) {
        if self.window.fullscreen_state() != FullscreenType::Off {
            self.window
                .set_display_mode(self.original_displaymode)
                .unwrap();
            self.window.set_fullscreen(FullscreenType::Off).unwrap();
        }
    }
}

pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, znear: f32, zfar: f32) -> Mat4 {
    let two = 2.0;
    let mut mat = Mat4::identity();

    mat[(0, 0)] = two / (right - left);
    mat[(0, 3)] = -(right + left) / (right - left);
    mat[(1, 1)] = two / (top - bottom);
    mat[(1, 3)] = -(top + bottom) / (top - bottom);
    mat[(2, 2)] = -two / (zfar - znear);
    mat[(2, 3)] = -(zfar + znear) / (zfar - znear);

    mat
}

#[derive(Clone, Debug)]
pub struct VertexAttribDefinition {
    pub number_of_components: usize,
    pub offset_of_first_element: usize,
}

pub struct VertexArrayBind<'a> {
    vertex_array: &'a VertexArray,
    gl_for_drop: Gl,
}

impl<'a> VertexArrayBind<'a> {
    pub fn draw(&self, gl: &Gl) {
        unsafe {
            gl.draw_arrays(
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
                self.gl_for_drop.disable_vertex_attrib_array(i as u32);
            }
            self.gl_for_drop.bind_vertex_array(0);
        }
    }
}

#[derive(Clone)]
struct VertexArrayResource {
    buffer_id: c_uint,
    vertex_array_id: c_uint,
    gl_for_drop: Gl,
}

impl Drop for VertexArrayResource {
    fn drop(&mut self) {
        unsafe {
            self.gl_for_drop.delete_buffers(1, &self.buffer_id);
            self.gl_for_drop
                .delete_vertex_arrays(1, &self.vertex_array_id);
        }
    }
}

#[derive(Clone)]
pub struct VertexArray {
    pub raw: Vec<u8>,
    buffers: Arc<VertexArrayResource>,
    vertex_count: usize,
    stride: c_int,
    vertex_attrib_pointer_defs: Vec<VertexAttribDefinition>,
    draw_mode: MyGlEnum,
}

impl VertexArray {
    #[allow(dead_code)]
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn bind(&self, gl: &Gl) -> VertexArrayBind {
        unsafe {
            gl.bind_vertex_array(self.buffers.vertex_array_id);
            gl.bind_buffer(MyGlEnum::ARRAY_BUFFER, self.buffers.buffer_id);

            for (i, def) in self.vertex_attrib_pointer_defs.iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32); // this is "layout (location = 0)" in vertex shader
                gl.vertex_attrib_pointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    MyGlEnum::FLOAT, // data type
                    false as u8,     // normalized (int-to-float conversion)
                    self.stride,     // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const c_void,
                );
            }

            VertexArrayBind {
                gl_for_drop: gl.clone(),
                vertex_array: self,
            }
        }
    }

    pub fn bind_dynamic<T>(&mut self, gl: &Gl, vertices: &[T]) -> VertexArrayBind {
        unsafe {
            gl.bind_vertex_array(self.buffers.vertex_array_id);
            gl.bind_buffer(MyGlEnum::ARRAY_BUFFER, self.buffers.buffer_id);

            for (i, def) in self.vertex_attrib_pointer_defs.iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32); // this is "layout (location = 0)" in vertex shader
                gl.vertex_attrib_pointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    MyGlEnum::FLOAT, // data type
                    false as u8,     // normalized (int-to-float conversion)
                    self.stride,     // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const c_void,
                );
            }

            gl.buffer_data(
                MyGlEnum::ARRAY_BUFFER,                               // target
                (vertices.len() * std::mem::size_of::<T>()) as isize, // size of data in bytes
                vertices.as_ptr() as *const c_void,                   // pointer to data
                MyGlEnum::DYNAMIC_DRAW,                               // usage
            );
            self.vertex_count = vertices.len();

            VertexArrayBind {
                gl_for_drop: gl.clone(),
                vertex_array: self,
            }
        }
    }

    pub fn new_static<T>(
        gl: &Gl,
        draw_mode: MyGlEnum,
        vertices: Vec<T>,
        definitions: Vec<VertexAttribDefinition>,
    ) -> VertexArray {
        VertexArray::new(gl, draw_mode, vertices, definitions, MyGlEnum::STATIC_DRAW)
    }

    pub fn new_dynamic<T>(
        gl: &Gl,
        draw_mode: MyGlEnum,
        vertices: Vec<T>,
        definitions: Vec<VertexAttribDefinition>,
    ) -> VertexArray {
        VertexArray::new(gl, draw_mode, vertices, definitions, MyGlEnum::DYNAMIC_DRAW)
    }

    pub fn new<T>(
        gl: &Gl,
        draw_mode: MyGlEnum,
        mut vertices: Vec<T>,
        definitions: Vec<VertexAttribDefinition>,
        usage: MyGlEnum,
    ) -> VertexArray {
        let vertex_count = vertices.len();
        let mut vbo: c_uint = 0;
        unsafe {
            gl.gen_buffers(1, &mut vbo);
            gl.bind_buffer(MyGlEnum::ARRAY_BUFFER, vbo);
            gl.buffer_data(
                MyGlEnum::ARRAY_BUFFER,                               // target
                (vertices.len() * std::mem::size_of::<T>()) as isize, // size of data in bytes
                vertices.as_ptr() as *const c_void,                   // pointer to data
                usage,                                                // usage
            );
        }
        let mut vao: c_uint = 0;
        let stride = (std::mem::size_of::<T>()) as c_int;
        unsafe {
            gl.gen_vertex_arrays(1, &mut vao);
            gl.bind_vertex_array(vao);
            gl.bind_buffer(MyGlEnum::ARRAY_BUFFER, vbo);

            for (i, def) in definitions.iter().enumerate() {
                gl.enable_vertex_attrib_array(i as u32); // this is "layout (location = 0)" in vertex shader
                gl.vertex_attrib_pointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    MyGlEnum::FLOAT, // data type
                    false as u8,     // normalized (int-to-float conversion)
                    stride,          // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const c_void,
                );
            }
        }

        unsafe {
            gl.bind_buffer(MyGlEnum::ARRAY_BUFFER, 0);
            gl.bind_vertex_array(0);
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
                gl_for_drop: gl.clone(),
            }),
            vertex_count,
            stride,
            vertex_attrib_pointer_defs: definitions,
        }
    }
}

pub struct Shader {
    id: c_uint,
    gl_for_drop: Gl,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl_for_drop.delete_shader(self.id);
        }
    }
}

impl Shader {
    fn get_program_err(gl: &Gl, program_id: c_uint) -> String {
        let mut len: c_int = 0;
        unsafe {
            gl.get_programiv(program_id, MyGlEnum::INFO_LOG_LENGTH, &mut len);
        }
        let error = create_whitespace_cstring_with_len(len as usize);
        unsafe {
            gl.get_program_info_log(
                program_id,
                len,
                std::ptr::null_mut(),
                error.as_ptr() as *mut c_char,
            );
        }
        return error.to_string_lossy().into_owned();
    }

    pub fn get_location(gl: &Gl, program_id: c_uint, name: &str) -> c_int {
        let cname = CString::new(name).expect("expected uniform name to have no nul bytes");
        unsafe {
            let ret = gl
                .get_uniform_location(program_id, cname.as_bytes_with_nul().as_ptr() as *const i8);
            assert_ne!(ret, -1, "{}", name);
            return ret;
        }
    }

    pub fn from_source(gl: &Gl, source: &str, kind: MyGlEnum) -> Result<Shader, String> {
        let c_str: &CStr = &CString::new(source).unwrap();
        let id = unsafe { gl.create_shader(kind) };
        unsafe {
            gl.shader_source(id, 1, &c_str.as_ptr(), std::ptr::null());
            gl.compile_shader(id);
        }
        let mut success: c_int = 1;
        unsafe {
            gl.get_shaderiv(id, MyGlEnum::COMPILE_STATUS, &mut success);
        }
        if success == 0 {
            let mut len: c_int = 0;
            unsafe {
                gl.get_shaderiv(id, MyGlEnum::INFO_LOG_LENGTH, &mut len);
            }
            let mut buffer = Vec::<u8>::with_capacity(len as usize);
            unsafe {
                gl.get_shader_info_log(
                    id,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut i8,
                );
                buffer.set_len(len as usize);
                Err(String::from_utf8_unchecked(buffer))
            }
        } else {
            Ok(Shader {
                id,
                gl_for_drop: gl.clone(),
            })
        }
    }

    pub fn id(&self) -> c_uint {
        self.id
    }
}

pub struct ActiveShaderProgram<'a, P> {
    #[allow(dead_code)]
    id: c_uint,
    pub params: &'a P,
}

pub struct ShaderParam3x3fv(pub c_int);
impl ShaderParam3x3fv {
    pub fn set(&self, gl: &Gl, matrix: &Mat3) {
        unsafe {
            gl.uniform_matrix3fv(
                self.0,
                1,           // count
                false as u8, // transpose
                matrix.as_slice().as_ptr(),
            );
        }
    }
}

pub struct ShaderParam3fv(pub c_int);
impl ShaderParam3fv {
    pub fn set(&self, gl: &Gl, vector: &[f32; 3]) {
        unsafe {
            gl.uniform3fv(
                self.0,
                1, // count
                vector.as_ptr(),
            );
        }
    }
}

pub struct ShaderParam4ubv(pub c_int);
impl ShaderParam4ubv {
    pub fn set(&self, gl: &Gl, vector: &[u8; 4]) {
        unsafe {
            gl.uniform4fv(
                self.0,
                1, // count
                vec![
                    vector[0] as f32 / 255.0,
                    vector[1] as f32 / 255.0,
                    vector[2] as f32 / 255.0,
                    vector[3] as f32 / 255.0,
                ]
                .as_ptr(),
            );
        }
    }

    #[allow(dead_code)]
    pub fn set_f32(&self, gl: &Gl, vector: &[f32; 4]) {
        unsafe {
            gl.uniform4fv(
                self.0,
                1, // count
                vector.as_ptr(),
            );
        }
    }
}

pub struct ShaderParam2fv(pub c_int);
impl ShaderParam2fv {
    pub fn set(&self, gl: &Gl, vector: &[f32; 2]) {
        unsafe {
            gl.uniform2fv(
                self.0,
                1, // count
                vector.as_ptr(),
            );
        }
    }
}

pub struct ShaderParam2i(pub c_int);
impl ShaderParam2i {
    pub fn set(&self, gl: &Gl, a: c_int, b: c_int) {
        unsafe {
            gl.uniform2i(self.0, a, b);
        }
    }
}

pub struct ShaderParam1f(pub c_int);
impl ShaderParam1f {
    pub fn set(&self, gl: &Gl, value: f32) {
        unsafe {
            gl.uniform1f(self.0, value);
        }
    }
}

pub struct ShaderParam1i(pub c_int);
impl ShaderParam1i {
    pub fn set(&self, gl: &Gl, value: c_int) {
        unsafe {
            gl.uniform1i(self.0, value);
        }
    }
}

pub struct ShaderParam4x4fv(pub c_int);
impl ShaderParam4x4fv {
    pub fn set(&self, gl: &Gl, matrix: &Mat4) {
        unsafe {
            gl.uniform_matrix4fv(
                self.0,
                1,           // count
                false as u8, // transpose
                matrix.as_slice().as_ptr(),
            );
        }
    }
}

pub struct ShaderProgram<P> {
    id: c_uint,
    params: P,
    gl_for_drop: Gl,
}

impl<P> ShaderProgram<P> {
    pub fn from_shaders<F>(gl: &Gl, shaders: &[Shader], func: F) -> Result<ShaderProgram<P>, String>
    where
        F: Fn(c_uint) -> P,
    {
        let program_id = unsafe { gl.create_program() };

        for shader in shaders {
            unsafe {
                gl.attach_shader(program_id, shader.id());
            }
        }

        unsafe {
            gl.link_program(program_id);
        }

        let mut success: c_int = 1;
        unsafe {
            gl.get_programiv(program_id, MyGlEnum::LINK_STATUS, &mut success);
        }

        if success == 0 {
            return Err(Shader::get_program_err(gl, program_id));
        }

        for shader in shaders {
            unsafe {
                gl.detach_shader(program_id, shader.id());
            }
        }

        Ok(ShaderProgram {
            id: program_id,
            params: func(program_id),
            gl_for_drop: gl.clone(),
        })
    }

    pub fn gl_use(&self, gl: &Gl) -> ActiveShaderProgram<P> {
        unsafe {
            gl.use_program(self.id);
        }
        ActiveShaderProgram {
            id: self.id,
            params: &self.params,
        }
    }
}

impl<P> Drop for ShaderProgram<P> {
    fn drop(&mut self) {
        unsafe {
            self.gl_for_drop.delete_program(self.id);
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
