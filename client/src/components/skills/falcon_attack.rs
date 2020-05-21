use nalgebra::{Isometry2, Vector2};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::{CharacterStateComponent, SpriteRenderDescriptorComponent};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::status::attrib_mod::WalkingSpeedModifierStatus;
use crate::components::status::status::{ApplyStatusComponent, StatusEnum};
use crate::render::render_command::RenderCommandCollector;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::{AssetResources, SystemVariables};
use nphysics2d::object::DefaultColliderHandle;
use rustarok_common::attack::{DamageDisplayType, HpModificationRequest, HpModificationType};
use rustarok_common::common::{v2, EngineTime, GameTime, Local, Percentage, Vec2};
use rustarok_common::components::char::{EntityId, StaticCharDataComponent, Team};
use rustarok_common::config::CommonConfigs;
use specs::prelude::*;
use std::collections::HashSet;

pub struct FalconAttackSkill;

pub const FALCON_ATTACK_SKILL: &'static FalconAttackSkill = &FalconAttackSkill;

impl SkillDef for FalconAttackSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_scapegoat.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let sys_vars = ecs_world.read_resource::<SystemVariables>();
        let configs = &ecs_world
            .read_resource::<CommonConfigs>()
            .skills
            .falcon_attack;

        let angle_in_rad = params.char_to_skill_dir.angle(&Vector2::y());
        let angle_in_rad = if params.char_to_skill_dir.x > 0.0 {
            angle_in_rad
        } else {
            -angle_in_rad
        };
        {
            for (falcon, sprite) in (
                &mut ecs_world.write_storage::<FalconComponent>(),
                &mut ecs_world.write_storage::<SpriteRenderDescriptorComponent>(),
            )
                .join()
            {
                if falcon.owner_entity_id != params.caster_entity_id {
                    continue;
                }
                let end_pos = params.caster_pos
                    + (params.char_to_skill_dir * configs.attributes.casting_range);
                let now = ecs_world.read_resource::<EngineTime>().now();
                falcon.set_state_to_attack(
                    now,
                    configs.duration_in_seconds,
                    params.caster_pos,
                    end_pos,
                    sprite,
                );
                let extents = v2(configs.attributes.width.unwrap(), 2.5);

                let (coll_handle, _body_handle) = ecs_world
                    .write_resource::<PhysicEngine>()
                    .add_cuboid_skill_area(params.caster_pos, angle_in_rad, extents);
                return Some(Box::new(FalconAttackSkillManifestation {
                    damaged_entities: HashSet::with_capacity(8),
                    extents,
                    start_pos: params.caster_pos,
                    path: end_pos - params.caster_pos,
                    rot_angle_in_rad: angle_in_rad,
                    created_at: now,
                    die_at: now.add_seconds(configs.duration_in_seconds),
                    falcon_collider_handle: coll_handle,
                    falcon_owner_id: params.caster_entity_id,
                    team: ecs_world
                        .read_storage::<StaticCharDataComponent>()
                        .get(params.caster_entity_id.into())
                        .unwrap()
                        .team,
                    damage: configs.damage,
                    slow: configs.slow,
                    slow_duration: configs.slow_duration,
                }));
            }
        }

        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Directional
    }
}

struct FalconAttackSkillManifestation {
    damaged_entities: HashSet<EntityId<Local>>,
    extents: Vec2,
    start_pos: Vec2,
    path: Vec2,
    rot_angle_in_rad: f32,
    created_at: GameTime<Local>,
    die_at: GameTime<Local>,
    falcon_collider_handle: DefaultColliderHandle,
    falcon_owner_id: EntityId<Local>,
    team: Team,
    damage: u32,
    slow: Percentage,
    slow_duration: f32,
}

impl SkillManifestation for FalconAttackSkillManifestation {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let falcon_collider_handle = self.falcon_collider_handle;
        if self.die_at.has_already_passed(params.time().now()) {
            params
                .physics_world
                .colliders
                .remove(falcon_collider_handle);
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        } else {
            let my_collisions = params
                .all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == falcon_collider_handle);
            for (_key, coll) in my_collisions {
                if let Some(char_collider) = params
                    .physics_world
                    .colliders
                    .get(coll.character_coll_handle)
                {
                    let target_char_entity_id: EntityId<Local> = *char_collider
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if let Some(target_char) = params
                        .static_char_data_storage
                        .get(target_char_entity_id.into())
                    {
                        if !self.team.can_attack(target_char.team)
                            || self.damaged_entities.contains(&target_char_entity_id)
                        {
                            continue;
                        }
                        params.add_hp_mod_request(HpModificationRequest {
                            src_entity: self.falcon_owner_id,
                            dst_entity: target_char_entity_id,
                            typ: HpModificationType::SpellDamage(
                                self.damage,
                                DamageDisplayType::Combo(2),
                            ),
                        });
                        params.apply_status(ApplyStatusComponent::from_status(
                            self.falcon_owner_id,
                            target_char_entity_id,
                            StatusEnum::WalkingSpeedModifierStatus(
                                WalkingSpeedModifierStatus::new(
                                    params.time().now(),
                                    self.slow,
                                    self.slow_duration,
                                ),
                            ),
                        ));
                        self.damaged_entities.insert(target_char_entity_id);
                    }
                }
            }
            let duration_percentage = params
                .time()
                .now()
                .percentage_between(self.created_at, self.die_at);
            let new_pos = self.start_pos + self.path * duration_percentage;
            let falcon_body = params
                .physics_world
                .colliders
                .get_mut(self.falcon_collider_handle)
                .unwrap();
            falcon_body.set_position(Isometry2::translation(new_pos.x, new_pos.y));
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        now: GameTime<Local>,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        let duration_percentage = now.percentage_between(self.created_at, self.die_at);
        let pos = self.start_pos + self.path * duration_percentage;
        render_commands
            .rectangle_3d()
            .pos_2d(&pos)
            .rotation_rad(self.rot_angle_in_rad)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x, self.extents.y)
            .add();
    }
}
