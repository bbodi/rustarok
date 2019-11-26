use std::os::raw::c_uint;

use serde::Serialize;

use crate::my_gl::{Gl, MyGlEnum};

pub const DUMMY_TEXTURE_ID_FOR_TEST: TextureId = TextureId(0);

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TextureId(pub(super) usize);

struct GlTextureContext {
    native_id: GlNativeTextureId,
    gl_for_drop: Gl,
}

impl Drop for GlTextureContext {
    fn drop(&mut self) {
        unsafe { self.gl_for_drop.delete_textures(1, &(self.native_id).0) }
    }
}

pub struct GlTexture {
    context: GlTextureContext,
    pub width: i32,
    pub height: i32,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Serialize)]
pub struct GlNativeTextureId(pub c_uint);

impl GlTexture {
    pub(super) fn new(
        gl: &Gl,
        texture_id: GlNativeTextureId,
        width: i32,
        height: i32,
    ) -> GlTexture {
        GlTexture {
            context: (GlTextureContext {
                native_id: texture_id,
                gl_for_drop: gl.clone(),
            }),
            width,
            height,
        }
    }

    pub fn id(&self) -> GlNativeTextureId {
        (self.context.native_id).clone()
    }

    pub fn bind(&self, gl: &Gl, texture_index: MyGlEnum) {
        unsafe {
            gl.active_texture(texture_index);
            gl.bind_texture(MyGlEnum::TEXTURE_2D, self.context.native_id);
        }
    }
}
