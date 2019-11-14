use specs::LazyUpdate;

use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};

use crate::components::{HpModificationRequest, HpModificationType, SoundEffectComponent};
use crate::configs::DevConfig;
use crate::systems::SystemVariables;

pub struct HealSkill;

pub const HEAL_SKILL: &'static HealSkill = &HealSkill;

impl SkillDef for HealSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let target_entity_id = params.target_entity.unwrap();
        let entities = &ecs_world.entities();
        let updater = ecs_world.read_resource::<LazyUpdate>();
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        let entity = entities.create();
        updater.insert(
            entity,
            SoundEffectComponent {
                target_entity_id,
                sound_id: sys_vars.assets.sounds.heal,
                pos: params.caster_pos,
                start_time: sys_vars.time,
            },
        );
        sys_vars.hp_mod_requests.push(HpModificationRequest {
            src_entity: params.caster_entity_id,
            dst_entity: target_entity_id,
            typ: HpModificationType::Heal(ecs_world.read_resource::<DevConfig>().skills.heal.heal),
        });
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
