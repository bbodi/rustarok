use nalgebra::Vector3;
use rustarok_common::common::v3;
use rustarok_common::grf::binary_reader::BinaryReader;
use std::borrow::ToOwned;

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

#[derive(Debug, Clone)]
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
    pub longitude: i32,
    pub latitude: i32,
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
    pub opacity: f32,
    pub direction: [f32; 3],
}

#[derive(Debug)]
pub struct Rsw {
    pub ground: GroundData,
    pub water: WaterData,
    pub file: FileData,
    pub light: LightData,
    pub models: Vec<RswModelInstance>,
    pub lights: Vec<MapLight>,
    pub sounds: Vec<MapSound>,
    pub effects: Vec<MapEffect>,
}

#[derive(Debug)]
pub struct RswModelInstance {
    pub name: String,
    pub anim_type: i32,
    pub anim_speed: f32,
    pub block_type: i32,
    pub filename: String,
    pub node_name: String,
    pub pos: Vector3<f32>,
    pub rot: Vector3<f32>,
    pub scale: Vector3<f32>,
}

#[derive(Debug)]
pub struct MapLight {
    name: String,
    pos: Vector3<f32>,
    color: [i32; 3],
    range: f32,
}

#[derive(Debug)]
pub struct MapEffect {
    name: String,
    pos: Vector3<f32>,
    id: i32,
    delay: f32,
    param: [f32; 4],
}

#[derive(Debug)]
pub struct MapSound {
    name: String,
    file: String,
    pos: Vector3<f32>,
    vol: f32,
    width: i32,
    height: i32,
    range: f32,
    cycle: f32,
}

impl Rsw {
    pub(super) fn load(mut buf: BinaryReader) -> Self {
        let header = buf.string(4);
        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        if header != "GRSW" {
            panic!();
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

        fn calc_dir(longitude: i32, latitude: i32) -> [f32; 3] {
            let longitude = (longitude as f32).to_radians();
            let latitude = (latitude as f32).to_radians();
            [
                -longitude.cos() * latitude.sin(),
                -latitude.cos(),
                -longitude.sin() * latitude.sin(),
            ]
        }

        let light = if version >= 1.5 {
            let longitude = buf.next_i32();
            let latitude = buf.next_i32();
            LightData {
                longitude,
                latitude,
                diffuse: [buf.next_f32(), buf.next_f32(), buf.next_f32()],
                ambient: [buf.next_f32(), buf.next_f32(), buf.next_f32()],
                opacity: if version >= 1.7 { buf.next_f32() } else { 1.0 },
                direction: calc_dir(longitude, latitude),
            }
        } else {
            LightData {
                longitude: 45,
                latitude: 45,
                diffuse: [1.0, 1.0, 1.0],
                ambient: [0.3, 0.3, 0.3],
                opacity: 1.0,
                direction: calc_dir(45, 45),
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
        let mut models: Vec<RswModelInstance> = Vec::with_capacity(count as usize);
        let mut lights: Vec<MapLight> = Vec::with_capacity(count as usize);
        let mut sounds: Vec<MapSound> = Vec::with_capacity(count as usize);
        let mut effects: Vec<MapEffect> = Vec::with_capacity(count as usize);
        for _i in 0..count {
            let typ = buf.next_i32();
            match typ {
                1 => models.push(RswModelInstance {
                    name: if version >= 1.3 {
                        buf.string(40)
                    } else {
                        "".to_owned()
                    },
                    anim_type: if version >= 1.3 { buf.next_i32() } else { 0 },
                    anim_speed: if version >= 1.3 { buf.next_f32() } else { 0.0 },
                    block_type: if version >= 1.3 { buf.next_i32() } else { 0 },
                    filename: buf.string(80),
                    node_name: buf.string(80),
                    pos: v3(
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                    ),
                    rot: v3(buf.next_f32(), buf.next_f32(), buf.next_f32()),
                    scale: v3(
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                    ),
                }),
                2 => lights.push(MapLight {
                    name: buf.string(80),
                    pos: v3(
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                    ),
                    color: [buf.next_i32(), buf.next_i32(), buf.next_i32()],
                    range: buf.next_f32(),
                }),
                3 => sounds.push(MapSound {
                    name: buf.string(80),
                    file: buf.string(80),
                    pos: v3(
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                    ),
                    vol: buf.next_f32(),
                    width: buf.next_i32(),
                    height: buf.next_i32(),
                    range: buf.next_f32(),
                    cycle: if version >= 2.0 { buf.next_f32() } else { 0.0 },
                }),
                4 => effects.push(MapEffect {
                    name: buf.string(80),
                    pos: v3(
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                        buf.next_f32() / 5.0,
                    ),
                    id: buf.next_i32(),
                    delay: buf.next_f32() * 10.0,
                    param: [
                        buf.next_f32(),
                        buf.next_f32(),
                        buf.next_f32(),
                        buf.next_f32(),
                    ],
                }),
                _ => panic!("Wrong entity type: {}", typ),
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
