use crate::GameTime;
use rustarok_common::common::Local;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeathStatus {
    pub started: GameTime<Local>,
    pub remove_char_at: GameTime<Local>,
    pub is_npc: bool,
}

impl DeathStatus {
    pub fn new(now: GameTime<Local>, is_npc: bool) -> DeathStatus {
        DeathStatus {
            is_npc,
            started: now,
            remove_char_at: now.add_seconds(2.0),
        }
    }
}
