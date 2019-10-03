use specs::{Entities, LazyUpdate};

use crate::components::char::ActionPlayMode;
use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};
use crate::components::status::status::{ApplyStatusComponent, PoisonStatus};
use crate::components::StrEffectComponent;
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::systems::SystemVariables;

pub struct PoisonSkill;

pub const POISON_SKILL: &'static PoisonSkill = &PoisonSkill;

impl SkillDef for PoisonSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\tf_poison.bmp"
    }

    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(finish_cast_data, PoisonSkill::do_finish_cast),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}

impl PoisonSkill {
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
                effect_id: StrEffectType::Poison.into(),
                pos: finish_cast.skill_pos.unwrap(),
                start_time: now,
                die_at: Some(now.add_seconds(0.7)),
                play_mode: ActionPlayMode::Repeat,
            },
        );
        let configs = &dev_configs.skills.poison;
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                finish_cast.caster_entity_id,
                finish_cast.target_entity.unwrap(),
                Box::new(PoisonStatus {
                    poison_caster_entity_id: finish_cast.caster_entity_id,
                    started: now,
                    until: now.add_seconds(configs.duration_seconds),
                    next_damage_at: now,
                    damage: configs.damage,
                }),
            ));
    }
}
