use nalgebra::{Isometry2, Vector3};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::status::status::{ApplyStatusInAreaComponent, StatusEnum};
use crate::components::status::stun::StunStatus;
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_single_layer_action;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{AssetResources, SystemVariables};
use rustarok_common::attack::{AreaAttackComponent, DamageDisplayType, HpModificationType};
use rustarok_common::common::{v2_to_v3, v3_to_v2, EngineTime, LocalTime};
use rustarok_common::common::{v3, Vec2};
use rustarok_common::components::char::{
    CharDir, LocalCharEntityId, StaticCharDataComponent, StatusNature,
};
use rustarok_common::config::{CommonConfigs, GazXplodiumChargeSkillConfigInner};
use specs::world::WorldExt;
use specs::ReadStorage;
use vek::QuadraticBezier3;

pub struct GazXplodiumChargeSkill;

pub const GAZ_XPLODIUM_CHARGE_SKILL: &'static GazXplodiumChargeSkill = &GazXplodiumChargeSkill;

impl SkillDef for GazXplodiumChargeSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\ra_detonator.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        Some(Box::new(GazXplodiumChargeSkillManifestation::new(
            params.caster_entity_id,
            params.caster_pos,
            params.skill_pos.unwrap(),
            &mut ecs_world.write_resource::<PhysicEngine>(),
            ecs_world.read_resource::<EngineTime>().now(),
            ecs_world
                .read_resource::<CommonConfigs>()
                .skills
                .gaz_xplodium_charge
                .inner
                .clone(),
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }
}

struct GazXplodiumChargeSkillManifestation {
    end_pos: Vec2,
    current_pos: Vector3<f32>,
    current_target_pos: Vector3<f32>,
    caster_id: LocalCharEntityId,
    started_at: LocalTime,
    configs: GazXplodiumChargeSkillConfigInner,
    bezier: QuadraticBezier3<f32>,
}

impl GazXplodiumChargeSkillManifestation {
    fn new(
        caster_id: LocalCharEntityId,
        start_pos: Vec2,
        end_pos: Vec2,
        _physics_world: &mut PhysicEngine,
        now: LocalTime,
        configs: GazXplodiumChargeSkillConfigInner,
    ) -> GazXplodiumChargeSkillManifestation {
        let ctrl = v2_to_v3(&(start_pos - (end_pos - start_pos))) + v3(0.0, 20.0, 0.0);
        GazXplodiumChargeSkillManifestation {
            end_pos,
            current_pos: Vector3::new(start_pos.x, 1.0, start_pos.y),
            started_at: now,
            caster_id,
            current_target_pos: v2_to_v3(&end_pos),
            configs,
            bezier: QuadraticBezier3 {
                start: vek::Vec3::new(start_pos.x, 0.0, start_pos.y),
                ctrl: vek::Vec3::new(ctrl.x, ctrl.y, ctrl.z),
                end: vek::Vec3::new(end_pos.x, 0.0, end_pos.y),
            },
        }
    }
}

impl SkillManifestation for GazXplodiumChargeSkillManifestation {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let travel_duration_percentage = params.time().now().percentage_between(
            self.started_at,
            self.started_at
                .add_seconds(self.configs.missile_travel_duration_seconds),
        );
        if travel_duration_percentage < 1.0 {
            let pos = self.bezier.evaluate(travel_duration_percentage);
            self.current_pos = v3(pos.x, pos.y, pos.z);
        } else {
            let end_time = self
                .started_at
                .add_seconds(self.configs.missile_travel_duration_seconds)
                .add_seconds(self.configs.detonation_duration);
            if end_time.has_already_passed(params.time().now()) {
                if let Some(caster_team) = params
                    .static_char_data_storage
                    .get(self.caster_id.into())
                    .map(|caster| caster.team)
                {
                    let area_shape =
                        Box::new(ncollide2d::shape::Ball::new(self.configs.explosion_area));
                    let area_isom = Isometry2::new(self.end_pos, 0.0);
                    params.add_area_hp_mod_request(AreaAttackComponent {
                        // TODO2
                        //                        area_shape: area_shape.clone(),
                        //                        area_isom: area_isom.clone(),
                        source_entity_id: self.caster_id,
                        typ: HpModificationType::SpellDamage(
                            self.configs.damage,
                            DamageDisplayType::SingleNumber,
                        ),
                        except: None,
                    });
                    params.apply_area_status(ApplyStatusInAreaComponent {
                        source_entity_id: self.caster_id,
                        status: StatusEnum::StunStatus(StunStatus::new(
                            self.caster_id,
                            params.time().now(),
                            self.configs.stun_duration_seconds,
                        )),
                        area_shape,
                        area_isom,
                        except: None,
                        nature: StatusNature::Harmful,
                        caster_team,
                    });
                    params.create_entity_with_comp(StrEffectComponent {
                        effect_id: StrEffectType::Explosion.into(),
                        pos: self.end_pos,
                        start_time: params.time().now(),
                        die_at: None,
                        play_mode: ActionPlayMode::Once,
                    });
                }
                params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
            }
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        now: LocalTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        let missile_landed = self
            .started_at
            .add_seconds(self.configs.missile_travel_duration_seconds)
            .has_already_passed(now);
        let dir = CharDir::determine_dir(
            &v3_to_v2(&self.current_target_pos),
            &v3_to_v2(&self.current_pos),
        );
        let anim = SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: self
                .started_at
                .add_seconds(self.configs.missile_travel_duration_seconds),
            animation_ends_at: LocalTime::from(0.0),
            forced_duration: None,
            direction: dir,
            fps_multiplier: 1.0,
        };
        render_single_layer_action(
            now,
            &anim,
            &assets.sprites.ginseng_bullet,
            &self.current_pos,
            [0, 0],
            false,
            1.0,
            if missile_landed {
                ActionPlayMode::PlayThenHold
            } else {
                ActionPlayMode::FixFrame(0)
            },
            &[255, 255, 255, 255],
            render_commands,
        );
        if missile_landed {
            let detonation_duration_perc = now.percentage_between(
                self.started_at
                    .add_seconds(self.configs.missile_travel_duration_seconds),
                self.started_at
                    .add_seconds(self.configs.missile_travel_duration_seconds)
                    .add_seconds(self.configs.detonation_duration),
            );
            let number = 4 - (detonation_duration_perc / 0.25) as usize;
            // render countdown number
            let anim = SpriteRenderDescriptorComponent {
                action_index: CharActionIndex::Idle as usize,
                animation_started: LocalTime::from(0.0),
                animation_ends_at: LocalTime::from(0.0),
                forced_duration: None,
                direction: CharDir::from(number),
                fps_multiplier: 1.0,
            };
            render_single_layer_action(
                now,
                &anim,
                &assets.sprites.timefont,
                &Vector3::new(self.end_pos.x, 2.0, self.end_pos.y),
                [0, 0],
                false,
                0.5,
                ActionPlayMode::FixFrame(0),
                &[255, 255, 255, 255],
                render_commands,
            );

            // render clock
            render_commands
                .sprite_3d()
                .pos_2d(&self.end_pos)
                .y(1.0)
                .add(assets.sprites.clock);

            // render area
            render_commands
                .horizontal_texture_3d()
                .pos(&self.end_pos)
                .rotation_rad(now.as_millis() as f32 % 6.28)
                .fix_size(self.configs.explosion_area * 2.0)
                .add(assets.sprites.magic_target);
        }
    }
}
