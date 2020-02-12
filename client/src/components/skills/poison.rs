use specs::LazyUpdate;

use crate::components::char::ActionPlayMode;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{ApplyStatusComponent, PoisonStatus, StatusEnum};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::systems::SystemVariables;
use rustarok_common::common::EngineTime;
use rustarok_common::config::CommonConfigs;

pub struct PosionSkill;

pub const POISON_SKILL: &'static PosionSkill = &PosionSkill;

impl SkillDef for PosionSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\tf_poison.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        let entities = &ecs_world.entities();
        let updater = ecs_world.read_resource::<LazyUpdate>();
        let now = ecs_world.read_resource::<EngineTime>().now();
        updater.insert(
            entities.create(),
            StrEffectComponent {
                effect_id: StrEffectType::Poison.into(),
                pos: params.skill_pos.unwrap(),
                start_time: now,
                die_at: Some(now.add_seconds(0.7)),
                play_mode: ActionPlayMode::Repeat,
            },
        );
        let configs = &ecs_world.read_resource::<CommonConfigs>().skills.poison;
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_status(
                params.caster_entity_id,
                params.target_entity.unwrap(),
                StatusEnum::PoisonStatus(PoisonStatus {
                    poison_caster_entity_id: params.caster_entity_id,
                    started: now,
                    until: now.add_seconds(configs.duration_seconds),
                    next_damage_at: now,
                    damage: configs.damage,
                }),
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}
