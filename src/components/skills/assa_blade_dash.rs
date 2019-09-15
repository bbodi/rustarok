use nalgebra::{Isometry2, Vector2};
use specs::{Entities, LazyUpdate};

use crate::common::ElapsedTime;
use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharOutlook, CharacterStateComponent,
    SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::skills::skill::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{
    ApplyStatusComponent, Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{AreaAttackComponent, AttackType, DamageDisplayType};
use crate::runtime_assets::map::{CollisionGroup, PhysicEngine};
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::render_single_layer_action;
use crate::systems::SystemVariables;
use nphysics2d::object::Body;
use nphysics2d::object::BodyStatus;

pub struct AssaBladeDashSkill;

pub const ASSA_BLADE_DASH_SKILL: &'static AssaBladeDashSkill = &AssaBladeDashSkill;

impl SkillDef for AssaBladeDashSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_incagi.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster: &CharacterStateComponent,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        // allow to go through anything
        if let Some(collider) = physics_world.colliders.get_mut(caster.collider_handle) {
            let mut cg = collider.collision_groups().clone();
            cg.modify_membership(CollisionGroup::Player as usize, false);
            cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, true);
            collider.set_collision_groups(cg);
        }

        if let Some(body) = physics_world.bodies.get_mut(caster.body_handle) {
            body.set_status(BodyStatus::Kinematic);
        }

        let angle = char_to_skill_dir.angle(&Vector2::y());
        let angle = if char_to_skill_dir.x > 0.0 {
            angle
        } else {
            -angle
        };
        system_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                caster_entity_id,
                caster_entity_id,
                Box::new(AssaBladeDashStatus {
                    caster_entity_id,
                    started_at: system_vars.time,
                    ends_at: system_vars.time.add_seconds(
                        system_vars
                            .dev_configs
                            .skills
                            .assa_blade_dash
                            .duration_seconds,
                    ),
                    start_pos: caster.pos(),
                    center: caster.pos()
                        + char_to_skill_dir
                            * (system_vars
                                .dev_configs
                                .skills
                                .assa_blade_dash
                                .attributes
                                .casting_range
                                / 2.0),
                    rot_radian: angle,
                    vector: char_to_skill_dir
                        * system_vars
                            .dev_configs
                            .skills
                            .assa_blade_dash
                            .attributes
                            .casting_range,
                    shadow1_pos: Vector2::zeros(),
                    shadow2_pos: Vector2::zeros(),
                    forward_damage_done: false,
                    backward_damage_done: false,
                }),
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Directional
    }
}

#[derive(Clone)]
pub struct AssaBladeDashStatus {
    pub caster_entity_id: CharEntityId,
    pub started_at: ElapsedTime,
    pub ends_at: ElapsedTime,
    pub start_pos: WorldCoords,
    pub center: WorldCoords,
    pub rot_radian: f32,
    pub vector: WorldCoords,
    pub shadow1_pos: WorldCoords,
    pub shadow2_pos: WorldCoords,
    pub forward_damage_done: bool,
    pub backward_damage_done: bool,
}

impl Status for AssaBladeDashStatus {
    fn dupl(&self) -> Box<dyn Status> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        false
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [0, 0, 0, 0]
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        char_state: &CharacterStateComponent,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if let Some(body) = physics_world.bodies.rigid_body_mut(char_state.body_handle) {
            if self.ends_at.has_already_passed(system_vars.time) {
                body.set_status(BodyStatus::Dynamic);
                if let Some(collider) = physics_world.colliders.get_mut(char_state.collider_handle)
                {
                    let mut cg = collider.collision_groups().clone();
                    cg.modify_membership(CollisionGroup::Player as usize, true);
                    cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, false);
                    collider.set_collision_groups(cg);
                }
                StatusUpdateResult::RemoveIt
            } else {
                let duration_percentage = system_vars
                    .time
                    .percentage_between(self.started_at, self.ends_at);
                let pos = if duration_percentage < 0.5 {
                    let forward_perc = duration_percentage * 2.0;
                    self.shadow1_pos = self.start_pos + self.vector * (forward_perc - 0.1).max(0.0);
                    self.shadow2_pos = self.start_pos + self.vector * (forward_perc - 0.2).max(0.0);
                    self.start_pos + self.vector * forward_perc
                } else {
                    let backward_perc = (1.0 - duration_percentage) * 2.0;
                    self.shadow1_pos =
                        self.start_pos + self.vector * (backward_perc + 0.1).min(1.0);
                    self.shadow2_pos =
                        self.start_pos + self.vector * (backward_perc + 0.2).min(1.0);
                    self.start_pos + self.vector * backward_perc
                };
                if !self.forward_damage_done && duration_percentage > 0.25 {
                    let conf = &system_vars.dev_configs.skills.assa_blade_dash;
                    system_vars.area_attacks.push(AreaAttackComponent {
                        area_shape: Box::new(ncollide2d::shape::Cuboid::new(
                            Vector2::new(
                                conf.attributes.width.unwrap_or(1.0),
                                conf.attributes.casting_range,
                            ) / 2.0,
                        )),
                        area_isom: Isometry2::new(self.center, self.rot_radian),
                        source_entity_id: self.caster_entity_id,
                        typ: AttackType::Basic(conf.first_damage, DamageDisplayType::SingleNumber),
                    });
                    self.forward_damage_done = true;
                } else if !self.backward_damage_done && duration_percentage > 0.75 {
                    let conf = &system_vars.dev_configs.skills.assa_blade_dash;
                    system_vars.area_attacks.push(AreaAttackComponent {
                        area_shape: Box::new(ncollide2d::shape::Cuboid::new(
                            Vector2::new(
                                conf.attributes.width.unwrap_or(1.0),
                                conf.attributes.casting_range,
                            ) / 2.0,
                        )),
                        area_isom: Isometry2::new(self.center, self.rot_radian),
                        source_entity_id: self.caster_entity_id,
                        typ: AttackType::Basic(conf.second_damage, DamageDisplayType::SingleNumber),
                    });
                    self.backward_damage_done = true;
                }
                body.set_position(Isometry2::translation(pos.x, pos.y));
                StatusUpdateResult::KeepIt
            }
        } else {
            StatusUpdateResult::RemoveIt
        }
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        let duration_percentage = system_vars
            .time
            .percentage_between(self.started_at, self.ends_at);
        let half_duration = system_vars
            .dev_configs
            .skills
            .assa_blade_dash
            .duration_seconds
            / 2.0;
        match char_state.outlook {
            CharOutlook::Player {
                job_id,
                head_index,
                sex,
            } => {
                let body_sprite = {
                    let sprites = &system_vars.assets.sprites.character_sprites;
                    &sprites[&job_id][sex as usize]
                };
                let head_res = {
                    let sprites = &system_vars.assets.sprites.head_sprites;
                    &sprites[sex as usize][head_index]
                };
                for (pos, alpha, time_offset) in &[
                    (char_state.pos(), 255, 0.0),
                    (self.shadow1_pos, 175, 0.05),
                    (self.shadow2_pos, 100, 0.1),
                ] {
                    let anim_descr = if duration_percentage < 0.5 {
                        SpriteRenderDescriptorComponent {
                            action_index: CharActionIndex::Attacking1 as usize,
                            animation_started: self.started_at.add_seconds(*time_offset),
                            animation_ends_at: ElapsedTime(0.0),
                            forced_duration: Some(ElapsedTime(half_duration)),
                            direction: char_state.dir(),
                            fps_multiplier: 1.0,
                        }
                    } else {
                        SpriteRenderDescriptorComponent {
                            action_index: CharActionIndex::Attacking1 as usize,
                            animation_started: self
                                .started_at
                                .add_seconds(half_duration + *time_offset),
                            animation_ends_at: ElapsedTime(0.0),
                            forced_duration: Some(ElapsedTime(half_duration)),
                            direction: (char_state.dir() + 4) % 8,
                            fps_multiplier: 1.0,
                        }
                    };
                    let offset = render_single_layer_action(
                        system_vars.time,
                        &anim_descr,
                        body_sprite,
                        render_commands.yaw,
                        &pos,
                        [0, 0],
                        true,
                        1.0,
                        ActionPlayMode::Repeat,
                        &[255, 255, 0, *alpha],
                        render_commands,
                    );

                    render_single_layer_action(
                        system_vars.time,
                        &anim_descr,
                        head_res,
                        render_commands.yaw,
                        &pos,
                        offset,
                        false,
                        1.0,
                        ActionPlayMode::Repeat,
                        &[255, 255, 0, *alpha],
                        render_commands,
                    );
                }
            }
            CharOutlook::Monster(monster_id) => {
                let body_res = {
                    let sprites = &system_vars.assets.sprites.monster_sprites;
                    &sprites[&monster_id]
                };
            }
        }
    }

    fn stack(&self, _other: Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
