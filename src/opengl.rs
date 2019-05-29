use nalgebra::Matrix4;
use std::ffi::{CString, CStr};
use sdl2::surface::Surface;
use std::path::Path;

pub struct GlTexture {
    id: gl::types::GLuint,
}

impl GlTexture {
    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> GlTexture {
        use sdl2::image::LoadSurface;
        GlTexture::from_surface(sdl2::surface::Surface::from_file(path).unwrap())
    }

    pub fn from_surface(surface: Surface) -> GlTexture {
        let mut texture_id: gl::types::GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            let mode = if surface.pixel_format_enum().byte_size_per_pixel() == 4 {
                gl::RGBA
            } else { gl::RGB };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                mode as i32,
                surface.width() as i32,
                surface.height() as i32,
                0,
                mode as u32,
                gl::UNSIGNED_BYTE,
                surface.without_lock().unwrap().as_ptr() as *const gl::types::GLvoid,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        GlTexture {
            id: texture_id,
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

    pub fn new<T>(vertices: &Vec<T>, definitions: &[VertexAttribDefinition]) -> VertexArray {
        let mut vbo: gl::types::GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }
        println!("size of data in bytes: {}", (vertices.len() * std::mem::size_of::<T>()));
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER, // target
                (vertices.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr, // size of data in bytes
                vertices.as_ptr() as *const gl::types::GLvoid, // pointer to data
                gl::STATIC_DRAW, // usage
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0); // unbind the buffer
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
        println!("Free VertexArray {}", self.buffer_id);
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
            // allocate buffer of correct size
            let mut buffer: Vec<u8> = Vec::with_capacity(len as usize + 1);
            // fill it with len spaces
            buffer.extend([b' '].iter().cycle().take(len as usize));
            // convert buffer to CString
            let error = create_whitespace_cstring_with_len(len as usize);
            return Err(error.to_string_lossy().into_owned());
        }
        Ok(Shader { id })
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
            gl::UniformMatrix4fv(location, 1, gl::FALSE, matrix.as_slice().as_ptr() as *const f32);
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