use crate::common::BinaryReader;
use sdl2::pixels::PixelFormatEnum;
use crate::video::{GlTexture, VertexArray, VertexAttribDefinition};
use std::collections::HashMap;
use crate::systems::render::ONE_SPRITE_PIXEL_SIZE_IN_3D;

pub struct StrFile {
    pub max_key: u32,
    pub fps: u32,
    pub layers: Vec<StrLayer>,
    pub textures: Vec<GlTexture>,
}

pub struct StrLayer {
    pub key_frames: Vec<StrKeyFrame>
}

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
    pub fn load(mut buf: BinaryReader) -> StrFile {
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

        let mut texture_names_to_index: HashMap<String, usize> = HashMap::new();
        let mut textures: Vec<GlTexture> = Vec::new();

        let layers = (0..layer_num).map(|_i| {
            let texture_names: Vec<String> = (0..buf.next_u32()).map(|_i| {
                let texture_name = buf.string(128);
                if !texture_names_to_index.contains_key(&texture_name) {
                    let texture = GlTexture::from_file("d:\\Games\\TalonRO\\grf\\data\\texture\\effect\\".to_owned() + &texture_name);
                    textures.push(texture);
                    let size = texture_names_to_index.len();
                    texture_names_to_index.insert(texture_name.clone(), size);
                }
                texture_name
            }).collect();
            let key_frames: Vec<StrKeyFrame> = (0..buf.next_u32()).map(|_i| {
                let frame = buf.next_i32();
                let typ = buf.next_u32();
                let pos = [buf.next_f32(), buf.next_f32()];
                let uv = [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()];
                let xy = [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()];
                StrKeyFrame {
                    frame,
                    typ: if typ == 0 { KeyFrameType::Start } else { KeyFrameType::End },
                    pos,
                    uv,
                    xy,
                    texture_index: texture_names_to_index[&texture_names[buf.next_f32() as usize]],
                    anitype: buf.next_u32(),
                    delay: buf.next_f32(),
                    angle: buf.next_f32() / (1024.0 / 360.0),
                    color: [buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0],
                    src_alpha: buf.next_u32(),
                    dst_alpha: buf.next_u32(),
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