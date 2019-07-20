use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, ColliderHandle, RigidBody};
use ncollide2d::world::CollisionGroups;
use crate::{STATIC_MODELS_COLLISION_GROUP, LIVING_COLLISION_GROUP, SKILL_AREA_COLLISION_GROUP, PhysicsWorld, ElapsedTime};
use nalgebra::{Vector2, Vector3, Point2, Matrix4, Rotation3, Point3, Isometry2};
use crate::systems::{SystemVariables, Collision};
use std::sync::{Arc, Mutex};
use crate::video::draw_lines_inefficiently;
use crate::systems::render::{render_sprite, RenderDesktopClientSystem};
use crate::components::char::{MonsterSpriteComponent, SpriteRenderDescriptor, CharState, CastingSkillData, CharacterStateComponent};
use crate::components::controller::WorldCoords;
use crate::components::{StrEffectComponent, AttackComponent, AttackType};
use ncollide2d::query::PointQuery;

pub trait SkillManifestation {
    fn update(
        &mut self,
        entity_id: Entity,
        all_collisions_in_world: &Vec<Collision>,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
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

    pub fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &Vec<Collision>,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        let mut skill = self.skill.lock().unwrap();
        skill.update(self_entity_id,
                     all_collisions_in_world,
                     system_vars,
                     entities,
                     char_storage,
                     physics_world,
                     updater,
        );
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
        caster_entity_id: Entity,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Box<SkillManifestation>;

    fn get_casting_time(&self) -> ElapsedTime;

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

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Skills {
    TestSkill,
    BrutalTestSkill,
}

impl Skills {

    pub fn render_casting_box(
        casting_area_size: &Vector2<f32>,
        mouse_pos: &Point2<f32>,
        char_pos: &Vector2<f32>,
        system_vars: &SystemVariables,
    ) {
        let half = casting_area_size / 2.0;
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

        let skill_3d_pos = Vector3::<f32>::new(mouse_pos.x, 0.0, mouse_pos.y);
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

    pub fn render_casting_box2(
        pos: &Vector2<f32>,
        half: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_vars: &SystemVariables,
    ) {
        let rot_matrix = Matrix4::<f32>::identity();
        let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rot_angle_in_rad).to_homogeneous();
        let rot_matrix = rot_matrix * rotation;

        let self_pos_3d = Vector3::new(pos.x, 0.0, pos.y);
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
    }
}

impl SkillDescriptor for Skills {
    fn create_manifestation(
        &self,
        caster_entity_id: Entity,
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
                        caster_entity_id,
                        physics_world,
                        &mouse_pos.coords,
                        angle_in_rad,
                        system_vars.time,
                        entities,
                        updater,
                    )
                )
            }
            Skills::BrutalTestSkill => {
                let angle_in_rad = (mouse_pos.coords - char_pos).angle(&Vector2::y());
                let angle_in_rad = if mouse_pos.x > char_pos.x { angle_in_rad } else { -angle_in_rad };
                Box::new(
                    BrutalSkillManifest::new(
                        caster_entity_id,
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

    fn get_casting_time(&self) -> ElapsedTime {
        match self {
            Skills::TestSkill => ElapsedTime(0.3),
            Skills::BrutalTestSkill => ElapsedTime(1.0),
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
                Skills::render_casting_box(
                    &Vector2::new(3.0, 1.0),
                    &casting_state.mouse_pos_when_casted,
                    char_pos,
                    system_vars,
                );
            }
            Skills::BrutalTestSkill => {
                Skills::render_casting_box(
                    &Vector2::new(10.0, 10.0),
                    &casting_state.mouse_pos_when_casted,
                    char_pos,
                    system_vars,
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
        match self {
            Skills::TestSkill => {
                Skills::render_casting_box(
                    &Vector2::new(3.0, 1.0),
                    mouse_pos,
                    char_pos,
                    system_vars,
                );
            }
            Skills::BrutalTestSkill => {
                Skills::render_casting_box(
                    &Vector2::new(10.0, 10.0),
                    mouse_pos,
                    char_pos,
                    system_vars,
                );
            }
        }
    }
}

pub struct PushBackWallSkill {
    pub caster_entity_id: Entity,
    pub collider_handle: ColliderHandle,
    pub effect_ids: Vec<Entity>,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
}


pub struct BrutalSkillManifest {
    pub caster_entity_id: Entity,
    pub effect_ids: Vec<Entity>,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
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
    pub fn new(
        caster_entity_id: Entity,
        physics_world: &mut PhysicsWorld,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> PushBackWallSkill {
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
            caster_entity_id,
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
              self_entity_id: Entity,
              all_collisions_in_world: &Vec<Collision>,
              system_vars: &SystemVariables,
              entities: &specs::Entities,
              char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
              physics_world: &mut PhysicsWorld,
              updater: &mut specs::Write<LazyUpdate>) {
        if self.die_at.has_passed(system_vars.time) {
            physics_world.remove_colliders(&[self.collider_handle]);
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            let my_collisions = all_collisions_in_world.iter().filter(|coll| coll.other_coll_handle == self.collider_handle);
            for coll in my_collisions {
                let char_body_handle = physics_world.collider(coll.character_coll_handle).unwrap().body();
                let char_body = physics_world.rigid_body_mut(char_body_handle).unwrap();
                let char_entity_id = *char_body.user_data().map(|v| v.downcast_ref().unwrap()).unwrap();
                let char_state = char_storage.get_mut(char_entity_id).unwrap();
                char_state.cannot_control_until.run_at_least_until_seconds(system_vars.time, 1.0);
                char_state.set_state(CharState::ReceivingDamage, char_state.dir());
                // TODO: the phys system already stops the char before this collision would have been processed,
                // so remove the rigid body from the skill and push the player without it
                char_body.set_linear_velocity(char_body.velocity().linear * -1.0);

                let damage_entity = entities.create();
                updater.insert(damage_entity, AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: char_entity_id,
                    typ: AttackType::Skill(Skills::TestSkill),
                });
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables) {
        Skills::render_casting_box2(
            &self.pos,
            &self.half_extents,
            self.rot_angle_in_rad,
            &system_vars,
        );
    }
}

impl BrutalSkillManifest {
    pub fn new(
        caster_entity_id: Entity,
        physics_world: &mut PhysicsWorld,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> BrutalSkillManifest {
        let half_extents = Vector2::new(5.0, 5.0);

//        let effect_ids = (0..11*11).map(|i| {
//            let x = (-5.0 + (i%10) as f32);
//            let y = (-5.0 + (i/10) as f32);
//            skill_center + rotate_vec2(rot_angle_in_rad, &Vector2::new(x, y))
//        }).map(|effect_coords| {
//            let effect_comp = StrEffectComponent {
//                effect: "StrEffect::FireWall".to_owned(),
//                pos: Point2::new(effect_coords.x, effect_coords.y),
//                start_time: system_time,
//                die_at: system_time.add_seconds(3.0),
//                duration: ElapsedTime(3.0),
//            };
//            let effect_entity = entities.create();
//            updater.insert(effect_entity, effect_comp);
//            effect_entity
//        }).collect();
        let effect_comp = StrEffectComponent {
            effect: "StrEffect::LordOfVermilion".to_owned(),
            pos: Point2::new(skill_center.x, skill_center.y),
            start_time: system_time,
            die_at: system_time.add_seconds(3.0),
            duration: ElapsedTime(3.0),
        };
        let effect_entity = entities.create();
        updater.insert(effect_entity, effect_comp);
        let effect_ids = vec![effect_entity];
        BrutalSkillManifest {
            caster_entity_id,
            effect_ids,
            rot_angle_in_rad,
            pos: *skill_center,
            half_extents,
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(2.0),
            next_damage_at: system_time,
        }
    }
}


impl SkillManifestation for BrutalSkillManifest {
    fn update(&mut self,
              self_entity_id: Entity,
              all_collisions_in_world: &Vec<Collision>,
              system_vars: &SystemVariables,
              entities: &specs::Entities,
              char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
              physics_world: &mut PhysicsWorld,
              updater: &mut specs::Write<LazyUpdate>) {
        if self.die_at.has_passed(system_vars.time) {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            if self.next_damage_at.has_not_passed(system_vars.time) {
                return;
            }
            self.next_damage_at = system_vars.time.add_seconds(0.5);
            for (target_entity_id, char_state) in (entities, char_storage).join() {
                // TODO: AABB ignores rotation
                dbg!(Isometry2::new(-self.pos, 0.0).inverse_transform_point(&char_state.pos()).coords);
                if ncollide2d::shape::Cuboid::new(self.half_extents).contains_point(
                    dbg!(&Isometry2::new(self.pos, 0.0)),
                    dbg!(&char_state.pos()),
                ) {
                    dbg!("COLLISION");
                    char_state.cannot_control_until.run_at_least_until_seconds(system_vars.time, 0.2);
                    char_state.set_state(CharState::ReceivingDamage, char_state.dir());
                    let damage_entity = entities.create();
                    updater.insert(damage_entity, AttackComponent {
                        src_entity: self.caster_entity_id,
                        dst_entity: target_entity_id,
                        typ: AttackType::Skill(Skills::BrutalTestSkill),
                    });
                }
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables) {
        Skills::render_casting_box2(
            &self.pos,
            &self.half_extents,
            self.rot_angle_in_rad,
            &system_vars,
        );
    }
}