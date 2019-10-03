use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};
use crate::components::status::status::{RemoveStatusComponent, StatusNature};
use crate::configs::DevConfig;
use crate::systems::SystemVariables;
use specs::{Entities, LazyUpdate};

pub struct CureSkill;

pub const CURE_SKILL: &'static CureSkill = &CureSkill;

impl CureSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        sys_vars
            .remove_statuses
            .push(RemoveStatusComponent::by_status_nature(
                finish_cast.caster_entity_id,
                finish_cast.target_entity.unwrap(),
                StatusNature::Harmful,
            ));
    }
}

impl SkillDef for CureSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\so_el_cure.bmp"
    }
    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(finish_cast_data, CureSkill::do_finish_cast),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
