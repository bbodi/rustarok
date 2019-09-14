use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{RemoveStatusComponent, StatusNature};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;

pub struct CureSkill;

pub const CURE_SKILL: &'static CureSkill = &CureSkill;

impl SkillDef for CureSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\so_el_cure.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster: &CharacterStateComponent,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        system_vars
            .remove_statuses
            .push(RemoveStatusComponent::by_status_nature(
                caster_entity_id,
                target_entity.unwrap(),
                StatusNature::Harmful,
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
