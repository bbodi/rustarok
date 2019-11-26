use specs::LazyUpdate;

use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, RemoveStatusComponent, RemoveStatusComponentPayload, StatusEnum,
    StatusEnumDiscriminants,
};
use crate::components::StrEffectComponent;
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::systems::atk_calc::AttackSystem;
use crate::systems::SystemVariables;

pub struct MountingSkill;

pub const MOUNTING_SKILL: &'static MountingSkill = &MountingSkill;

impl SkillDef for MountingSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\su_pickypeck.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
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
                    pos: params.caster_pos,
                    start_time: now,
                    die_at: Some(now.add_seconds(0.7)),
                    play_mode: ActionPlayMode::PlayThenHold,
                },
            );
        }
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        if let Some(target_char) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(params.caster_entity_id.0)
        {
            if target_char.statuses.is_mounted() {
                sys_vars.remove_statuses.push(RemoveStatusComponent {
                    source_entity_id: params.caster_entity_id,
                    target_entity_id: params.caster_entity_id,
                    status: RemoveStatusComponentPayload::RemovingStatusDiscr(
                        StatusEnumDiscriminants::MountedStatus,
                    ),
                })
            } else {
                let mounted_speedup = AttackSystem::calc_mounted_speedup(
                    target_char,
                    &ecs_world.read_resource::<DevConfig>(),
                );
                sys_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_status(
                        params.caster_entity_id,
                        params.caster_entity_id,
                        StatusEnum::MountedStatus {
                            speedup: mounted_speedup,
                        },
                    ));
            }
        }
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::NoTarget
    }
}
