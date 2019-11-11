use crate::common::Vec2;
use crate::components::char::{
    CharOutlook, CharacterEntityBuilder, CharacterStateComponent, NpcComponent, TurretComponent,
    TurretControllerComponent,
};
use crate::components::controller::{CharEntityId, ControllerComponent, ControllerEntityId};
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType};
use crate::configs::DevConfig;
use crate::consts::{JobId, MonsterId};
use crate::runtime_assets::map::{CollisionGroup, PhysicEngine};
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
        _caster_pos: Vec2,
        skill_pos: Option<Vec2>,
        _char_to_skill_dir: &Vec2,
        _target_entity: Option<CharEntityId>,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let entities = &ecs_world.entities();
            let updater = &ecs_world.read_resource::<LazyUpdate>();
            let char_entity_id = CharEntityId(entities.create());
            updater.insert(char_entity_id.0, NpcComponent);
            CharacterEntityBuilder::new(char_entity_id, "turret")
                .insert_sprite_render_descr_component(updater)
                .insert_turret_component(caster_entity_id, updater)
                .physics(
                    skill_pos.unwrap(),
                    &mut ecs_world.write_resource::<PhysicEngine>(),
                    |builder| builder.collision_group(CollisionGroup::Turret).circle(1.0),
                )
                .char_state(updater, &ecs_world.read_resource::<DevConfig>(), |ch| {
                    ch.outlook(CharOutlook::Monster(MonsterId::Dimik))
                        .job_id(JobId::Turret)
                        .team(caster.team)
                });

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
        _caster_pos: Vec2,
        _skill_pos: Option<Vec2>,
        _char_to_skill_dir: &Vec2,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut World,
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
        _caster_pos: Vec2,
        _skill_pos: Option<Vec2>,
        _char_to_skill_dir: &Vec2,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut World,
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
