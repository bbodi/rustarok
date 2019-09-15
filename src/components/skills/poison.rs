use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{ApplyStatusComponent, MainStatuses};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;

pub struct PosionSkill;

pub const POISON_SKILL: &'static PosionSkill = &PosionSkill;

impl SkillDef for PosionSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\tf_poison.bmp"
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
        updater.insert(
            entities.create(),
            StrEffectComponent {
                effect_id: StrEffectType::Poison.into(),
                pos: skill_pos.unwrap(),
                start_time: system_vars.time,
                die_at: Some(system_vars.time.add_seconds(0.7)),
                play_mode: ActionPlayMode::Repeat,
            },
        );
        system_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_main_status(
                caster_entity_id,
                target_entity.unwrap(),
                MainStatuses::Poison(system_vars.dev_configs.skills.poison.damage),
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}
