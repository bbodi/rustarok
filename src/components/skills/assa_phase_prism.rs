use nalgebra::{Isometry2, Vector2};
use specs::{Entity, LazyUpdate};

use crate::common::ElapsedTime;
use crate::components::char::CharacterStateComponent;
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::assa_blade_dash::AssaBladeDashStatus;
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, WorldCollisions,
};
use crate::components::status::status::{
    ApplyStatusComponent, Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::configs::DevConfig;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use nphysics2d::object::DefaultColliderHandle;

pub struct AssaPhasePrismSkill;

pub const ASSA_PHASE_PRISM_SKILL: &'static AssaPhasePrismSkill = &AssaPhasePrismSkill;

impl SkillDef for AssaPhasePrismSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_scapegoat.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster_pos: WorldCoord,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let system_vars = ecs_world.read_resource::<SystemVariables>();
        let configs = &ecs_world
            .read_resource::<DevConfig>()
            .skills
            .assa_phase_prism;
        Some(Box::new(AssaPhasePrismSkillManifestation::new(
            caster_entity_id,
            caster_pos,
            *char_to_skill_dir,
            &mut ecs_world.write_resource::<PhysicEngine>(),
            system_vars.time,
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
    start_pos: WorldCoord,
    pos: WorldCoord,
    caster_id: CharEntityId,
    dir: Vector2<f32>,
    collider_handle: DefaultColliderHandle,
    started_at: ElapsedTime,
    ends_at: ElapsedTime,
    casting_range: f32,
    swap_duration_unit_per_second: f32,
}

impl AssaPhasePrismSkillManifestation {
    fn new(
        caster_id: CharEntityId,
        pos: WorldCoord,
        dir: Vector2<f32>,
        physics_world: &mut PhysicEngine,
        now: ElapsedTime,
        duration: f32,
        casting_range: f32,
        swap_duration_unit_per_second: f32,
    ) -> AssaPhasePrismSkillManifestation {
        let (collider_handle, _body_handle) =
            physics_world.add_cuboid_skill_area(pos, 0.0, v2!(1, 1));
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
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        let now = system_vars.time;
        let self_collider_handle = self.collider_handle;
        if self.ends_at.has_already_passed(now) {
            physics_world.colliders.remove(self_collider_handle);
            updater.remove::<SkillManifestationComponent>(self_entity_id);
        } else {
            // move forward
            let duration_percentage = system_vars
                .time
                .percentage_between(self.started_at, self.ends_at);
            self.pos = self.start_pos + self.dir * (self.casting_range * duration_percentage);
            if let Some(collider) = physics_world.colliders.get_mut(self_collider_handle) {
                collider.set_position(Isometry2::translation(self.pos.x, self.pos.y));
            }
            // check collisions
            let my_collisions = all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                if let Some(char_collider) = physics_world.colliders.get(coll.character_coll_handle)
                {
                    let target_char_entity_id: CharEntityId = *char_collider
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if target_char_entity_id == self.caster_id {
                        continue;
                    }
                    if let (Some(caster), Some(target)) = (
                        char_storage.get(self.caster_id.0),
                        char_storage.get(target_char_entity_id.0),
                    ) {
                        physics_world.colliders.remove(self_collider_handle);
                        updater.remove::<SkillManifestationComponent>(self_entity_id);
                        let distance = (target.pos() - caster.pos()).magnitude();
                        // add status to the caster
                        let ends_at = system_vars
                            .time
                            .add_seconds((distance * self.swap_duration_unit_per_second).max(0.5));
                        system_vars.apply_statuses.push(
                            ApplyStatusComponent::from_secondary_status(
                                self.caster_id,
                                self.caster_id,
                                Box::new(AssaPhasePrismStatus {
                                    caster_entity_id: self.caster_id,
                                    started_at: system_vars.time,
                                    ends_at,
                                    start_pos: caster.pos(),
                                    vector: target.pos() - caster.pos(),
                                }),
                            ),
                        );
                        caster.set_noncollidable(physics_world);

                        // add status to the target
                        system_vars.apply_statuses.push(
                            ApplyStatusComponent::from_secondary_status(
                                self.caster_id,
                                target_char_entity_id,
                                Box::new(AssaPhasePrismStatus {
                                    caster_entity_id: self.caster_id,
                                    started_at: system_vars.time,
                                    ends_at,
                                    start_pos: target.pos(),
                                    vector: caster.pos() - target.pos(),
                                }),
                            ),
                        );
                        target.set_noncollidable(physics_world);
                    }
                    if let Some(caster) = char_storage.get_mut(self.caster_id.0) {
                        caster
                            .statuses
                            .remove::<AssaBladeDashStatus, _>(|status| true);
                    }
                }
            }
        }
    }

    fn render(
        &self,
        _now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .sprite_3d()
            .pos_2d(&self.pos)
            .y(2.0)
            .add(&assets.sprites.fire_particle);
    }
}

#[derive(Clone)]
pub struct AssaPhasePrismStatus {
    pub caster_entity_id: CharEntityId,
    pub started_at: ElapsedTime,
    pub ends_at: ElapsedTime,
    pub start_pos: WorldCoord,
    pub vector: WorldCoord,
}

impl Status for AssaPhasePrismStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        false
    }

    fn can_target_cast(&self) -> bool {
        false
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [0, 255, 255, 255]
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        char_state: &mut CharacterStateComponent,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if let Some(body) = physics_world.bodies.rigid_body_mut(char_state.body_handle) {
            if self.ends_at.has_already_passed(system_vars.time) {
                char_state.set_collidable(physics_world);
                StatusUpdateResult::RemoveIt
            } else {
                let duration_percentage = system_vars
                    .time
                    .percentage_between(self.started_at, self.ends_at);
                let pos = self.start_pos + self.vector * duration_percentage;
                body.set_position(Isometry2::translation(pos.x, pos.y));
                StatusUpdateResult::KeepIt
            }
        } else {
            StatusUpdateResult::RemoveIt
        }
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::Replace
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
