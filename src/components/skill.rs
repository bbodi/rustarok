use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use ncollide2d::world::CollisionGroups;
use crate::{STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP, SKILL_AREA_COLLISION_GROUP, PhysicsWorld};
use nalgebra::Vector2;
use crate::systems::SystemVariables;

trait SkillManifestation {
    fn update(&mut self, system_vars: SystemVariables);
}

#[storage(HashMapStorage)]
#[derive(Component)]
pub struct PushBackWallSkillComponent {
    pub collider_handle: ColliderHandle,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
}

impl PushBackWallSkillComponent {
    pub fn new(physics_world: &mut PhysicsWorld, pos: Vector2<f32>, entity_id: Entity) -> PushBackWallSkillComponent {
        let half_extents = Vector2::new(10.0, 20.0);

        let cuboid = ShapeHandle::new(
            ncollide2d::shape::Cuboid::new(half_extents)
        );
        let collider_handle = ColliderDesc::new(cuboid)
            .sensor(true)
            .density(10.0)
            .translation(pos)
            .user_data(entity_id)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[SKILL_AREA_COLLISION_GROUP])
                .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP])
            )
            .build(physics_world)
            .handle();
        PushBackWallSkillComponent {
            collider_handle,
            pos,
            half_extents,
        }
    }
}

impl SkillManifestation for PushBackWallSkillComponent {
    fn update(&mut self, system_vars: SystemVariables) {

    }
}