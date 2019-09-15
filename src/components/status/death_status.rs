use crate::components::status::status::{Status, StatusNature, StatusStackingResult};
use crate::components::ApplyForceComponent;
use crate::ElapsedTime;

#[derive(Clone, Debug)]
pub struct DeathStatus {
    pub started: ElapsedTime,
    pub remove_char_at: ElapsedTime,
    is_npc: bool,
}

impl DeathStatus {
    pub fn new(now: ElapsedTime, is_npc: bool) -> Box<DeathStatus> {
        Box::new(DeathStatus {
            is_npc,
            started: now,
            remove_char_at: now.add_seconds(2.0),
        })
    }
}

impl Status for DeathStatus {
    fn dupl(&self) -> Box<dyn Status> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        false
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Harmful
    }

    fn can_target_cast(&self) -> bool {
        false
    }

    fn get_render_color(&self, now: ElapsedTime) -> [u8; 4] {
        [
            255,
            255,
            255,
            if self.is_npc {
                255 - (now.percentage_between(self.started, self.remove_char_at) * 255.0) as u8
            } else {
                255
            },
        ]
    }

    fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn stack(&self, _other: Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }
}
