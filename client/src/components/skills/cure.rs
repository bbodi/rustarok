use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::RemoveStatusComponent;
use crate::systems::SystemVariables;
use rustarok_common::components::char::StatusNature;
use specs::world::WorldExt;
pub struct CureSkill;

pub const CURE_SKILL: &'static CureSkill = &CureSkill;

impl SkillDef for CureSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\so_el_cure.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        sys_vars
            .remove_statuses
            .push(RemoveStatusComponent::by_status_nature(
                params.caster_entity_id,
                params.target_entity.unwrap(),
                StatusNature::Harmful,
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
