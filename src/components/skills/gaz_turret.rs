use crate::components::char::{
    attach_char_components, CharOutlook, CharType, CharacterStateComponent, NpcComponent,
    TurretComponent, TurretControllerComponent,
};
use crate::components::controller::{
    CharEntityId, ControllerComponent, ControllerEntityId, WorldCoords,
};
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::configs::DevConfig;
use crate::consts::{JobId, MonsterId};
use crate::runtime_assets::map::{CollisionGroup, PhysicEngine};
use crate::systems::SystemVariables;
use nalgebra::{Point2, Vector2};
use specs::prelude::*;
use specs::LazyUpdate;

pub struct GazTurretSkill;

pub const GAZ_TURRET_SKILL: &'static GazTurretSkill = &GazTurretSkill;

impl SkillDef for GazTurretSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_cartcannon.bmp"
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
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let entities = &ecs_world.entities();
            let updater = ecs_world.read_resource::<LazyUpdate>();
            let system_vars = ecs_world.read_resource::<SystemVariables>();
            let char_entity_id = CharEntityId(entities.create());
            updater.insert(char_entity_id.0, NpcComponent);
            attach_char_components(
                "turret".to_owned(),
                char_entity_id,
                &updater,
                &mut ecs_world.write_resource::<PhysicEngine>(),
                Point2::new(skill_pos.unwrap().x, skill_pos.unwrap().y), // TODO: why is it point2?
                CharOutlook::Monster(MonsterId::Dimik),
                JobId::Turret,
                1,
                caster.team,
                CharType::Minion,
                CollisionGroup::NonPlayer,
                &[
                    CollisionGroup::NonPlayer,
                    CollisionGroup::Player,
                    CollisionGroup::StaticModel,
                    CollisionGroup::NonCollidablePlayer,
                ],
                &ecs_world.read_resource::<DevConfig>(),
            );
            updater.insert(
                char_entity_id.0,
                TurretComponent {
                    owner_entity_id: caster_entity_id,
                    preferred_target: None,
                },
            );

            let controller_id = ControllerEntityId(entities.create());
            updater.insert(controller_id.0, ControllerComponent::new(char_entity_id));
            updater.insert(controller_id.0, TurretControllerComponent);
        }
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }
}

pub struct GazDestroyTurretSkill;
pub const GAZ_DESTROY_TURRET_SKILL: &'static GazDestroyTurretSkill = &GazDestroyTurretSkill;

impl SkillDef for GazDestroyTurretSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_remodeling_cart.bmp"
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
        if target_entity
            .and_then(|it| {
                ecs_world
                    .read_storage::<TurretComponent>()
                    .get(it.0)
                    .map(|turret| turret.owner_entity_id == caster_entity_id)
            })
            .unwrap_or(false)
        {
            let target_entity = target_entity.unwrap();
            if let Some(turret) = ecs_world
                .write_storage::<CharacterStateComponent>()
                .get_mut(target_entity.0)
            {
                turret.hp = 0;
            }
        }

        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyButNoSelf
    }
}

pub struct GazTurretTargetSkill;
pub const GAZ_TURRET_TARGET_SKILL: &'static GazTurretTargetSkill = &GazTurretTargetSkill;

impl SkillDef for GazTurretTargetSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gs_bullseye.bmp"
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
        for turret in (&mut ecs_world.write_storage::<TurretComponent>()).join() {
            if turret.owner_entity_id == caster_entity_id {
                turret.preferred_target = target_entity;
            }
        }

        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}
