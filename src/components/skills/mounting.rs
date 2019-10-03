use specs::LazyUpdate;

use crate::components::char::ActionPlayMode;
use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};
use crate::components::status::status::{ApplyStatusComponent, MainStatuses};
use crate::components::StrEffectComponent;
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::specs::prelude::*;
use crate::systems::SystemVariables;

pub struct MountingSkill;

pub const MOUNTING_SKILL: &'static MountingSkill = &MountingSkill;

impl SkillDef for MountingSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\su_pickypeck.bmp"
    }

    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(finish_cast_data, MountingSkill::do_finish_cast),
        );
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::NoTarget
    }
}

impl MountingSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        let now = sys_vars.time;
        updater.insert(
            entities.create(),
            StrEffectComponent {
                effect_id: StrEffectType::Concentration.into(),
                pos: finish_cast.caster_pos,
                start_time: now,
                die_at: Some(now.add_seconds(0.7)),
                play_mode: ActionPlayMode::Once,
            },
        );

        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_main_status(
                finish_cast.caster_entity_id,
                finish_cast.caster_entity_id,
                MainStatuses::Mounted,
            ));
    }
}
