use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{ApplyStatusComponent, MainStatuses};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;

pub struct MountingSkill;

pub const MOUNTING_SKILL: &'static MountingSkill = &MountingSkill;

impl SkillDef for MountingSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\su_pickypeck.bmp"
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
            .push(ApplyStatusComponent::from_main_status(
                caster_entity_id,
                caster_entity_id,
                MainStatuses::Mounted,
            ));
        updater.insert(
            entities.create(),
            StrEffectComponent {
                effect_id: StrEffectType::Concentration.into(),
                pos: caster.pos(),
                start_time: system_vars.time,
                die_at: system_vars.time.add_seconds(0.7),
            },
        );
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::NoTarget
    }
}
