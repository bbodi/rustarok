use crate::video::{GlTexture, GlTextureIndex};
use byteorder::{LittleEndian, WriteBytesExt};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Write;

#[derive(Debug, Serialize)]
struct TextureDatabaseEntry {
    hash: String,
    gl_textures: Vec<(GlTextureIndex, u32, u32)>,
}

#[derive(Debug, Serialize)]
struct TextureDatabase {
    entries: HashMap<String, TextureDatabaseEntry>,
}

#[derive(Debug, Serialize)]
pub struct AssetDatabase {
    texture_db: TextureDatabase,
}

impl AssetDatabase {
    pub fn new() -> AssetDatabase {
        AssetDatabase {
            texture_db: TextureDatabase {
                entries: HashMap::new(),
            },
        }
    }

    pub fn register_texture(&mut self, path: &str, textures: &[&GlTexture]) {
        let mut hasher = DefaultHasher::new();

        for texture in textures {
            let mut buffer =
                Vec::<u8>::with_capacity((texture.width * texture.height * 4) as usize);
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, texture.id().0);
                gl::GetTexImage(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    buffer.as_mut_ptr() as *mut gl::types::GLvoid,
                );
            }
            hasher.write(buffer.as_slice());
        }
        let hash = hasher.finish();

        self.texture_db.entries.insert(
            format!("{:?}", path.as_bytes()),
            TextureDatabaseEntry {
                hash: hash.to_string(),
                gl_textures: textures
                    .iter()
                    .map(|it| (it.id(), it.width as u32, it.height as u32))
                    .collect(),
            },
        );
    }

    pub fn copy_texture(&self, path_in_byte_form: &str, dst_buf: &mut Vec<u8>) {
        if let Some(texture_entry) = self.texture_db.entries.get(path_in_byte_form) {
            let mut offset = 0;
            dst_buf
                .write_u16::<LittleEndian>(path_in_byte_form.len() as u16)
                .unwrap();
            offset += 2;
            dst_buf.write(path_in_byte_form.as_bytes()).unwrap();
            offset += path_in_byte_form.len();

            let hash_str = texture_entry.hash.to_string();
            dst_buf
                .write_u16::<LittleEndian>(hash_str.len() as u16)
                .unwrap();
            offset += 2;
            dst_buf.write(hash_str.as_bytes()).unwrap();
            offset += hash_str.len();

            dst_buf
                .write_u16::<LittleEndian>(texture_entry.gl_textures.len() as u16)
                .unwrap();
            offset += 2;

            for (texture_id, w, h) in &texture_entry.gl_textures {
                dst_buf.write_u16::<LittleEndian>(*w as u16).unwrap();
                offset += std::mem::size_of::<u16>();
                dst_buf.write_u16::<LittleEndian>(*h as u16).unwrap();
                offset += std::mem::size_of::<u16>();

                let raw_size = std::mem::size_of::<u8>() * (*w as usize) * (*h as usize) * 4;
                for i in 0..raw_size {
                    dst_buf.push(0)
                }
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, texture_id.0);
                    gl::GetTexImage(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        dst_buf.as_mut_ptr().offset(offset as isize) as *mut gl::types::GLvoid,
                    );
                }
                offset += raw_size;
            }
        }
    }
}
