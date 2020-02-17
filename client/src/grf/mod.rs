use crate::grf::act::{Action, ActionFile, ActionFrame};
use crate::grf::texture::{TextureId, DUMMY_TEXTURE_ID_FOR_TEST};

pub mod act;
pub mod asset_async_loader;
pub mod asset_loader;
pub mod database;
pub mod gnd;
pub mod rsm;
pub mod rsw;
pub mod spr;
pub mod str;
pub mod texture;

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
                        duration_in_millis: 0,
                    })
                    .collect(),
                sounds: vec![],
            },
            textures: vec![DUMMY_TEXTURE_ID_FOR_TEST],
        }
    }
}
