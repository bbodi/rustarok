use crate::grf::rsm::BoundingBox;
use crate::grf::texture::{GlNativeTextureId, GlTexture, TextureId};
use crate::my_gl::Gl;
use crate::runtime_assets::map::ModelRenderData;
use serde::Serialize;
use std::collections::HashMap;

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

    #[inline]
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

    pub(super) fn reserve_model_slots(&mut self, count: usize) -> Vec<usize> {
        (0..count)
            .map(|_| {
                let model_index = self.models.len();
                self.models.push(ModelRenderData {
                    bounding_box: BoundingBox::new(),
                    alpha: 0,
                    model: vec![],
                });
                model_index
            })
            .collect()
    }

    pub(super) fn fill_bulk_reserved_model_slot(
        &mut self,
        model_index: usize,
        model_render_data: ModelRenderData,
        name: String,
    ) {
        self.models[model_index] = model_render_data;
        self.model_name_to_index
            .insert(AssetDatabase::replace_non_ascii_chars(&name), model_index);
    }

    pub fn get_texture_id(&self, path: &str) -> Option<TextureId> {
        let key = AssetDatabase::replace_non_ascii_chars(&path);
        return self.texture_db.entries.get(&key).map(|it| it.clone());
    }

    #[inline]
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
            .map(|_| {
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
        let mut ret = String::with_capacity(name.chars().count());
        name.chars().for_each(|it| {
            if it.is_ascii() {
                ret.push(it);
            } else {
                ret.push((it as u32 - 0x7F) as u8 as char);
            }
        });
        return ret;
    }
}
