use crate::common::{rotate_vec2, v2_to_v3};
use crate::components::char::{CastingSkillData, CharacterStateComponent};
use crate::components::controller::WorldCoords;
use crate::components::skills::absorb_shield::AbsorbStatus;
use crate::components::skills::fire_bomb::FireBombStatus;
use crate::components::skills::lightning::{LightningManifest, LightningSkill};
use crate::components::status::{
    ApplyStatusComponent, MainStatuses, RemoveStatusComponent, StatusType,
};
use crate::components::{
    ApplyForceComponent, AreaAttackComponent, AttackComponent, AttackType, StrEffectComponent,
};
use crate::systems::render::RenderDesktopClientSystem;
use crate::systems::{Collision, SystemVariables};
use crate::video::draw_lines_inefficiently;
use crate::{ElapsedTime, PhysicsWorld, SKILL_AREA_COLLISION_GROUP, STATIC_MODELS_COLLISION_GROUP};
use nalgebra::{Isometry2, Matrix4, Point3, Rotation3, Vector2, Vector3};
use ncollide2d::shape::ShapeHandle;
use ncollide2d::world::CollisionGroups;
use nphysics2d::object::{ColliderDesc, ColliderHandle};
use specs::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strum_macros::EnumIter;

pub trait SkillManifestation {
    fn update(
        &mut self,
        entity_id: Entity,
        all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    );

    fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>);
}

#[storage(HashMapStorage)]
#[derive(Component)]
pub struct SkillManifestationComponent {
    pub self_entity_id: Entity,
    pub skill: Arc<Mutex<Box<dyn SkillManifestation>>>,
}

impl SkillManifestationComponent {
    pub fn new(
        self_entity_id: Entity,
        skill: Box<dyn SkillManifestation>,
    ) -> SkillManifestationComponent {
        SkillManifestationComponent {
            self_entity_id,
            skill: Arc::new(Mutex::new(skill)),
        }
    }

    pub fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        let mut skill = self.skill.lock().unwrap();
        skill.update(
            self_entity_id,
            all_collisions_in_world,
            system_vars,
            entities,
            char_storage,
            physics_world,
            updater,
        );
    }

    pub fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>) {
        let skill = self.skill.lock().unwrap();
        skill.render(system_vars, view_matrix);
    }
}

unsafe impl Sync for SkillManifestationComponent {}

unsafe impl Send for SkillManifestationComponent {}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, EnumIter)]
pub enum Skills {
    FireWall,
    BrutalTestSkill,
    Lightning,
    Heal,
    Mounting,
    Poison,
    Cure,
    FireBomb,
    AbsorbShield,
}

impl Skills {
    pub fn get_icon_path(&self) -> &'static str {
        match self {
            Skills::FireWall => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mg_firewall.bmp"
            }
            Skills::BrutalTestSkill => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
            }
            Skills::Lightning => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wl_chainlightning.bmp"
            }
            Skills::Heal => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp",
            Skills::Mounting => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\su_pickypeck.bmp"
            }
            Skills::Poison => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\tf_poison.bmp",
            Skills::Cure => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\so_el_cure.bmp",
            Skills::FireBomb => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_makebomb.bmp"
            }
            Skills::AbsorbShield => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
            }
        }
    }

    pub fn limit_vector_into_range(
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        range: f32,
    ) -> (Vector2<f32>, Vector2<f32>) {
        let dir2d = mouse_pos - char_pos;
        let dir_vector = dir2d.normalize();
        let pos = char_pos + dir_vector * dir2d.magnitude().min(range);
        return (pos, dir_vector);
    }

    pub fn render_casting_box(
        casting_area_size: &Vector2<f32>,
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        system_vars: &SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        let half = casting_area_size / 2.0;
        let bottom_left = v2!(-half.x, -half.y);
        let top_left = v2!(-half.x, half.y);
        let top_right = v2!(half.x, half.y);
        let bottom_right = v2!(half.x, -half.y);
        // rotate
        let rot_matrix = Matrix4::<f32>::identity();
        let angle = char_to_skill_dir.angle(&Vector2::y());
        let angle = if char_to_skill_dir.x > 0.0 {
            angle
        } else {
            -angle
        };
        let rotation =
            Rotation3::from_axis_angle(&nalgebra::Unit::new_normalize(Vector3::y()), angle)
                .to_homogeneous();
        let rot_matrix = rot_matrix * rotation;

        let bottom_left = rot_matrix.transform_point(&p3!(bottom_left.x, 1, bottom_left.y));
        let top_left = rot_matrix.transform_point(&p3!(top_left.x, 1, top_left.y));
        let top_right = rot_matrix.transform_point(&p3!(top_right.x, 1, top_right.y));
        let bottom_right = rot_matrix.transform_point(&p3!(bottom_right.x, 1, bottom_right.y));

        let skill_pos = v2_to_v3(skill_pos);
        draw_lines_inefficiently(
            &system_vars.shaders.trimesh_shader,
            &system_vars.matrices.projection,
            view_matrix,
            &[
                skill_pos + bottom_left.coords,
                skill_pos + top_left.coords,
                skill_pos + top_right.coords,
                skill_pos + bottom_right.coords,
            ],
            &[0.0, 1.0, 0.0, 1.0],
        );
    }

    pub fn render_casting_box2(
        pos: &Vector2<f32>,
        half: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_vars: &SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        let rot_matrix = Matrix4::<f32>::identity();
        let rotation = Rotation3::from_axis_angle(
            &nalgebra::Unit::new_normalize(Vector3::y()),
            rot_angle_in_rad,
        )
        .to_homogeneous();
        let rot_matrix = rot_matrix * rotation;

        let self_pos_3d = v2_to_v3(&pos);
        let bottom_left =
            self_pos_3d + rot_matrix.transform_point(&p3!(-half.x, 1, -half.y)).coords;
        let top_left = self_pos_3d + rot_matrix.transform_point(&p3!(-half.x, 1, half.y)).coords;
        let top_right = self_pos_3d + rot_matrix.transform_point(&p3!(half.x, 1, half.y)).coords;
        let bottom_right =
            self_pos_3d + rot_matrix.transform_point(&p3!(half.x, 1, -half.y)).coords;

        draw_lines_inefficiently(
            &system_vars.shaders.trimesh_shader,
            &system_vars.matrices.projection,
            view_matrix,
            &[bottom_left, top_left, top_right, bottom_right],
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

impl Skills {
    pub fn finish_cast(
        &self,
        caster_entity_id: Entity,
        char_pos: &Vector2<f32>,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<Entity>,
        physics_world: &mut PhysicsWorld,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        match self {
            Skills::FireWall => {
                let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
                let angle_in_rad = if char_to_skill_dir.x > 0.0 {
                    angle_in_rad
                } else {
                    -angle_in_rad
                };
                Some(Box::new(PushBackWallSkill::new(
                    caster_entity_id,
                    physics_world,
                    &skill_pos.unwrap(),
                    angle_in_rad,
                    system_vars.time,
                    entities,
                    updater,
                )))
            }
            Skills::BrutalTestSkill => {
                let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
                let angle_in_rad = if char_to_skill_dir.x > 0.0 {
                    angle_in_rad
                } else {
                    -angle_in_rad
                };
                Some(Box::new(BrutalSkillManifest::new(
                    caster_entity_id,
                    &skill_pos.unwrap(),
                    angle_in_rad,
                    system_vars.time,
                    entities,
                    updater,
                )))
            }
            Skills::Lightning => Some(Box::new(LightningManifest::new(
                caster_entity_id,
                &skill_pos.unwrap(),
                char_to_skill_dir,
                system_vars.time,
                entities,
            ))),
            Skills::Heal => {
                system_vars.attacks.push(AttackComponent {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity.unwrap(),
                    typ: AttackType::Heal(8000),
                });
                None
            }
            Skills::Mounting => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_main_status(
                        caster_entity_id,
                        caster_entity_id,
                        MainStatuses::Mounted,
                    ));
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect: "StrEffect::Concentration".to_owned(),
                        pos: *char_pos,
                        start_time: system_vars.time,
                        die_at: system_vars.time.add_seconds(0.7),
                        duration: ElapsedTime(0.7),
                    },
                );
                None
            }
            Skills::Poison => {
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect: "hunter_poison".to_owned(),
                        pos: skill_pos.unwrap(),
                        start_time: system_vars.time,
                        die_at: system_vars.time.add_seconds(0.7),
                        duration: ElapsedTime(0.7),
                    },
                );
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_main_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        MainStatuses::Poison,
                    ));
                None
            }
            Skills::Cure => {
                system_vars
                    .remove_statuses
                    .push(RemoveStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        StatusType::Harmful,
                    ));
                None
            }
            Skills::FireBomb => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        Box::new(FireBombStatus {
                            caster_entity_id,
                            started: system_vars.time,
                            until: system_vars.time.add_seconds(2.0),
                        }),
                    ));
                None
            }
            Skills::AbsorbShield => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        Box::new(AbsorbStatus::new(caster_entity_id, system_vars.time)),
                    ));
                None
            }
        }
    }

    pub fn get_casting_time(&self, char_state: &CharacterStateComponent) -> ElapsedTime {
        let t = match self {
            Skills::FireWall => 0.3,
            Skills::BrutalTestSkill => 1.0,
            Skills::Lightning => 0.7,
            Skills::Heal => 0.3,
            Skills::Mounting => {
                if char_state.statuses.is_mounted() {
                    0.0
                } else {
                    2.0
                }
            }
            Skills::Poison => 0.5,
            Skills::Cure => 0.5,
            Skills::FireBomb => 0.5,
            Skills::AbsorbShield => 0.1,
        };
        return ElapsedTime(t);
    }

    pub fn get_casting_range(&self) -> f32 {
        match self {
            Skills::FireWall => 10.0,
            Skills::BrutalTestSkill => 20.,
            Skills::Lightning => 7.0,
            Skills::Heal => 10.0,
            Skills::Mounting => 0.0,
            Skills::Poison => 10.0,
            Skills::Cure => 10.0,
            Skills::FireBomb => 10.0,
            Skills::AbsorbShield => 10.0,
        }
    }

    pub fn get_skill_target_type(&self) -> SkillTargetType {
        match self {
            Skills::FireWall => SkillTargetType::Area,
            Skills::BrutalTestSkill => SkillTargetType::Area,
            Skills::Lightning => SkillTargetType::Area,
            Skills::Heal => SkillTargetType::OnlyAllyAndSelf,
            Skills::Mounting => SkillTargetType::NoTarget,
            Skills::Poison => SkillTargetType::OnlyEnemy,
            Skills::Cure => SkillTargetType::OnlyAllyAndSelf,
            Skills::FireBomb => SkillTargetType::OnlyEnemy,
            Skills::AbsorbShield => SkillTargetType::OnlyAllyAndSelf,
        }
    }

    pub fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &mut SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        match self {
            _ => {
                RenderDesktopClientSystem::render_str(
                    "StrEffect::Moonstar",
                    casting_state.cast_started,
                    char_pos,
                    system_vars,
                    view_matrix,
                );
                if let Some(target_area_pos) = casting_state.target_area_pos {
                    self.render_target_selection(
                        &target_area_pos,
                        &casting_state.char_to_skill_dir_when_casted,
                        system_vars,
                        view_matrix,
                    );
                }
            }
        }
    }

    pub fn is_casting_allowed(
        &self,
        caster_id: Entity,
        target_entity: Option<Entity>,
        target_distance: f32,
    ) -> bool {
        match self.get_skill_target_type() {
            SkillTargetType::Area => true,
            SkillTargetType::NoTarget => true,
            SkillTargetType::AnyEntity => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyAllyButNoSelf => {
                target_entity.map(|it| it != caster_id).unwrap_or(false)
                    && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyAllyAndSelf => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyEnemy => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlySelf => target_entity.map(|it| it == caster_id).unwrap_or(false),
        }
    }

    pub fn render_target_selection(
        &self,
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        system_vars: &SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        match self {
            Skills::FireWall => {
                Skills::render_casting_box(
                    &v2!(3.0, 1.0),
                    skill_pos,
                    char_to_skill_dir,
                    system_vars,
                    view_matrix,
                );
            }
            Skills::BrutalTestSkill => {
                Skills::render_casting_box(
                    &v2!(10.0, 10.0),
                    skill_pos,
                    char_to_skill_dir,
                    system_vars,
                    view_matrix,
                );
            }
            Skills::Lightning => {
                LightningSkill::render_target_selection(
                    skill_pos,
                    char_to_skill_dir,
                    system_vars,
                    view_matrix,
                );
            }
            Skills::Heal => {}
            Skills::Mounting => {}
            Skills::Poison => {}
            Skills::Cure => {}
            Skills::FireBomb => {}
            Skills::AbsorbShield => {}
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
        ]
        .iter()
        .map(|effect_coords| {
            let effect_comp = StrEffectComponent {
                effect: "firewall".to_owned(),
                pos: *effect_coords,
                start_time: system_time,
                die_at: system_time.add_seconds(3.0),
                duration: ElapsedTime(3.0),
            };
            let effect_entity = entities.create();
            updater.insert(effect_entity, effect_comp);
            effect_entity
        })
        .collect();

        let cuboid = ShapeHandle::new(ncollide2d::shape::Cuboid::new(half_extents));
        let collider_handle = ColliderDesc::new(cuboid)
            .translation(*skill_center)
            .rotation(rot_angle_in_rad.to_degrees())
            .collision_groups(
                CollisionGroups::new()
                    .with_membership(&[SKILL_AREA_COLLISION_GROUP])
                    .with_blacklist(&[STATIC_MODELS_COLLISION_GROUP]),
            )
            .sensor(true)
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
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.die_at.has_passed(system_vars.time) {
            physics_world.remove_colliders(&[self.collider_handle]);
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            // TODO: wouldn't it be better to use the area push functionality?
            let my_collisions = all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self.collider_handle);
            for (_key, coll) in my_collisions {
                if let Some(char_body_handle) = physics_world
                    .collider(coll.character_coll_handle)
                    .map(|it| it.body())
                {
                    let char_body = physics_world.rigid_body_mut(char_body_handle).unwrap();
                    let char_entity_id = *char_body
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if let Some(char_state) = char_storage.get(char_entity_id) {
                        let push_dir = self.pos - char_state.pos();
                        let push_dir = if push_dir.x == 0.0 && push_dir.y == 0.0 {
                            v2!(1, 0) // "random"
                        } else {
                            -push_dir.normalize()
                        };
                        dbg!("Firewall push attack");
                        system_vars.attacks.push(AttackComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: char_entity_id,
                            typ: AttackType::SpellDamage(600),
                        });
                        system_vars.pushes.push(ApplyForceComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: char_entity_id,
                            force: push_dir * 20.0,
                            body_handle: char_body_handle,
                            duration: 1.0,
                        });
                    }
                }
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>) {
        Skills::render_casting_box2(
            &self.pos,
            &self.half_extents,
            self.rot_angle_in_rad,
            &system_vars,
            view_matrix,
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
        //                effect: "firewall".to_owned(),
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
            pos: *skill_center,
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
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &specs::ReadStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
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
            system_vars.area_attacks.push(AreaAttackComponent {
                area_shape: Box::new(ncollide2d::shape::Cuboid::new(self.half_extents)),
                area_isom: Isometry2::new(self.pos, self.rot_angle_in_rad),
                source_entity_id: self.caster_entity_id,
                typ: AttackType::SpellDamage(600),
            });
        }
    }

    fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>) {
        Skills::render_casting_box2(
            &self.pos,
            &self.half_extents,
            self.rot_angle_in_rad,
            &system_vars,
            view_matrix,
        );
    }
}
