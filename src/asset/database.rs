use crate::asset::rsm::BoundingBox;
use crate::asset::texture::{GlNativeTextureId, GlTexture, TextureId};
use crate::my_gl::{Gl, MyGlEnum};
use crate::runtime_assets::map::ModelRenderData;
use byteorder::{LittleEndian, WriteBytesExt};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::os::raw::c_void;

#[derive(Debug, Serialize)]
struct TextureDatabase {
    entries: HashMap<String, TextureId>,
}

#[derive(Serialize)]
pub struct AssetDatabase {
    texture_db: TextureDatabase,
    model_name_to_index: HashMap<String, usize>,
    #[serde(skip)]
    models: Vec<ModelRenderData>,
    #[serde(skip)]
    textures: Vec<GlTexture>,
}

impl AssetDatabase {
    pub fn new() -> AssetDatabase {
        AssetDatabase {
            texture_db: TextureDatabase {
                entries: HashMap::with_capacity(512),
            },
            model_name_to_index: HashMap::with_capacity(512),
            models: Vec::with_capacity(512),
            textures: Vec::with_capacity(8192),
        }
    }

    pub fn get_model(&self, index: usize) -> &ModelRenderData {
        &self.models[index]
    }

    pub fn get_model_index(&self, name: &str) -> usize {
        self.model_name_to_index[&AssetDatabase::replace_non_ascii_chars(&name)]
    }

    pub fn register_model(&mut self, name: &str, model: ModelRenderData) {
        self.model_name_to_index.insert(
            AssetDatabase::replace_non_ascii_chars(name),
            self.models.len(),
        );
        self.models.push(model);
    }

    pub fn reserve_model_slot(&mut self, name: &str) -> usize {
        let model_index = self.models.len();
        self.model_name_to_index
            .insert(AssetDatabase::replace_non_ascii_chars(&name), model_index);
        self.models.push(ModelRenderData {
            bounding_box: BoundingBox::new(),
            alpha: 0,
            model: vec![],
        });
        return model_index;
    }

    pub fn fill_reserved_model_slot(&mut self, model_index: usize, data: ModelRenderData) {
        self.models[model_index] = data;
    }

    pub fn get_texture_id(&self, path: &str) -> Option<TextureId> {
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        return self.texture_db.entries.get(&key).map(|it| it.clone());
    }

    pub fn get_texture(&self, i: TextureId) -> &GlTexture {
        return &self.textures[i.0];
    }

    pub fn register_texture(&mut self, path: &str, gl_texture: GlTexture) -> TextureId {
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        if self.texture_db.entries.contains_key(&key) {
            panic!("Texture already exists with this name: {}", key);
        }

        let texture_id = TextureId(self.textures.len());
        self.textures.push(gl_texture);
        self.texture_db.entries.insert(key, texture_id);
        return texture_id;
    }

    pub(super) fn reserve_texture_slot(&mut self, gl: &Gl, path: &str) -> TextureId {
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        if self.texture_db.entries.contains_key(&key) {
            panic!("Texture already exists with this name: {}", key);
        }

        let texture_id = TextureId(self.textures.len());
        self.textures
            .push(GlTexture::new(gl, GlNativeTextureId(0), 0, 0));
        self.texture_db.entries.insert(key, texture_id);
        return texture_id;
    }

    pub(super) fn reserve_texture_slots(&mut self, gl: &Gl, count: usize) -> Vec<TextureId> {
        (0..count)
            .map(|it| {
                let texture_id = TextureId(self.textures.len());
                self.textures
                    .push(GlTexture::new(gl, GlNativeTextureId(0), 0, 0));
                texture_id
            })
            .collect()
    }

    pub(super) fn fill_reserved_texture_slot(
        &mut self,
        texture_id: TextureId,
        gl_texture: GlTexture,
    ) {
        self.textures[texture_id.0] = gl_texture;
    }

    pub(super) fn fill_bulk_reserved_texture_slot(
        &mut self,
        texture_id: TextureId,
        gl_texture: GlTexture,
        name: String,
    ) {
        self.textures[texture_id.0] = gl_texture;

        let key = AssetDatabase::replace_non_ascii_chars(&name);
        if self.texture_db.entries.contains_key(&key) {
            panic!("Texture already exists with this name: {}", key);
        }
        self.texture_db.entries.insert(key, texture_id);
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
        if let Some(texture_id) = self.texture_db.entries.get(path_in_byte_form) {
            let texture_entry = &self.textures[texture_id.0];
            let mut offset = 0;
            dst_buf
                .write_u16::<LittleEndian>(path_in_byte_form.len() as u16)
                .unwrap();
            offset += 2;
            dst_buf.write(path_in_byte_form.as_bytes()).unwrap();
            offset += path_in_byte_form.len();

            dst_buf
                .write_u16::<LittleEndian>(texture_entry.width as u16)
                .unwrap();
            offset += std::mem::size_of::<u16>();
            dst_buf
                .write_u16::<LittleEndian>(texture_entry.height as u16)
                .unwrap();
            offset += std::mem::size_of::<u16>();

            let raw_size = std::mem::size_of::<u8>()
                * (texture_entry.width as usize)
                * (texture_entry.height as usize)
                * 4;
            for _ in 0..raw_size {
                dst_buf.push(0)
            }
            unsafe {
                gl.ActiveTexture(MyGlEnum::TEXTURE0);
                gl.BindTexture(MyGlEnum::TEXTURE_2D, texture_entry.id());
                gl.GetTexImage(
                    MyGlEnum::TEXTURE_2D,
                    0,
                    MyGlEnum::RGBA,
                    MyGlEnum::UNSIGNED_BYTE,
                    dst_buf.as_mut_ptr().offset(offset as isize) as *mut c_void,
                );
            }
        }
    }
}
