use specs::LazyUpdate;

use crate::common::Vec2;
use crate::components::char::ActionPlayMode;
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{ApplyStatusComponent, MainStatuses};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
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
        caster_pos: Vec2,
        _skill_pos: Option<Vec2>,
        _char_to_skill_dir: &Vec2,
        _target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        {
            let entities = &ecs_world.entities();
            let updater = ecs_world.read_resource::<LazyUpdate>();
            let now = ecs_world.read_resource::<SystemVariables>().time;
            updater.insert(
                entities.create(),
                StrEffectComponent {
                    effect_id: StrEffectType::Concentration.into(),
                    pos: caster_pos,
                    start_time: now,
                    die_at: Some(now.add_seconds(0.7)),
                    play_mode: ActionPlayMode::PlayThenHold,
                },
            );
        }
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_main_status(
                caster_entity_id,
                caster_entity_id,
                MainStatuses::Mounted,
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::NoTarget
    }
}
