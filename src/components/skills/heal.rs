use specs::{Entities, LazyUpdate};

use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};

use crate::components::{AttackComponent, AttackType, SoundEffectComponent};
use crate::configs::DevConfig;
use crate::systems::SystemVariables;

pub struct HealSkill;

pub const HEAL_SKILL: &'static HealSkill = &HealSkill;

impl HealSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        let target_entity_id = finish_cast.target_entity.unwrap();
        let entity = entities.create();
        updater.insert(
            entity,
            SoundEffectComponent {
                target_entity_id,
                sound_id: sys_vars.assets.sounds.heal,
                pos: finish_cast.caster_pos,
                start_time: sys_vars.time,
            },
        );
        sys_vars.attacks.push(AttackComponent {
            src_entity: finish_cast.caster_entity_id,
            dst_entity: target_entity_id,
            typ: AttackType::Heal(dev_configs.skills.heal.heal),
        });
    }
}

impl SkillDef for HealSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp"
    }

    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(finish_cast_data, HealSkill::do_finish_cast),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
