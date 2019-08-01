use crate::components::char::CharacterStateComponent;
use crate::components::skills::skill::SkillManifestation;
use crate::components::{AttackComponent, AttackType};
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::{AssetResources, Collision, SystemVariables};
use crate::{ElapsedTime, PhysicsWorld, SKILL_AREA_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP};
use nalgebra::Vector2;
use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use specs::{Entity, LazyUpdate};
use std::collections::HashMap;

pub struct HealApplierArea {
    pub collider_handle: ColliderHandle,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub name: &'static str,
    pub attack_type: AttackType,
    pub interval: f32,
    pub caster_entity_id: Entity,
    pub next_action_at: ElapsedTime,
}

impl HealApplierArea {
    pub fn new(
        name: &'static str,
        attack_type: AttackType,
        skill_center: &Vector2<f32>,
        size: Vector2<f32>,
        interval: f32,
        caster_entity_id: Entity,
        physics_world: &mut PhysicsWorld,
    ) -> HealApplierArea {
        let half_extents = v2!(size.x / 2.0, size.y / 2.0);

        let cuboid = ShapeHandle::new(ncollide2d::shape::Cuboid::new(half_extents));
        let collider_handle = ColliderDesc::new(cuboid)
            .translation(*skill_center)
            .collision_groups(
                CollisionGroups::new()
                    .with_membership(&[SKILL_AREA_COLLISION_GROUP])
                    .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP]),
            )
            .sensor(true)
            .build(physics_world)
            .handle();

        HealApplierArea {
            collider_handle,
            name,
            interval,
            attack_type,
            pos: *skill_center,
            half_extents,
            caster_entity_id,
            next_action_at: ElapsedTime(0.0),
        }
    }
}

impl SkillManifestation for HealApplierArea {
    fn update(
        &mut self,
        _self_entity_id: Entity,
        all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        _updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.next_action_at.has_passed(system_vars.time) {
            let self_collider_handle = self.collider_handle;
            let my_collisions = all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                let char_body_handle = physics_world
                    .collider(coll.character_coll_handle)
                    .unwrap()
                    .body();
                let char_body = physics_world.rigid_body_mut(char_body_handle).unwrap();
                let char_entity_id = *char_body
                    .user_data()
                    .map(|v| v.downcast_ref().unwrap())
                    .unwrap();
                system_vars.attacks.push(AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: char_entity_id,
                    typ: self.attack_type,
                });
                self.next_action_at = system_vars.time.add_seconds(self.interval);
            }
        }
    }

    fn render(
        &self,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        render_commands
            .prepare_for_3d()
            .pos_2d(&self.pos)
            .color(&[0.0, 1.0, 0.0, 1.0])
            .add_rectangle_command(&(self.half_extents * 2.0));

        render_commands
            .prepare_for_3d()
            .pos_2d(&self.pos)
            .y(3.0)
            .color(&if self.next_action_at.has_passed(now) {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [0.3, 0.3, 0.3, 1.0]
            })
            .add_sprite_command(&assets.texts.custom_texts[self.name], false);
    }
}
