use crate::asset::BinaryReader;
use crate::video::GlTexture;
use sdl2::pixels::PixelFormatEnum;

pub struct SpriteFile {
    pub frames: Vec<Frame>,
}

pub enum SpriteType {
    PAL,
    ABGR,
}

pub struct Frame {
    pub typ: SpriteType,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct SpriteTexture {
    pub original_width: usize,
    pub original_height: usize,
    pub texture: GlTexture,
}

impl SpriteFile {
    pub(super) fn load(mut buf: BinaryReader) -> Self {
        let header = buf.string(2);
        let version = buf.next_u8() as f32 / 10.0 + buf.next_u8() as f32;
        if header != "SP" {
            panic!("Invalig Sprite header: {}", header);
        }

        let indexed_frame_count = buf.next_u16() as usize;
        let rgba_frame_count = if version > 1.1 { buf.next_u16() } else { 0 };
        let indexed_frames = if version < 2.1 {
            SpriteFile::read_indexed_frames(&mut buf, indexed_frame_count)
        } else {
            SpriteFile::read_indexed_frames_rle(&mut buf, indexed_frame_count)
        };

        let rgba_frames = SpriteFile::read_rgba_frames(&mut buf, rgba_frame_count);

        let palette = if version > 1.0 {
            buf.skip(((buf.len() - 1024) - buf.tell()) as u32);
            buf.next(1024)
        } else {
            Vec::new()
        };

        let mut frames = Vec::with_capacity(indexed_frames.len() + rgba_frames.len());

        frames.extend(
            indexed_frames
                .into_iter()
                .map(|frame| SpriteFile::to_rgba(frame, &palette)),
        );
        frames.extend(rgba_frames);

        SpriteFile { frames }
    }

    fn read_indexed_frames(buf: &mut BinaryReader, indexed_frame_count: usize) -> Vec<Frame> {
        (0..indexed_frame_count)
            .map(|_i| {
                let width = buf.next_u16();
                let height = buf.next_u16();
                Frame {
                    typ: SpriteType::PAL,
                    width: width as usize,
                    height: height as usize,
                    data: buf.next(width as u32 * height as u32),
                }
            })
            .collect()
    }

    fn to_rgba(frame: Frame, pal: &Vec<u8>) -> Frame {
        let mut buf = Vec::<u8>::with_capacity((frame.width * frame.height * 4) as usize);
        for y in 0..frame.height {
            for x in 0..frame.width {
                let idx1 = frame.data[(y * frame.width + x)] as usize * 4;
                buf.push(pal[idx1 + 0]); // r
                buf.push(pal[idx1 + 1]); // g
                buf.push(pal[idx1 + 2]); // b
                buf.push(if idx1 != 0 { 255 } else { 0 }); // a
            }
        }
        Frame {
            typ: SpriteType::ABGR,
            data: buf,
            ..frame
        }
    }

    fn read_indexed_frames_rle(buf: &mut BinaryReader, indexed_frame_count: usize) -> Vec<Frame> {
        (0..indexed_frame_count)
            .map(|_i| {
                let width = buf.next_u16();
                let height = buf.next_u16();
                let end = buf.next_u16() as usize + buf.tell();
                let mut data = Vec::<u8>::with_capacity(width as usize * height as usize);
                while buf.tell() < end {
                    let c = buf.next_u8();
                    data.push(c);
                    if c == 0 {
                        let count = buf.next_u8();
                        if count == 0 {
                            data.push(count);
                        } else {
                            for _i in 1..count {
                                data.push(c);
                            }
                        }
                    }
                }
                Frame {
                    typ: SpriteType::PAL,
                    width: width as usize,
                    height: height as usize,
                    data,
                }
            })
            .collect()
    }

    fn read_rgba_frames(buf: &mut BinaryReader, rgba_frame_count: u16) -> Vec<Frame> {
        (0..rgba_frame_count)
            .map(|_i| {
                let width = buf.next_u16();
                let height = buf.next_u16();
                let mut data = buf.next(width as u32 * height as u32 * 4);
                // it seems ABGR sprites are stored upside down
                data.reverse();
                Frame {
                    typ: SpriteType::ABGR,
                    width: width as usize,
                    height: height as usize,
                    data,
                }
            })
            .collect()
    }
}

impl SpriteTexture {
    pub fn from(mut frame: Frame) -> SpriteTexture {
        let frame_surface = sdl2::surface::Surface::from_data(
            &mut frame.data,
            frame.width as u32,
            frame.height as u32,
            (4 * frame.width) as u32,
            PixelFormatEnum::RGBA32,
        )
        .unwrap();
        // Calculate new texture size and move the sprite into the center
        let gl_width = frame.width; //.next_power_of_two();
        let gl_height = frame.height; //.next_power_of_two();
        let start_x = ((gl_width - frame.width) as f32 * 0.5).floor() as u32;
        let start_y = ((gl_height - frame.height) as f32 * 0.5).floor() as u32;

        let mut opengl_surface =
            sdl2::surface::Surface::new(gl_width as u32, gl_height as u32, PixelFormatEnum::RGBA32)
                .unwrap();

        let dst_rect = sdl2::rect::Rect::new(
            start_x as i32,
            start_y as i32,
            frame.width as u32,
            frame.height as u32,
        );
        frame_surface
            .blit(None, &mut opengl_surface, dst_rect)
            .unwrap();

        SpriteTexture {
            original_width: frame.width,
            original_height: frame.height,
            texture: GlTexture::from_surface(opengl_surface, gl::NEAREST),
        }
    }
}
