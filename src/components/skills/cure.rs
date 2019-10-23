use nalgebra::Vector2;

use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{RemoveStatusComponent, StatusNature};
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
        caster_pos: WorldCoord,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        sys_vars
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
