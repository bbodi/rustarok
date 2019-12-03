use crate::common::{v2, Vec2, Vec2u};
use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{SkillManifestation, SkillManifestationUpdateParam};
use crate::components::{HpModificationRequest, HpModificationType};
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, CharEntityId};
use crate::{ElapsedTime, PhysicEngine};
use nalgebra::Vector2;
use nphysics2d::object::DefaultColliderHandle;
use specs::ReadStorage;

pub struct HealApplierArea {
    pub collider_handle: DefaultColliderHandle,
    pub extents: Vector2<u16>,
    pub pos: Vec2,
    pub name: &'static str,
    pub attack_type: HpModificationType,
    pub interval: f32,
    pub caster_entity_id: CharEntityId,
    pub next_action_at: ElapsedTime,
}

impl HealApplierArea {
    pub fn new(
        name: &'static str,
        attack_type: HpModificationType,
        skill_center: &Vec2,
        size: Vec2u,
        interval: f32,
        caster_entity_id: CharEntityId,
        physics_world: &mut PhysicEngine,
    ) -> HealApplierArea {
        let (collider_handle, _body_handle) = physics_world.add_cuboid_skill_area(
            *skill_center,
            0.0,
            v2(size.x as f32, size.y as f32),
        );

        HealApplierArea {
            collider_handle,
            name,
            interval,
            attack_type,
            pos: *skill_center,
            extents: size,
            caster_entity_id,
            next_action_at: ElapsedTime(0.0),
        }
    }
}

impl SkillManifestation for HealApplierArea {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        if self.next_action_at.has_already_passed(params.now()) {
            let self_collider_handle = self.collider_handle;
            let my_collisions = params
                .all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                let char_collider = params
                    .physics_world
                    .colliders
                    .get(coll.character_coll_handle)
                    .unwrap();
                let char_entity_id = *char_collider
                    .user_data()
                    .map(|v| v.downcast_ref().unwrap())
                    .unwrap();
                params.add_hp_mod_request(HpModificationRequest {
                    src_entity: self.caster_entity_id,
                    dst_entity: char_entity_id,
                    typ: self.attack_type,
                });
                self.next_action_at = params.now().add_seconds(self.interval);
            }
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<CharacterStateComponent>,
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
            .size(self.extents.x as f32, self.extents.y as f32)
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
            .add(assets.texts.custom_texts[self.name]);
    }
}
