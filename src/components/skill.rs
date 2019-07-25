use specs::prelude::*;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use ncollide2d::world::CollisionGroups;
use crate::{STATIC_MODELS_COLLISION_GROUP, SKILL_AREA_COLLISION_GROUP, PhysicsWorld, ElapsedTime};
use nalgebra::{Vector2, Vector3, Point2, Matrix4, Rotation3, Point3, Isometry2};
use crate::systems::{SystemVariables, Collision};
use std::sync::{Arc, Mutex};
use crate::video::{draw_lines_inefficiently, draw_circle_inefficiently};
use crate::components::char::{CastingSkillData, CharacterStateComponent};
use crate::components::controller::WorldCoords;
use crate::components::{StrEffectComponent, AttackComponent, AttackType};
use ncollide2d::query::{Proximity};

#[macro_export]
macro_rules! v2 {
    ($x:expr, $y:expr) => { Vector2::<f32>::new($x as f32, $y as f32) }
}

#[macro_export]
macro_rules! v3 {
    ($x:expr, $y:expr, $z:expr) => { Vector3::<f32>::new($x as f32, $y as f32, $z as f32) }
}

#[macro_export]
macro_rules! p2 {
    ($x:expr, $y:expr) => { Point2::<f32>::new($x as f32, $y as f32) }
}

#[macro_export]
macro_rules! p3 {
    ($x:expr, $y:expr, $z:expr) => { Point3::<f32>::new($x as f32, $y as f32, $z as f32) }
}


pub trait SkillManifestation {
    fn update(
        &mut self,
        entity_id: Entity,
        all_collisions_in_world: &Vec<Collision>,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
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
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
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
        let skill = self.skill.lock().unwrap();
        skill.render(system_vars);
    }
}

unsafe impl Sync for SkillManifestationComponent {}

unsafe impl Send for SkillManifestationComponent {}


pub trait SkillDescriptor {
    fn finish_cast(
        &self,
        caster_entity_id: Entity,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        target_entity: Option<Entity>,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<SkillManifestation>>;

    fn get_casting_time(&self) -> ElapsedTime;
    fn get_casting_range(&self) -> f32;

    fn get_skill_target_type(&self) -> SkillTargetType;

    fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables);

    fn is_casting_allowed(
        &self,
        caster_id: Entity,
        target_entity: Option<Entity>,
        target_distance: f32,
    ) -> bool;

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
    Lightning,
    Heal,
//    Mounting,
}

impl Skills {
    pub fn damage_chars(
        entities: &Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        updater: &mut specs::Write<LazyUpdate>,
        skill_shape: impl ncollide2d::shape::Shape<f32>,
        skill_isom: Isometry2<f32>,
        caster_entity_id: Entity,
        skill: Skills,
    ) {
        for (target_entity_id, char_state) in (entities, char_storage).join() {
            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &skill_isom, &skill_shape,
                &Isometry2::new(char_state.pos().coords, 0.0), &ncollide2d::shape::Ball::new(1.0),
                0.0
            );
            if coll_result == Proximity::Intersecting {
                let damage_entity = entities.create();
                updater.insert(damage_entity, AttackComponent {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: AttackType::Skill(skill),
                });
            }
        }
    }

    fn limit_vector_into_range(char_pos: &Vector2<f32>, mouse_pos: &WorldCoords, range: f32) -> (Vector3<f32>, Vector3<f32>) {
        let dir2d = mouse_pos.coords - char_pos;
        let dir_vector = v2_to_v3(&dir2d).normalize();
        let pos = v2_to_v3(&char_pos) + dir_vector * dir2d.magnitude().min(range);
        return (pos, dir_vector);
    }

    pub fn render_casting_box(
        casting_area_size: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        char_pos: &Vector2<f32>,
        system_vars: &SystemVariables,
    ) {
        let half = casting_area_size / 2.0;
        let bottom_left = v2!(-half.x, -half.y);
        let top_left = v2!(-half.x, half.y);
        let top_right = v2!(half.x, half.y);
        let bottom_right = v2!(half.x, -half.y);
        // rotate
        let rot_matrix = Matrix4::<f32>::identity();
        let angle = (mouse_pos.coords - char_pos).angle(&Vector2::y());
        let angle = if mouse_pos.x > char_pos.x { angle } else { -angle };
        let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), angle).to_homogeneous();
        let rot_matrix = rot_matrix * rotation;

        let bottom_left = rot_matrix.transform_point(&p3!(bottom_left.x, 1, bottom_left.y));
        let top_left = rot_matrix.transform_point(&p3!(top_left.x, 1, top_left.y));
        let top_right = rot_matrix.transform_point(&p3!(top_right.x, 1, top_right.y));
        let bottom_right = rot_matrix.transform_point(&p3!(bottom_right.x, 1, bottom_right.y));

        let skill_3d_pos = p2_to_v3(&mouse_pos);
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

        let self_pos_3d = v2_to_v3(&pos);
        let bottom_left = self_pos_3d + rot_matrix.transform_point(&p3!(-half.x, 1, -half.y)).coords;
        let top_left = self_pos_3d + rot_matrix.transform_point(&p3!(-half.x, 1, half.y)).coords;
        let top_right = self_pos_3d + rot_matrix.transform_point(&p3!(half.x, 1, half.y)).coords;
        let bottom_right = self_pos_3d + rot_matrix.transform_point(&p3!(half.x, 1, -half.y)).coords;

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

#[derive(Eq, PartialEq)]
pub enum SkillTargetType {
    /// casts immediately
    NoTarget,
    Area,
    AnyEntity,
    OnlyAllyButNoSelf,
    OnlyAllyAndSelf,
    OnlyEnemy,
    OnlySelf,
}

impl SkillDescriptor for Skills {
    fn finish_cast(
        &self,
        caster_entity_id: Entity,
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        target_entity: Option<Entity>,
        physics_world: &mut PhysicsWorld,
        system_vars: &SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        match self {
            Skills::TestSkill => {
                let angle_in_rad = (mouse_pos.coords - char_pos).angle(&Vector2::y());
                let angle_in_rad = if mouse_pos.x > char_pos.x { angle_in_rad } else { -angle_in_rad };
                Some(Box::new(
                    PushBackWallSkill::new(
                        caster_entity_id,
                        physics_world,
                        &mouse_pos.coords,
                        angle_in_rad,
                        system_vars.time,
                        entities,
                        updater,
                    )
                ))
            }
            Skills::BrutalTestSkill => {
                let angle_in_rad = (mouse_pos.coords - char_pos).angle(&Vector2::y());
                let angle_in_rad = if mouse_pos.x > char_pos.x { angle_in_rad } else { -angle_in_rad };
                Some(Box::new(
                    BrutalSkillManifest::new(
                        caster_entity_id,
                        &mouse_pos.coords,
                        angle_in_rad,
                        system_vars.time,
                        entities,
                        updater,
                    )
                ))
            }
            Skills::Lightning => {
                let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(
                    &char_pos,
                    &mouse_pos,
                    self.get_casting_range(),
                );
                Some(Box::new(
                    LightningManifest::new(
                        caster_entity_id,
                        skill_3d_pos,
                        dir_vector,
                        system_vars.time,
                        entities,
                    )
                ))
            }
            Skills::Heal => {
                let damage_entity = entities.create();
                updater.insert(damage_entity, AttackComponent {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity.unwrap(),
                    typ: AttackType::Skill(Skills::Heal),
                });
                None
            }
        }
    }

    fn get_casting_time(&self) -> ElapsedTime {
        match self {
            Skills::TestSkill => ElapsedTime(0.3),
            Skills::BrutalTestSkill => ElapsedTime(1.0),
            Skills::Lightning => ElapsedTime(0.7),
            Skills::Heal => { ElapsedTime(0.3) }
        }
    }

    fn get_casting_range(&self) -> f32 {
        match self {
            Skills::TestSkill => 10.0,
            Skills::BrutalTestSkill => 20.,
            Skills::Lightning => 7.0,
            Skills::Heal => { 10.0 }
        }
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        match self {
            Skills::TestSkill => SkillTargetType::Area,
            Skills::BrutalTestSkill => SkillTargetType::Area,
            Skills::Lightning => SkillTargetType::Area,
            Skills::Heal => SkillTargetType::OnlyAllyAndSelf,
        }
    }

    fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables,
    ) {
        match self {
            _ => {
                self.render_target_selection(
                    char_pos,
                    &casting_state.mouse_pos_when_casted,
                    system_vars,
                );
            }
        }
    }

    fn is_casting_allowed(
        &self,
        caster_id: Entity,
        target_entity: Option<Entity>,
        target_distance: f32,
    ) -> bool {
        let close_enough = self.get_casting_range() >= target_distance;
        close_enough && match self.get_skill_target_type() {
            SkillTargetType::Area => true,
            SkillTargetType::NoTarget => true,
            SkillTargetType::AnyEntity => target_entity.is_some(),
            SkillTargetType::OnlyAllyButNoSelf => target_entity.map(|it| it != caster_id).unwrap_or(false),
            SkillTargetType::OnlyAllyAndSelf => target_entity.is_some(),
            SkillTargetType::OnlyEnemy => target_entity.is_some(),
            SkillTargetType::OnlySelf => target_entity.map(|it| it == caster_id).unwrap_or(false),
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
                    &v2!(3.0, 1.0),
                    mouse_pos,
                    char_pos,
                    system_vars,
                );
            }
            Skills::BrutalTestSkill => {
                Skills::render_casting_box(
                    &v2!(10.0, 10.0),
                    mouse_pos,
                    char_pos,
                    system_vars,
                );
            }
            Skills::Lightning => {
                let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(&char_pos, &mouse_pos, self.get_casting_range());
                for i in 0..3 {
                    draw_circle_inefficiently(&system_vars.shaders.trimesh_shader,
                                              &system_vars.matrices.projection,
                                              &system_vars.matrices.view,
                                              &(skill_3d_pos + dir_vector * i as f32 * 2.2),
                                              1.0,
                                              &[0.0, 1.0, 0.0, 1.0]);
                }
            }
            Skills::Heal => {}
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

pub fn v3_to_v2(input: &Vector3<f32>) -> Vector2<f32> {
    v2!(input.x, input.z)
}

pub fn v3_to_p2(input: &Vector3<f32>) -> Point2<f32> {
    Point2::new(input.x, input.z)
}

pub fn p3_to_p2(input: &Point3<f32>) -> Point2<f32> {
    Point2::new(input.x, input.z)
}

pub fn p3_to_v2(input: &Point3<f32>) -> Vector2<f32> {
    v2!(input.x, input.z)
}

pub fn v2_to_p3(input: &Vector2<f32>) -> Point3<f32> {
    p3!(input.x, 0.0, input.y)
}

pub fn p2_to_v3(input: &Point2<f32>) -> Vector3<f32> {
    Vector3::new(input.x, 0.0, input.y)
}

pub fn p2_to_v2(input: &Point2<f32>) -> Vector2<f32> {
    input.coords
}

pub fn v2_to_v3(input: &Vector2<f32>) -> Vector3<f32> {
    Vector3::new(input.x, 0.0, input.y)
}

pub fn v2<T: Into<f32>>(x: T, y: T) -> Vector2<f32> {
    v2!(x.into(), y.into())
}

pub fn v2_to_p2(input: &Vector2<f32>) -> Point2<f32> {
    Point2::new(input.x, input.y)
}

pub fn rotate_vec(rad: f32, vec: &Vector2<f32>) -> Point2<f32> {
    let rot_matrix = Matrix4::<f32>::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad).to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&v2_to_p3(&vec));
    return p3_to_p2(&rotated);
}

pub fn rotate_vec2(rad: f32, vec: &Vector2<f32>) -> Vector2<f32> {
    let rot_matrix = Matrix4::<f32>::identity();
    let rotation = Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), rad).to_homogeneous();
    let rot_matrix = rot_matrix * rotation;
    let rotated = rot_matrix.transform_point(&v2_to_p3(vec));
    return p3_to_v2(&rotated);
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
        let half_extents = v2!(1.5, 0.5);
        let effect_ids = [
            skill_center + rotate_vec2(rot_angle_in_rad, &v2!(-1.0, 0.0)),
            *skill_center,
            skill_center + rotate_vec2(rot_angle_in_rad, &v2!(1.0, 0.0)),
        ].iter().map(|effect_coords| {
            let effect_comp = StrEffectComponent {
                effect: "StrEffect::FireWall".to_owned(),
                pos: v2_to_p2(effect_coords),
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
              char_storage: &specs::ReadStorage<CharacterStateComponent>,
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

impl BrutalSkillManifest {
    pub fn new(
        caster_entity_id: Entity,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> BrutalSkillManifest {
        let half_extents = v2!(5.0, 5.0);

//        let effect_ids = (0..11*11).map(|i| {
//            let x = (-5.0 + (i%10) as f32);
//            let y = (-5.0 + (i/10) as f32);
//            skill_center + rotate_vec2(rot_angle_in_rad, &v2!(x, y))
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
            pos: v2_to_p2(&skill_center),
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
              char_storage: &specs::ReadStorage<CharacterStateComponent>,
              _physics_world: &mut PhysicsWorld,
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
            Skills::damage_chars(
                entities,
                char_storage,
                updater,
                ncollide2d::shape::Cuboid::new(self.half_extents),
                Isometry2::new(self.pos, self.rot_angle_in_rad),
                self.caster_entity_id,
                Skills::BrutalTestSkill,
            );
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

pub struct LightningManifest {
    pub caster_entity_id: Entity,
    pub effect_id: Entity,
    pub pos: Vector3<f32>,
    pub dir_vector: Vector3<f32>,
    pub created_at: ElapsedTime,
    pub next_action_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub last_skill_pos: Vector2<f32>,
    pub action_count: u8,
}

impl LightningManifest {
    pub fn new(
        caster_entity_id: Entity,
        skill_center: Vector3<f32>,
        dir_vector: Vector3<f32>,
        now: ElapsedTime,
        entities: &specs::Entities,
    ) -> LightningManifest {
        LightningManifest {
            caster_entity_id,
            effect_id: entities.create(),
            pos: skill_center,
            created_at: now,
            next_action_at: now,
            next_damage_at: now,
            last_skill_pos: v3_to_v2(&skill_center),
            action_count: 0,
            dir_vector,
        }
    }
}


impl SkillManifestation for LightningManifest {
    fn update(&mut self,
              self_entity_id: Entity,
              _all_collisions_in_world: &Vec<Collision>,
              system_vars: &SystemVariables,
              entities: &specs::Entities,
              char_storage: &specs::ReadStorage<CharacterStateComponent>,
              physics_world: &mut PhysicsWorld,
              updater: &mut specs::Write<LazyUpdate>) {
        if self.created_at.add_seconds(12.0).has_passed(system_vars.time) {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            updater.remove::<StrEffectComponent>(self.effect_id);
        } else {
            if self.next_action_at.has_passed(system_vars.time) {
                updater.remove::<StrEffectComponent>(self.effect_id);
                let effect_comp = match self.action_count {
                    0 => {
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&self.pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    1 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    2 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    3 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    4 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    5 => {
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos: v3_to_p2(&self.pos),
                            start_time: system_vars.time,
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    _ => {
                        return;
                    }
                };
                self.last_skill_pos = p2_to_v2(&effect_comp.pos.clone());
                updater.insert(self.effect_id, effect_comp);
                self.action_count += 1;
                self.next_action_at = system_vars.time.add_seconds(2.0);
                self.next_damage_at = system_vars.time; // do a damage right away at the new skill location
            }
            if self.next_damage_at.has_passed(system_vars.time) {
                Skills::damage_chars(
                    entities,
                    char_storage,
                    updater,
                    ncollide2d::shape::Ball::new(1.0),
                    Isometry2::new(self.last_skill_pos, 0.0),
                    self.caster_entity_id,
                    Skills::Lightning,
                );
                self.next_damage_at = self.next_damage_at.add_seconds(0.6);
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables) {
        for i in self.action_count..3 {
            draw_circle_inefficiently(&system_vars.shaders.trimesh_shader,
                                      &system_vars.matrices.projection,
                                      &system_vars.matrices.view,
                                      &(self.pos + self.dir_vector * i as f32 * 2.2),
                                      1.0,
                                      &[0.0, 1.0, 0.0, 1.0]);
        }
        // backwards
        if self.action_count >= 4 {
            for i in self.action_count..6 {
                draw_circle_inefficiently(&system_vars.shaders.trimesh_shader,
                                          &system_vars.matrices.projection,
                                          &system_vars.matrices.view,
                                          &(self.pos + self.dir_vector * (5 - i) as f32 * 2.2),
                                          1.0,
                                          &[0.0, 1.0, 0.0, 1.0]);
            }
        }
    }
}