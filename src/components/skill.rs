use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use ncollide2d::world::CollisionGroups;
use crate::{STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP, SKILL_AREA_COLLISION_GROUP, PhysicsWorld, ElapsedTime};
use nalgebra::{Vector2, Vector3, Point2, Matrix4, Rotation3, Point3};
use crate::systems::SystemVariables;
use std::sync::{Arc, Mutex};
use crate::video::draw_lines_inefficiently;
use crate::systems::render::{render_sprite, RenderDesktopClientSystem};
use crate::components::char::{MonsterSpriteComponent, SpriteRenderDescriptor, CharState, CastingSkillData};
use crate::components::controller::WorldCoords;
use crate::components::StrEffectComponent;

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
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Box<SkillManifestation>;

    fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables);

    fn render_target_selection(
        &self,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        system_vars: &SystemVariables,
    );
}

pub enum Skills {
    TestSkill,
}

impl SkillDescriptor for Skills {
    fn create_manifestation(
        &self,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Box<SkillManifestation> {
        match self {
            Skills::TestSkill => {
                let angle_in_rad = (mouse_pos.coords - char_pos).angle(&Vector2::y());
                let angle_in_rad = if mouse_pos.x > char_pos.x { angle_in_rad } else { -angle_in_rad };
                Box::new(
                    PushBackWallSkill::new(
                        physics_world,
                        &mouse_pos.coords,
                        angle_in_rad,
                        system_vars.time,
                        entities,
                        updater,
                    )
                )
            }
        }
    }

    fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables,
    ) {
        match self {
            Skills::TestSkill => {
                let half_extents = Vector2::new(1.5, 0.5);
                let half = half_extents;
                let bottom_left = Vector2::new(-half.x, -half.y);
                let top_left = Vector2::new(-half.x, half.y);
                let top_right = Vector2::new(half.x, half.y);
                let bottom_right = Vector2::new(half.x, -half.y);
                // rotate
                let skill_pos = casting_state.mouse_pos_when_casted;
                let mut rot_matrix = Matrix4::<f32>::identity();
                let angle = (skill_pos.coords - char_pos).angle(&Vector2::y());
                let angle = if skill_pos.x > char_pos.x { angle } else { -angle };
                let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), angle).to_homogeneous();
                let rot_matrix = rot_matrix * rotation;

                let bottom_left = rot_matrix.transform_point(&Point3::new(bottom_left.x, 1.0, bottom_left.y));
                let top_left = rot_matrix.transform_point(&Point3::new(top_left.x, 1.0, top_left.y));
                let top_right = rot_matrix.transform_point(&Point3::new(top_right.x, 1.0, top_right.y));
                let bottom_right = rot_matrix.transform_point(&Point3::new(bottom_right.x, 1.0, bottom_right.y));

                let skill_3d_pos = Vector3::<f32>::new(skill_pos.x, 0.0, skill_pos.y);
                draw_lines_inefficiently(
                    &system_vars.shaders.trimesh_shader,
                    &system_vars.matrices.projection,
                    &system_vars.matrices.view,
                    &[
                        skill_3d_pos + bottom_left.coords,
                        skill_3d_pos + top_left.coords,
                        skill_3d_pos + top_right.coords,
                        skill_3d_pos + bottom_right.coords,
                    ],
                    &[0.0, 1.0, 0.0, 1.0],
                );
            }
        }
    }

    fn render_target_selection(
        &self,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        system_vars: &SystemVariables,
    ) {
        let mouse_3d_pos = Vector3::<f32>::new(mouse_pos.x, 0.0, mouse_pos.y);
        match self {
            Skills::TestSkill => {
                let half_extents = Vector2::new(1.5, 0.5);
                let half = half_extents;
                let bottom_left = Vector2::new(-half.x, -half.y);
                let top_left = Vector2::new(-half.x, half.y);
                let top_right = Vector2::new(half.x, half.y);
                let bottom_right = Vector2::new(half.x, -half.y);
                // rotate
                let mut rot_matrix = Matrix4::<f32>::identity();
                let angle = (mouse_pos.coords - char_pos).angle(&Vector2::y());
                let angle = if mouse_pos.x > char_pos.x { angle } else { -angle };
                let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), angle).to_homogeneous();
                let rot_matrix = rot_matrix * rotation;

                let bottom_left = rot_matrix.transform_point(&Point3::new(bottom_left.x, 1.0, bottom_left.y));
                let top_left = rot_matrix.transform_point(&Point3::new(top_left.x, 1.0, top_left.y));
                let top_right = rot_matrix.transform_point(&Point3::new(top_right.x, 1.0, top_right.y));
                let bottom_right = rot_matrix.transform_point(&Point3::new(bottom_right.x, 1.0, bottom_right.y));
                draw_lines_inefficiently(
                    &system_vars.shaders.trimesh_shader,
                    &system_vars.matrices.projection,
                    &system_vars.matrices.view,
                    &[
                        mouse_3d_pos + bottom_left.coords,
                        mouse_3d_pos + top_left.coords,
                        mouse_3d_pos + top_right.coords,
                        mouse_3d_pos + bottom_right.coords,
                    ],
                    &[0.0, 1.0, 0.0, 1.0],
                );
            }
        }
    }
}

pub struct PushBackWallSkill {
    pub collider_handle: ColliderHandle,
    pub effect_ids: Vec<Entity>,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
}

pub fn rotate_vec(rad: f32, vec: &Vector2<f32>) -> Point2<f32> {
    let rot_matrix = Matrix4::<f32>::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad).to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&Point3::new(vec.x, 0.0, vec.y));
    return Point2::new(rotated.x, rotated.z);
}

pub fn rotate_vec2(rad: f32, vec: &Vector2<f32>) -> Vector2<f32> {
    let rot_matrix = Matrix4::<f32>::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad).to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&Point3::new(vec.x, 0.0, vec.y));
    return Vector2::new(rotated.x, rotated.z);
}

impl PushBackWallSkill {
    pub fn new(physics_world: &mut PhysicsWorld,
               skill_center: &Vector2<f32>,
               rot_angle_in_rad: f32,
               system_time: ElapsedTime,
               entities: &specs::Entities,
               updater: &mut specs::Write<LazyUpdate>) -> PushBackWallSkill {
        let half_extents = Vector2::new(1.5, 0.5);
        let effect_ids = [
            skill_center + rotate_vec2(rot_angle_in_rad, &Vector2::new(-1.0, 0.0)),
            *skill_center,
            skill_center + rotate_vec2(rot_angle_in_rad, &Vector2::new(1.0, 0.0)),
        ].iter().map(|effect_coords| {
            let effect_comp = StrEffectComponent {
                effect: "StrEffect::FireWall".to_owned(),
                pos: Point2::new(effect_coords.x, effect_coords.y),
                start_time: system_time,
                die_at: system_time.add_seconds(3.0),
                duration: ElapsedTime(3.0),
            };
            let effect_entity = entities.create();
            updater.insert(effect_entity, effect_comp);
            effect_entity
        }).collect();

        let cuboid = ShapeHandle::new(
            ncollide2d::shape::Cuboid::new(half_extents)
        );
        let collider_handle = ColliderDesc::new(cuboid)
            .density(1000.0)
            .translation(*skill_center)
            .rotation(rot_angle_in_rad.to_degrees())
//            .user_data(entity_id)
            .collision_groups(CollisionGroups::new()
                .with_membership(&[SKILL_AREA_COLLISION_GROUP])
                .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP])
            )
            .build(physics_world)
            .handle();

        PushBackWallSkill {
            effect_ids,
            collider_handle,
            rot_angle_in_rad,
            pos: *skill_center,
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
        if self.die_at.has_passed(system_vars.time) {
            physics_world.remove_colliders(&[self.collider_handle]);
            updater.remove::<SkillManifestationComponent>(entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables) {
        let half = self.half_extents;

        let rot_matrix = Matrix4::<f32>::identity();
        let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), self.rot_angle_in_rad).to_homogeneous();
        let rot_matrix = rot_matrix * rotation;

        let self_pos_3d = Vector3::new(self.pos.x, 0.0, self.pos.y);
        let bottom_left = self_pos_3d + rot_matrix.transform_point(&Point3::new(-half.x, 1.0, -half.y)).coords;
        let top_left = self_pos_3d + rot_matrix.transform_point(&Point3::new(-half.x, 1.0, half.y)).coords;
        let top_right = self_pos_3d + rot_matrix.transform_point(&Point3::new(half.x, 1.0, half.y)).coords;
        let bottom_right = self_pos_3d + rot_matrix.transform_point(&Point3::new(half.x, 1.0, -half.y)).coords;

        draw_lines_inefficiently(
            &system_vars.shaders.trimesh_shader,
            &system_vars.matrices.projection,
            &system_vars.matrices.view,
            &[
                bottom_left,
                top_left,
                top_right,
                bottom_right,
            ],
            &[0.0, 1.0, 0.0, 1.0],
        );
//        render_sprite(&system_vars,
//                      &SpriteRenderDescriptor {
//                          action_index: 0,
//                          animation_started: self.created_at,
//                          forced_duration: None,
//                          direction: 0,
//                      },
//                      &system_vars.sprites.effect_sprites.torch,
//                      &system_vars.matrices.view,
//                      None,
//                      &self.pos,
//                      [0, 0],
//                      true,
//                      1.0,
//                      &[1.0, 1.0, 1.0, 1.0]);
    }
}