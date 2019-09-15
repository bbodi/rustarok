use crate::components::char::{
    CharAttributeModifier, CharAttributeModifierCollector, CharacterStateComponent, Percentage,
};
use crate::components::controller::CharEntityId;
use crate::components::status::status::{Status, StatusNature, StatusUpdateResult};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;
use crate::ElapsedTime;
use specs::LazyUpdate;

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

    fn typ(&self) -> StatusNature {
        StatusNature::Supportive // depends
    }

    fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        modifiers.change_armor(
            CharAttributeModifier::AddPercentage(self.modifier),
            self.started,
            self.until,
        );
    }

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        _char_state: &CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(system_vars.time) {
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }
}
