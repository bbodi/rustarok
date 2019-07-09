use crate::common::BinaryReader;
use sdl2::pixels::PixelFormatEnum;
use crate::video::GlTexture;

pub struct StrFile {
    pub fps: u32,
    pub layers: Vec<StrLayer>,
}

pub struct StrLayer {
    pub texture_names: Vec<String>,
    pub key_frames: Vec<StrKeyFrame>
}


pub struct StrKeyFrame {
    pub frame: i32,
    pub typ: u32,
    pub pos: [f32; 2],
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub aniframe: f32,
    pub anitype: u32,
    pub angle: f32,
    pub delay: f32,
    pub color: [f32; 4],
    pub src_alpha: u32,
    pub dst_alpha: u32,
    pub mtpreset: u32
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

        let layers = (0..layer_num).map(|_i| {
            let texture_names: Vec<String> = (0..buf.next_u32()).map(|_i| {
                buf.string(128)
            }).collect();
            let key_frames: Vec<StrKeyFrame> = (0..buf.next_u32()).map(|_i| {
                StrKeyFrame {
                    frame: buf.next_i32(),
                    typ: buf.next_u32(),
                    pos: [buf.next_f32(), buf.next_f32()],
                    uv: [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()],
                    xy: [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()],
                    aniframe: buf.next_f32(),
                    anitype: buf.next_u32(),
                    delay: buf.next_f32(),
                    angle: buf.next_f32() / (1024.0/360.0),
                    color: [buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0, buf.next_f32() / 255.0],
                    src_alpha: buf.next_u32(),
                    dst_alpha: buf.next_u32(),
                    mtpreset: buf.next_u32(),
                }
            }).collect();

            StrLayer {
                texture_names,
                key_frames,
            }
        }).collect();
        StrFile {
            fps,
            layers
        }
    }
}