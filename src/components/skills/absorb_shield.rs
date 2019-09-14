use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::status::ApplyStatusComponent;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;

pub struct AbsorbShieldSkill;

pub const ABSORB_SHIELD_SKILL: &'static AbsorbShieldSkill = &AbsorbShieldSkill;

impl SkillDef for AbsorbShieldSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
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
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                caster_entity_id,
                target_entity.unwrap(),
                Box::new(AbsorbStatus::new(
                    caster_entity_id,
                    system_vars.time,
                    system_vars
                        .dev_configs
                        .skills
                        .absorb_shield
                        .duration_seconds,
                )),
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
