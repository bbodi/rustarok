use nalgebra::{Isometry2, Vector2};

use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, StatusEnum, StatusUpdateParams, StatusUpdateResult,
};
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_single_layer_action;
use crate::systems::{AssetResources, SystemVariables};
use rustarok_common::attack::{
    AreaAttackComponent, DamageDisplayType, HpModificationType, WeaponType,
};
use rustarok_common::common::{v2, v2_to_v3, EngineTime, LocalTime, Vec2};
use rustarok_common::components::char::{
    CharDir, CharOutlook, LocalCharEntityId, LocalCharStateComp, StaticCharDataComponent,
};
use rustarok_common::config::{AssaBladeDashSkillConfig, CommonConfigs};
use specs::world::WorldExt;

pub struct AssaBladeDashSkill;

pub const ASSA_BLADE_DASH_SKILL: &'static AssaBladeDashSkill = &AssaBladeDashSkill;

impl SkillDef for AssaBladeDashSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_incagi.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .write_storage::<LocalCharStateComp>()
            .get_mut(params.caster_entity_id.into())
        {
            let angle = params.char_to_skill_dir.angle(&Vector2::y());
            let angle = if params.char_to_skill_dir.x > 0.0 {
                angle
            } else {
                -angle
            };

            let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
            let configs = ecs_world
                .read_resource::<CommonConfigs>()
                .skills
                .assa_blade_dash
                .clone();
            let now = ecs_world.read_resource::<EngineTime>().now();
            sys_vars
                .apply_statuses
                .push(ApplyStatusComponent::from_status(
                    params.caster_entity_id,
                    params.caster_entity_id,
                    StatusEnum::AssaBladeDashStatus(AssaBladeDashStatus {
                        caster_entity_id: params.caster_entity_id,
                        started_at: now,
                        ends_at: now.add_seconds(configs.duration_seconds),
                        start_pos: caster.pos(),
                        center: caster.pos()
                            + params.char_to_skill_dir * (configs.attributes.casting_range / 2.0),
                        rot_radian: angle,
                        vector: params.char_to_skill_dir * configs.attributes.casting_range,
                        shadow1_pos: Vector2::zeros(),
                        shadow2_pos: Vector2::zeros(),
                        forward_damage_done: false,
                        backward_damage_done: false,
                        half_duration: (configs.duration_seconds * 1000f32) as u32 / 2,
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

#[derive(Clone, Debug)]
pub struct AssaBladeDashStatus {
    pub caster_entity_id: LocalCharEntityId,
    pub started_at: LocalTime,
    pub ends_at: LocalTime,
    pub start_pos: Vec2,
    pub center: Vec2,
    pub rot_radian: f32,
    pub half_duration: u32,
    pub vector: Vec2,
    pub shadow1_pos: Vec2,
    pub shadow2_pos: Vec2,
    pub forward_damage_done: bool,
    pub backward_damage_done: bool,
    pub configs: AssaBladeDashSkillConfig,
}

impl AssaBladeDashStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        // TODO2 physics
        //        if let Some(body) = params
        //            .physics_world
        //            .bodies
        //            .rigid_body_mut(params.target_char.body_handle)
        //        {
        //            if self.ends_at.has_already_passed(params.time.now()) {
        //                params.target_char.set_collidable(params.physics_world);
        //                StatusUpdateResult::RemoveIt
        //            } else {
        //                let duration_percentage = params
        //                    .time
        //                    .now()
        //                    .percentage_between(self.started_at, self.ends_at);
        //                let pos = if duration_percentage < 0.5 {
        //                    let forward_perc = duration_percentage * 2.0;
        //                    self.shadow1_pos = self.start_pos + self.vector * (forward_perc - 0.1).max(0.0);
        //                    self.shadow2_pos = self.start_pos + self.vector * (forward_perc - 0.2).max(0.0);
        //                    self.start_pos + self.vector * forward_perc
        //                } else {
        //                    let backward_perc = (1.0 - duration_percentage) * 2.0;
        //                    self.shadow1_pos =
        //                        self.start_pos + self.vector * (backward_perc + 0.1).min(1.0);
        //                    self.shadow2_pos =
        //                        self.start_pos + self.vector * (backward_perc + 0.2).min(1.0);
        //                    self.start_pos + self.vector * backward_perc
        //                };
        //                body.set_position(Isometry2::translation(pos.x, pos.y));
        //
        //                if !self.forward_damage_done && duration_percentage > 0.25 {
        //                    params.area_hp_mod_requests.push(AreaAttackComponent {
        //                        // TODO2
        //                        //                            area_shape: Box::new(ncollide2d::shape::Cuboid::new(
        //                        //                                v2(
        //                        //                                    self.configs.attributes.width.unwrap_or(1.0),
        //                        //                                    self.configs.attributes.casting_range,
        //                        //                                ) / 2.0,
        //                        //                            )),
        //                        //                            area_isom: Isometry2::new(self.center, self.rot_radian),
        //                        source_entity_id: self.caster_entity_id,
        //                        typ: HpModificationType::BasicDamage(
        //                            self.configs.first_damage,
        //                            DamageDisplayType::SingleNumber,
        //                            WeaponType::Sword,
        //                        ),
        //                        except: None,
        //                    });
        //                    self.forward_damage_done = true;
        //                } else if !self.backward_damage_done && duration_percentage > 0.75 {
        //                    params.area_hp_mod_requests.push(AreaAttackComponent {
        //                        // TODO2
        //                        //                            area_shape: Box::new(ncollide2d::shape::Cuboid::new(
        //                        //                                v2(
        //                        //                                    self.configs.attributes.width.unwrap_or(1.0),
        //                        //                                    self.configs.attributes.casting_range,
        //                        //                                ) / 2.0,
        //                        //                            )),
        //                        //                            area_isom: Isometry2::new(self.center, self.rot_radian),
        //                        source_entity_id: self.caster_entity_id,
        //                        typ: HpModificationType::BasicDamage(
        //                            self.configs.second_damage,
        //                            DamageDisplayType::SingleNumber,
        //                            WeaponType::Sword,
        //                        ),
        //                        except: None,
        //                    });
        //                    self.backward_damage_done = true;
        //                }
        //                StatusUpdateResult::KeepIt
        //            }
        //        } else {
        //            StatusUpdateResult::RemoveIt
        //        }
        StatusUpdateResult::RemoveIt
    }

    pub fn render(
        &self,
        static_data: &StaticCharDataComponent,
        auth_state: &LocalCharStateComp,
        now: LocalTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        let duration_percentage = now.percentage_between(self.started_at, self.ends_at);
        match static_data.outlook {
            CharOutlook::Human {
                job_sprite_id,
                head_index,
                sex,
            } => {
                let body_sprite = {
                    let sprites = &assets.sprites.character_sprites;
                    &sprites[&job_sprite_id][1][sex as usize]
                };
                let head_res = {
                    let sprites = &assets.sprites.head_sprites;
                    &sprites[sex as usize][head_index]
                };
                for (pos, alpha, time_offset) in &[
                    (auth_state.pos(), 255, 0),
                    (self.shadow1_pos, 175, 50),
                    (self.shadow2_pos, 100, 100),
                ] {
                    let anim_descr = if duration_percentage < 0.5 {
                        SpriteRenderDescriptorComponent {
                            action_index: CharActionIndex::Attacking1 as usize,
                            animation_started: self.started_at.add_millis(*time_offset),
                            animation_ends_at: LocalTime::from(0.0),
                            forced_duration: Some(LocalTime::from(self.half_duration)),
                            direction: auth_state.dir(),
                            fps_multiplier: 1.0,
                        }
                    } else {
                        SpriteRenderDescriptorComponent {
                            action_index: CharActionIndex::Attacking1 as usize,
                            animation_started: self
                                .started_at
                                .add_millis(self.half_duration + *time_offset),
                            animation_ends_at: LocalTime::from(0.0),
                            forced_duration: Some(LocalTime::from(self.half_duration)),
                            direction: CharDir::from((auth_state.dir().as_usize() + 4) % 8),
                            fps_multiplier: 1.0,
                        }
                    };
                    let offset = render_single_layer_action(
                        now,
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
                        now,
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
}
