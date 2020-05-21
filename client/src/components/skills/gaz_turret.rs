use crate::components::char::{
    CharacterEntityBuilder, CharacterStateComponent, NpcComponent, TurretComponent,
    TurretControllerComponent,
};
use crate::components::controller::LocalPlayerController;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::runtime_assets::map::PhysicEngine;

use rustarok_common::common::Local;
use rustarok_common::components::char::{
    create_common_player_entity, CharOutlook, CharType, CollisionGroup, ControllerEntityId,
    EntityId, JobId, LocalCharStateComp, MonsterId, StaticCharDataComponent,
};
use rustarok_common::components::controller::ControllerComponent;
use rustarok_common::config::CommonConfigs;
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
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let caster_team = {
            let char_storage = ecs_world.read_storage::<StaticCharDataComponent>();
            char_storage
                .get(params.caster_entity_id.into())
                .map(|it| it.team)
        };
        if let Some(caster_team) = caster_team {
            let turret_id = {
                EntityId::from(
                    create_common_player_entity(
                        "Turret".to_owned(),
                        ecs_world,
                        CharType::Minion,
                        JobId::Turret,
                        params.skill_pos.unwrap(),
                        caster_team,
                        CharOutlook::Monster(MonsterId::Dimik),
                    )
                    .with(TurretComponent {
                        owner_entity_id: params.caster_entity_id,
                        preferred_target: None,
                    })
                    .build(),
                )
            };

            ControllerEntityId::new(
                ecs_world
                    .create_entity()
                    .with(ControllerComponent::new(turret_id))
                    .with(TurretControllerComponent)
                    .build(),
            );
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
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if params
            .target_entity
            .and_then(|it| {
                ecs_world
                    .read_storage::<TurretComponent>()
                    .get(it.into())
                    .map(|turret| turret.owner_entity_id == params.caster_entity_id)
            })
            .unwrap_or(false)
        {
            let target_entity = params.target_entity.unwrap();
            if let Some(turret) = ecs_world
                .write_storage::<LocalCharStateComp<Local>>()
                .get_mut(target_entity.into())
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
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        for turret in (&mut ecs_world.write_storage::<TurretComponent>()).join() {
            if turret.owner_entity_id == params.caster_entity_id {
                turret.preferred_target = params.target_entity;
            }
        }

        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}
