use nalgebra::{Isometry2, Vector2};

use crate::common::{v2, ElapsedTime, Vec2};
use crate::components::char::Percentage;
use crate::components::char::{CharacterStateComponent, SpriteRenderDescriptorComponent, Team};
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::status::attrib_mod::WalkingSpeedModifierStatus;
use crate::components::status::status::ApplyStatusComponent;
use crate::components::{DamageDisplayType, HpModificationRequest, HpModificationType};
use crate::configs::DevConfig;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use nphysics2d::object::DefaultColliderHandle;
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
        let configs = &ecs_world.read_resource::<DevConfig>().skills.falcon_attack;

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
                falcon.set_state_to_attack(
                    sys_vars.time,
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
                    created_at: sys_vars.time,
                    die_at: sys_vars.time.add_seconds(configs.duration_in_seconds),
                    falcon_collider_handle: coll_handle,
                    falcon_owner_id: params.caster_entity_id,
                    team: ecs_world
                        .read_storage::<CharacterStateComponent>()
                        .get(params.caster_entity_id.0)
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
    damaged_entities: HashSet<CharEntityId>,
    extents: Vec2,
    start_pos: Vec2,
    path: Vec2,
    rot_angle_in_rad: f32,
    created_at: ElapsedTime,
    die_at: ElapsedTime,
    falcon_collider_handle: DefaultColliderHandle,
    falcon_owner_id: CharEntityId,
    team: Team,
    damage: u32,
    slow: Percentage,
    slow_duration: f32,
}

impl SkillManifestation for FalconAttackSkillManifestation {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let falcon_collider_handle = self.falcon_collider_handle;
        if self.die_at.has_already_passed(params.now()) {
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
                    let target_char_entity_id: CharEntityId = *char_collider
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if let Some(target_char) = params.char_storage.get(target_char_entity_id.0) {
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
                        params.apply_status(ApplyStatusComponent::from_secondary_status(
                            self.falcon_owner_id,
                            target_char_entity_id,
                            Box::new(WalkingSpeedModifierStatus::new(
                                params.now(),
                                self.slow,
                                self.slow_duration,
                            )),
                        ));
                        self.damaged_entities.insert(target_char_entity_id);
                    }
                }
            }
            let duration_percentage = params
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
        now: ElapsedTime,
        _tick: u64,
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
