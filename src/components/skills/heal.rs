use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};

use crate::components::{AttackComponent, AttackType, SoundEffectComponent};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;

pub struct HealSkill;

pub const HEAL_SKILL: &'static HealSkill = &HealSkill;

impl SkillDef for HealSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster: &CharacterStateComponent,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        let target_entity_id = target_entity.unwrap();
        let entity = entities.create();
        updater.insert(
            entity,
            SoundEffectComponent {
                target_entity_id,
                sound_id: system_vars.assets.sounds.heal,
                pos: caster.pos(),
                start_time: system_vars.time,
            },
        );
        system_vars.attacks.push(AttackComponent {
            src_entity: caster_entity_id,
            dst_entity: target_entity_id,
            typ: AttackType::Heal(system_vars.dev_configs.skills.heal.heal),
        });
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}
