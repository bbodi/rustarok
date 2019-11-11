use nalgebra::{Isometry2, Vector2};
use specs::{Entities, LazyUpdate};

use crate::common::{v2, v2_to_v3, ElapsedTime, Vec2};
use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharOutlook, CharacterStateComponent,
    SpriteRenderDescriptorComponent,
};
use crate::components::controller::CharEntityId;
use crate::components::skills::basic_attack::WeaponType;
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{
    ApplyStatusComponent, Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{AreaAttackComponent, DamageDisplayType, HpModificationType};
use crate::configs::{AssaBladeDashSkillConfig, DevConfig};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::render_single_layer_action;
use crate::systems::SystemVariables;

pub struct AssaBladeDashSkill;

pub const ASSA_BLADE_DASH_SKILL: &'static AssaBladeDashSkill = &AssaBladeDashSkill;

impl SkillDef for AssaBladeDashSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_incagi.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        _caster_pos: Vec2,
        _skill_pos: Option<Vec2>,
        char_to_skill_dir: &Vec2,
        _target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .write_storage::<CharacterStateComponent>()
            .get_mut(caster_entity_id.0)
        {
            let angle = char_to_skill_dir.angle(&Vector2::y());
            let angle = if char_to_skill_dir.x > 0.0 {
                angle
            } else {
                -angle
            };

            let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
            let configs = ecs_world
                .read_resource::<DevConfig>()
                .skills
                .assa_blade_dash
                .clone();
            let now = sys_vars.time;
            sys_vars
                .apply_statuses
                .push(ApplyStatusComponent::from_secondary_status(
                    caster_entity_id,
                    caster_entity_id,
                    Box::new(AssaBladeDashStatus {
                        caster_entity_id,
                        started_at: now,
                        ends_at: now.add_seconds(configs.duration_seconds),
                        start_pos: caster.pos(),
                        center: caster.pos()
                            + char_to_skill_dir * (configs.attributes.casting_range / 2.0),
                        rot_radian: angle,
                        vector: char_to_skill_dir * configs.attributes.casting_range,
                        shadow1_pos: Vector2::zeros(),
                        shadow2_pos: Vector2::zeros(),
                        forward_damage_done: false,
                        backward_damage_done: false,
                        half_duration: configs.duration_seconds / 2.0,
                        configs,
                    }),
                ));
        }
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
    pub start_pos: Vec2,
    pub center: Vec2,
    pub rot_radian: f32,
    pub half_duration: f32,
    pub vector: Vec2,
    pub shadow1_pos: Vec2,
    pub shadow2_pos: Vec2,
    pub forward_damage_done: bool,
    pub backward_damage_done: bool,
    pub configs: AssaBladeDashSkillConfig,
}

impl Status for AssaBladeDashStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn on_apply(
        &mut self,
        _self_entity_id: CharEntityId,
        target_char: &mut CharacterStateComponent,
        _entities: &Entities,
        _updater: &mut LazyUpdate,
        _sys_vars: &SystemVariables,
        physics_world: &mut PhysicEngine,
    ) {
        // allow to go through anything
        target_char.set_noncollidable(physics_world);
    }

    fn can_target_move(&self) -> bool {
        false
    }

    fn can_target_cast(&self) -> bool {
        true
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [0, 0, 0, 0]
    }

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        char_state: &mut CharacterStateComponent,
        physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        _entities: &Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if let Some(body) = physics_world.bodies.rigid_body_mut(char_state.body_handle) {
            if self.ends_at.has_already_passed(sys_vars.time) {
                char_state.set_collidable(physics_world);
                StatusUpdateResult::RemoveIt
            } else {
                let duration_percentage = sys_vars
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
                body.set_position(Isometry2::translation(pos.x, pos.y));

                if !self.forward_damage_done && duration_percentage > 0.25 {
                    sys_vars.area_hp_mod_requests.push(AreaAttackComponent {
                        area_shape: Box::new(ncollide2d::shape::Cuboid::new(
                            v2(
                                self.configs.attributes.width.unwrap_or(1.0),
                                self.configs.attributes.casting_range,
                            ) / 2.0,
                        )),
                        area_isom: Isometry2::new(self.center, self.rot_radian),
                        source_entity_id: self.caster_entity_id,
                        typ: HpModificationType::BasicDamage(
                            self.configs.first_damage,
                            DamageDisplayType::SingleNumber,
                            WeaponType::Sword,
                        ),
                        except: None,
                    });
                    self.forward_damage_done = true;
                } else if !self.backward_damage_done && duration_percentage > 0.75 {
                    sys_vars.area_hp_mod_requests.push(AreaAttackComponent {
                        area_shape: Box::new(ncollide2d::shape::Cuboid::new(
                            v2(
                                self.configs.attributes.width.unwrap_or(1.0),
                                self.configs.attributes.casting_range,
                            ) / 2.0,
                        )),
                        area_isom: Isometry2::new(self.center, self.rot_radian),
                        source_entity_id: self.caster_entity_id,
                        typ: HpModificationType::BasicDamage(
                            self.configs.second_damage,
                            DamageDisplayType::SingleNumber,
                            WeaponType::Sword,
                        ),
                        except: None,
                    });
                    self.backward_damage_done = true;
                }
                StatusUpdateResult::KeepIt
            }
        } else {
            StatusUpdateResult::RemoveIt
        }
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        let duration_percentage = sys_vars
            .time
            .percentage_between(self.started_at, self.ends_at);
        match char_state.outlook {
            CharOutlook::Player {
                job_sprite_id,
                head_index,
                sex,
            } => {
                let body_sprite = {
                    let sprites = &sys_vars.assets.sprites.character_sprites;
                    &sprites[&job_sprite_id][1][sex as usize]
                };
                let head_res = {
                    let sprites = &sys_vars.assets.sprites.head_sprites;
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
                            forced_duration: Some(ElapsedTime(self.half_duration)),
                            direction: char_state.dir(),
                            fps_multiplier: 1.0,
                        }
                    } else {
                        SpriteRenderDescriptorComponent {
                            action_index: CharActionIndex::Attacking1 as usize,
                            animation_started: self
                                .started_at
                                .add_seconds(self.half_duration + *time_offset),
                            animation_ends_at: ElapsedTime(0.0),
                            forced_duration: Some(ElapsedTime(self.half_duration)),
                            direction: (char_state.dir() + 4) % 8,
                            fps_multiplier: 1.0,
                        }
                    };
                    let offset = render_single_layer_action(
                        sys_vars.time,
                        &anim_descr,
                        body_sprite,
                        &v2_to_v3(pos),
                        [0, 0],
                        true,
                        1.0,
                        ActionPlayMode::Repeat,
                        &[255, 255, 0, *alpha],
                        render_commands,
                    );

                    render_single_layer_action(
                        sys_vars.time,
                        &anim_descr,
                        head_res,
                        &v2_to_v3(pos),
                        offset,
                        false,
                        1.0,
                        ActionPlayMode::Repeat,
                        &[255, 255, 0, *alpha],
                        render_commands,
                    );
                }
            }
            CharOutlook::Monster(_monster_id) => {}
        }
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
