use rustarok_common::grf::binary_reader::BinaryReader;
use std::rc::Rc;
use std::sync::Arc;

pub struct SpriteFile {
    pub frames: Vec<SprFrame>,
    pub buffer: Vec<u8>,
}

pub enum SpriteType {
    PAL,
    ABGR,
}

pub struct SprFrame {
    pub typ: SpriteType,
    pub width: usize,
    pub height: usize,
    pub data_index: usize,
    //    pub data: &'a [u8],
}

impl SpriteFile {
    pub(super) fn read_header(buf: &mut BinaryReader) -> (f32, usize, u16) {
        let header = buf.string(2);
        let version = buf.next_u8() as f32 / 10.0 + buf.next_u8() as f32;
        if header != "SP" {
            panic!("Invalig Sprite header: {}", header);
        }

        let indexed_frame_count = buf.next_u16() as usize;
        let rgba_frame_count = if version > 1.1 { buf.next_u16() } else { 0 };
        return (version, indexed_frame_count, rgba_frame_count);
    }

    pub(super) fn load(
        mut reader: BinaryReader,
        palette: Option<&[u8]>,
        version: f32,
        indexed_frame_count: usize,
        rgba_frame_count: u16,
    ) -> Self {
        let indexed_frames = if version < 2.1 {
            SpriteFile::read_indexed_frames(&mut reader, indexed_frame_count)
        } else {
            SpriteFile::read_indexed_frames_rle(&mut reader, indexed_frame_count)
        };

        let rgba_frames = SpriteFile::read_rgba_frames(&mut reader, rgba_frame_count);

        let palette = {
            let default_palette = if version > 1.0 {
                reader.skip(((reader.len() - 1024) - reader.tell()) as u32);
                reader.get_slice(reader.tell(), 1024)
            } else {
                &[]
            };
            palette.map(|it| it).unwrap_or(default_palette)
        };

        // allocate memory for all frames
        let buf_size = indexed_frames
            .iter()
            .map(|it| it.width * it.height * 4)
            .sum::<usize>()
            + rgba_frames
                .iter()
                .map(|it| it.width * it.height * 4)
                .sum::<usize>();
        let mut buffer = Vec::with_capacity(buf_size);

        let mut frames = Vec::with_capacity(indexed_frames.len() + rgba_frames.len());

        if version < 2.1 {
            frames.extend(indexed_frames.into_iter().map(|frame| {
                let data_index = frame.data_index;
                SpriteFile::indexed_to_rgba(
                    frame,
                    &palette,
                    &mut buffer,
                    &reader.as_slice_from(data_index),
                )
            }));
        } else {
            frames.extend(indexed_frames.into_iter().map(|frame| {
                let data_index = frame.data_index;
                SpriteFile::indexed_to_rgba_rle(
                    frame,
                    &palette,
                    &mut buffer,
                    reader.as_slice_from(data_index),
                )
            }));
        }
        frames.extend(
            rgba_frames
                .into_iter()
                .map(|frame| SpriteFile::copy_rgba_frames(frame, &mut buffer, &reader)),
        );

        SpriteFile { buffer, frames }
    }

    fn read_indexed_frames(buf: &mut BinaryReader, indexed_frame_count: usize) -> Vec<SprFrame> {
        (0..indexed_frame_count)
            .map(|_i| {
                let width = buf.next_u16();
                let height = buf.next_u16();
                let frame = SprFrame {
                    typ: SpriteType::PAL,
                    width: width as usize,
                    height: height as usize,
                    data_index: buf.tell(),
                };
                buf.skip(width as u32 * height as u32);
                //                buf.next(width as u32 * height as u32).to_vec();
                frame
            })
            .collect()
    }

    fn copy_rgba_frames(frame: SprFrame, dst_buf: &mut Vec<u8>, reader: &BinaryReader) -> SprFrame {
        let data_index = dst_buf.len();
        dst_buf.extend_from_slice(
            reader.get_slice(frame.data_index, (frame.width * frame.height * 4)),
        );
        SprFrame {
            typ: SpriteType::ABGR,
            data_index,
            ..frame
        }
    }

    fn indexed_to_rgba_rle(
        frame: SprFrame,
        pal: &[u8],
        dst_buf: &mut Vec<u8>,
        src_buf: &[u8],
    ) -> SprFrame {
        let mut index = 0;
        let len = BinaryReader::as_u16(src_buf, index) as usize;
        index += 2;

        // extract rle encoding into the dst buf, one item looks like [palette, 0, 0, 0]
        let start_dst_index = dst_buf.len();
        while index - 2 < len {
            let c = src_buf[index];
            index += 1;
            dst_buf.push(c);
            // fillers
            dst_buf.push(0);
            dst_buf.push(0);
            dst_buf.push(0);
            if c == 0 {
                let count = src_buf[index];
                index += 1;
                if count == 0 {
                    dst_buf.push(0);
                    // fillers
                    dst_buf.push(0);
                    dst_buf.push(0);
                    dst_buf.push(0);
                } else {
                    for _i in 1..count {
                        dst_buf.push(c);
                        // fillers
                        dst_buf.push(0);
                        dst_buf.push(0);
                        dst_buf.push(0);
                    }
                }
            }
        }
        // replace palette indices with rgba colors
        for i in 0..frame.height * frame.width {
            let dst_index = start_dst_index + i * 4;
            let idx1 = dst_buf[dst_index] as usize * 4;
            dst_buf[dst_index + 0] = pal[idx1 + 0];
            dst_buf[dst_index + 1] = pal[idx1 + 1];
            dst_buf[dst_index + 2] = pal[idx1 + 2];
            dst_buf[dst_index + 3] = if idx1 != 0 { 255 } else { 0 };
        }

        SprFrame {
            typ: SpriteType::ABGR,
            data_index: start_dst_index,
            ..frame
        }
    }

    fn indexed_to_rgba(
        frame: SprFrame,
        pal: &[u8],
        dst_buf: &mut Vec<u8>,
        reader: &[u8],
    ) -> SprFrame {
        let data_index = dst_buf.len();
        for y in 0..frame.height {
            for x in 0..frame.width {
                let idx1 = reader[(y * frame.width + x)] as usize * 4;
                //                let idx1 = frame.data[(y * frame.width + x)] as usize * 4;
                dst_buf.push(pal[idx1 + 0]); // r
                dst_buf.push(pal[idx1 + 1]); // g
                dst_buf.push(pal[idx1 + 2]); // b
                dst_buf.push(if idx1 != 0 { 255 } else { 0 }); // a
            }
        }
        SprFrame {
            typ: SpriteType::ABGR,
            data_index,
            //            data: buf,
            ..frame
        }
    }

    fn read_indexed_frames_rle(
        reader: &mut BinaryReader,
        indexed_frame_count: usize,
    ) -> Vec<SprFrame> {
        (0..indexed_frame_count)
            .map(|_i| {
                let width = reader.next_u16();
                let height = reader.next_u16();
                let data_index = reader.tell();
                let size = reader.next_u16();
                reader.skip(size as u32);

                SprFrame {
                    typ: SpriteType::PAL,
                    width: width as usize,
                    height: height as usize,
                    data_index,
                }
            })
            .collect()
    }

    fn read_rgba_frames(buf: &mut BinaryReader, rgba_frame_count: u16) -> Vec<SprFrame> {
        (0..rgba_frame_count)
            .map(|_i| {
                let width = buf.next_u16();
                let height = buf.next_u16();
                //                let mut data = buf.next(width as u32 * height as u32 * 4).to_vec();
                //                 it seems ABGR sprites are stored upside down
                //                data.reverse();
                let frame = SprFrame {
                    typ: SpriteType::ABGR,
                    width: width as usize,
                    height: height as usize,
                    data_index: buf.tell(), //                    data,
                };
                buf.skip(width as u32 * height as u32 * 4);
                frame
            })
            .collect()
    }
}
