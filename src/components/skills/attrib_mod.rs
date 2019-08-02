use crate::asset::SpriteResource;
use crate::components::char::{CharAttributeModifier, CharAttributeModifierCollector, Percentage};
use crate::components::controller::WorldCoords;
use crate::components::status::{Status, StatusStackingResult, StatusType, StatusUpdateResult};
use crate::components::ApplyForceComponent;
use crate::consts::JobId;
use crate::systems::atk_calc::AttackOutcome;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::ElapsedTime;
use specs::{Entity, LazyUpdate};

#[derive(Clone)]
pub struct ArmorModifierStatus {
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub modifier: Percentage,
}

impl ArmorModifierStatus {
    pub fn new(now: ElapsedTime, modifier: Percentage) -> ArmorModifierStatus {
        ArmorModifierStatus {
            started: now,
            until: now.add_seconds(10.0),
            modifier,
        }
    }
}

impl Status for ArmorModifierStatus {
    fn dupl(&self) -> Box<dyn Status> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        true
    }

    fn typ(&self) -> StatusType {
        StatusType::Supportive // depends
    }

    fn can_target_cast(&self) -> bool {
        true
    }

    fn get_render_color(&self) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        modifiers.change_armor(
            CharAttributeModifier::AddPercentage(self.modifier),
            self.started,
            self.until,
        );
    }

    fn calc_render_sprite<'a>(
        &self,
        _job_id: JobId,
        _head_index: usize,
        _sex: Sex,
        _sprites: &'a Sprites,
    ) -> Option<&'a SpriteResource> {
        None
    }

    fn update(
        &mut self,
        _self_char_id: Entity,
        _char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.has_passed(system_vars.time) {
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    fn affect_incoming_damage(&mut self, outcome: AttackOutcome) -> AttackOutcome {
        outcome
    }

    fn allow_push(&mut self, _push: &ApplyForceComponent) -> bool {
        true
    }

    fn render(
        &self,
        _char_pos: &WorldCoords,
        _system_vars: &mut SystemVariables,
        _render_commands: &mut RenderCommandCollectorComponent,
    ) {

    }

    fn get_status_completion_percent(&self, _now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        None
    }

    fn stack(&mut self, _other: Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::AddTheNewStatus
    }
}
