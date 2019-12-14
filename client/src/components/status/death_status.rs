use crate::ElapsedTime;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeathStatus {
    pub started: ElapsedTime,
    pub remove_char_at: ElapsedTime,
    pub is_npc: bool,
}

impl DeathStatus {
    pub fn new(now: ElapsedTime, is_npc: bool) -> DeathStatus {
        DeathStatus {
            is_npc,
            started: now,
            remove_char_at: now.add_seconds(2.0),
        }
    }
}
