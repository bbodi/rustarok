use std::os::raw::c_uint;
use std::path::Path;

use serde::export::fmt::Display;
use serde::Serialize;

use crate::asset::database::AssetDatabase;
use crate::asset::AssetLoader;
use crate::my_gl::{Gl, MyGlEnum};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TextureId(pub(super) usize);

impl TextureId {
    pub fn as_u32(&self) -> u32 {
        return self.0 as u32;
    }
}

struct GlTextureContext {
    native_id: GlNativeTextureId,
    gl_for_drop: Gl,
}

impl Drop for GlTextureContext {
    fn drop(&mut self) {
        unsafe {
            self.gl_for_drop
                .DeleteTextures(1, &(self.native_id).0 as *const c_uint)
        }
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
            gl.ActiveTexture(texture_index);
            gl.BindTexture(MyGlEnum::TEXTURE_2D, self.context.native_id);
        }
    }

    pub fn from_file<P: AsRef<Path>>(gl: &Gl, path: P, asset_db: &mut AssetDatabase) -> TextureId
    where
        P: Display,
    {
        use sdl2::image::LoadSurface;
        let mut surface = sdl2::surface::Surface::from_file(&path).unwrap();
        let mut optimized_surf = sdl2::surface::Surface::new(
            surface.width(),
            surface.height(),
            sdl2::pixels::PixelFormatEnum::RGBA32,
        )
        .unwrap();
        surface
            .set_color_key(true, sdl2::pixels::Color::RGB(255, 0, 255))
            .unwrap();
        surface.blit(None, &mut optimized_surf, None).unwrap();
        log::trace!("Texture from file --> {}", &path);
        return AssetLoader::create_texture_from_surface(
            gl,
            &path.to_string(),
            optimized_surf,
            MyGlEnum::NEAREST,
            asset_db,
        );
    }
}
