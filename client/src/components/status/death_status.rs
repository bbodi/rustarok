use crate::LocalTime;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeathStatus {
    pub started: LocalTime,
    pub remove_char_at: LocalTime,
    pub is_npc: bool,
}

impl DeathStatus {
    pub fn new(now: LocalTime, is_npc: bool) -> DeathStatus {
        DeathStatus {
            is_npc,
            started: now,
            remove_char_at: now.add_seconds(2.0),
        }
    }
}
