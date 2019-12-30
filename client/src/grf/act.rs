use crate::grf::asset_async_loader::SPRITE_UPSCALE_FACTOR;
use crate::render::render_sys::COLOR_WHITE;
use rustarok_common::grf::binary_reader::BinaryReader;
use std::ops::RangeBounds;

#[derive(Debug, Clone)]
pub struct ActionFile {
    pub actions: Vec<Action>,
    pub sounds: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub frames: Vec<ActionFrame>,
    pub delay: u32,
    pub duration: f32,
}

#[derive(Debug, Clone)]
pub struct ActionFrame {
    pub layers: Vec<Layer>,
    pub sound: i32,
    pub positions: Vec<[i32; 2]>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub pos: [i32; 2],
    pub sprite_frame_index: i32,
    // can be -1!!
    pub is_mirror: bool,
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub angle: i32,
    pub spr_type: i32,
    pub width: i32,
    pub height: i32,
}

impl ActionFile {
    pub fn remove_frames_in_every_direction<R>(&mut self, action_index: usize, range: R)
    where
        R: RangeBounds<usize> + Clone,
    {
        for i in 0..8 {
            self.actions[action_index + i].frames.drain(range.clone());
        }
    }

    pub(super) fn load(mut buf: BinaryReader) -> Self {
        let header = buf.string(2);
        if header != "AC" {
            panic!("Invalig Action header: {}", header);
        }

        let version = buf.next_u8() as f32 / 10.0 + buf.next_u8() as f32;

        let action_acount = buf.next_u16() as usize;
        buf.skip(10);

        let mut actions: Vec<Action> = (0..action_acount)
            .map(|_i| Action {
                frames: ActionFile::read_animations(&mut buf, version),
                delay: 150,
                duration: 0.0,
            })
            .collect();
        let sounds = if version >= 2.1 {
            (0..buf.next_i32()).map(|_i| buf.string(40)).collect()
        } else {
            vec![]
        };
        actions.iter_mut().for_each(|a| {
            if version >= 2.2 {
                a.delay = (buf.next_f32() * 25f32) as u32;
            }
            a.duration = a.delay as f32 / 1000.0 * a.frames.len() as f32;
        });
        return ActionFile { actions, sounds };
    }

    fn read_animations(buf: &mut BinaryReader, version: f32) -> Vec<ActionFrame> {
        let animation_count = buf.next_u32() as usize;
        (0..animation_count)
            .map(|_i| {
                let _unknown = buf.skip(32);
                ActionFrame {
                    layers: ActionFile::read_layers(buf, version),
                    sound: if version >= 2.0 { buf.next_i32() } else { -1 },
                    positions: if version >= 2.3 {
                        (0..buf.next_i32())
                            .map(|_i| {
                                buf.skip(4);
                                let pos = [
                                    buf.next_i32() * SPRITE_UPSCALE_FACTOR as i32,
                                    buf.next_i32() * SPRITE_UPSCALE_FACTOR as i32,
                                ];
                                buf.skip(4);
                                pos
                            })
                            .collect()
                    } else {
                        vec![]
                    },
                }
            })
            .collect()
    }

    fn read_layers(buf: &mut BinaryReader, version: f32) -> Vec<Layer> {
        let layer_count = buf.next_u32() as usize;
        (0..layer_count)
            .map(|_i| {
                let pos = [
                    buf.next_i32() * SPRITE_UPSCALE_FACTOR as i32,
                    buf.next_i32() * SPRITE_UPSCALE_FACTOR as i32,
                ];
                let sprite_frame_index = buf.next_i32();
                let is_mirror = buf.next_i32() != 0;
                let color = if version >= 2.0 {
                    [buf.next_u8(), buf.next_u8(), buf.next_u8(), buf.next_u8()]
                } else {
                    COLOR_WHITE
                };
                let scale = if version >= 2.0 {
                    let scale_0 = buf.next_f32();
                    [
                        scale_0,
                        if version <= 2.3 {
                            scale_0
                        } else {
                            buf.next_f32()
                        },
                    ]
                } else {
                    [1.0, 1.0]
                };
                let angle = if version >= 2.0 { buf.next_i32() } else { 0 };
                let spr_type = if version >= 2.0 { buf.next_i32() } else { 0 };
                let width = if version >= 2.5 { buf.next_i32() } else { 0 };
                let height = if version >= 2.5 { buf.next_i32() } else { 0 };

                Layer {
                    pos,
                    sprite_frame_index,
                    is_mirror,
                    scale,
                    color,
                    angle,
                    spr_type,
                    width,
                    height,
                }
            })
            .filter(|it| it.sprite_frame_index >= 0) // for head sprites, the first layer refers to sprite '-1', which is skipped anyway during rendering
            .collect()
    }
}
