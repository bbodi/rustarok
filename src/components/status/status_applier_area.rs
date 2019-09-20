use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{SkillManifestation, WorldCollisions};
use crate::components::status::status::{ApplyStatusComponent, ApplyStatusComponentPayload};
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use nalgebra::Vector2;
use nphysics2d::object::DefaultColliderHandle;
use specs::{Entity, LazyUpdate};

pub struct StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    pub collider_handle: DefaultColliderHandle,
    pub extents: Vector2<u16>,
    pub pos: Vector2<f32>,
    pub name: String,
    pub status_creator: F,
    pub caster_entity_id: CharEntityId,
    pub next_action_at: ElapsedTime,
}

impl<F> StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    pub fn new(
        name: String,
        status_creator: F,
        skill_center: &Vector2<f32>,
        size: Vector2<u16>,
        caster_entity_id: CharEntityId,
        physics_world: &mut PhysicEngine,
    ) -> StatusApplierArea<F> {
        let collider_handle =
            physics_world.add_cuboid_skill_area(*skill_center, 0.0, v2!(size.x, size.y));
        StatusApplierArea {
            collider_handle,
            name,
            status_creator,
            pos: *skill_center,
            extents: size,
            caster_entity_id,
            next_action_at: ElapsedTime(0.0),
        }
    }
}

impl<F> SkillManifestation for StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    fn update(
        &mut self,
        _self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        _updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.next_action_at.has_already_passed(system_vars.time) {
            let self_collider_handle = self.collider_handle;
            let my_collisions = all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                let char_collider = physics_world
                    .colliders
                    .get(coll.character_coll_handle)
                    .unwrap();
                let char_entity_id = *char_collider
                    .user_data()
                    .map(|v| v.downcast_ref().unwrap())
                    .unwrap();
                system_vars.apply_statuses.push(ApplyStatusComponent {
                    source_entity_id: self.caster_entity_id,
                    target_entity_id: char_entity_id,
                    status: (self.status_creator)(system_vars.time),
                });
                self.next_action_at = system_vars.time.add_seconds(2.0);
            }
        }
    }

    fn render(
        &self,
        now: ElapsedTime,
        _tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x, self.extents.y)
            .add();

        render_commands
            .sprite_3d()
            .pos_2d(&self.pos)
            .y(3.0)
            .color(&if self.next_action_at.has_already_passed(now) {
                [0, 255, 0, 255]
            } else {
                [77, 77, 77, 255]
            })
            .add(&assets.texts.custom_texts[&self.name]);
    }
}
