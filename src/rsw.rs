use std::borrow::ToOwned;
use crate::common::{Vec3, BinaryReader};

#[derive(Debug)]
pub struct GroundData {
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
}

#[derive(Debug)]
pub struct FileData {
    ini: String,
    gnd: String,
    gat: String,
    src: String,
}

#[derive(Debug)]
pub struct WaterData {
    pub level: f32,
    pub typ: i32,
    pub wave_height: f32,
    pub wave_speed: f32,
    pub wave_pitch: f32,
    pub anim_speed: i32,
    pub images: [i32; 32],
}

#[derive(Debug)]
pub struct LightData {
    longitude: i32,
    latitude: i32,
    diffuse: Vec3,
    ambient: Vec3,
    opacity: f32,
    direction: Vec3,
}

#[derive(Debug)]
pub struct Rsw {
    pub ground: GroundData,
    pub water: WaterData,
    pub file: FileData,
    pub light: LightData,
    pub models: Vec<MapModel>,
    pub lights: Vec<MapLight>,
    pub sounds: Vec<MapSound>,
    pub effects: Vec<MapEffect>,
}

#[derive(Debug)]
pub struct MapModel {
    name: String,
    anim_type: i32,
    anim_speed: f32,
    block_type: i32,
    filename: String,
    nodename: String,
    pos: Vec3,
    rot: Vec3,
    scale: Vec3,
}

#[derive(Debug)]
pub struct MapLight {
    name: String,
    pos: Vec3,
    color: [i32; 3],
    range: f32,
}

#[derive(Debug)]
pub struct MapEffect {
    name: String,
    pos: Vec3,
    id: i32,
    delay: f32,
    param: [f32; 4],
}

#[derive(Debug)]
pub struct MapSound {
    name: String,
    file: String,
    pos: Vec3,
    vol: f32,
    width: i32,
    height: i32,
    range: f32,
    cycle: f32,
}

impl Rsw {
    pub fn load(mut buf: BinaryReader) -> Rsw {
        let header = buf.string(4);
        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        if header != "GRSW" {
            // shit
        }

        let file = FileData {
            ini: buf.string(40),
            gnd: buf.string(40),
            gat: buf.string(40),
            src: if version >= 1.4 {
                buf.string(40)
            } else {
                "".to_owned()
            },
        };

        let water = if version >= 1.8 {
            let water_level = buf.next_f32();
            WaterData {
                level: water_level,
                typ: buf.next_i32(),
                wave_height: buf.next_f32() / 5.0,
                wave_speed: buf.next_f32(),
                wave_pitch: buf.next_f32(),
                anim_speed: if version >= 1.9 { buf.next_i32() } else { 0 },
                images: [0; 32],
            }
        } else {
            let water_level = if version >= 1.3 { buf.next_f32() } else { 0.0 };
            WaterData {
                level: water_level,
                typ: 0,
                wave_height: 0.2,
                wave_speed: 2.0,
                wave_pitch: 50.0,
                anim_speed: 3,
                images: [0; 32],
            }
        };

        let light = if version >= 1.5 {
            LightData {
                longitude: buf.next_i32(),
                latitude: buf.next_i32(),
                diffuse: Vec3(buf.next_f32(), buf.next_f32(), buf.next_f32()),
                ambient: Vec3(buf.next_f32(), buf.next_f32(), buf.next_f32()),
                opacity: if version >= 1.7 { buf.next_f32() } else { 1.0 },
                direction: Vec3(0f32, 0f32, 0f32),
            }
        } else {
            LightData {
                longitude: 45,
                latitude: 45,
                diffuse: Vec3(1.0, 1.0, 1.0),
                ambient: Vec3(0.3, 0.3, 0.3),
                opacity: 1.0,
                direction: Vec3(0f32, 0f32, 0f32),
            }
        };

        let ground = if version >= 1.6 {
            GroundData {
                top: buf.next_i32(),
                bottom: buf.next_i32(),
                left: buf.next_i32(),
                right: buf.next_i32(),
            }
        } else {
            GroundData {
                top: -500,
                bottom: 500,
                left: -500,
                right: 500,
            }
        };

        let count = buf.next_i32();
        println!("version: {:?}", version);
        println!("index: {:?}", buf.tell());
        println!("ground: {:?}", ground);
        println!("water: {:?}", water);
        println!("light: {:?}", light);
        println!("Count: {}", count);
        let mut models: Vec<MapModel> = Vec::with_capacity(count as usize);
        let mut lights: Vec<MapLight> = Vec::with_capacity(count as usize);
        let mut sounds: Vec<MapSound> = Vec::with_capacity(count as usize);
        let mut effects: Vec<MapEffect> = Vec::with_capacity(count as usize);
        for i in 0..count {
            let typ = buf.next_i32();
            match typ {
                1 => {
                    models.push(MapModel {
                        name: if version >= 1.3 { buf.string(40) } else { "".to_owned() },
                        anim_type: if version >= 1.3 { buf.next_i32() } else { 0 },
                        anim_speed: if version >= 1.3 { buf.next_f32() } else { 0.0 },
                        block_type: if version >= 1.3 { buf.next_i32() } else { 0 },
                        filename: buf.string(80),
                        nodename: buf.string(80),
                        pos: Vec3(buf.next_f32() / 5.0, buf.next_f32() / 5.0, buf.next_f32() / 5.0),
                        rot: Vec3(buf.next_f32(), buf.next_f32(), buf.next_f32()),
                        scale: Vec3(buf.next_f32() / 5.0, buf.next_f32() / 5.0, buf.next_f32() / 5.0),
                    })
                }
                2 => lights.push(MapLight {
                    name: buf.string(80),
                    pos: Vec3(buf.next_f32() / 5.0, buf.next_f32() / 5.0, buf.next_f32() / 5.0),
                    color: [buf.next_i32(), buf.next_i32(), buf.next_i32()],
                    range: buf.next_f32(),
                }),
                3 => sounds.push(MapSound {
                    name: buf.string(80),
                    file: buf.string(80),
                    pos: Vec3(buf.next_f32() / 5.0, buf.next_f32() / 5.0, buf.next_f32() / 5.0),
                    vol: buf.next_f32(),
                    width: buf.next_i32(),
                    height: buf.next_i32(),
                    range: buf.next_f32(),
                    cycle: if version >= 2.0 { buf.next_f32() } else { 0.0 },
                }),
                4 => effects.push(MapEffect {
                    name: buf.string(80),
                    pos: Vec3(buf.next_f32() / 5.0, buf.next_f32() / 5.0, buf.next_f32() / 5.0),
                    id: buf.next_i32(),
                    delay: buf.next_f32() * 10.0,
                    param: [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()],
                }),
                _ => panic!("Wrong entity type: {}", typ)
            }
        }
        models.shrink_to_fit();
        lights.shrink_to_fit();
        effects.shrink_to_fit();
        sounds.shrink_to_fit();

        return Rsw {
            ground,
            water,
            file,
            light,
            models,
            lights,
            sounds,
            effects,
        };
    }
}