use nalgebra::Vector2;

use crate::components::char::CharacterStateComponent;
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};

use crate::components::{AttackComponent, AttackType, DamageDisplayType};
use crate::systems::SystemVariables;

pub struct BasicAttackSkill;

pub const BASIC_ATTACK_SKILL: &'static BasicAttackSkill = &BasicAttackSkill;

impl SkillDef for BasicAttackSkill {
    fn get_icon_path(&self) -> &'static str {
        ""
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster_pos: WorldCoords,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let target_entity_id = target_entity.unwrap();
        let mut system_vars = ecs_world.write_resource::<SystemVariables>();
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            system_vars.attacks.push(AttackComponent {
                src_entity: caster_entity_id,
                dst_entity: target_entity_id,
                typ: AttackType::Basic(
                    caster.calculated_attribs().attack_damage as u32,
                    DamageDisplayType::SingleNumber,
                ),
            });
        }
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}
