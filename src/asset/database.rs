use crate::my_gl::{Gl, MyGlEnum};
use crate::runtime_assets::map::ModelRenderData;
use crate::video::{GlNativeTextureId, GlTexture};
use byteorder::{LittleEndian, WriteBytesExt};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Write;
use std::os::raw::c_void;

#[derive(Debug, Serialize)]
struct TextureDatabaseEntry {
    hash: String,
    gl_textures: Vec<(GlNativeTextureId, u32, u32)>,
}

#[derive(Debug, Serialize)]
struct TextureDatabase {
    entries: HashMap<String, TextureDatabaseEntry>,
}

#[derive(Serialize)]
pub struct AssetDatabase {
    texture_db: TextureDatabase,
    model_name_to_index: HashMap<String, usize>,
    #[serde(skip)]
    models: Vec<ModelRenderData>,
}

impl AssetDatabase {
    pub fn new() -> AssetDatabase {
        AssetDatabase {
            texture_db: TextureDatabase {
                entries: HashMap::with_capacity(512),
            },
            model_name_to_index: HashMap::with_capacity(512),
            models: Vec::with_capacity(512),
        }
    }

    pub fn get_model(&self, index: usize) -> &ModelRenderData {
        &self.models[index]
    }

    pub fn get_model_index(&self, name: &str) -> usize {
        self.model_name_to_index[&AssetDatabase::replace_non_ascii_chars(&name)]
    }

    pub fn register_models(&mut self, models: HashMap<String, ModelRenderData>) {
        self.models = Vec::with_capacity(models.len());
        self.model_name_to_index = HashMap::with_capacity(models.len());
        for (name, data) in models.into_iter() {
            self.model_name_to_index.insert(
                AssetDatabase::replace_non_ascii_chars(&name),
                self.models.len(),
            );
            self.models.push(data);
        }
    }

    pub fn get_texture(&self, gl: &Gl, path: &str) -> Option<GlTexture> {
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        return self.texture_db.entries.get(&key).map(|it| {
            let t: (GlNativeTextureId, u32, u32) = it.gl_textures[0];
            GlTexture::new(gl, t.0, t.1 as i32, t.2 as i32)
        });
    }

    pub fn register_texture(&mut self, gl: &Gl, path: &str, textures: &[&GlTexture]) {
        let mut hasher = DefaultHasher::new();

        for texture in textures {
            let mut buffer =
                Vec::<u8>::with_capacity((texture.width * texture.height * 4) as usize);
            unsafe {
                gl.ActiveTexture(MyGlEnum::TEXTURE0);
                gl.BindTexture(MyGlEnum::TEXTURE_2D, texture.id().0);
                gl.GetTexImage(
                    MyGlEnum::TEXTURE_2D,
                    0,
                    MyGlEnum::RGBA,
                    MyGlEnum::UNSIGNED_BYTE,
                    buffer.as_mut_ptr() as *mut c_void,
                );
            }
            hasher.write(buffer.as_slice());
        }
        let hash = hasher.finish();
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        if self.texture_db.entries.contains_key(&key) {
            panic!("Texture already exists with this name: {}", key);
        }

        self.texture_db.entries.insert(
            key,
            TextureDatabaseEntry {
                hash: hash.to_string(),
                gl_textures: textures
                    .iter()
                    .map(|it| (it.id(), it.width as u32, it.height as u32))
                    .collect(),
            },
        );
    }

    pub fn replace_non_ascii_chars(name: &str) -> String {
        let mut ret = String::with_capacity(name.len() * 2);
        name.chars().for_each(|it| {
            if it.is_ascii() {
                ret.push(it);
            } else {
                it.to_string()
                    .as_bytes()
                    .iter()
                    .for_each(|it| ret.push_str(&it.to_string()));
            }
        });
        return ret;
    }

    pub fn copy_model_into(&self, name: &str, dst_buf: &mut Vec<u8>) {
        if let Some(index) = self.model_name_to_index.get(name) {
            dst_buf
                .write_u16::<LittleEndian>(name.len() as u16)
                .unwrap();
            dst_buf.write(name.as_bytes()).unwrap();

            let model: &ModelRenderData = &self.models[*index];

            dst_buf
                .write_u16::<LittleEndian>(model.model.len() as u16)
                .unwrap();
            for node in &model.model {
                dst_buf
                    .write_u16::<LittleEndian>(node.len() as u16)
                    .unwrap();
                for same_texture_face in node {
                    dst_buf
                        .write_u16::<LittleEndian>(same_texture_face.texture_name.len() as u16)
                        .unwrap();
                    dst_buf
                        .write(same_texture_face.texture_name.as_bytes())
                        .unwrap();
                    same_texture_face.vao.write_into(dst_buf);
                }
            }
        }
    }

    pub fn copy_texture_into(&self, gl: &Gl, path_in_byte_form: &str, dst_buf: &mut Vec<u8>) {
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
                for _ in 0..raw_size {
                    dst_buf.push(0)
                }
                unsafe {
                    gl.ActiveTexture(MyGlEnum::TEXTURE0);
                    gl.BindTexture(MyGlEnum::TEXTURE_2D, texture_id.0);
                    gl.GetTexImage(
                        MyGlEnum::TEXTURE_2D,
                        0,
                        MyGlEnum::RGBA,
                        MyGlEnum::UNSIGNED_BYTE,
                        dst_buf.as_mut_ptr().offset(offset as isize) as *mut c_void,
                    );
                }
                offset += raw_size;
            }
        }
    }
}
