use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::status::ApplyStatusComponent;
use crate::configs::DevConfig;
use crate::systems::SystemVariables;
use specs::{Entities, LazyUpdate};

pub struct AbsorbShieldSkill;

pub const ABSORB_SHIELD_SKILL: &'static AbsorbShieldSkill = &AbsorbShieldSkill;

impl AbsorbShieldSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        let now = sys_vars.time;
        let duration_seconds = dev_configs.skills.absorb_shield.duration_seconds;
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                finish_cast.caster_entity_id,
                finish_cast.target_entity.unwrap(),
                Box::new(AbsorbStatus::new(
                    finish_cast.caster_entity_id,
                    now,
                    duration_seconds,
                )),
            ));
    }
}

impl SkillDef for AbsorbShieldSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
    }

    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(
                finish_cast_data,
                AbsorbShieldSkill::do_finish_cast,
            ),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
