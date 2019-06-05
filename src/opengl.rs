use nalgebra::{Matrix4, Vector3, Matrix3};
use std::ffi::{CString, CStr};
use sdl2::surface::Surface;
use std::path::Path;
use sdl2::pixels::{PixelFormatEnum, Color};
use std::rc::Rc;
use std::os::raw::c_void;
use std::hash::{Hash, Hasher};
use std::fmt::Display;
use sdl2::render::BlendMode;

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
    context: Rc<GlTextureContext>,
    pub width: i32,
    pub height: i32,
}

//impl Hash for GlTexture {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//        self.context.0.hash(state);
//    }
//}
//
//impl Eq for GlTexture {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//        self.context.0.hash(state);
//    }
//}

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
        surface.set_color_key(true, Color::RGB(255, 0, 255));
        surface.blit(None, &mut optimized_surf, None);
        println!("Texture from file --> {}", &path);
        GlTexture::from_surface(optimized_surf)
    }

    pub fn from_surface(mut surface: Surface) -> GlTexture {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            println!("Texture from_surface {}", texture_id);
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
            context: Rc::new(GlTextureContext(texture_id)),
            width: surface.width() as i32,
            height: surface.height() as i32,
        }
    }

    pub fn from_data(data: &Vec<u8>, width: i32, height: i32) -> GlTexture {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            println!("Texture from_data {}", texture_id);
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
            context: Rc::new(GlTextureContext(texture_id)),
            width,
            height,
        }
    }
}

pub struct VertexAttribDefinition {
    pub number_of_components: usize,
    pub offset_of_first_element: usize,
}

pub struct VertexArray {
    buffer_id: gl::types::GLuint,
    vertex_array_id: gl::types::GLuint,
}

impl VertexArray {
    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.vertex_array_id);
        }
    }

    pub fn new<T>(vertices: &[T], definitions: &[VertexAttribDefinition]) -> VertexArray {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER, // target
                (vertices.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr, // size of data in bytes
                vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::STATIC_DRAW, // usage
            );
        }
        let mut vao: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }
        unsafe {
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            for (i, def) in definitions.iter().enumerate() {
                gl::EnableVertexAttribArray(i as u32); // this is "layout (location = 0)" in vertex shader
                gl::VertexAttribPointer(
                    i as u32, // index of the generic vertex attribute ("layout (location = 0)")
                    def.number_of_components as i32,
                    gl::FLOAT, // data type
                    gl::FALSE, // normalized (int-to-float conversion)
                    (std::mem::size_of::<T>()) as gl::types::GLint, // stride (byte offset between consecutive attributes)
                    (std::mem::size_of::<f32>() * def.offset_of_first_element) as *const gl::types::GLvoid,
                );
            }
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        VertexArray {
            buffer_id: vbo,
            vertex_array_id: vao,
        }
    }
}

impl Drop for VertexArray {
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

pub struct Program {
    id: gl::types::GLuint,
}

impl Program {
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

    pub fn set_vec3(&self, name: &str, vector: &[f32]) {
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

    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
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
            return Program::get_program_err(program_id);
        }

        for shader in shaders {
            unsafe { gl::DetachShader(program_id, shader.id()); }
        }

        Ok(Program { id: program_id })
    }

    fn get_program_err(program_id: gl::types::GLuint) -> Result<Program, String> {
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

impl Drop for Program {
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