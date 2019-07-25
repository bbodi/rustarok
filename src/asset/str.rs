use crate::video::GlTexture;
use std::collections::HashMap;
use crate::asset::{BinaryReader, AssetLoader};

pub struct StrFile {
    pub max_key: u32,
    pub fps: u32,
    pub layers: Vec<StrLayer>,
    pub textures: Vec<GlTexture>,
}

pub struct StrLayer {
    pub key_frames: Vec<StrKeyFrame>
}

#[derive(PartialEq, Eq, Debug)]
pub enum KeyFrameType {
    Start,
    End,
}

pub struct StrKeyFrame {
    pub frame: i32,
    pub typ: KeyFrameType,
    pub pos: [f32; 2],
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub texture_index: usize,
    pub anitype: u32,
    pub angle: f32,
    pub delay: f32,
    pub color: [f32; 4],
    pub src_alpha: u32,
    pub dst_alpha: u32,
    pub mtpreset: u32,
}


impl StrFile {
    pub(super) fn load(asset_loader: &AssetLoader, mut buf: BinaryReader, str_name: &str) -> Self {
        let header = buf.string(4);
        if header != "STRM" {
            panic!("Invalig STR header: {}", header);
        }
        if buf.next_u32() != 0x94 {
            panic!("invalid version!");
        }

        let fps = buf.next_u32();
        let max_key = buf.next_u32();
        let layer_num = buf.next_u32();
        buf.skip(16);

        let d3d_to_gl_blend = [
            gl::ZERO, // 0
            gl::ZERO,
            gl::ONE,
            gl::SRC_COLOR,
            gl::ONE_MINUS_SRC_COLOR,
            gl::SRC_ALPHA, // 5
            gl::ONE_MINUS_SRC_ALPHA,
            gl::DST_ALPHA,
            gl::ONE_MINUS_DST_ALPHA,
            gl::DST_COLOR,
            gl::ONE_MINUS_DST_COLOR, // 10
            gl::SRC_ALPHA_SATURATE,
            gl::CONSTANT_COLOR,
            gl::ONE_MINUS_CONSTANT_ALPHA, // 13
        ];

        let mut texture_names_to_index: HashMap<String, usize> = HashMap::new();
        let mut textures: Vec<GlTexture> = Vec::new();

        let layers = (0..layer_num).map(|_i| {
            let texture_names: Vec<String> = (0..buf.next_u32()).map(|_i| {
                let texture_name = buf.string(128);
                if !texture_names_to_index.contains_key(&texture_name) {
                    let path = format!("data\\texture\\effect\\{}", texture_name);
                    let surface = asset_loader.load_sdl_surface(&path);
                    let surface = surface.unwrap_or_else(|e| {
                        log::warn!("Missing texture when loading {}, path: {}, {}", str_name, path, e);
                        asset_loader.backup_surface()
                    });
                    let texture = GlTexture::from_surface(surface, gl::NEAREST);
                    textures.push(texture);
                    let size = texture_names_to_index.len();
                    texture_names_to_index.insert(texture_name.clone(), size);
                }
                texture_name
            }).collect();
            // TODO: skip layers where key_frames.is_empty()
            let key_frames: Vec<StrKeyFrame> = (0..buf.next_u32()).map(|_i| {
                let frame = buf.next_i32();
                let typ = if buf.next_u32() == 0 { KeyFrameType::Start } else { KeyFrameType::End };
                let pos = [buf.next_f32(), buf.next_f32()];
                let uv = [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()];
                let xy = [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()];
                StrKeyFrame {
                    frame,
                    typ,
                    pos,
                    uv,
                    xy,
                    texture_index: texture_names_to_index[&texture_names[buf.next_f32() as usize]],
                    anitype: buf.next_u32(),
                    delay: buf.next_f32(),
                    angle: buf.next_f32() / (1024.0 / 360.0),
                    color: [buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0],
                    src_alpha: d3d_to_gl_blend[buf.next_u32() as usize],
                    dst_alpha: d3d_to_gl_blend[buf.next_u32() as usize],
                    mtpreset: buf.next_u32(),
                }
            }).collect();

            StrLayer {
                key_frames,
            }
        })
            .filter(|layer| !layer.key_frames.is_empty())
            .collect();
        StrFile {
            max_key,
            fps,
            layers,
            textures,
        }
    }
}