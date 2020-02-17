use nalgebra::Isometry2;
use specs::ReadStorage;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::{
    ActionPlayMode, CastingSkillData, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::status::status::{ApplyStatusComponent, StatusEnum};
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::{render_action, RenderDesktopClientSystem, COLOR_WHITE};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{AssetResources, SystemVariables};
use crate::LocalTime;
use rustarok_common::common::{v2, EngineTime, Vec2};
use rustarok_common::components::char::{
    CharDir, LocalCharEntityId, LocalCharStateComp, StaticCharDataComponent,
};
use rustarok_common::config::{CommonConfigs, SkillConfigPyroBlastInner};
use specs::world::WorldExt;

pub struct WizPyroBlastSkill;

pub const WIZ_PYRO_BLAST_SKILL: &'static WizPyroBlastSkill = &WizPyroBlastSkill;

impl SkillDef for WizPyroBlastSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\ht_blastmine.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        let configs = ecs_world
            .read_resource::<CommonConfigs>()
            .skills
            .wiz_pyroblast
            .inner
            .clone();

        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_status(
                params.caster_entity_id,
                params.target_entity.unwrap(),
                StatusEnum::PyroBlastTargetStatus(PyroBlastTargetStatus {
                    caster_entity_id: params.caster_entity_id,
                    splash_radius: configs.splash_radius,
                }),
            ));
        Some(Box::new(PyroBlastManifest::new(
            params.caster_entity_id,
            params.caster_pos,
            params.target_entity.unwrap(),
            ecs_world.read_resource::<EngineTime>().now(),
            &mut ecs_world.write_resource::<PhysicEngine>(),
            configs,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }

    fn render_casting(
        &self,
        char_pos: &Vec2,
        casting_state: &CastingSkillData,
        assets: &AssetResources,
        time: &EngineTime,
        dev_configs: &CommonConfigs,
        render_commands: &mut RenderCommandCollector,
        char_storage: &ReadStorage<LocalCharStateComp>,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Moonstar,
            casting_state.cast_started,
            char_pos,
            assets,
            time.now(),
            render_commands,
            ActionPlayMode::Repeat,
        );
        let casting_percentage = time
            .now()
            .percentage_between(casting_state.cast_started, casting_state.cast_ends);

        if let Some(target_char) = char_storage.get(casting_state.target_entity.unwrap().into()) {
            render_commands
                .horizontal_texture_3d()
                .pos(&target_char.pos())
                .rotation_rad(3.14 * casting_percentage)
                .fix_size(
                    (dev_configs.skills.wiz_pyroblast.inner.splash_radius
                        * 2.0
                        * casting_percentage)
                        .max(0.5),
                )
                .add(assets.sprites.magic_target)
        }
        let anim_descr = SpriteRenderDescriptorComponent {
            action_index: 16,
            animation_started: casting_state.cast_started,
            animation_ends_at: LocalTime::from(0.0),
            forced_duration: Some(dev_configs.skills.wiz_pyroblast.attributes.casting_time),
            direction: CharDir::South,
            fps_multiplier: 1.0,
        };
        render_action(
            time.now(),
            &anim_descr,
            &assets.sprites.effect_sprites.plasma,
            &(char_pos + casting_state.char_to_skill_dir_when_casted),
            [0, 0],
            false,
            dev_configs.skills.wiz_pyroblast.inner.ball_size * casting_percentage,
            ActionPlayMode::Reverse,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

pub struct PyroBlastManifest {
    pub caster_entity_id: LocalCharEntityId,
    pub pos: Vec2,
    pub target_last_pos: Vec2,
    pub target_entity_id: LocalCharEntityId,
    pub created_at: LocalTime,
    pub configs: SkillConfigPyroBlastInner,
}

impl PyroBlastManifest {
    pub fn new(
        caster_entity_id: LocalCharEntityId,
        pos: Vec2,
        target_entity_id: LocalCharEntityId,
        created_at: LocalTime,
        _physics_world: &mut PhysicEngine,
        configs: SkillConfigPyroBlastInner,
    ) -> PyroBlastManifest {
        PyroBlastManifest {
            caster_entity_id,
            pos,
            target_last_pos: v2(0.0, 0.0),
            target_entity_id,
            created_at,
            configs,
        }
    }
}

impl SkillManifestation for PyroBlastManifest {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        // TODO2
        //        let (target_pos, collide) = if let Some(target_char) =
        //            params.char_storage.get_mut(self.target_entity_id.into())
        //        {
        //            let target_pos = target_char.pos();
        //            let dir_vector = target_pos - self.pos;
        //            let distance = dir_vector.magnitude();
        //            if distance > 2.0 {
        //                let dir_vector = dir_vector.normalize();
        //                self.pos = self.pos + (dir_vector * params.time().dt() * self.configs.moving_speed);
        //                (target_pos, false)
        //            } else {
        //                target_char.statuses.remove_if(|status| {
        //                    if let StatusEnum::PyroBlastTargetStatus(status) = status {
        //                        status.caster_entity_id == self.caster_entity_id
        //                    } else {
        //                        false
        //                    }
        //                });
        //                (target_pos, true)
        //            }
        //        } else {
        //            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        //            (v2(0.0, 0.0), false)
        //        };
        //        if collide {
        //            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        //            params.add_hp_mod_request(HpModificationRequest {
        //                src_entity: self.caster_entity_id,
        //                dst_entity: self.target_entity_id,
        //                typ: HpModificationType::SpellDamage(
        //                    self.configs.damage,
        //                    DamageDisplayType::SingleNumber,
        //                ),
        //            });
        //            let area_shape = Box::new(ncollide2d::shape::Ball::new(self.configs.splash_radius));
        //            let area_isom = Isometry2::new(target_pos, 0.0);
        //            params.add_area_hp_mod_request(AreaAttackComponent {
        //                area_shape,
        //                area_isom,
        //                source_entity_id: self.caster_entity_id,
        //                typ: HpModificationType::SpellDamage(
        //                    self.configs.secondary_damage,
        //                    DamageDisplayType::SingleNumber,
        //                ),
        //                except: Some(self.target_entity_id),
        //            });
        //            params.create_entity_with_comp(StrEffectComponent {
        //                effect_id: StrEffectType::Explosion.into(),
        //                pos: target_pos,
        //                start_time: params.time().now(),
        //                die_at: None,
        //                play_mode: ActionPlayMode::Once,
        //            });
        //        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        now: LocalTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        let anim_descr = SpriteRenderDescriptorComponent {
            action_index: 0,
            animation_started: LocalTime::from(0.0),
            animation_ends_at: LocalTime::from(0.0),
            forced_duration: None,
            direction: CharDir::South,
            fps_multiplier: 1.0,
        };
        render_action(
            now,
            &anim_descr,
            &assets.sprites.effect_sprites.plasma,
            &self.pos,
            [0, 0],
            false,
            self.configs.ball_size,
            ActionPlayMode::Repeat,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

#[derive(Clone, Debug)]
pub struct PyroBlastTargetStatus {
    pub caster_entity_id: LocalCharEntityId,
    pub splash_radius: f32,
}

impl PyroBlastTargetStatus {
    pub fn render(
        &self,
        char_pos: Vec2,
        now: LocalTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        render_commands
            .horizontal_texture_3d()
            .pos(&char_pos)
            .rotation_rad(now.as_millis() as f32 % 6.28)
            .fix_size(self.splash_radius * 2.0)
            .add(assets.sprites.magic_target);
    }
}
