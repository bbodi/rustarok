use crate::components::char::CharAttributeModifierCollector;
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::status::status::{
    Status, StatusStackingResult, StatusType, StatusUpdateResult,
};
use crate::components::ApplyForceComponent;
use crate::systems::atk_calc::AttackOutcome;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::SystemVariables;
use crate::ElapsedTime;
use specs::LazyUpdate;

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

    fn typ(&self) -> StatusType {
        StatusType::Harmful
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

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, _modifiers: &mut CharAttributeModifierCollector) {}

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        _char_pos: &WorldCoords,
        _system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        StatusUpdateResult::KeepIt
    }

    fn affect_incoming_damage(&mut self, outcome: AttackOutcome) -> AttackOutcome {
        outcome
    }

    fn allow_push(&mut self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn render(
        &self,
        _char_pos: &WorldCoords,
        _system_vars: &SystemVariables,
        _render_commands: &mut RenderCommandCollectorComponent,
    ) {
    }

    fn get_status_completion_percent(&self, _now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        None
    }

    fn stack(&mut self, _other: Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }
}
