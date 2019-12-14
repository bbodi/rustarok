use nalgebra::Isometry2;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, StatusEnum, StatusEnumDiscriminants, StatusUpdateParams,
    StatusUpdateResult,
};
use crate::configs::DevConfig;
use crate::render::render_command::RenderCommandCollector;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{AssetResources, SystemVariables};
use nphysics2d::object::DefaultColliderHandle;
use rustarok_common::common::{v2, ElapsedTime, EngineTime, Vec2};
use rustarok_common::components::char::CharEntityId;
use specs::ReadStorage;

pub struct AssaPhasePrismSkill;

pub const ASSA_PHASE_PRISM_SKILL: &'static AssaPhasePrismSkill = &AssaPhasePrismSkill;

impl SkillDef for AssaPhasePrismSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_scapegoat.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let sys_vars = ecs_world.read_resource::<SystemVariables>();
        let configs = &ecs_world
            .read_resource::<DevConfig>()
            .skills
            .assa_phase_prism;
        let now = ecs_world.read_resource::<EngineTime>().now();
        Some(Box::new(AssaPhasePrismSkillManifestation::new(
            params.caster_entity_id,
            params.caster_pos,
            params.char_to_skill_dir,
            &mut ecs_world.write_resource::<PhysicEngine>(),
            now,
            configs.duration_seconds,
            configs.attributes.casting_range,
            configs.swap_duration_unit_per_second,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Directional
    }
}

struct AssaPhasePrismSkillManifestation {
    start_pos: Vec2,
    pos: Vec2,
    caster_id: CharEntityId,
    dir: Vec2,
    collider_handle: DefaultColliderHandle,
    started_at: ElapsedTime,
    ends_at: ElapsedTime,
    casting_range: f32,
    swap_duration_unit_per_second: f32,
}

impl AssaPhasePrismSkillManifestation {
    fn new(
        caster_id: CharEntityId,
        pos: Vec2,
        dir: Vec2,
        physics_world: &mut PhysicEngine,
        now: ElapsedTime,
        duration: f32,
        casting_range: f32,
        swap_duration_unit_per_second: f32,
    ) -> AssaPhasePrismSkillManifestation {
        let (collider_handle, _body_handle) =
            physics_world.add_cuboid_skill_area(pos, 0.0, v2(1.0, 1.0));
        AssaPhasePrismSkillManifestation {
            start_pos: pos,
            started_at: now,
            ends_at: now.add_seconds(duration),
            pos: pos,
            caster_id,
            dir,
            collider_handle,
            casting_range,
            swap_duration_unit_per_second,
        }
    }
}

impl SkillManifestation for AssaPhasePrismSkillManifestation {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let now = params.time().now();
        let self_collider_handle = self.collider_handle;
        if self.ends_at.has_already_passed(now) {
            params.physics_world.colliders.remove(self_collider_handle);
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        } else {
            // move forward
            let duration_percentage = now.percentage_between(self.started_at, self.ends_at);
            self.pos = self.start_pos + self.dir * (self.casting_range * duration_percentage);
            if let Some(collider) = params.physics_world.colliders.get_mut(self_collider_handle) {
                collider.set_position(Isometry2::translation(self.pos.x, self.pos.y));
            }
            // check collisions
            let my_collisions = params
                .all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
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
                    if target_char_entity_id == self.caster_id {
                        continue;
                    }
                    let ends_at = if let (Some(caster), Some(target)) = (
                        params.char_storage.get(self.caster_id.into()),
                        params.char_storage.get(target_char_entity_id.into()),
                    ) {
                        caster.set_noncollidable(params.physics_world);
                        target.set_noncollidable(params.physics_world);
                        let distance = (target.pos() - caster.pos()).magnitude();
                        // add status to the caster
                        let ends_at = now
                            .add_seconds((distance * self.swap_duration_unit_per_second).max(0.5));
                        Some((ends_at, caster.pos(), target.pos()))
                    } else {
                        None
                    };
                    if let Some((ends_at, caster_pos, target_pos)) = ends_at {
                        params.physics_world.colliders.remove(self_collider_handle);
                        params
                            .remove_component::<SkillManifestationComponent>(params.self_entity_id);

                        params.apply_status(ApplyStatusComponent::from_status(
                            self.caster_id,
                            self.caster_id,
                            StatusEnum::AssaPhasePrismStatus(AssaPhasePrismStatus {
                                caster_entity_id: self.caster_id,
                                started_at: now,
                                ends_at,
                                start_pos: caster_pos,
                                vector: target_pos - caster_pos,
                            }),
                        ));
                        // add status to the target
                        params.apply_status(ApplyStatusComponent::from_status(
                            self.caster_id,
                            target_char_entity_id,
                            StatusEnum::AssaPhasePrismStatus(AssaPhasePrismStatus {
                                caster_entity_id: self.caster_id,
                                started_at: now,
                                ends_at,
                                start_pos: target_pos,
                                vector: caster_pos - target_pos,
                            }),
                        ));
                    }
                    if let Some(caster) = params.char_storage.get_mut(self.caster_id.into()) {
                        caster
                            .statuses
                            .remove(StatusEnumDiscriminants::AssaBladeDashStatus);
                    }
                }
            }
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<CharacterStateComponent>,
        _now: ElapsedTime,
        _tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .sprite_3d()
            .pos_2d(&self.pos)
            .y(2.0)
            .add(assets.sprites.fire_particle);
    }
}

#[derive(Clone, Debug)]
pub struct AssaPhasePrismStatus {
    pub caster_entity_id: CharEntityId,
    pub started_at: ElapsedTime,
    pub ends_at: ElapsedTime,
    pub start_pos: Vec2,
    pub vector: Vec2,
}

impl AssaPhasePrismStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if let Some(body) = params
            .physics_world
            .bodies
            .rigid_body_mut(params.target_char.body_handle)
        {
            if self.ends_at.has_already_passed(params.time.now()) {
                params.target_char.set_collidable(params.physics_world);
                StatusUpdateResult::RemoveIt
            } else {
                let duration_percentage = params
                    .time
                    .now()
                    .percentage_between(self.started_at, self.ends_at);
                let pos = self.start_pos + self.vector * duration_percentage;
                body.set_position(Isometry2::translation(pos.x, pos.y));
                StatusUpdateResult::KeepIt
            }
        } else {
            StatusUpdateResult::RemoveIt
        }
    }
}
