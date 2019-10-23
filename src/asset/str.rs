use crate::asset::database::AssetDatabase;
use crate::asset::texture::TextureId;
use crate::asset::{AssetLoader, BinaryReader};
use crate::my_gl::{Gl, MyGlBlendEnum, MyGlEnum};
use byteorder::{LittleEndian, WriteBytesExt};
use encoding::ByteWriter;
use std::collections::HashMap;
use std::path::Path;

pub struct StrFile {
    pub max_key: u32,
    pub fps: u32,
    pub layers: Vec<StrLayer>,
    pub textures: Vec<TextureId>,
}

pub struct StrLayer {
    pub key_frames: Vec<StrKeyFrame>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum KeyFrameType {
    Start,
    End,
}

pub struct StrKeyFrame {
    pub frame: i32,
    pub typ: KeyFrameType,
    pub pos: [f32; 2],
    //    pub uv: [f32; 8], it is not used, don't store it
    pub xy: [f32; 8],
    pub color: [u8; 4],
    pub angle: f32,
    pub src_alpha: MyGlBlendEnum,
    pub dst_alpha: MyGlBlendEnum,
    pub texture_index: usize,
    //    pub anitype: u32, not used
    //    pub delay: f32, not used O-O
    //    pub mtpreset: u32, not used
}

impl StrFile {
    pub fn write_into(&self, buffer: &mut Vec<u8>) {
        buffer.write_u32::<LittleEndian>(self.max_key).unwrap();

        buffer.write_u32::<LittleEndian>(self.fps).unwrap();
        buffer
            .write_u16::<LittleEndian>(self.layers.len() as u16)
            .unwrap();
        buffer
            .write_u16::<LittleEndian>(self.textures.len() as u16)
            .unwrap();
        for texture in &self.textures {
            buffer.write_u32::<LittleEndian>(texture.0 as u32).unwrap();
        }
        for layer in &self.layers {
            buffer
                .write_u16::<LittleEndian>(layer.key_frames.len() as u16)
                .unwrap();
            for frame in &layer.key_frames {
                buffer.write_i32::<LittleEndian>(frame.frame).unwrap();
                buffer.write_byte(frame.typ as u8);
                buffer.write_f32::<LittleEndian>(frame.pos[0]).unwrap();
                buffer.write_f32::<LittleEndian>(frame.pos[1]).unwrap();
                for xy in &frame.xy {
                    buffer.write_f32::<LittleEndian>(*xy).unwrap();
                }
                for color in &frame.color {
                    buffer.write_u8(*color).unwrap();
                }
                buffer.write_f32::<LittleEndian>(frame.angle).unwrap();
                buffer
                    .write_i32::<LittleEndian>(frame.src_alpha as i32)
                    .unwrap();
                buffer
                    .write_i32::<LittleEndian>(frame.dst_alpha as i32)
                    .unwrap();
                buffer
                    .write_u16::<LittleEndian>(frame.texture_index as u16)
                    .unwrap();
            }
        }
    }

    pub(super) fn load(
        gl: &Gl,
        asset_loader: &AssetLoader,
        asset_db: &mut AssetDatabase,
        mut buf: BinaryReader,
        str_name: &str,
    ) -> Self {
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
            MyGlBlendEnum::ZERO, // 0
            MyGlBlendEnum::ZERO,
            MyGlBlendEnum::ONE,
            MyGlBlendEnum::SRC_COLOR,
            MyGlBlendEnum::ONE_MINUS_SRC_COLOR,
            MyGlBlendEnum::SRC_ALPHA, // 5
            MyGlBlendEnum::ONE_MINUS_SRC_ALPHA,
            MyGlBlendEnum::DST_ALPHA,
            MyGlBlendEnum::ONE_MINUS_DST_ALPHA,
            MyGlBlendEnum::DST_COLOR,
            MyGlBlendEnum::ONE_MINUS_DST_COLOR, // 10
            MyGlBlendEnum::SRC_ALPHA_SATURATE,
            MyGlBlendEnum::CONSTANT_COLOR,
            MyGlBlendEnum::ONE_MINUS_CONSTANT_ALPHA, // 13
        ];

        let mut texture_names_to_index: HashMap<String, usize> = HashMap::new();
        let mut textures: Vec<TextureId> = Vec::new();

        let layers = (0..layer_num)
            .map(|_i| {
                let texture_names: Vec<String> = (0..buf.next_u32())
                    .map(|_i| {
                        let texture_name = buf.string(128);
                        if !texture_names_to_index.contains_key(&texture_name) {
                            let base = Path::new("data")
                                .join("texture")
                                .join("effect")
                                .join(str_name);
                            let root = base.parent().unwrap();
                            let path = format!(
                                "{}\\{}",
                                root.to_str().unwrap().replace("/", "\\"),
                                texture_name
                            );
                            let surface = asset_loader.load_sdl_surface(&path);
                            let surface = surface.unwrap_or_else(|e| {
                                log::warn!(
                                    "Missing texture when loading {}, path: {}, {}",
                                    str_name,
                                    path,
                                    e
                                );
                                asset_loader.backup_surface()
                            });
                            let texture =
                                asset_db.get_texture_id(&texture_name).unwrap_or_else(|| {
                                    AssetLoader::create_texture_from_surface(
                                        gl,
                                        &texture_name,
                                        surface,
                                        MyGlEnum::NEAREST,
                                        asset_db,
                                    )
                                });
                            textures.push(texture);
                            let size = texture_names_to_index.len();
                            texture_names_to_index.insert(texture_name.clone(), size);
                        }
                        texture_name
                    })
                    .collect();
                // TODO: skip layers where key_frames.is_empty()
                let key_frames: Vec<StrKeyFrame> = (0..buf.next_u32())
                    .map(|_i| {
                        let frame = buf.next_i32();
                        let typ = if buf.next_u32() == 0 {
                            KeyFrameType::Start
                        } else {
                            KeyFrameType::End
                        };
                        let pos = [buf.next_f32(), buf.next_f32()];
                        //                        let uv = [
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        buf.next_f32();
                        //                        ];
                        let xy = [
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                            buf.next_f32(),
                        ];
                        StrKeyFrame {
                            frame,
                            typ,
                            pos,
                            //                            uv,
                            xy,
                            texture_index: texture_names_to_index
                                [&texture_names[buf.next_f32() as usize]],
                            //                            anitype: buf.next_u32(),
                            //                            delay: buf.next_f32(),
                            angle: {
                                {
                                    buf.next_u32(); // anitype
                                    buf.next_f32(); // delay
                                }
                                buf.next_f32() / (1024.0 / 360.0)
                            },
                            color: [
                                buf.next_f32() as u8,
                                buf.next_f32() as u8,
                                buf.next_f32() as u8,
                                buf.next_f32() as u8,
                            ],
                            src_alpha: d3d_to_gl_blend[buf.next_u32() as usize],
                            dst_alpha: {
                                let ret = d3d_to_gl_blend[buf.next_u32() as usize];
                                buf.next_u32(); // mtpreset
                                ret
                            },
                            //                            mtpreset: buf.next_u32(),
                        }
                    })
                    .collect();

                StrLayer { key_frames }
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
