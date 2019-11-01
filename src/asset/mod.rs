use crate::asset::act::{Action, ActionFile, ActionFrame};
use crate::asset::texture::{TextureId, DUMMY_TEXTURE_ID_FOR_TEST};

pub mod act;
pub mod asset_async_loader;
pub mod asset_loader;
pub mod binary_reader;
pub mod database;
pub mod gat;
pub mod gnd;
pub mod rsm;
pub mod rsw;
pub mod spr;
pub mod str;
pub mod texture;

#[derive(Debug, Clone)]
pub struct GrfEntry {
    pack_size: u32,
    length_aligned: u32,
    real_size: u32,
    typ: u8,
    offset: u32,
}

#[derive(Clone)]
pub struct SpriteResource {
    pub action: ActionFile,
    pub textures: Vec<TextureId>,
}

impl SpriteResource {
    pub fn new_for_test() -> SpriteResource {
        SpriteResource {
            action: ActionFile {
                actions: (1..80)
                    .map(|_| Action {
                        frames: vec![ActionFrame {
                            layers: vec![],
                            sound: 0,
                            positions: vec![],
                        }],
                        delay: 0,
                        duration: 0.0,
                    })
                    .collect(),
                sounds: vec![],
            },
            textures: vec![DUMMY_TEXTURE_ID_FOR_TEST],
        }
    }
}
