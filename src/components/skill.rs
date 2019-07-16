use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use ncollide2d::world::CollisionGroups;
use crate::{STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP, SKILL_AREA_COLLISION_GROUP, PhysicsWorld, ElapsedTime};
use nalgebra::{Vector2, Vector3, Point2};
use crate::systems::SystemVariables;
use std::sync::{Arc, Mutex};
use crate::video::draw_lines_inefficiently;
use crate::systems::render::render_sprite;
use crate::components::char::{MonsterSpriteComponent, SpriteRenderDescriptor};
use crate::components::controller::WorldCoords;

pub trait SkillManifestation {
    fn update(
        &mut self,
        entity_id: Entity,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    );

    fn render(&self, system_vars: &SystemVariables);
}

#[storage(HashMapStorage)]
#[derive(Component)]
pub struct SkillManifestationComponent {
    pub entity_id: Entity,
    pub skill: Arc<Mutex<Box<SkillManifestation>>>,
}

impl SkillManifestationComponent {
    pub fn new(
        entity_id: Entity,
        skill: Box<SkillManifestation>,
    ) -> SkillManifestationComponent {
        SkillManifestationComponent {
            entity_id,
            skill: Arc::new(Mutex::new(skill)),
        }
    }

    pub fn update(&mut self,
                  entity_id: Entity,
                  system_vars: &SystemVariables,
                  entities: &specs::Entities,
                  physics_world: &mut PhysicsWorld,
                  updater: &mut specs::Write<LazyUpdate>) {
        let mut skill = self.skill.lock().unwrap();
        skill.update(entity_id, system_vars, entities, physics_world, updater);
    }

    pub fn render(&self, system_vars: &SystemVariables) {
        let mut skill = self.skill.lock().unwrap();
        skill.render(system_vars);
    }
}

unsafe impl Sync for SkillManifestationComponent {}

unsafe impl Send for SkillManifestationComponent {}


pub trait SkillDescriptor {
    fn create_manifestation(
        &self,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
    ) -> Box<SkillManifestation>;

    fn render_target_selection(
        &self,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        system_vars: &SystemVariables,
    );
}

pub struct TestSkill {
    //  TODO: it is needed for create manifestation, not here
    pub pos: WorldCoords,
}

impl SkillDescriptor for TestSkill {
    fn create_manifestation(
        &self,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
    ) -> Box<SkillManifestation> {
        Box::new(
            PushBackWallSkill::new(
                physics_world,
                self.pos.coords,
                &system_vars.time,
            )
        )
    }

    fn render_target_selection(
        &self,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        system_vars: &SystemVariables
    ) {
        let half_extents = Vector2::new(1.0, 2.0);
        let half = half_extents;
        let bottom_left = mouse_pos.coords - Vector2::new(-half.x, -half.y);
        let top_left = mouse_pos.coords - Vector2::new(-half.x, half.y);
        let top_right = mouse_pos.coords - Vector2::new(half.x, half.y);
        let bottom_right = mouse_pos.coords - Vector2::new(half.x, -half.y);
        draw_lines_inefficiently(
            &system_vars.shaders.trimesh_shader,
            &system_vars.matrices.projection,
            &system_vars.matrices.view,
            &[
                Vector3::new(bottom_left.x, 1.0, bottom_left.y),
                Vector3::new(top_left.x, 1.0, top_left.y),
                Vector3::new(top_right.x, 1.0, top_right.y),
                Vector3::new(bottom_right.x, 1.0, bottom_right.y),
            ],
            &[0.0, 1.0, 0.0, 1.0],
        );
    }
}

pub struct PushBackWallSkill {
    pub collider_handle: ColliderHandle,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
}

impl PushBackWallSkill {
    pub fn new(physics_world: &mut PhysicsWorld,
               pos: Vector2<f32>,
               system_time: &ElapsedTime) -> PushBackWallSkill {
        let half_extents = Vector2::new(1.0, 2.0);

        let cuboid = ShapeHandle::new(
            ncollide2d::shape::Cuboid::new(half_extents)
        );
        let collider_handle = ColliderDesc::new(cuboid)
            .density(1000.0)
            .translation(pos)
//            .user_data(entity_id)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[SKILL_AREA_COLLISION_GROUP])
                .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP])
            )
            .build(physics_world)
            .handle();
        PushBackWallSkill {
            collider_handle,
            pos,
            half_extents,
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(2.0),
        }
    }
}

impl SkillManifestation for PushBackWallSkill {
    fn update(&mut self,
              entity_id: Entity,
              system_vars: &SystemVariables,
              entities: &specs::Entities,
              physics_world: &mut PhysicsWorld,
              updater: &mut specs::Write<LazyUpdate>) {
        if self.die_at.has_passed(&system_vars.time) {
            physics_world.remove_colliders(&[self.collider_handle]);
            updater.remove::<SkillManifestationComponent>(entity_id);
        }
    }

    fn render(&self, system_vars: &SystemVariables) {
        let half = self.half_extents;
        let bottom_left = self.pos - Vector2::new(-half.x, -half.y);
        let top_left = self.pos - Vector2::new(-half.x, half.y);
        let top_right = self.pos - Vector2::new(half.x, half.y);
        let bottom_right = self.pos - Vector2::new(half.x, -half.y);
        draw_lines_inefficiently(
            &system_vars.shaders.trimesh_shader,
            &system_vars.matrices.projection,
            &system_vars.matrices.view,
            &[
                Vector3::new(bottom_left.x, 1.0, bottom_left.y),
                Vector3::new(top_left.x, 1.0, top_left.y),
                Vector3::new(top_right.x, 1.0, top_right.y),
                Vector3::new(bottom_right.x, 1.0, bottom_right.y),
            ],
            &[0.0, 1.0, 0.0, 1.0],
        );
        render_sprite(&system_vars,
                      &SpriteRenderDescriptor {
                          action_index: 0,
                          animation_started: self.created_at,
                          forced_duration: None,
                          direction: 0,
                      },
                      &system_vars.sprites.effect_sprites.torch,
                      &system_vars.matrices.view,
                      None,
                      &self.pos,
                      [0, 0],
                      true,
                      1.0,
                      &[1.0, 1.0, 1.0, 1.0]);
    }
}